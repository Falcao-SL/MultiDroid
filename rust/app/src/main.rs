// MultiDroid em Rust e Slint: um código para Mac, Windows e Linux
// (docs/spec/MULTIPLATAFORMA.md). Por enquanto, a janela da etapa R-0.

slint::include_modules!();

use multidroid_core::{platform, tokens};

fn cor(hex: &str) -> slint::Color {
    let (r, g, b) = tokens::rgb(hex);
    slint::Color::from_rgb_u8(r, g, b)
}

fn main() -> Result<(), slint::PlatformError> {
    let t = tokens::load();
    let ui = AppWindow::new()?;
    let tema = ui.global::<Tema>();
    // Tema escuro dos tokens, o mesmo das capturas da 0.2.
    let e = &t.cor.escuro;
    tema.set_fundo(cor(&e.fundo));
    tema.set_superficie(cor(&e.superficie));
    tema.set_borda(cor(&e.borda));
    tema.set_sombra(cor(&e.sombra));
    tema.set_tinta(cor(&e.tinta));
    tema.set_tinta_sobre_acento(cor(&e.tinta_sobre_acento));
    tema.set_azul(cor(&t.cor.acento.azul));
    tema.set_verde(cor(&t.cor.acento.verde));
    tema.set_largura_borda(t.forma.largura_borda);
    tema.set_raio(t.forma.raio);
    tema.set_sombra_x(t.forma.sombra.x);
    tema.set_sombra_y(t.forma.sombra.y);
    ui.set_versao(env!("CARGO_PKG_VERSION").into());
    ui.set_pasta(platform::data_dir().display().to_string().into());
    ui.run()
}
