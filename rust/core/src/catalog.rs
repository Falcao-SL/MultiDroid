//! Catálogo dos pacotes que o MultiDroid instala: o emulador e as imagens de sistema.
//! Porte de `Images` e `ImageStore.parse` do app SwiftUI (mac/Sources/MultiDroid/Images.swift).
//!
//! As listas vêm do repositório oficial do Android; o MultiDroid nunca embute nem
//! redistribui imagem: cada pessoa baixa e aceita a licença.

use crate::ini;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const EMULATOR: &str = "emulator";
pub const REPO: &str = "https://dl.google.com/android/repository/";
/// Tipos de imagem oferecidos, cada um com a sua lista no repositório.
pub const TAGS: [&str; 4] = [
    "google_apis_playstore",
    "google_apis",
    "default",
    "android-wear",
];
/// Do Android 11 (API 30) em diante.
pub const MIN_API: f64 = 30.0;

/// O sistema em que o app roda decide a arquitetura das imagens e o host do emulador.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Host {
    /// "macosx", "windows" ou "linux", como no XML do repositório.
    pub os: &'static str,
    /// "aarch64" ou "x64".
    pub arch: &'static str,
    /// ABI das imagens nativas desse host: arm64-v8a no Apple Silicon, x86_64 no PC.
    pub abi: &'static str,
}

impl Host {
    pub const MAC_ARM: Host = Host {
        os: "macosx",
        arch: "aarch64",
        abi: "arm64-v8a",
    };
    pub const WINDOWS_X64: Host = Host {
        os: "windows",
        arch: "x64",
        abi: "x86_64",
    };
    pub const LINUX_X64: Host = Host {
        os: "linux",
        arch: "x64",
        abi: "x86_64",
    };

    /// O host deste binário.
    pub fn current() -> Host {
        if cfg!(target_os = "macos") {
            Host::MAC_ARM
        } else if cfg!(target_os = "windows") {
            Host::WINDOWS_X64
        } else {
            Host::LINUX_X64
        }
    }
}

/// Pasta de cada lista: a das imagens sem Google (tag "default") fica em sys-img/android.
pub fn list_dir(tag: &str) -> &str {
    if tag == "default" { "android" } else { tag }
}

pub fn tag_list_url(tag: &str) -> String {
    format!("{REPO}sys-img/{}/sys-img2-3.xml", list_dir(tag))
}

/// "system-images;android-34;google_apis;arm64-v8a" → ("34", "google_apis"); None para o
/// que não é imagem de sistema que o MultiDroid gerencia neste host.
pub fn parts(path: &str, host: Host) -> Option<(&str, &str)> {
    let p: Vec<&str> = path.split(';').collect();
    if p.len() != 4 || p[0] != "system-images" || p[3] != host.abi {
        return None;
    }
    let api = p[1].strip_prefix("android-")?;
    let tag = p[2];
    if !TAGS
        .iter()
        .any(|t| tag == *t || tag.starts_with(&format!("{t}-")))
    {
        return None;
    }
    Some((api, tag))
}

pub fn is_wear(path: &str, host: Host) -> bool {
    parts(path, host).is_some_and(|(_, tag)| tag.starts_with("android-wear"))
}

/// "36.1" → 36.1, para ordenar.
pub fn api_number(path: &str, host: Host) -> f64 {
    parts(path, host)
        .and_then(|(api, _)| api.parse().ok())
        .unwrap_or(0.0)
}

/// Versão do Android que a pessoa reconhece: API 33 é o Android 13; "36.1" vira "16 (API 36.1)".
pub fn android_version(api: &str) -> String {
    let mut nums = api.split('.');
    let Some(n) = nums.next().and_then(|n| n.parse::<u32>().ok()) else {
        return api.to_string();
    };
    let v = match n {
        30 => "11".to_string(),
        31 => "12".to_string(),
        32 => "12L".to_string(),
        n if n >= 33 => (n - 20).to_string(),
        _ => api.to_string(),
    };
    let minor: u32 = nums.next().and_then(|m| m.parse().ok()).unwrap_or(0);
    if minor == 0 {
        v
    } else {
        format!("{v} (API {api})")
    }
}

