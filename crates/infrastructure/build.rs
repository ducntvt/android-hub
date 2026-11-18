fn main() {
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("bundled protoc");
    let mut config = tonic_build::Config::new();
    config.protoc_executable(protoc);
    tonic_build::configure()
        .build_server(false)
        .compile_protos_with_config(config, &["proto/emulator_controller.proto"], &["proto"])
        .expect("emulator protocol");
    println!("cargo:rerun-if-changed=proto/emulator_controller.proto");
}
