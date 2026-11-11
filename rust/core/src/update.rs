//! Atualização do próprio MultiDroid Preview: consulta o feed do site
//! (`releases.json`), baixa o instalador do canal preview, confere SHA-256
//! quando `SHA256SUMS.txt` existe na release, e só instala quando o usuário
//! aperta o botão.

use crate::idioma as t;
use crate::install;
use crate::platform;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

const FEED_URL: &str = "https://multidroid.falcaosl.com/releases.json";
const FEED_FALLBACK: &str =
    "https://raw.githubusercontent.com/Falcao-SL/MultiDroid/main/site/releases.json";
const GITHUB_API: &str = "https://api.github.com/repos/Falcao-SL/MultiDroid/releases?per_page=20";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Preview,
}

#[derive(Debug, Clone)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct Available {
    pub tag: String,
    pub version: String,
    pub release_url: String,
    pub asset: Asset,
    pub sha256: Option<String>,
}

#[derive(Debug)]
pub enum Error {
    Http(String),
    Io(io::Error),
    Parse(String),
    Checksum { expected: String, got: String },
    NoAsset,
    Install(String),
    Cancelled,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Http(e) => write!(f, "{}", t::tf("the download failed: {e}", &[("e", e)])),
            Error::Io(e) => write!(f, "{e}"),
            Error::Parse(e) => write!(f, "{e}"),
            Error::Checksum { .. } => write!(
                f,
                "{}",
                t::t("the downloaded file doesn't match the published SHA-256")
            ),
            Error::NoAsset => write!(
                f,
                "{}",
                t::t("no installer for this system in the latest release")
            ),
            Error::Install(e) => write!(f, "{e}"),
            Error::Cancelled => write!(f, "{}", t::t("download cancelled")),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

#[derive(Deserialize)]
struct FeedFile {
    stable: Option<FeedChannel>,
    preview: Option<FeedChannel>,
}

#[derive(Deserialize)]
struct FeedChannel {
    tag: String,
    url: String,
    assets: Vec<FeedAsset>,
}

#[derive(Deserialize)]
struct FeedAsset {
    name: String,
    url: String,
    #[serde(default)]
    size: u64,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    #[serde(default)]
    state: String,
}

/// Versão embutida no binário (tag completa no pacote de release, senão Cargo).
pub fn installed_version() -> &'static str {
    option_env!("MULTIDROID_RELEASE").unwrap_or(env!("CARGO_PKG_VERSION"))
}

/// Há uma versão publicada mais nova neste canal?
pub fn check(channel: Channel) -> Result<Option<Available>, Error> {
    let feed = fetch_feed()?;
    let ch = match channel {
        Channel::Stable => feed.stable.as_ref(),
        Channel::Preview => feed.preview.as_ref(),
    }
    .ok_or_else(|| Error::Parse("channel missing from releases.json".into()))?;

    let version = ch.tag.trim_start_matches('v').to_string();
    if !is_newer(installed_version(), &version) {
        return Ok(None);
    }
    let asset = pick_asset(&ch.assets)?;
    let mut available = Available {
        tag: ch.tag.clone(),
        version,
        release_url: ch.url.clone(),
        asset,
        sha256: None,
    };
    available.sha256 = fetch_sha256(&available.tag, &available.asset.name)
        .ok()
        .flatten();
    Ok(Some(available))
}

fn fetch_feed() -> Result<FeedFile, Error> {
    match get_json::<FeedFile>(FEED_URL) {
        Ok(f) if f.stable.is_some() || f.preview.is_some() => Ok(f),
        _ => match get_json::<FeedFile>(FEED_FALLBACK) {
            Ok(f) if f.stable.is_some() || f.preview.is_some() => Ok(f),
            _ => feed_from_github(),
        },
    }
}

