//! The UI crate's one diagnostics seam (#31): a line for whoever launched
//! the app. Natively that is the terminal (stderr for warnings, stdout for
//! plain notes), exactly where `eprintln!` / `println!` put it before. In a
//! browser std has no stderr or stdout - a line printed there is dropped -
//! so on wasm32 the same line goes to the page's console instead. Shared
//! code says `diag_warn!` / `diag_info!` and never prints directly; the
//! law test below keeps it that way ("Log diagnostics, never silence").
//! Provenance: our own code, specced from the std `eprintln!` docs and the
//! web-sys `console` API.

/// A warning or refusal: stderr natively, `console.warn` in a browser.
pub fn warn(line: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{line}");
    #[cfg(target_arch = "wasm32")]
    eframe::web_sys::console::warn_1(&line.into());
}

/// A plain note: stdout natively, `console.log` in a browser.
pub fn info(line: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    println!("{line}");
    #[cfg(target_arch = "wasm32")]
    eframe::web_sys::console::log_1(&line.into());
}

/// `eprintln!`'s shape over [`warn`].
macro_rules! diag_warn {
    ($($arg:tt)*) => { $crate::diag::warn(&format!($($arg)*)) };
}

/// `println!`'s shape over [`info`].
macro_rules! diag_info {
    ($($arg:tt)*) => { $crate::diag::info(&format!($($arg)*)) };
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    /// Files that may print directly: this seam, code that is never
    /// compiled for wasm32 (the native window, its GPU bring-up, the
    /// headless proof and the test-device helper), and test files.
    const NATIVE_OR_TEST: &[&str] = &[
        "diag.rs",
        "main.rs",
        "egui_app/shell_window.rs",
        "egui_app/shell_window/frame.rs",
        "egui_app/gpu_init.rs",
        "egui_app/os_quit.rs",
        "egui_app/frame_watch.rs",
        // gpu.rs prints only inside its #[cfg(test)] module; shell.rs only
        // inside the native-only `headless` proof; headless_gpu.rs only in
        // the native-only `device_for_test`.
        "egui_app/gpu.rs",
        "egui_app/shell.rs",
        "headless_gpu.rs",
    ];

    const DIRECT_PRINTS: &[&str] = &["eprintln!(", "println!(", "eprint!(", "print!(", "dbg!("];

    fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    #[test]
    fn shared_ui_code_prints_only_through_the_diag_seam() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files(&src, &mut files);
        assert!(files.len() > 50, "the source walk found too few files to mean anything");
        let mut offenders = Vec::new();
        for path in files {
            let rel = path.strip_prefix(&src).unwrap_or(&path).to_string_lossy().replace('\\', "/");
            let is_test = rel.ends_with("tests.rs") || rel.contains("/tests/") || rel.ends_with("test_harness.rs");
            if is_test || NATIVE_OR_TEST.contains(&rel.as_str()) {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            for (n, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                if DIRECT_PRINTS.iter().any(|m| code.contains(m)) {
                    offenders.push(format!("{rel}:{}", n + 1));
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "these lines print directly, which a browser drops - use diag_warn!/diag_info!: {offenders:?}"
        );
    }

    #[test]
    fn the_macros_take_println_shaped_arguments() {
        let n = 3;
        diag_warn!("diag test: a warning with {n} and {}", "an argument");
        diag_info!("diag test: a note");
    }
}
