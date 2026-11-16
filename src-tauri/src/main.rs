// no console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
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
    ngm_desktop_lib::run()
}
