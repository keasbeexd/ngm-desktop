// no console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Linux release builds started from a terminal: start again on our own,
/// in a new session with no terminal, and leave. Closing the terminal would
/// otherwise end the AppImage's helper (which holds the app's files) and
/// blank the window. Never from the app launcher or login (no terminal).
#[cfg(all(target_os = "linux", not(debug_assertions)))]
fn detach_from_terminal() {
    use std::os::unix::process::CommandExt;
    extern "C" {
        fn isatty(fd: i32) -> i32;
        fn setsid() -> i32;
    }
    // SAFETY: plain libc calls with no pointers
    let on_terminal = unsafe { isatty(0) == 1 || isatty(1) == 1 || isatty(2) == 1 };
    if !on_terminal || std::env::var_os("NGM_DESKTOP_DETACHED").is_some() {
        return;
    }
    let program = std::env::var_os("APPIMAGE")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_exe().ok());
    let Some(program) = program else { return };
    let mut cmd = std::process::Command::new(program);
    cmd.args(std::env::args_os().skip(1))
        .env("NGM_DESKTOP_DETACHED", "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    // SAFETY: setsid only, between fork and exec (async-signal-safe)
    unsafe {
        cmd.pre_exec(|| {
            setsid();
            Ok(())
        });
    }
    if cmd.spawn().is_ok() {
        std::process::exit(0);
    }
}

fn main() {
    #[cfg(all(target_os = "linux", not(debug_assertions)))]
    detach_from_terminal();
    // Linux: WebKitGTK's DMA-BUF renderer crashes the window on some setups
    // (NVIDIA under Wayland: "Error 71 (Protocol error)"). A window this small
    // doesn't need it. Set before any thread starts; a player's own setting wins.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    // Linux: the name trays show on hover ("NGM Desktop", not the program's
    // file name). GLib is already part of the app there (GTK).
    #[cfg(target_os = "linux")]
    {
        extern "C" {
            fn g_set_application_name(name: *const std::ffi::c_char);
        }
        // SAFETY: a static, NUL-terminated string; called once, before GTK starts
        unsafe { g_set_application_name(c"NGM Desktop".as_ptr()) };
    }
    // Linux: keep running when the terminal that started it closes (a tray
    // app shouldn't quit on the terminal's "hang up" signal).
    #[cfg(target_os = "linux")]
    {
        extern "C" {
            fn signal(signum: i32, handler: usize) -> usize;
        }
        const SIGHUP: i32 = 1;
        const SIG_IGN: usize = 1;
        // SAFETY: ignoring a signal before any thread starts
        unsafe { signal(SIGHUP, SIG_IGN) };
    }
    ngm_desktop_lib::run()
}
