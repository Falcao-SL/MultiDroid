//! Baixar, conferir e extrair um pacote do catálogo. Porte de `ImageStore.install` e
//! `unpack` do app SwiftUI, com uma diferença: a extração é feita aqui, pela biblioteca
//! de zip, sem chamar `ditto` nem `unzip` (o ditto não lê alguns zips do repositório,
//! como o do Wear OS 33 r04 e o do AOSP 33 r02).

use crate::idioma as t;
use crate::catalog::{self, RemotePackage};
use sha1::{Digest, Sha1};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Http(String),
    Checksum { expected: String, got: String },
    Zip(String),
    Empty,
    NoSpace { needed: u64 },
    Cancelled,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Http(e) => write!(f, "{}", t::tf("the download failed: {e}", &[("e", e)])),
            Error::Checksum { .. } => {
                write!(f, "{}", t::t("the downloaded file doesn't match the published SHA-1"))
            }
            Error::Zip(e) => write!(f, "{}", t::tf("couldn't extract the file: {e}", &[("e", e)])),
            Error::Empty => write!(f, "{}", t::t("empty file")),
            Error::Cancelled => write!(f, "{}", t::t("download cancelled")),
            Error::NoSpace { needed } => write!(
                f,
                "{}",
                t::tf("not enough space: needs {gb} GB free", &[("gb", &t::num(*needed as f64 / 1e9, 1))])
            ),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

