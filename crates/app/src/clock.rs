//! The monotonic-clock seam (#31, M13.3): code that times itself reads
//! `camcraft_app::clock::Instant`, never `std::time::Instant` directly.
//! Native builds get std's own type (the alias costs nothing). On
//! wasm32-unknown-unknown std's `Instant::now()` panics - there is no
//! clock syscall - so there the alias is `web_time::Instant`, which reads
//! the browser's `performance.now()` behind the same API.
//! Provenance: our own seam; the web-time crate's documented drop-in API.

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;
#[cfg(target_arch = "wasm32")]
pub use web_time::Instant;

#[cfg(test)]
mod tests {
    use super::Instant;
    use std::time::Duration;

    #[test]
    fn clock_seam_is_monotonic_and_does_duration_math() {
        let a = Instant::now();
        let b = Instant::now();
        assert!(b >= a);
        let later = a + Duration::from_millis(5);
        assert_eq!(later.duration_since(a), Duration::from_millis(5));
        assert!(crate::logging::elapsed_ms(a) < 60_000);
    }
}
