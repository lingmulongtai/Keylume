fn main() {
    if std::env::var_os("CARGO_FEATURE_DESKTOP").is_some() {
        #[cfg(target_os = "macos")]
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
            for source in ["src/desktop/mac_controls.m", "src/desktop/mac_system.m"] {
                println!("cargo:rerun-if-changed={source}");
            }
            cc::Build::new()
                .files(["src/desktop/mac_controls.m", "src/desktop/mac_system.m"])
                .flag("-fobjc-arc")
                .compile("keylume_macos");
            for framework in ["AppKit", "ApplicationServices", "CoreAudio", "AudioToolbox"] {
                println!("cargo:rustc-link-lib=framework={framework}");
            }
        }
        tauri_build::build()
    }
}
