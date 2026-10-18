//! O que muda por sistema (docs/spec/MULTIPLATAFORMA.md §2.1): pastas e cópia
//! de aparelho. Frames e métricas entram nas etapas seguintes.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Pasta do usuário: HOME no Mac e no Linux, USERPROFILE no Windows.
pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// Onde o MultiDroid guarda emulador, imagens, aparelhos e preferências.
/// No Mac é a mesma pasta da 0.2, para a versão Rust abrir o que ela criou.
pub fn data_dir() -> PathBuf {
    let home = home();
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

/// Copia uma pasta inteira para `dst` (que não pode existir). No Mac usa o clone
/// do APFS (`cp -c`), instantâneo e sem gastar disco até as cópias divergirem; no
/// Linux, reflink quando o sistema de arquivos aceita (Btrfs, XFS). Se o comando
/// falhar, e no Windows, copia arquivo por arquivo.
pub fn clone_dir(src: &Path, dst: &Path) -> io::Result<()> {
    if dst.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "o destino já existe",
        ));
    }
    let cmd = if cfg!(target_os = "macos") {
        Some(
            Command::new("/bin/cp")
                .arg("-cR")
                .arg(src)
                .arg(dst)
                .status(),
        )
    } else if cfg!(target_os = "linux") {
        Some(
            Command::new("cp")
                .args(["-a", "--reflink=auto"])
                .arg(src)
                .arg(dst)
                .status(),
        )
    } else {
        None
    };
    if let Some(Ok(s)) = cmd {
        if s.success() {
            return Ok(());
        }
        let _ = std::fs::remove_dir_all(dst);
    }
    copy_tree(src, dst)
}

fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_tree(&e.path(), &to)?;
        } else {
            std::fs::copy(e.path(), to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn copia_pasta_com_subpastas() {
        let t = tempfile::tempdir().unwrap();
        let a = t.path().join("a");
        std::fs::create_dir_all(a.join("sub")).unwrap();
        std::fs::write(a.join("sub/x.img"), b"123").unwrap();
        let b = t.path().join("b");
        super::clone_dir(&a, &b).unwrap();
        assert_eq!(std::fs::read(b.join("sub/x.img")).unwrap(), b"123");
        assert!(super::clone_dir(&a, &b).is_err());
        super::copy_tree(&a, &t.path().join("c")).unwrap();
        assert!(t.path().join("c/sub/x.img").is_file());
    }

    #[test]
    fn pasta_de_dados_termina_no_nome_do_app() {
        let d = super::data_dir();
        let last = d.file_name().unwrap().to_string_lossy().to_lowercase();
        assert_eq!(last, "multidroid");
    }
}
