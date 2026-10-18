//! O que muda por sistema (docs/spec/MULTIPLATAFORMA.md §2.1). Começa pela pasta
//! de dados; duplicar, frames e métricas entram nas etapas seguintes.

use std::path::PathBuf;

/// Onde o MultiDroid guarda emulador, imagens, aparelhos e preferências.
/// No Mac é a mesma pasta da 0.2, para a versão Rust abrir o que ela criou.
pub fn data_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/MultiDroid")
    } else if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or(home)
            .join("MultiDroid")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"))
            .join("multidroid")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn pasta_de_dados_termina_no_nome_do_app() {
        let d = super::data_dir();
        let last = d.file_name().unwrap().to_string_lossy().to_lowercase();
        assert_eq!(last, "multidroid");
    }
}