fn feed_from_github() -> Result<FeedFile, Error> {
    let releases: Vec<GhRelease> = get_json(GITHUB_API)?;
    let mut feed = FeedFile {
        stable: None,
        preview: None,
    };
    for r in releases {
        if r.draft {
            continue;
        }
        let assets: Vec<FeedAsset> = r
            .assets
            .into_iter()
            .filter(|a| a.state.is_empty() || a.state == "uploaded")
            .filter(|a| a.browser_download_url.starts_with("https://"))
            .map(|a| FeedAsset {
                name: a.name,
                url: a.browser_download_url,
                size: a.size,
            })
            .collect();
        if assets.is_empty() {
            continue;
        }
        let ch = FeedChannel {
            tag: r.tag_name,
            url: r.html_url,
            assets,
        };
        if r.prerelease {
            if feed.preview.is_none() {
                feed.preview = Some(ch);
            }
        } else if feed.stable.is_none() {
            feed.stable = Some(ch);
        }
        if feed.stable.is_some() && feed.preview.is_some() {
            break;
        }
    }
    if feed.stable.is_none() && feed.preview.is_none() {
        return Err(Error::Parse("no published release found".into()));
    }
    Ok(feed)
}

fn get_json<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T, Error> {
    let text = get_text(url)?;
    serde_json::from_str(&text).map_err(|e| Error::Parse(e.to_string()))
}

fn get_text(url: &str) -> Result<String, Error> {
    if !(url.starts_with("https://") || url.starts_with("http://127.0.0.1")) {
        return Err(Error::Http("only HTTPS downloads are allowed".into()));
    }
    let mut req = ureq::get(url);
    if url.contains("api.github.com") {
        req = req
            .set("User-Agent", "MultiDroid-updater")
            .set("Accept", "application/vnd.github+json");
    }
    req.call()
        .map_err(|e| Error::Http(e.to_string()))?
        .into_string()
        .map_err(Error::Io)
}

fn pick_asset(assets: &[FeedAsset]) -> Result<Asset, Error> {
    let name = preferred_asset_name(assets).ok_or(Error::NoAsset)?;
    let a = assets
        .iter()
        .find(|a| a.name == name)
        .ok_or(Error::NoAsset)?;
    if !a.url.starts_with("https://") {
        return Err(Error::Http("installer URL must be HTTPS".into()));
    }
    Ok(Asset {
        name: a.name.clone(),
        url: a.url.clone(),
        size: a.size,
    })
}

/// Escolhe o instalador certo para o SO/arch em que o app está rodando.
fn preferred_asset_name(assets: &[FeedAsset]) -> Option<String> {
    let names: Vec<&str> = assets.iter().map(|a| a.name.as_str()).collect();
    #[cfg(target_os = "macos")]
    {
        // Preview arm64 DMG; estável é MultiDroid-<ver>.dmg sem sufixo.
        names
            .iter()
            .find(|n| {
                n.ends_with("-mac-arm64.dmg")
                    || (n.starts_with("MultiDroid-")
                        && n.ends_with(".dmg")
                        && !n.contains("Preview")
                        && !n.contains("QA"))
            })
            .map(|s| (*s).to_string())
    }
    #[cfg(target_os = "windows")]
    {
        names
            .iter()
            .find(|n| n.ends_with("-windows-x64-setup.exe") || n.ends_with(".exe"))
            .map(|s| (*s).to_string())
    }
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("APPIMAGE").is_some() {
            if let Some(n) = names.iter().find(|n| n.ends_with(".AppImage")) {
                return Some((*n).to_string());
            }
        }
        // deb/rpm pelo gerenciador; AppImage como fallback portátil.
        if Path::new("/usr/bin/apt-get").exists() || Path::new("/usr/bin/dpkg").exists() {
            if let Some(n) = names.iter().find(|n| n.ends_with(".deb")) {
                return Some((*n).to_string());
            }
        }
        if Path::new("/usr/bin/rpm").exists() || Path::new("/usr/bin/dnf").exists() {
            if let Some(n) = names.iter().find(|n| n.ends_with(".rpm")) {
                return Some((*n).to_string());
            }
        }
        if let Some(n) = names.iter().find(|n| n.ends_with(".AppImage")) {
            return Some((*n).to_string());
        }
        if let Some(n) = names.iter().find(|n| n.ends_with(".deb")) {
            return Some((*n).to_string());
        }
        names
            .iter()
            .find(|n| n.ends_with(".rpm"))
            .map(|s| (*s).to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = names;
        None
    }
}

