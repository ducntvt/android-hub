fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let source = "src/physical/stream/videotoolbox.c";
        let object = out.join("videotoolbox.o");
        let library = out.join("libhub_videotoolbox.a");
        let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
            "aarch64" => "arm64",
            "x86_64" => "x86_64",
            _ => panic!("Unsupported macOS target"),
        };
        assert!(
            std::process::Command::new("xcrun")
                .args([
                    "clang",
                    "-std=c11",
                    "-O2",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-arch",
                    arch,
                    "-mmacosx-version-min=13.0",
                    "-c",
                    source,
                    "-o"
                ])
                .arg(&object)
                .status()
                .expect("Xcode clang")
                .success(),
            "VideoToolbox shim compile"
        );
        assert!(
            std::process::Command::new("xcrun")
                .args(["libtool", "-static", "-o"])
                .arg(&library)
                .arg(&object)
                .status()
                .expect("Xcode libtool")
                .success(),
            "VideoToolbox shim archive"
        );
        println!("cargo:rustc-link-search=native={}", out.display());
        println!("cargo:rustc-link-lib=static=hub_videotoolbox");
        for framework in [
            "VideoToolbox",
            "CoreMedia",
            "CoreVideo",
            "CoreFoundation",
            "Accelerate",
        ] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
        println!("cargo:rerun-if-changed={source}");
    }

    let protoc = protoc_bin_vendored::protoc_bin_path().expect("bundled protoc");
    let mut config = tonic_build::Config::new();
    config.protoc_executable(protoc);
    tonic_build::configure()
        .build_server(false)
        .compile_protos_with_config(config, &["proto/emulator_controller.proto"], &["proto"])
        .expect("emulator protocol");
    println!("cargo:rerun-if-changed=proto/emulator_controller.proto");
}
