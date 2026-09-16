#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    #[cfg(feature = "desktop")]
    keylume_lib::desktop::run();
}
