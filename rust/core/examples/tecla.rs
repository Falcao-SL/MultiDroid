//! Uma tecla do Android num aparelho ligado (GoHome, GoBack, AppSwitch…).
//!   cargo run --release -p multidroid-core --example tecla -- <aparelho> <tecla>

use multidroid_core::emulator::{self, Link, Processes};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = Processes::new()
        .emulators()
        .into_iter()
        .find(|r| r.avd == a[1])
        .expect("não está ligado");
    let token = emulator::grpc_token(r.pid).expect("sem token");
    let link = Link::connect(r.grpc.expect("sem gRPC"), &token)
        .await
        .unwrap();
    link.key(&a[2]).await;
    eprintln!("{} em {}", a[2], a[1]);
}