/// SHA-1 em hexadecimal minúsculo, como o repositório publica.
pub fn sha1_hex(data: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(data);
    hex(&h.finalize())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-1 de um arquivo, lido em blocos (as imagens passam de 1 GB).
pub fn file_sha1(path: &Path) -> io::Result<String> {
    let mut f = File::open(path)?;
    let mut h = Sha1::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

// MARK: Licença

/// Aceite gravado em <sdk>/licenses/<id> com o SHA-1 do texto, no formato do sdkmanager:
/// se o texto da licença mudar, o aceite antigo deixa de valer.
pub fn accepted(pkg: &RemotePackage, sdk: &Path) -> bool {
    let want = sha1_hex(pkg.license_text.as_bytes());
    fs::read_to_string(sdk.join("licenses").join(&pkg.license))
        .map(|t| t.lines().any(|l| l.trim() == want))
        .unwrap_or(false)
}

pub fn accept(pkg: &RemotePackage, sdk: &Path) -> io::Result<()> {
    let dir = sdk.join("licenses");
    fs::create_dir_all(&dir)?;
    let file = dir.join(&pkg.license);
    let old = fs::read_to_string(&file).unwrap_or_default();
    fs::write(
        file,
        format!("{old}\n{}", sha1_hex(pkg.license_text.as_bytes())),
    )
}

// MARK: Catálogo remoto

fn get_text(url: &str) -> Result<String, Error> {
    ureq::get(url)
        .call()
        .map_err(|e| Error::Http(e.to_string()))?
        .into_string()
        .map_err(Error::Io)
}

/// O emulador e as imagens que o repositório oferece para este host. Cada lista é
/// independente: uma que falhe não esconde as outras, e vira uma linha em `falhas`.
pub fn fetch_remote(host: catalog::Host) -> (Vec<RemotePackage>, Vec<String>) {
    let mut out = vec![];
    let mut falhas = vec![];
    match get_text(&format!("{}repository2-3.xml", catalog::REPO)).map(|x| {
        catalog::parse(&x, catalog::EMULATOR, catalog::REPO, host)
            .map_err(|e| Error::Zip(e.to_string()))
    }) {
        Ok(Ok(Some(p))) => out.push(p),
        Ok(Ok(None)) => falhas.push(t::t("emulator (not available for this system)").into()),
        Ok(Err(e)) | Err(e) => falhas.push(format!("emulador ({e})")),
    }
    // O motor leve só existe no Linux, e é só ele que usa o adb.
    if cfg!(target_os = "linux") {
        match get_text(&format!("{}repository2-3.xml", catalog::REPO)).map(|x| {
            catalog::parse(&x, crate::adb::PLATFORM_TOOLS, catalog::REPO, host)
                .map_err(|e| Error::Zip(e.to_string()))
        }) {
            Ok(Ok(Some(p))) => out.push(p),
            Ok(Ok(None)) => {}
            Ok(Err(e)) | Err(e) => falhas.push(format!("platform-tools ({e})")),
        }
    }
    for tag in catalog::TAGS {
        let url = catalog::tag_list_url(tag);
        let base = url
            .rsplit_once('/')
            .map(|(b, _)| format!("{b}/"))
            .unwrap_or_default();
        match get_text(&url).map(|x| catalog::images_in(&x, &base, host)) {
            Ok(Ok(v)) => out.extend(v),
            Ok(Err(e)) => falhas.push(format!("{tag} ({e})")),
            Err(e) => falhas.push(format!("{tag} ({e})")),
        }
    }
    (out, falhas)
}

/// Remove um pacote instalado (a pasta dele no SDK).
pub fn remove(path: &str, sdk: &Path) -> io::Result<()> {
    let d = catalog::dir(path, sdk);
    if d.exists() {
        fs::remove_dir_all(d)
    } else {
        Ok(())
    }
}

// MARK: Download e extração

/// Baixa `url` para `dest`, chamando `progress` com a fração de 0 a 1. Para quando
/// `cancel` vira true.
pub fn download(
    url: &str,
    dest: &Path,
    total: u64,
    cancel: &AtomicBool,
    mut progress: impl FnMut(f64),
) -> Result<(), Error> {
    let resp = ureq::get(url)
        .call()
        .map_err(|e| Error::Http(e.to_string()))?;
    let mut reader = resp.into_reader();
    let mut out = File::create(dest)?;
    let mut buf = vec![0u8; 1 << 20];
    let mut done: u64 = 0;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        done += n as u64;
        if total > 0 {
            progress((done as f64 / total as f64).min(1.0));
        }
    }
    out.flush()?;
    Ok(())
}

/// Extrai o zip e põe a única pasta do topo (emulator/ ou arm64-v8a/) no lugar do pacote.
/// O que já estava instalado só é trocado depois que a extração deu certo.
pub fn unpack(zip_path: &Path, path: &str, sdk: &Path, work: &Path) -> Result<PathBuf, Error> {
    let tmp = work.join(format!("tmp-{}", std::process::id()));
    if tmp.exists() {
        fs::remove_dir_all(&tmp)?;
    }
    fs::create_dir_all(&tmp)?;
    let result = (|| {
        let mut archive =
            zip::ZipArchive::new(File::open(zip_path)?).map_err(|e| Error::Zip(e.to_string()))?;
        // Mantém as permissões do zip: o executável do emulador precisa continuar executável.
        archive
            .extract(&tmp)
            .map_err(|e| Error::Zip(e.to_string()))?;
        let top = fs::read_dir(&tmp)?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                !p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            })
            .ok_or(Error::Empty)?;
        let dst = catalog::dir(path, sdk);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        if dst.exists() {
            fs::remove_dir_all(&dst)?;
        }
        fs::rename(&top, &dst)?;
        Ok(dst)
    })();
    let _ = fs::remove_dir_all(&tmp);
    result
}

