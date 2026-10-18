//! Lista o que está instalado na pasta do MultiDroid: `cargo run -p multidroid-core --example instaladas`.
//! No Mac é a mesma pasta da 0.2, então mostra o que o app SwiftUI instalou.

use multidroid_core::{catalog, platform};

fn main() {
    let host = catalog::Host::current();
    let sdk = platform::data_dir().join("sdk");
    println!("{}", sdk.display());
    match catalog::installed(catalog::EMULATOR, &sdk) {
        Some(rev) => println!("  {} {rev}", catalog::title(catalog::EMULATOR, host)),
        None => println!("  (emulador não instalado)"),
    }
    for (path, rev) in catalog::scan_installed(&sdk, host) {
        println!("  {} (revisão {rev})", catalog::title(&path, host));
    }
}
