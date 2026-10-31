//! Hiding the daemon's console window on Windows.
//!
//! The binary is a console app so the control subcommands (`pause`,
//! `resume`, `status`, `switch`) still print and return exit codes at a
//! prompt. The daemon has no caller to read that output, so it hides the
//! window the console allocates for it — the overlay is its only window.

/// Hide the process's console window, if it has one. A no-op when there
/// is none (e.g. launched with stdio pipes instead of a console).
#[cfg(target_os = "windows")]
pub fn hide_window() {
    use core::ffi::{c_int, c_void};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetConsoleWindow() -> *mut c_void;
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn ShowWindow(hwnd: *mut c_void, cmd_show: c_int) -> c_int;
    }

    const SW_HIDE: c_int = 0;
    unsafe {
        let window = GetConsoleWindow();
        if !window.is_null() {
            ShowWindow(window, SW_HIDE);
        }
    }
}
