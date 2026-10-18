//! Tokens de design lidos de docs/design/tokens.json, a mesma fonte do app
//! SwiftUI: a interface Rust e a do Mac não podem divergir.

use serde::Deserialize;

/// O arquivo entra no binário na compilação; mudar o JSON muda os dois apps.
pub const TOKENS_JSON: &str = include_str!("../../../docs/design/tokens.json");

#[derive(Debug, Deserialize)]
pub struct Tokens {
    pub cor: Cores,
    pub forma: Forma,
}

#[derive(Debug, Deserialize)]
pub struct Cores {
    pub claro: Tema,
    pub escuro: Tema,
    pub acento: Acentos,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tema {
    pub fundo: String,
    pub superficie: String,
    pub borda: String,
    pub sombra: String,
    pub tinta: String,
    pub tinta_sobre_acento: String,
}

#[derive(Debug, Deserialize)]
pub struct Acentos {
    pub azul: String,
    pub rosa: String,
    pub amarelo: String,
    pub verde: String,
    pub ciano: String,
    pub roxo: String,
    pub laranja: String,
    pub vermelho: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Forma {
    pub largura_borda: f32,
    pub raio: f32,
    pub sombra: Sombra,
}

#[derive(Debug, Deserialize)]
pub struct Sombra {
    pub x: f32,
    pub y: f32,
    pub desfoque: f32,
}

pub fn load() -> Tokens {
    serde_json::from_str(TOKENS_JSON).expect("docs/design/tokens.json inválido")
}

/// "#7AA7FF" → (0x7a, 0xa7, 0xff).
pub fn rgb(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).expect("cor em #RRGGBB");
    (p(0), p(2), p(4))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_os_tokens_do_app() {
        let t = load();
        assert_eq!(t.forma.largura_borda, 2.5);
        assert_eq!(t.forma.raio, 10.0);
        assert_eq!(
            (t.forma.sombra.x, t.forma.sombra.y, t.forma.sombra.desfoque),
            (4.0, 4.0, 0.0)
        );
        assert_eq!(rgb(&t.cor.acento.azul), (0x7a, 0xa7, 0xff));
        assert_eq!(rgb(&t.cor.escuro.fundo), (0x17, 0x17, 0x1c));
    }
}