/// Instalação completa: confere espaço, baixa, confere o SHA-1 e extrai.
/// O aceite da licença é pedido antes, pela interface (`accepted`/`accept`).
pub fn install(
    pkg: &RemotePackage,
    sdk: &Path,
    work: &Path,
    cancel: &AtomicBool,
    progress: impl FnMut(f64),
) -> Result<PathBuf, Error> {
    fs::create_dir_all(work)?;
    let free = crate::platform::free_space(work);
    // Zip baixado + extraído: o dobro do tamanho, com folga.
    let needed = pkg.size.saturating_mul(3);
    if free.is_some_and(|f| f < needed) {
        return Err(Error::NoSpace { needed });
    }
    let zip = work.join(format!("dl-{}.zip", std::process::id()));
    let result = (|| {
        download(&pkg.url, &zip, pkg.size, cancel, progress)?;
        let got = file_sha1(&zip)?;
        if got != pkg.sha1 {
            return Err(Error::Checksum {
                expected: pkg.sha1.clone(),
                got,
            });
        }
        unpack(&zip, &pkg.path, sdk, work)
    })();
    let _ = fs::remove_file(&zip);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Host, installed};

    fn pkg(path: &str, text: &str) -> RemotePackage {
        RemotePackage {
            path: path.into(),
            revision: "4".into(),
            size: 0,
            sha1: String::new(),
            url: String::new(),
            license: "exemplo-licenca".into(),
            license_text: text.into(),
        }
    }

    #[test]
    fn sha1_conhecido() {
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn aceite_vale_so_para_o_mesmo_texto() {
        let sdk = tempfile::tempdir().unwrap();
        let p = pkg("x", "Texto 1");
        assert!(!accepted(&p, sdk.path()));
        accept(&p, sdk.path()).unwrap();
        assert!(accepted(&p, sdk.path()));
        assert!(!accepted(&pkg("x", "Texto 2, mudou"), sdk.path()));
    }

    /// Zip como os do repositório: uma pasta no topo, com source.properties dentro.
    fn zip_de_imagem(dir: &Path) -> PathBuf {
        let z = dir.join("img.zip");
        let mut w = zip::ZipWriter::new(File::create(&z).unwrap());
        let opts = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        w.add_directory("arm64-v8a/", opts).unwrap();
        w.start_file("arm64-v8a/source.properties", opts).unwrap();
        w.write_all(b"Pkg.Revision=4\nAbi=arm64-v8a\n").unwrap();
        w.start_file("arm64-v8a/system.img", opts).unwrap();
        w.write_all(&[0u8; 64]).unwrap();
        w.finish().unwrap();
        z
    }

    #[test]
    fn extrai_no_lugar_do_pacote_e_troca_o_antigo() {
        let t = tempfile::tempdir().unwrap();
        let sdk = t.path().join("sdk");
        let path = "system-images;android-33;android-wear;arm64-v8a";
        let old = catalog::dir(path, &sdk);
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("velho.txt"), "x").unwrap();
        let z = zip_de_imagem(t.path());
        let dst = unpack(&z, path, &sdk, t.path()).unwrap();
        assert_eq!(dst, old);
        assert!(!dst.join("velho.txt").exists());
        assert!(dst.join("system.img").is_file());
        assert_eq!(installed(path, &sdk).as_deref(), Some("4"));
        assert_eq!(catalog::scan_installed(&sdk, Host::MAC_ARM).len(), 1);
        // Nada de pasta temporária esquecida.
        assert!(
            fs::read_dir(t.path())
                .unwrap()
                .flatten()
                .all(|e| !e.file_name().to_string_lossy().starts_with("tmp-"))
        );
    }

    #[test]
    fn zip_corrompido_nao_apaga_o_que_estava_instalado() {
        let t = tempfile::tempdir().unwrap();
        let sdk = t.path().join("sdk");
        let path = "system-images;android-33;default;arm64-v8a";
        let old = catalog::dir(path, &sdk);
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("source.properties"), "Pkg.Revision=2\n").unwrap();
        let z = t.path().join("ruim.zip");
        fs::write(&z, b"isto nao e um zip").unwrap();
        assert!(matches!(
            unpack(&z, path, &sdk, t.path()),
            Err(Error::Zip(_))
        ));
        assert_eq!(installed(path, &sdk).as_deref(), Some("2"));
    }
}
