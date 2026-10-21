//! Os aparelhos (AVDs) na pasta do MultiDroid e as operações que mudam essa pasta.
//! Porte de `Catalog.swift`: os mesmos dois arquivos que o emulador lê
//! (<nome>.ini e <nome>.avd/config.ini), com os campos que o avdmanager gravaria.
//! A diferença é a arquitetura, que vem do host: arm64 no Mac, x86_64 no PC.

use crate::catalog::{self, Host};
use crate::{ini, platform};
use std::fs;
use std::path::{Path, PathBuf};

/// Os tipos de aparelho (MAC.md §2.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Celular,
    Tablet,
    Dobravel,
    Relogio,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Celular, Kind::Tablet, Kind::Dobravel, Kind::Relogio];

    /// Nome que a interface mostra (em inglês, a língua principal do app).
    pub fn display(self) -> &'static str {
        match self {
            Kind::Celular => crate::idioma::t("Phone"),
            Kind::Tablet => crate::idioma::t("Tablet"),
            Kind::Dobravel => crate::idioma::t("Foldable"),
            Kind::Relogio => crate::idioma::t("Watch"),
        }
    }

    /// Identificador gravado nos aparelhos leves (`leve.rs`); não traduzir.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Celular => "Celular",
            Kind::Tablet => "Tablet",
            Kind::Dobravel => "Dobrável",
            Kind::Relogio => "Relógio",
        }
    }

    /// Perfil de hardware do emulador.
    pub fn hardware(self) -> &'static str {
        match self {
            Kind::Celular => "pixel_9",
            Kind::Tablet => "pixel_tablet",
            Kind::Dobravel => "pixel_9_pro_fold",
            Kind::Relogio => "wearos_large_round",
        }
    }

    pub fn ram_mb(self) -> u32 {
        2048
    }

    pub fn cores(self) -> u32 {
        if self == Kind::Relogio { 1 } else { 2 }
    }

    /// Tela (largura, altura, dpi). O tablet sai reduzido: a memória cresce com a resolução.
    pub fn lcd(self) -> (u32, u32, u32) {
        match self {
            Kind::Celular => (1080, 2424, 420),
            Kind::Tablet => (1280, 800, 160),
            Kind::Dobravel => (2076, 2152, 390),
            Kind::Relogio => (454, 454, 320),
        }
    }

    /// Memória estimada do processo, em GB (fase 0 no Mac).
    pub fn estimate_gb(self) -> f64 {
        match self {
            Kind::Celular => 4.5,
            Kind::Tablet => 5.5,
            Kind::Dobravel => 5.0,
            Kind::Relogio => 4.1,
        }
    }

    /// Tamanho padrão na grade livre, em células.
    pub fn grid_size(self) -> (i32, i32) {
        match self {
            Kind::Celular => (7, 14),
            Kind::Tablet => (12, 8),
            Kind::Dobravel => (9, 9),
            Kind::Relogio => (6, 6),
        }
    }

    pub fn guess(device: &str, image: &str) -> Kind {
        if image.contains("wear") || device.contains("wear") {
            Kind::Relogio
        } else if device.contains("tablet") {
            Kind::Tablet
        } else if device.contains("fold") {
            Kind::Dobravel
        } else {
            Kind::Celular
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    pub name: String,
    pub kind: Kind,
    /// "33", do caminho da imagem.
    pub api: String,
    /// "1080×2424".
    pub screen: String,
    pub dir: PathBuf,
    /// No formato de pacote: "system-images;android-33;default;arm64-v8a".
    pub image: String,
}

/// <dados>/avd e <dados>/sdk, como na 0.2.
pub fn avd_home() -> PathBuf {
    platform::data_dir().join("avd")
}

pub fn sdk_root() -> PathBuf {
    platform::data_dir().join("sdk")
}

/// Arquivos de execução que não podem ir junto numa cópia nem numa importação.
const LOCKS: [&str; 3] = [
    "hardware-qemu.ini.lock",
    "multiinstance.lock",
    "userdata-qemu.img.lock",
];

pub fn load(avd_home: &Path) -> Vec<Device> {
    let Ok(entries) = fs::read_dir(avd_home) else {
        return vec![];
    };
    let mut out: Vec<Device> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ini"))
        .filter_map(|ini_path| {
            let name = ini_path.file_stem()?.to_string_lossy().to_string();
            let top = ini::read(&ini_path);
            let dir = top
                .get("path")
                .map(PathBuf::from)
                .unwrap_or_else(|| avd_home.join(format!("{name}.avd")));
            let cfg = ini::read(&dir.join("config.ini"));
            if cfg.is_empty() {
                return None;
            }
            let sysdir = cfg.get("image.sysdir.1").cloned().unwrap_or_default();
            let api = sysdir
                .split(['/', '\\'])
                .find_map(|p| p.strip_prefix("android-"))
                .unwrap_or("?")
                .to_string();
            let g = |k: &str| cfg.get(k).cloned().unwrap_or_else(|| "?".into());
            Some(Device {
                kind: Kind::guess(cfg.get("hw.device.name").map_or("", |s| s), &sysdir),
                api,
                screen: format!("{}×{}", g("hw.lcd.width"), g("hw.lcd.height")),
                dir,
                image: sysdir
                    .split(['/', '\\'])
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(";"),
                name,
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Nome que vira pasta e argumento do emulador: letras, números, ponto, hífen, sublinhado.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

fn tag_display(tag: &str) -> &'static str {
    if tag.starts_with("android-wear") {
        "Wear OS"
    } else {
        match tag {
            "google_apis_playstore" => "Google Play",
            "google_apis" => "Google APIs",
            _ => "Default Android System Image",
        }
    }
}

fn top_ini(name: &str, dir: &Path, target: &str) -> String {
    format!(
        "avd.ini.encoding=UTF-8\npath={}\npath.rel=avd/{name}.avd\ntarget={target}\n\n",
        dir.display()
    )
}

/// O config.ini de um aparelho novo.
pub fn config(name: &str, kind: Kind, image: &str, host: Host) -> String {
    let (w, h, dpi) = kind.lcd();
    let wear = kind == Kind::Relogio;
    let tag = catalog::parts(image, host).map_or("default", |(_, t)| t);
    let display = tag_display(tag);
    let arch = if host.abi == "arm64-v8a" {
        "arm64"
    } else {
        "x86_64"
    };
    let sysdir = format!("{}/", image.replace(';', "/"));
    let yn = |b: bool| if b { "yes" } else { "no" };
    let mut c: Vec<(&str, String)> = vec![
        ("avd.ini.encoding", "UTF-8".into()),
        ("AvdId", name.into()),
        ("avd.ini.displayname", name.into()),
        ("abi.type", host.abi.into()),
        ("hw.cpu.arch", arch.into()),
        ("hw.cpu.ncore", kind.cores().to_string()),
        ("hw.ramSize", kind.ram_mb().to_string()),
        ("vm.heapSize", if wear { "48M" } else { "228M" }.into()),
        ("hw.device.manufacturer", "Google".into()),
        ("hw.device.name", kind.hardware().into()),
        ("hw.lcd.width", w.to_string()),
        ("hw.lcd.height", h.to_string()),
        ("hw.lcd.density", dpi.to_string()),
        ("hw.lcd.depth", "32".into()),
        (
            "hw.lcd.circular",
            if wear { "true" } else { "false" }.into(),
        ),
        ("hw.initialOrientation", "portrait".into()),
        ("hw.keyboard", "yes".into()),
        ("hw.mainKeys", yn(wear).into()),
        ("hw.gpu.enabled", "yes".into()),
        ("hw.gpu.mode", "host".into()),
        ("hw.accelerometer", "yes".into()),
        ("hw.gyroscope", "yes".into()),
        ("hw.battery", "yes".into()),
        ("hw.gps", "yes".into()),
        ("hw.sensors.proximity", "yes".into()),
        ("hw.sensors.light", "yes".into()),
        ("hw.sensors.orientation", "yes".into()),
        ("hw.camera.back", "emulated".into()),
        ("hw.camera.front", "none".into()),
        ("hw.audioInput", "no".into()),
        ("hw.audioOutput", "no".into()),
        ("hw.sdCard", "no".into()),
        (
            "disk.dataPartition.size",
            if wear { "2G" } else { "6G" }.into(),
        ),
        ("userdata.useQcow2", "no".into()),
        ("image.sysdir.1", sysdir),
        ("tag.id", tag.into()),
        ("tag.ids", tag.into()),
        ("tag.display", display.into()),
        ("tag.displaynames", display.into()),
        ("PlayStore.enabled", "no".into()),
        ("showDeviceFrame", "no".into()),
        ("fastboot.forceColdBoot", "no".into()),
        ("fastboot.forceFastBoot", "yes".into()),
        ("firstboot.bootFromLocalSnapshot", "yes".into()),
        ("firstboot.saveToLocalSnapshot", "yes".into()),
    ];
    if kind == Kind::Dobravel {
        // Dobradiça do perfil de dobrável, como o avdmanager descreve.
        for (k, v) in [
            ("hw.displayRegion.0.1.width", "1080"),
            ("hw.displayRegion.0.1.height", "2424"),
            ("hw.sensor.hinge", "yes"),
            ("hw.sensor.hinge.count", "1"),
            ("hw.sensor.hinge.type", "1"),
            ("hw.sensor.hinge.sub_type", "1"),
            ("hw.sensor.hinge.ranges", "0-180"),
            ("hw.sensor.hinge.defaults", "180"),
            ("hw.sensor.hinge.areas", "1038-0-0-2152"),
            (
                "hw.sensor.hinge_angles_posture_definitions",
                "0-30, 30-150, 150-180",
            ),
            ("hw.sensor.posture_list", "1, 2, 3"),
            ("hw.sensor.hinge.fold_to_displayRegion.0.1_at_posture", "1"),
            ("hw.sensor.hinge.resizable.config", "1"),
        ] {
            c.push((k, v.into()));
        }
    }
    c.iter().map(|(k, v)| format!("{k}={v}\n")).collect()
}

/// Cria um aparelho. O emulador cria o disco do usuário a partir da imagem no primeiro boot.
pub fn create(
    avd_home: &Path,
    sdk: &Path,
    name: &str,
    kind: Kind,
    image: &str,
    host: Host,
) -> Result<(), String> {
    if !valid_name(name) {
        return Err(crate::idioma::t("use only letters, numbers, dot, hyphen and underscore").into());
    }
    let dir = avd_home.join(format!("{name}.avd"));
    let top = avd_home.join(format!("{name}.ini"));
    if dir.exists() || top.exists() {
        return Err(crate::idioma::t("a device with that name already exists").into());
    }
    if catalog::installed(image, sdk).is_none() {
        return Err(format!(
            "baixe antes a imagem {} em Imagens",
            catalog::title(image, host)
        ));
    }
    let api = catalog::parts(image, host).map_or("33", |(a, _)| a);
    let r = (|| -> std::io::Result<()> {
        fs::create_dir_all(&dir)?;
        fs::write(&top, top_ini(name, &dir, &format!("android-{api}")))?;
        fs::write(dir.join("config.ini"), config(name, kind, image, host))
    })();
    r.map_err(|e| {
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_file(&top);
        e.to_string()
    })
}

/// Duplica: no Mac com clone do APFS, no Linux com reflink quando o disco aceita,
/// no Windows cópia simples. Sem locks e com o nome novo nos dois .ini.
pub fn duplicate(avd_home: &Path, d: &Device, new_name: &str) -> Result<(), String> {
    if !valid_name(new_name) {
        return Err(crate::idioma::t("use only letters, numbers, dot, hyphen and underscore").into());
    }
    let new_dir = avd_home.join(format!("{new_name}.avd"));
    let new_top = avd_home.join(format!("{new_name}.ini"));
    if new_dir.exists() || new_top.exists() {
        return Err(crate::idioma::t("a device with that name already exists").into());
    }
    platform::clone_dir(&d.dir, &new_dir).map_err(|e| e.to_string())?;
    for junk in LOCKS {
        let _ = fs::remove_file(new_dir.join(junk));
    }
    let cfg_path = new_dir.join("config.ini");
    if let Ok(cfg) = fs::read_to_string(&cfg_path) {
        let cfg = cfg
            .lines()
            .map(|l| match l.split_once('=') {
                Some(("AvdId", _)) => format!("AvdId={new_name}"),
                Some(("avd.ini.displayname", _)) => format!("avd.ini.displayname={new_name}"),
                _ => l.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(&cfg_path, cfg).map_err(|e| e.to_string())?;
    }
    let target = ini::read(&avd_home.join(format!("{}.ini", d.name)))
        .remove("target")
        .unwrap_or_else(|| format!("android-{}", d.api));
    fs::write(&new_top, top_ini(new_name, &new_dir, &target)).map_err(|e| e.to_string())
}

pub fn delete(avd_home: &Path, d: &Device) -> Result<(), String> {
    fs::remove_dir_all(&d.dir).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(avd_home.join(format!("{}.ini", d.name)));
    Ok(())
}

/// Aparelhos do Android Studio (~/.android/avd), para importar.
pub fn studio_avd_home() -> PathBuf {
    platform::home().join(".android").join("avd")
}

/// Traz um aparelho do Android Studio para a pasta do MultiDroid (move: no mesmo
/// disco é instantâneo). Leva o estado salvo e os apps instalados.
pub fn import_from_studio(studio: &Path, avd_home: &Path, name: &str) -> Result<(), String> {
    let top = ini::read(&studio.join(format!("{name}.ini")));
    let src = top
        .get("path")
        .map(PathBuf::from)
        .unwrap_or_else(|| studio.join(format!("{name}.avd")));
    let dst = avd_home.join(format!("{name}.avd"));
    if dst.exists() {
        return Err(crate::idioma::tf("{name} already exists in MultiDroid", &[("name", &name)]));
    }
    fs::create_dir_all(avd_home).map_err(|e| e.to_string())?;
    if fs::rename(&src, &dst).is_err() {
        // Outro disco: copia e apaga a origem só depois.
        platform::clone_dir(&src, &dst).map_err(|e| e.to_string())?;
        fs::remove_dir_all(&src).map_err(|e| e.to_string())?;
    }
    for junk in LOCKS {
        let _ = fs::remove_file(dst.join(junk));
    }
    let target = top.get("target").map_or("android-33", |s| s);
    fs::write(
        avd_home.join(format!("{name}.ini")),
        top_ini(name, &dst, target),
    )
    .map_err(|e| e.to_string())?;
    let _ = fs::remove_file(studio.join(format!("{name}.ini")));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMG: &str = "system-images;android-33;default;arm64-v8a";

    fn sdk_com_imagem(root: &Path) -> PathBuf {
        let sdk = root.join("sdk");
        let d = catalog::dir(IMG, &sdk);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("source.properties"), "Pkg.Revision=2\n").unwrap();
        sdk
    }

    #[test]
    fn nomes_validos() {
        assert!(valid_name("Personal"));
        assert!(valid_name("loja-2_b.x"));
        assert!(!valid_name(""));
        assert!(!valid_name("com espaço"));
        assert!(!valid_name("../fora"));
        assert!(!valid_name(".oculto"));
    }

    #[test]
    fn cria_le_duplica_e_apaga() {
        let t = tempfile::tempdir().unwrap();
        let sdk = sdk_com_imagem(t.path());
        let avd = t.path().join("avd");
        create(&avd, &sdk, "Loja", Kind::Celular, IMG, Host::MAC_ARM).unwrap();
        assert!(create(&avd, &sdk, "Loja", Kind::Celular, IMG, Host::MAC_ARM).is_err());
        let list = load(&avd);
        assert_eq!(list.len(), 1);
        let d = &list[0];
        assert_eq!(
            (d.name.as_str(), d.kind, d.api.as_str()),
            ("Loja", Kind::Celular, "33")
        );
        assert_eq!(d.screen, "1080×2424");
        assert_eq!(d.image, IMG);

        fs::write(d.dir.join("multiinstance.lock"), "").unwrap();
        duplicate(&avd, d, "Loja2").unwrap();
        let list = load(&avd);
        assert_eq!(list.len(), 2);
        let c = &list[1];
        assert_eq!(c.name, "Loja2");
        assert!(!c.dir.join("multiinstance.lock").exists());
        let cfg = ini::read(&c.dir.join("config.ini"));
        assert_eq!(cfg["AvdId"], "Loja2");
        assert_eq!(cfg["avd.ini.displayname"], "Loja2");
        let top = ini::read(&avd.join("Loja2.ini"));
        assert_eq!(top["target"], "android-33");
        assert_eq!(PathBuf::from(&top["path"]), c.dir);

        delete(&avd, d).unwrap();
        assert_eq!(load(&avd).len(), 1);
    }

    #[test]
    fn sem_imagem_nao_cria() {
        let t = tempfile::tempdir().unwrap();
        let avd = t.path().join("avd");
        let e = create(
            &avd,
            &t.path().join("sdk"),
            "X",
            Kind::Celular,
            IMG,
            Host::MAC_ARM,
        )
        .unwrap_err();
        assert!(e.starts_with("baixe antes"), "{e}");
        assert!(load(&avd).is_empty());
    }

    #[test]
    fn config_segue_o_host() {
        let x86 = "system-images;android-33;android-wear;x86_64";
        let c = ini::parse(&config("W", Kind::Relogio, x86, Host::LINUX_X64));
        assert_eq!(c["abi.type"], "x86_64");
        assert_eq!(c["hw.cpu.arch"], "x86_64");
        assert_eq!(c["tag.display"], "Wear OS");
        assert_eq!(c["hw.lcd.circular"], "true");
        assert_eq!(
            c["image.sysdir.1"],
            "system-images/android-33/android-wear/x86_64/"
        );
        let f = ini::parse(&config("F", Kind::Dobravel, IMG, Host::MAC_ARM));
        assert_eq!(f["hw.cpu.arch"], "arm64");
        assert_eq!(f["hw.sensor.hinge"], "yes");
    }

    #[test]
    fn importa_do_android_studio() {
        let t = tempfile::tempdir().unwrap();
        let studio = t.path().join("studio");
        let avd = t.path().join("avd");
        let src = studio.join("Velho.avd");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("config.ini"),
            "hw.device.name=pixel_tablet\nimage.sysdir.1=system-images/android-34/google_apis/arm64-v8a/\n",
        )
        .unwrap();
        fs::write(
            studio.join("Velho.ini"),
            format!("path={}\ntarget=android-34\n", src.display()),
        )
        .unwrap();
        import_from_studio(&studio, &avd, "Velho").unwrap();
        assert!(!src.exists() && !studio.join("Velho.ini").exists());
        let list = load(&avd);
        assert_eq!(list[0].kind, Kind::Tablet);
        assert_eq!(list[0].api, "34");
    }
}
