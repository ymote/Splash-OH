use std::env;
use std::path::PathBuf;

/// Compile the ArkUI shim against the OpenHarmony SDK headers.
///
/// The sysroot comes from OHOS_BASE_SDK_HOME (what the DevEco/命令行 toolchain
/// sets) or OHOS_SDK_NATIVE. We only ever build this for the ohos target — on a
/// host build there is no ArkUI to link against, so the shim is skipped and the
/// crate still compiles for tests/tooling.
fn main() {
    println!("cargo:rerun-if-changed=src/arkui/shim.cpp");
    println!("cargo:rerun-if-env-changed=OHOS_BASE_SDK_HOME");
    println!("cargo:rerun-if-env-changed=OHOS_SDK_NATIVE");

    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("ohos") {
        println!(
            "cargo:warning=target `{target}` is not OpenHarmony — skipping the ArkUI shim. \
             Build with --target aarch64-unknown-linux-ohos for a real device library."
        );
        return;
    }

    let native = env::var("OHOS_SDK_NATIVE")
        .map(PathBuf::from)
        .or_else(|_| env::var("OHOS_BASE_SDK_HOME").map(|s| PathBuf::from(s).join("native")))
        .expect("set OHOS_SDK_NATIVE or OHOS_BASE_SDK_HOME so the ArkUI headers can be found");

    let include = native.join("sysroot/usr/include");
    assert!(
        include.join("arkui/native_node.h").is_file(),
        "ArkUI NDK headers not found under {} — is this a full OpenHarmony native SDK?",
        include.display()
    );

    cc::Build::new()
        .file("src/arkui/shim.cpp")
        .cpp(true)
        .include(&include)
        .flag_if_supported("-std=c++17")
        .warnings(true)
        .compile("splash_arkui_shim");

    // The ArkUI native node API and the NodeContent bridge.
    println!("cargo:rustc-link-lib=dylib=ace_ndk.z");
    println!("cargo:rustc-link-lib=dylib=ace_napi.z");
    // OpenHarmony's native HTTP stack — lets the Splash VM fetch live data.
    println!("cargo:rustc-link-lib=dylib=net_http");
    // Native device facts reachable from Rust with no C++ shim and no ArkTS:
    // what the phone is, how the panel is configured, what the battery is doing.
    println!("cargo:rustc-link-lib=dylib=deviceinfo_ndk.z");
    println!("cargo:rustc-link-lib=dylib=native_display_manager");
    println!("cargo:rustc-link-lib=dylib=ohbattery_info");
    // Sensors and haptics. Vibration additionally needs
    // ohos.permission.VIBRATE declared in module.json5.
    println!("cargo:rustc-link-lib=dylib=ohsensor");
    println!("cargo:rustc-link-lib=dylib=ohvibrator.z");
    // What the default route actually is, including whether a proxy sits
    // in front of it -- a stale one is invisible from inside a page.
    println!("cargo:rustc-link-lib=dylib=net_connection");
    // Screen capture returns an OH_PixelmapNative, whose accessors live here.
    println!("cargo:rustc-link-lib=dylib=pixelmap");
    println!("cargo:rustc-link-lib=dylib=time_service_ndk");
    println!("cargo:rustc-link-lib=dylib=ohnotification");
}
