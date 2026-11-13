//! The trace pass's readback (#31 split it out of trace.rs): march one
//! frame on the GPU and bring it back to the CPU as the sim's `Preview`,
//! the parity tests' entry. Native only - it waits on the device for the
//! mapped buffer, which a browser's single thread cannot; the viewport
//! never reads a frame back (the composite samples it on the GPU).
//! Provenance: our own code, moved unchanged from trace.rs (M7.2 B),
//! specced from the wgpu buffer-mapping docs.

use egui_wgpu::wgpu;

use camcraft_app::stock_sim::preview::{Preview, PreviewCamera};

use super::{TracePass, OUT_STRIDE};

impl TracePass {
    /// March a frame and read it back as the sim's `Preview` - the
    /// parity tests' entry and the CPU-comparable answer. NaN depth
    /// and class 3 wherever the kernel reported no hit; `unresolved`
    /// counts its budget bails, the CPU law exactly.
    pub fn trace(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cam: &PreviewCamera,
        w: u32,
        h: u32,
        tol: f64,
    ) -> Result<Preview, String> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("trace-enc"),
        });
        let out = self.encode(device, queue, &mut enc, cam, w, h, tol)?;
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("trace-read"),
            size: out.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        enc.copy_buffer_to_buffer(&out, 0, &read, 0, out.size());
        queue.submit([enc.finish()]);
        if let Some(err) = pollster::block_on(scope.pop()) {
            return Err(format!("trace pass: the frame failed validation ({err})"));
        }
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        if device.poll(wgpu::PollType::wait_indefinitely()).is_err() {
            return Err("trace pass: the device was lost while marching".into());
        }
        if !matches!(rx.recv(), Ok(Ok(()))) {
            return Err("trace pass: the frame could not be read back".into());
        }
        let raw = read
            .slice(..)
            .get_mapped_range()
            .map_err(|e| format!("trace pass: readback map failed ({e})"))?;
        let frame = decode(&raw, w, h);
        drop(raw);
        read.unmap();
        Ok(frame)
    }
}

/// The kernel's per-pixel vec4s as the sim's frame type.
fn decode(raw: &[u8], w: u32, h: u32) -> Preview {
    let n = (w as usize) * (h as usize);
    let mut pv = Preview {
        w,
        h,
        depth: vec![f64::NAN; n],
        class: vec![3.0; n],
        shade: vec![0.0; n],
        unresolved: 0,
    };
    let word = |o: usize| {
        raw.get(o..o + 4)
            .and_then(|b| b.try_into().ok())
            .map_or(f32::NAN, f32::from_le_bytes)
    };
    for i in 0..n {
        let o = i * OUT_STRIDE as usize;
        let status = word(o + 12);
        if status == 1.0 {
            pv.depth[i] = f64::from(word(o));
            pv.class[i] = word(o + 4);
            pv.shade[i] = word(o + 8);
        } else if status == 2.0 {
            pv.unresolved += 1;
        }
    }
    pv
}
