fn main() {
    configure_linux_runtime();
    forgetag_lib::run();
}

#[cfg(target_os = "linux")]
fn configure_linux_runtime() {
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    if std::env::var_os("GDK_BACKEND").is_none() && std::env::var_os("DISPLAY").is_some() {
        std::env::set_var("GDK_BACKEND", "x11");
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_linux_runtime() {}
