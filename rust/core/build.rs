// Gera o cliente gRPC do emulador a partir do mesmo .proto do app SwiftUI
// (mac/proto/emulator_controller.proto). O protoc vem compilado no crate
// protoc-bin-vendored, para o build não depender de nada instalado no sistema.
fn main() {
    let proto = "../../mac/proto/emulator_controller.proto";
    println!("cargo:rerun-if-changed={proto}");
    // SAFETY: build script de uma thread só, antes de qualquer outra coisa.
    unsafe {
        std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());
    }
    tonic_build::configure()
        .build_server(false)
        .compile_protos(
            &[proto],
            &[
                std::path::PathBuf::from("../../mac/proto"),
                protoc_bin_vendored::include_path().unwrap(),
            ],
        )
        .expect("emulator_controller.proto não compilou");
}
