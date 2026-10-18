//! Lista os aparelhos e as mesas da pasta do MultiDroid, só lendo:
//!   cargo run -p multidroid-core --example aparelhos
//! No Mac, mostra o que a 0.2 criou (critério da etapa R-2).

use multidroid_core::{devices, platform, state};

fn main() {
    let devs = devices::load(&devices::avd_home());
    let st = state::State::load(&platform::data_dir().join("state.json"), 0.0);
    println!("teto: {} GB · mesas: {:?}", st.ceiling_gb, st.workspaces);
    for d in st.ordered(&devs) {
        let spot = st
            .spots
            .get(&d.name)
            .map(|s| format!("x{} y{} {}×{}", s.x, s.y, s.w, s.h))
            .unwrap_or_else(|| "sem lugar".into());
        println!(
            "{:<10} {:<9} API {:<3} {:<10} mesa {:<5} grupo {:<6} {spot}",
            d.name,
            d.kind.label(),
            d.api,
            d.screen,
            st.desk_of(&d.name),
            st.groups.get(&d.name).map_or("-", |s| s),
        );
    }
}
