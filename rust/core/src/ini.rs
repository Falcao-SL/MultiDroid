//! Leitura de arquivos chave=valor (source.properties, config.ini, os .ini do AVD),
//! como `readIni` do app SwiftUI.

use std::collections::HashMap;
use std::path::Path;

pub fn parse(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let (k, v) = line.split_once('=')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

/// Arquivo que não existe ou não é texto vira mapa vazio, como no Swift.
pub fn read(path: &Path) -> HashMap<String, String> {
    std::fs::read_to_string(path)
        .map(|t| parse(&t))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn le_chave_valor_com_espacos() {
        let m = super::parse("Pkg.Revision = 4\n# comentario\nAbi=arm64-v8a\n");
        assert_eq!(m["Pkg.Revision"], "4");
        assert_eq!(m["Abi"], "arm64-v8a");
        assert_eq!(m.len(), 2);
    }
}
