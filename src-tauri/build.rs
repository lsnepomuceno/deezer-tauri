use std::env;
use std::path::Path;

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&["report_media"]))
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest()),
    )
    .expect("failed to run tauri-build");
    embed_windows_manifest();
}

/// tauri-build embeds the Windows manifest (Common Controls v6) in the app binary only,
/// so test binaries that link Tauri fail to start with STATUS_ENTRYPOINT_NOT_FOUND.
/// Through the linker it reaches every target: the app, the tests and the examples.
fn embed_windows_manifest() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os != "windows" || target_env != "msvc" {
        return;
    }
    let manifest = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("set by Cargo"))
        .join("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed=windows-app-manifest.xml");
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
}