/// Nome que a interface mostra (o mesmo texto do app SwiftUI).
pub fn title(path: &str, host: Host) -> String {
    if path == EMULATOR {
        return "Emulador do Android".into();
    }
    let Some((api, tag)) = parts(path, host) else {
        return path.into();
    };
    let v = android_version(api);
    if tag.starts_with("android-wear") {
        return format!("Wear OS (Android {v})");
    }
    match tag {
        "google_apis_playstore" => format!("Android {v} com Google Play"),
        "google_apis" => format!("Android {v} com APIs do Google"),
        _ => format!("Android {v} sem Google (AOSP)"),
    }
}

/// Uma linha sob o título: para que serve e o que vem junto.
pub fn detail(path: &str, host: Host) -> String {
    if path == EMULATOR {
        return "Roda os aparelhos. Obrigatório.".into();
    }
    let Some((api, tag)) = parts(path, host) else {
        return String::new();
    };
    let uso = if tag.starts_with("android-wear") {
        "Relógio"
    } else {
        "Celular, tablet e dobrável"
    };
    let extra = match tag {
        "google_apis_playstore" => " · com Play Store",
        "google_apis" => " · sem Play Store; apps por APK",
        "default" => " · sem nenhum app do Google; apps por APK",
        _ => "",
    };
    format!("{uso} · API {api}{extra}")
}

/// A imagem sugerida ao criar cada tipo: Android 13, a mais leve medida.
pub fn default_image(wear: bool, host: Host) -> String {
    let tag = if wear {
        "android-wear"
    } else {
        "google_apis_playstore"
    };
    format!("system-images;android-33;{tag};{}", host.abi)
}

/// O que a fase 0 mediu (MAC.md §3) e vale avisar antes de baixar.
pub fn note(path: &str, host: Host) -> Option<String> {
    let (api, tag) = parts(path, host)?;
    let n: u32 = api.split('.').next()?.parse().ok()?;
    let wear = tag.starts_with("android-wear");
    Some(match (wear, tag, n) {
        (false, "google_apis_playstore", 33) => {
            "Recomendada: a mais leve, com Play Store (o emulador usa 2 GB de RAM; ~4,6 GB por celular).".into()
        }
        (false, "google_apis_playstore", 36) => {
            "Recomendada: a mais nova que ainda é leve, com Play Store (2,5 GB de RAM; ~5,3 GB por celular).".into()
        }
        (true, _, 33) => "Recomendada: a única versão do Wear OS que liga neste emulador.".into(),
        (true, _, n) if n > 33 => "Não ligou neste emulador nos testes do MultiDroid.".into(),
        (_, _, n) if n >= 37 => "O emulador força 4 GB de RAM nesta versão: ~6,5 GB por celular.".into(),
        (_, _, n) if n >= 34 => {
            "O emulador força 2,5 GB de RAM nesta versão: ~0,7 GB a mais por celular que o Android 13.".into()
        }
        _ => return None,
    })
}

/// Um pacote como o repositório o oferece.
#[derive(Clone, Debug, PartialEq)]
pub struct RemotePackage {
    pub path: String,
    pub revision: String,
    pub size: u64,
    pub sha1: String,
    pub url: String,
    pub license: String,
    pub license_text: String,
}

fn child<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

fn text_of(n: roxmltree::Node, name: &str) -> Option<String> {
    child(n, name)
        .and_then(|c| c.text())
        .map(|t| t.trim().to_string())
}

