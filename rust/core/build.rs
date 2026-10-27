// Gera o cliente gRPC do emulador a partir do mesmo .proto do app SwiftUI
// (mac/proto/emulator_controller.proto). O protoc vem compilado no crate
// protoc-bin-vendored, para o build não depender de nada instalado no sistema.
// O núcleo C (../native) entra no mesmo binário: CMake quando há `cmake` no
// PATH (é o caso do GitHub Actions); senão o compilador C direto, o mesmo código.
fn main() {
    compilar_nativo();
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

fn compilar_nativo() {
    let dir =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../native");
    println!(
        "cargo:rerun-if-changed={}",
        dir.join("CMakeLists.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        dir.join("src/frame.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        dir.join("src/pump.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        dir.join("include/md_frame.h").display()
    );
    let tem_cmake = std::process::Command::new("cmake")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if tem_cmake {
        compilar_com_cmake(&dir);
    } else {
        compilar_com_cc(&dir);
    }
    if cfg!(unix) {
        println!("cargo:rustc-link-lib=pthread");
    }
}

fn compilar_com_cmake(dir: &std::path::Path) {
    let dst = cmake::Config::new(dir).build();
    let lib = dst.join("lib");
    let estatica = if cfg!(windows) {
        lib.join("multidroid_frame_static.lib")
    } else {
        lib.join("libmultidroid_frame_static.a")
    };
    if !estatica.is_file() {
        panic!(
            "o CMake não instalou {} — o núcleo nativo não entrou no binário",
            estatica.display()
        );
    }
    let compartilhada = if cfg!(windows) {
        dst.join("bin").join("multidroid_frame.dll")
    } else if cfg!(target_os = "macos") {
        lib.join("libmultidroid_frame.dylib")
    } else {
        lib.join("libmultidroid_frame.so")
    };
    if !compartilhada.is_file() {
        panic!(
            "o CMake não gerou a biblioteca compartilhada em {}",
            compartilhada.display()
        );
    }
    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static=multidroid_frame_static");
}

fn compilar_com_cc(dir: &std::path::Path) {
    let mut b = cc::Build::new();
    b.file(dir.join("src/frame.c"))
        .file(dir.join("src/pump.c"))
        .include(dir.join("include"))
        .std("c11");
    if cfg!(target_os = "macos") {
        b.define("MD_BUILD_FOR_APPLE", None);
    } else if cfg!(windows) {
        b.define("MD_BUILD_FOR_WINDOWS", None);
    } else if cfg!(target_os = "linux") {
        b.define("MD_BUILD_FOR_LINUX", None);
    }
    if cfg!(unix) {
        b.flag_if_supported("-pthread");
    }
    b.compile("multidroid_frame_static");
}