fn fetch_sha256(tag: &str, asset_name: &str) -> Result<Option<String>, Error> {
    let url =
        format!("https://github.com/Falcao-SL/MultiDroid/releases/download/{tag}/SHA256SUMS.txt");
    let text = match get_text(&url) {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let digest = parts.next().unwrap_or("");
        let name = parts.next().unwrap_or("").trim_start_matches('*');
        if name == asset_name && digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Ok(Some(digest.to_ascii_lowercase()));
        }
    }
    Ok(None)
}

/// `remote` é mais novo que `current`? Aceita `0.2`, `0.2.1`, `0.3.0-alpha.3`.
pub fn is_newer(current: &str, remote: &str) -> bool {
    match (parse_version(current), parse_version(remote)) {
        (Some(c), Some(r)) => r > c,
        _ => remote != current && !remote.is_empty(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Ver {
    major: u32,
    minor: u32,
    patch: u32,
    /// None = release final; Some = pré-release (menor que a final do mesmo trio).
    pre: Option<String>,
}

impl PartialOrd for Ver {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ver {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(a), Some(b)) => cmp_pre(a, b),
            })
    }
}

fn cmp_pre(a: &str, b: &str) -> std::cmp::Ordering {
    let pa: Vec<&str> = a.split('.').collect();
    let pb: Vec<&str> = b.split('.').collect();
    for i in 0..pa.len().max(pb.len()) {
        match (pa.get(i), pb.get(i)) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) => {
                let ord = match (x.parse::<u32>(), y.parse::<u32>()) {
                    (Ok(nx), Ok(ny)) => nx.cmp(&ny),
                    _ => x.cmp(y),
                };
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            }
        }
    }
    std::cmp::Ordering::Equal
}

fn parse_version(s: &str) -> Option<Ver> {
    let s = s.trim().trim_start_matches('v');
    if s.is_empty() {
        return None;
    }
    let (num, pre) = match s.split_once('-') {
        Some((n, p)) => (n, Some(p.to_ascii_lowercase())),
        None => (s, None),
    };
    let mut parts = num.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let patch = parts.next().unwrap_or("0").parse().unwrap_or(0);
    Some(Ver {
        major,
        minor,
        patch,
        pre,
    })
}

fn file_sha256(path: &Path) -> io::Result<String> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Pasta temporária dos instaladores baixados.
pub fn download_dir() -> PathBuf {
    platform::data_dir().join("updates")
}

/// Baixa o asset, confere SHA-256 se houver, e devolve o caminho local.
pub fn download_update(
    available: &Available,
    cancel: &AtomicBool,
    mut progress: impl FnMut(f64),
) -> Result<PathBuf, Error> {
    let dir = download_dir();
    fs::create_dir_all(&dir)?;
    let dest = dir.join(&available.asset.name);
    if dest.exists() {
        let _ = fs::remove_file(&dest);
    }
    install::download(
        &available.asset.url,
        &dest,
        available.asset.size,
        cancel,
        &mut progress,
    )
    .map_err(|e| match e {
        install::Error::Cancelled => Error::Cancelled,
        install::Error::Http(s) => Error::Http(s),
        install::Error::Io(e) => Error::Io(e),
        other => Error::Install(other.to_string()),
    })?;
    if let Some(want) = &available.sha256 {
        let got = file_sha256(&dest)?;
        if got != *want {
            let _ = fs::remove_file(&dest);
            return Err(Error::Checksum {
                expected: want.clone(),
                got,
            });
        }
    }
    progress(1.0);
    Ok(dest)
}

/// Instala o pacote baixado e relança o app. Não retorna em sucesso no fluxo
/// que encerra o processo atual.
pub fn install_and_relaunch(installer: &Path, available: &Available) -> Result<(), Error> {
    #[cfg(target_os = "macos")]
    {
        let _ = available;
        install_macos_dmg(installer)
    }
    #[cfg(target_os = "windows")]
    {
        let _ = available;
        install_windows_nsis(installer)
    }
    #[cfg(target_os = "linux")]
    {
        install_linux(installer, available)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = (installer, available);
        Err(Error::Install("unsupported platform".into()))
    }
}