/// Lê de uma lista do repositório o pacote `path`, no canal estável e com o arquivo
/// do host. `base` é a pasta da lista, para montar a URL do arquivo.
pub fn parse(
    xml: &str,
    path: &str,
    base: &str,
    host: Host,
) -> Result<Option<RemotePackage>, roxmltree::Error> {
    let doc = roxmltree::Document::parse(xml)?;
    let licenses: BTreeMap<&str, String> = doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "license")
        .filter_map(|n| Some((n.attribute("id")?, n.text().unwrap_or("").to_string())))
        .collect();
    for el in doc.descendants().filter(|n| {
        n.is_element()
            && n.tag_name().name() == "remotePackage"
            && n.attribute("path") == Some(path)
    }) {
        // Só o canal estável (channel-0).
        let channel = child(el, "channelRef")
            .and_then(|c| c.attribute("ref"))
            .unwrap_or("channel-0");
        if channel != "channel-0" {
            continue;
        }
        let Some(archives) = child(el, "archives") else {
            continue;
        };
        for archive in archives
            .children()
            .filter(|c| c.is_element() && c.tag_name().name() == "archive")
        {
            if let Some(os) = text_of(archive, "host-os")
                && os != host.os
            {
                continue;
            }
            if let Some(arch) = text_of(archive, "host-arch")
                && arch != host.arch
            {
                continue;
            }
            let Some(complete) = child(archive, "complete") else {
                continue;
            };
            let (Some(file), Some(size), Some(sha1)) = (
                text_of(complete, "url"),
                text_of(complete, "size").and_then(|s| s.parse().ok()),
                text_of(complete, "checksum"),
            ) else {
                continue;
            };
            let rev = child(el, "revision")
                .map(|r| {
                    ["major", "minor", "micro"]
                        .iter()
                        .filter_map(|k| text_of(r, k))
                        .collect::<Vec<_>>()
                        .join(".")
                })
                .unwrap_or_default();
            let license = child(el, "uses-license")
                .and_then(|u| u.attribute("ref"))
                .unwrap_or("android-sdk-license")
                .to_string();
            let license_text = licenses.get(license.as_str()).cloned().unwrap_or_default();
            return Ok(Some(RemotePackage {
                path: path.to_string(),
                revision: rev,
                size,
                sha1,
                url: format!("{base}{file}"),
                license,
                license_text,
            }));
        }
    }
    Ok(None)
}

/// Todas as imagens de uma lista que o MultiDroid oferece neste host.
pub fn images_in(
    xml: &str,
    base: &str,
    host: Host,
) -> Result<Vec<RemotePackage>, roxmltree::Error> {
    let doc = roxmltree::Document::parse(xml)?;
    let mut paths: Vec<&str> = doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "remotePackage")
        .filter_map(|n| n.attribute("path"))
        .filter(|p| parts(p, host).is_some() && api_number(p, host) >= MIN_API)
        .collect();
    paths.sort();
    paths.dedup();
    let mut out = Vec::new();
    for p in paths {
        if let Some(pkg) = parse(xml, p, base, host)? {
            out.push(pkg);
        }
    }
    Ok(out)
}

/// "system-images;android-33;android-wear;arm64-v8a" → <root>/system-images/android-33/android-wear/arm64-v8a.
pub fn dir(path: &str, root: &Path) -> PathBuf {
    path.split(';')
        .fold(root.to_path_buf(), |d, part| d.join(part))
}

/// Versão instalada (Pkg.Revision do source.properties), ou None. O emulador só conta
/// se o executável estiver lá.
pub fn installed(path: &str, root: &Path) -> Option<String> {
    let d = dir(path, root);
    if path == EMULATOR {
        let exe = if cfg!(windows) {
            "emulator.exe"
        } else {
            "emulator"
        };
        if !d.join(exe).is_file() {
            return None;
        }
    }
    ini::read(&d.join("source.properties")).remove("Pkg.Revision")
}

