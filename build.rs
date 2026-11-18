use std::env;
use std::fs;
use std::path::Path;

extern crate winres;

fn main() {
    // Build native C/C++ dependencies
    build_hidapi();
    build_headsetcontrol();

    let mut res = winres::WindowsResource::new();
    res.set_icon("src/icons/main.ico");

    // Application manifest for dark mode support
    res.set_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3">
    <asmv3:application>
        <asmv3:windowsSettings>
            <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true</dpiAware>
            <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
            <activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">UTF-8</activeCodePage>
        </asmv3:windowsSettings>
    </asmv3:application>
    <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
        <application>
            <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
        </application>
    </compatibility>
    <dependency>
        <dependentAssembly>
            <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
        </dependentAssembly>
    </dependency>
</assembly>
"#);

    // register light mode icons (10,20,...,50)
    for i in (10..=50).step_by(10) {
        res.set_icon_with_id(&format!("src/icons/battery{i}.ico"), &format!("{i}"));
        let charging_i = i + 1;
        res.set_icon_with_id(
            &format!("src/icons/battery{charging_i}.ico"),
            &format!("{charging_i}"),
        );
    }

    for i in (15..=55).step_by(10) {
        res.set_icon_with_id(&format!("src/icons/battery{i}.ico"), &format!("{i}"));
        let charging_i = i + 1;
        res.set_icon_with_id(
            &format!("src/icons/battery{charging_i}.ico"),
            &format!("{charging_i}"),
        );
    }

    res.compile().unwrap();
}

/// Check if a library needs to be rebuilt
/// Returns true if the marker file doesn't exist or if any source files changed
fn needs_rebuild(marker_name: &str) -> bool {
    let out_dir = env::var("OUT_DIR").unwrap();
    let marker = Path::new(&out_dir).join(marker_name);
    !marker.exists()
}

/// Mark a library as successfully built
fn mark_built(marker_name: &str) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let marker = Path::new(&out_dir).join(marker_name);
    fs::write(marker, "").expect("Failed to write marker file");
}

/// Build the hidapi library for Windows HID device access
///
/// Compiles the Windows-specific HID implementation from the vendored hidapi source.
/// This provides the low-level USB HID communication layer needed by headsetcontrol.
fn build_hidapi() {
    println!("cargo:rerun-if-changed=vendor/hidapi/windows/hid.c");

    let out_dir = env::var("OUT_DIR").unwrap();

    // Skip build if library already exists and source hasn't changed
    if !needs_rebuild("hidapi.built") {
        println!("cargo:warning=Skipping hidapi build (already cached)");
        // Ensure cargo knows where to find the library
        println!("cargo:rustc-link-search=native={}", out_dir);
        println!("cargo:rustc-link-lib=static=hidapi");
        println!("cargo:rustc-link-lib=setupapi");
        return;
    }

    println!("cargo:warning=Building hidapi from source...");
    cc::Build::new()
        .file("vendor/hidapi/windows/hid.c")
        .include("vendor/hidapi/hidapi")
        .include("vendor/hidapi/windows")
        .warnings(false)
        .compile("hidapi");

    mark_built("hidapi.built");

    // Link Windows system libraries required by hidapi
    println!("cargo:rustc-link-lib=setupapi");
}

/// Build the HeadsetControl C++ library
///
/// Compiles the headsetcontrol library from source, which provides support for
/// controlling various gaming headsets (SteelSeries, Logitech, Corsair, etc.).
///
/// Configuration:
/// - C++20 standard (required for modern C++ features)
/// - Dynamic CRT (/MD) for minimal binary size  
/// - Optimized for size (/O1)
/// - MSVC-specific flags for C++20 conformance
fn build_headsetcontrol() {
    println!("cargo:rerun-if-changed=vendor/headsetcontrol/lib");

    let out_dir = env::var("OUT_DIR").unwrap();

    // Skip build if library already exists and source hasn't changed
    if !needs_rebuild("headsetcontrol.built") {
        println!("cargo:warning=Skipping headsetcontrol build (already cached)");
        // Ensure cargo knows where to find the library
        println!("cargo:rustc-link-search=native={}", out_dir);
        println!("cargo:rustc-link-lib=static=headsetcontrol_static");
        println!("cargo:rustc-link-lib=static=hidapi");
        return;
    }

    println!("cargo:warning=Building headsetcontrol from source (this may take a while)...");

    // Generate version.h
    let version_h_content = r#"#pragma once
#define VERSION "3.3.0-dirty"
"#;
    fs::write("vendor/headsetcontrol/lib/version.h", version_h_content)
        .expect("Failed to write version.h");

    let mut build = cc::Build::new();

    build
        .cpp(true)
        .std("c++20")
        .opt_level_str("1") // Optimize for size
        .static_crt(false)
        .include("vendor/headsetcontrol/lib")
        .include("vendor/headsetcontrol/lib/devices")
        .include("vendor/hidapi/hidapi")
        // Core library sources
        .file("vendor/headsetcontrol/lib/device.cpp")
        .file("vendor/headsetcontrol/lib/device_registry.cpp")
        .file("vendor/headsetcontrol/lib/globals.cpp")
        .file("vendor/headsetcontrol/lib/headsetcontrol.cpp")
        .file("vendor/headsetcontrol/lib/headsetcontrol_c.cpp")
        .file("vendor/headsetcontrol/lib/hid_utility.cpp")
        .file("vendor/headsetcontrol/lib/result_types.cpp")
        .file("vendor/headsetcontrol/lib/utility.cpp")
        .file("vendor/headsetcontrol/lib/devices/hid_device.cpp");

    // MSVC-specific configuration for minimal binary size
    if build.get_compiler().is_like_msvc() {
        build
            .flag("/W4") // Warning level 4
            .flag("/Zc:preprocessor") // Conforming preprocessor (required for C++20)
            .flag("/EHsc") // Exception handling
            .define("NDEBUG", None); // Release mode
    }

    // On MSVC, output name must be "headsetcontrol_static" to match linker expectations
    if build.get_compiler().is_like_msvc() {
        build.compile("headsetcontrol_static");
    } else {
        build.compile("headsetcontrol");
    }

    mark_built("headsetcontrol.built");

    // Link hidapi library
    println!("cargo:rustc-link-lib=static=hidapi");
}