#[cfg(target_os = "macos")]
fn current_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut p = exe.as_path();
    while let Some(parent) = p.parent() {
        if p.extension().and_then(|e| e.to_str()) == Some("app") {
            return Some(p.to_path_buf());
        }
        // …/App.app/Contents/MacOS/bin
        if parent.file_name().and_then(|n| n.to_str()) == Some("MacOS")
            && let Some(contents) = parent.parent()
            && let Some(app) = contents.parent()
            && app.extension().and_then(|e| e.to_str()) == Some("app")
        {
            return Some(app.to_path_buf());
        }
        p = parent;
    }
    None
}

#[cfg(target_os = "macos")]
fn install_macos_dmg(dmg: &Path) -> Result<(), Error> {
    let mount = download_dir().join(format!("mnt-{}", std::process::id()));
    let _ = fs::remove_dir_all(&mount);
    fs::create_dir_all(&mount)?;
    let attach = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
        .arg(&mount)
        .arg(dmg)
        .output()
        .map_err(|e| Error::Install(e.to_string()))?;
    if !attach.status.success() {
        return Err(Error::Install(
            String::from_utf8_lossy(&attach.stderr).into(),
        ));
    }
    let result = (|| {
        let bundled = find_app_in(&mount)
            .ok_or_else(|| Error::Install(t::t("the DMG has no .app bundle").into()))?;
        let dest = current_app_bundle()
            .filter(|p| p.starts_with("/Applications") || p.exists())
            .unwrap_or_else(|| PathBuf::from("/Applications").join(bundled.file_name().unwrap()));
        // Copia para um vizinho temporário e troca atomicamente quando dá.
        let parent = dest.parent().unwrap_or(Path::new("/Applications"));
        let staging = parent.join(format!(
            ".{}-update-{}",
            dest.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("MultiDroid"),
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&staging);
        let copy = Command::new("ditto")
            .arg(&bundled)
            .arg(&staging)
            .output()
            .map_err(|e| Error::Install(e.to_string()))?;
        if !copy.status.success() {
            // Sem permissão em /Applications: eleva com osascript.
            let script = format!(
                "do shell script \"ditto {} {}\" with administrator privileges",
                shell_quote(&bundled),
                shell_quote(&staging)
            );
            let elev = Command::new("osascript")
                .args(["-e", &script])
                .output()
                .map_err(|e| Error::Install(e.to_string()))?;
            if !elev.status.success() {
                return Err(Error::Install(String::from_utf8_lossy(&elev.stderr).into()));
            }
        }
        if dest.exists() {
            let _ = fs::remove_dir_all(&dest);
            if dest.exists() {
                let script = format!(
                    "do shell script \"rm -rf {} && mv {} {}\" with administrator privileges",
                    shell_quote(&dest),
                    shell_quote(&staging),
                    shell_quote(&dest)
                );
                let elev = Command::new("osascript")
                    .args(["-e", &script])
                    .output()
                    .map_err(|e| Error::Install(e.to_string()))?;
                if !elev.status.success() {
                    return Err(Error::Install(String::from_utf8_lossy(&elev.stderr).into()));
                }
            } else {
                fs::rename(&staging, &dest).or_else(|_| {
                    Command::new("mv")
                        .arg(&staging)
                        .arg(&dest)
                        .status()
                        .map_err(Error::Io)
                        .and_then(|s| {
                            if s.success() {
                                Ok(())
                            } else {
                                Err(Error::Install("mv failed".into()))
                            }
                        })
                })?;
            }
        } else {
            fs::rename(&staging, &dest).map_err(Error::Io)?;
        }
        let _ = Command::new("open").arg(&dest).status();
        Ok(())
    })();
    let _ = Command::new("hdiutil")
        .args(["detach", "-quiet"])
        .arg(&mount)
        .status();
    let _ = fs::remove_dir_all(&mount);
    result?;
    // Encerra este processo para a cópia nova assumir.
    std::process::exit(0);
}

#[cfg(target_os = "macos")]
fn find_app_in(dir: &Path) -> Option<PathBuf> {
    let mut apps = vec![];
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("app") {
                apps.push(p);
            }
        }
    }
    apps.into_iter().next()
}

#[cfg(target_os = "macos")]
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