/// Imagens instaladas numa pasta de SDK (a do MultiDroid ou a do Android Studio).
pub fn scan_installed(root: &Path, host: Host) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let base = root.join("system-images");
    let Ok(apis) = std::fs::read_dir(&base) else {
        return out;
    };
    for api in apis.flatten() {
        let Ok(tags) = std::fs::read_dir(api.path()) else {
            continue;
        };
        for tag in tags.flatten() {
            let path = format!(
                "system-images;{};{};{}",
                api.file_name().to_string_lossy(),
                tag.file_name().to_string_lossy(),
                host.abi
            );
            if parts(&path, host).is_some()
                && let Some(rev) = installed(&path, root)
            {
                out.insert(path, rev);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn notas_e_detalhes_como_na_02() {
        let h = Host::MAC_ARM;
        let play33 = default_image(false, h);
        assert_eq!(
            play33,
            "system-images;android-33;google_apis_playstore;arm64-v8a"
        );
        assert!(
            note(&play33, h)
                .unwrap()
                .starts_with("Recomendada: a mais leve")
        );
        assert!(
            note("system-images;android-34;android-wear;arm64-v8a", h)
                .unwrap()
                .starts_with("Não ligou")
        );
        assert!(
            note("system-images;android-37;default;arm64-v8a", h)
                .unwrap()
                .contains("4 GB")
        );
        assert_eq!(note("system-images;android-31;default;arm64-v8a", h), None);
        assert_eq!(
            detail("system-images;android-33;android-wear;arm64-v8a", h),
            "Relógio · API 33"
        );
        assert_eq!(
            default_image(true, Host::LINUX_X64),
            "system-images;android-33;android-wear;x86_64"
        );
    }

    use super::*;

    const MAC: Host = Host::MAC_ARM;

    /// Lista de exemplo com a estrutura das listas reais do repositório (os arquivos do
    /// Google não entram no repositório do MultiDroid).
    const LISTA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sys-img:sdk-sys-img xmlns:sys-img="http://schemas.android.com/sdk/android/repo/sys-img2/03">
  <license id="exemplo-licenca" type="text">Texto da licença de exemplo.</license>
  <channel id="channel-0">stable</channel>
  <channel id="channel-2">dev</channel>
  <remotePackage path="system-images;android-33;android-wear;arm64-v8a">
    <revision><major>4</major></revision>
    <uses-license ref="exemplo-licenca"/>
    <channelRef ref="channel-0"/>
    <archives>
      <archive>
        <complete><size>100</size><checksum>aaaa</checksum><url>arm64-v8a-33_r04.zip</url></complete>
      </archive>
    </archives>
  </remotePackage>
  <remotePackage path="system-images;android-36.1;android-wear;arm64-v8a">
    <revision><major>1</major></revision>
    <channelRef ref="channel-2"/>
    <archives><archive><complete><size>1</size><checksum>bbbb</checksum><url>dev.zip</url></complete></archive></archives>
  </remotePackage>
  <remotePackage path="system-images;android-33;android-wear;x86_64">
    <revision><major>4</major></revision>
    <archives><archive><complete><size>1</size><checksum>cccc</checksum><url>x86.zip</url></complete></archive></archives>
  </remotePackage>
  <remotePackage path="system-images;android-28;android-wear;arm64-v8a">
    <revision><major>9</major></revision>
    <archives><archive><complete><size>1</size><checksum>dddd</checksum><url>velha.zip</url></complete></archive></archives>
  </remotePackage>
</sys-img:sdk-sys-img>"#;

    const EMULADORES: &str = r#"<?xml version="1.0"?>
<sdk:sdk-repository xmlns:sdk="http://schemas.android.com/sdk/android/repo/repository2/03">
  <license id="android-sdk-license" type="text">Licença do SDK de exemplo.</license>
  <remotePackage path="emulator">
    <revision><major>36</major><minor>1</minor><micro>9</micro></revision>
    <channelRef ref="channel-0"/>
    <archives>
      <archive><complete><size>10</size><checksum>mac1</checksum><url>emulator-darwin_aarch64.zip</url></complete><host-os>macosx</host-os><host-arch>aarch64</host-arch></archive>
      <archive><complete><size>20</size><checksum>lin1</checksum><url>emulator-linux_x64.zip</url></complete><host-os>linux</host-os><host-arch>x64</host-arch></archive>
      <archive><complete><size>30</size><checksum>win1</checksum><url>emulator-windows_x64.zip</url></complete><host-os>windows</host-os><host-arch>x64</host-arch></archive>
    </archives>
  </remotePackage>
</sdk:sdk-repository>"#;

    #[test]
    fn partes_versao_e_titulo_como_no_swift() {
        assert_eq!(
            parts("system-images;android-34;google_apis;arm64-v8a", MAC),
            Some(("34", "google_apis"))
        );
        assert_eq!(
            parts("system-images;android-34;google_apis;x86_64", MAC),
            None
        );
        assert_eq!(
            parts("system-images;android-34;google_apis_ps16k;arm64-v8a", MAC),
            None
        );
        assert_eq!(
            parts(
                "system-images;android-36;android-wear-signed;arm64-v8a",
                MAC
            ),
            Some(("36", "android-wear-signed"))
        );
        assert_eq!(android_version("33"), "13");
        assert_eq!(android_version("32"), "12L");
        assert_eq!(android_version("36.1"), "16 (API 36.1)");
        assert_eq!(
            title("system-images;android-33;default;arm64-v8a", MAC),
            "Android 13 sem Google (AOSP)"
        );
        assert_eq!(
            title("system-images;android-33;android-wear;arm64-v8a", MAC),
            "Wear OS (Android 13)"
        );
        assert_eq!(title(EMULATOR, MAC), "Emulador do Android");
        assert_eq!(list_dir("default"), "android");
        assert!(tag_list_url("default").ends_with("sys-img/android/sys-img2-3.xml"));
    }

    #[test]
    fn le_o_pacote_estavel_com_licenca_e_url() {
        let base = "https://exemplo/sys-img/android-wear/";
        let p = parse(
            LISTA,
            "system-images;android-33;android-wear;arm64-v8a",
            base,
            MAC,
        )
        .unwrap()
        .unwrap();
        assert_eq!(p.revision, "4");
        assert_eq!(p.size, 100);
        assert_eq!(p.sha1, "aaaa");
        assert_eq!(
            p.url,
            "https://exemplo/sys-img/android-wear/arm64-v8a-33_r04.zip"
        );
        assert_eq!(p.license, "exemplo-licenca");
        assert_eq!(p.license_text, "Texto da licença de exemplo.");
        // Fora do canal estável: some.
        assert_eq!(
            parse(
                LISTA,
                "system-images;android-36.1;android-wear;arm64-v8a",
                base,
                MAC
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn oferece_so_o_que_serve_no_host() {
        let base = "https://exemplo/";
        let mac: Vec<String> = images_in(LISTA, base, MAC)
            .unwrap()
            .into_iter()
            .map(|p| p.path)
            .collect();
        // Sem a do canal dev, sem a x86_64 e sem a anterior ao Android 11.
        assert_eq!(mac, vec!["system-images;android-33;android-wear;arm64-v8a"]);
        let pc: Vec<String> = images_in(LISTA, base, Host::LINUX_X64)
            .unwrap()
            .into_iter()
            .map(|p| p.path)
            .collect();
        assert_eq!(pc, vec!["system-images;android-33;android-wear;x86_64"]);
    }

    #[test]
    fn emulador_do_host_certo() {
        let b = REPO;
        let mac = parse(EMULADORES, EMULATOR, b, MAC).unwrap().unwrap();
        assert_eq!(
            (mac.sha1.as_str(), mac.revision.as_str()),
            ("mac1", "36.1.9")
        );
        assert!(mac.url.ends_with("emulator-darwin_aarch64.zip"));
        assert_eq!(
            parse(EMULADORES, EMULATOR, b, Host::WINDOWS_X64)
                .unwrap()
                .unwrap()
                .sha1,
            "win1"
        );
        assert_eq!(
            parse(EMULADORES, EMULATOR, b, Host::LINUX_X64)
                .unwrap()
                .unwrap()
                .license_text,
            "Licença do SDK de exemplo."
        );
    }

    #[test]
    fn acha_as_imagens_instaladas() {
        let root = tempfile::tempdir().unwrap();
        let wear = "system-images;android-33;android-wear;arm64-v8a";
        let d = dir(wear, root.path());
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("source.properties"), "Pkg.Revision=4\n").unwrap();
        // Pasta sem source.properties não conta como instalada.
        std::fs::create_dir_all(dir(
            "system-images;android-33;default;arm64-v8a",
            root.path(),
        ))
        .unwrap();
        let found = scan_installed(root.path(), MAC);
        assert_eq!(found.len(), 1);
        assert_eq!(found[wear], "4");
        assert_eq!(installed(EMULATOR, root.path()), None);
    }
}
