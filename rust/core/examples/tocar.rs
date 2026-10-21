//! Um toque num aparelho ligado, em coordenadas da tela do Android.
//!   cargo run --release -p multidroid-core --example tocar -- <aparelho> <x> <y>

use multidroid_core::emulator::{self, Link, Processes};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (nome, x, y): (&str, i32, i32) = (&a[1], a[2].parse().unwrap(), a[3].parse().unwrap());
    let r = Processes::new().emulators().into_iter().find(|r| r.avd == nome).expect("não está ligado");
    let token = emulator::grpc_token(r.pid).expect("sem token");
    let link = Link::connect(r.grpc.expect("sem gRPC"), &token).await.unwrap();
    link.touch(x, y, true).await;
    tokio::time::sleep(Duration::from_millis(80)).await;
    link.touch(x, y, false).await;
    eprintln!("tocou {nome} em ({x}, {y})");
}