#[cfg(target_os = "windows")]
fn install_windows_nsis(installer: &Path) -> Result<(), Error> {
    // cargo-packager NSIS: /S = silencioso. currentUser → %LOCALAPPDATA%.
    let status = Command::new(installer)
        .arg("/S")
        .status()
        .map_err(|e| Error::Install(e.to_string()))?;
    if !status.success() {
        return Err(Error::Install(t::t("the installer failed").into()));
    }
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(platform::home);
    let candidates = [
        local.join("MultiDroid Preview").join("multidroid.exe"),
        local
            .join("Programs")
            .join("MultiDroid Preview")
            .join("multidroid.exe"),
        local
            .join("MultiDroid Preview")
            .join("MultiDroid Preview.exe"),
    ];
    let exe = candidates.into_iter().find(|p| p.exists()).ok_or_else(|| {
        Error::Install(t::t("installed, but the new executable wasn't found").into())
    })?;
    let _ = Command::new(&exe).spawn();
    std::process::exit(0);
}

#[cfg(target_os = "linux")]
fn install_linux(installer: &Path, available: &Available) -> Result<(), Error> {
    let name = available.asset.name.as_str();
    if name.ends_with(".AppImage") {
        return install_appimage(installer);
    }
    if name.ends_with(".deb") {
        // Não força root em silêncio: tenta pkexec; se falhar, abre o .deb.
        let pk = Command::new("pkexec")
            .args(["dpkg", "-i"])
            .arg(installer)
            .status();
        match pk {
            Ok(s) if s.success() => {
                let _ = Command::new("multidroid")
                    .spawn()
                    .or_else(|_| Command::new("/usr/bin/multidroid").spawn());
                std::process::exit(0);
            }
            _ => {
                let _ = Command::new("xdg-open").arg(installer).status();
                return Err(Error::Install(
                    t::t("open the .deb to finish installing, then restart MultiDroid").into(),
                ));
            }
        }
    }
    if name.ends_with(".rpm") {
        let pk = Command::new("pkexec")
            .args(["rpm", "-Uvh"])
            .arg(installer)
            .status();
        match pk {
            Ok(s) if s.success() => {
                let _ = Command::new("multidroid").spawn();
                std::process::exit(0);
            }
            _ => {
                let _ = Command::new("xdg-open").arg(installer).status();
                return Err(Error::Install(
                    t::t("open the .rpm to finish installing, then restart MultiDroid").into(),
                ));
            }
        }
    }
    Err(Error::Install(t::t("unsupported installer format").into()))
}

#[cfg(target_os = "linux")]
fn install_appimage(new_image: &Path) -> Result<(), Error> {
    let dest = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok());
    let Some(dest) = dest else {
        // Sem AppImage conhecido: deixa o arquivo pronto e pede para o usuário.
        let _ = Command::new("xdg-open")
            .arg(new_image.parent().unwrap_or(Path::new(".")))
            .status();
        return Err(Error::Install(
            t::t("AppImage downloaded — replace your current one and relaunch").into(),
        ));
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(new_image)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(new_image, perms)?;
    }
    let backup = dest.with_extension("AppImage.bak");
    if dest.exists() {
        let _ = fs::rename(&dest, &backup);
    }
    fs::copy(new_image, &dest).map_err(|e| {
        // Tenta restaurar.
        if backup.exists() {
            let _ = fs::rename(&backup, &dest);
        }
        Error::Io(e)
    })?;
    let _ = fs::remove_file(&backup);
    let _ = fs::remove_file(new_image);
    let child = Command::new(&dest).spawn().map_err(Error::Io)?;
    let _ = child;
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_preview_ordering() {
        assert!(is_newer("0.3.0-alpha.3", "0.3.0-alpha.4"));
        assert!(!is_newer("0.3.0-alpha.4", "0.3.0-alpha.3"));
        assert!(is_newer("0.3.0-alpha.4", "0.3.0"));
        assert!(!is_newer("0.3.0", "0.3.0-alpha.4"));
        assert!(is_newer("0.2", "0.2.1"));
        assert!(!is_newer("0.2.1", "0.2.1"));
        assert!(is_newer("0.2.1", "0.3.0-alpha.1"));
    }

    #[test]
    fn parse_short_versions() {
        assert_eq!(
            parse_version("v0.2"),
            Some(Ver {
                major: 0,
                minor: 2,
                patch: 0,
                pre: None
            })
        );
    }
}
