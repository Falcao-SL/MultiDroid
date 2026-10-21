//! Digita um texto num aparelho ligado (vai para o campo em foco).
//!   cargo run --release -p multidroid-core --example digitar -- <aparelho> <texto>

use multidroid_core::emulator::{self, Link, Processes};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = Processes::new().emulators().into_iter().find(|r| r.avd == a[1]).expect("não está ligado");
    let token = emulator::grpc_token(r.pid).expect("sem token");
    let link = Link::connect(r.grpc.expect("sem gRPC"), &token).await.unwrap();
    link.type_text(&a[2]).await;
    eprintln!("digitou em {}", a[1]);
}
