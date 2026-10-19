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
    // MULTIDROID_DATA troca a pasta (testes, ou dados em outro disco).
    if let Some(d) = std::env::var_os("MULTIDROID_DATA") {
        return PathBuf::from(d);
    }
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

const GIB: f64 = (1u64 << 30) as f64;

/// Memória física do computador, em GB binários (24 num Mac de 24 GB, como a 0.2).
pub fn physical_gb() -> f64 {
    let mut s = sysinfo::System::new();
    s.refresh_memory();
    s.total_memory() as f64 / GIB
}

/// Memória em uso e swap, em GB binários.
pub fn memory_use_gb() -> (f64, f64) {
    let mut s = sysinfo::System::new();
    s.refresh_memory();
    (s.used_memory() as f64 / GIB, s.used_swap() as f64 / GIB)
}

/// Espaço livre no disco de `path`, em bytes (o disco cujo ponto de montagem é o
/// prefixo mais longo do caminho). None se não souber.
pub fn free_space(path: &Path) -> Option<u64> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let p = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    disks
        .list()
        .iter()
        .filter(|d| p.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| d.available_space())
}

/// Onde as capturas de tela vão: <Imagens>/MultiDroid.
pub fn pictures_dir() -> PathBuf {
    home().join("Pictures").join("MultiDroid")
}

/// Registro do emulador de cada aparelho.
pub fn logs_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        home().join("Library/Logs/MultiDroid")
    } else {
        data_dir().join("logs")
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
