//! Motor leve do Linux (LINUX.md): Android em contêiner Redroid, direto no kernel
//! do PC, sem VM. Cada aparelho é um contêiner `multidroid-<nome>` com o /data numa
//! pasta do MultiDroid, para os apps e as contas sobreviverem a desligar. A tela e o
//! toque passam pelo adb e pelo protocolo do scrcpy (`scrcpy.rs`).
//!
//! Na fase 0 (CI, Linux x86_64) 8 Androids 13 ligaram juntos com ~1,2 GB cada.

use crate::devices::Kind;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Android 13 só 64 bits: o que a fase 0 mediu.
pub const IMAGE: &str = "redroid/redroid:13.0.0_64only-latest";
/// Porta do adb do aparelho i: 127.0.0.1:(PORTA0 + i).
pub const PORTA0: u16 = 5700;

/// Um aparelho leve, gravado em <dados>/leve/<nome>/leve.json.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Leve {
    #[serde(skip)]
    pub name: String,
    pub kind: String,
    pub image: String,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub port: u16,
}

impl Leve {
    pub fn kind(&self) -> Kind {
        Kind::ALL
            .iter()
            .copied()
            .find(|k| k.label() == self.kind)
            .unwrap_or(Kind::Celular)
    }

    /// Endereço do adb.
    pub fn serial(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    pub fn container(&self) -> String {
        container(&self.name)
    }
}

pub fn container(name: &str) -> String {
    format!("multidroid-{name}")
}

pub fn home(data: &Path) -> PathBuf {
    data.join("leve")
}

/// Tela do Redroid por tipo: menor que a do emulador, porque o vídeo é codificado
/// no próprio Android (a fase 0 usou 720×1600).
pub fn screen(kind: Kind) -> (u32, u32, u32) {
    match kind {
        Kind::Celular => (720, 1600, 320),
        Kind::Tablet => (1280, 800, 160),
        Kind::Dobravel => (1380, 1434, 320),
        Kind::Relogio => (454, 454, 320),
    }
}

/// Docker se houver, senão Podman. O Redroid precisa de contêiner privilegiado: o
/// Podman sem root sai na hora (código 129, medido no CI), então ele só vale
/// quando é o único e roda como root.
pub fn runtime() -> Option<&'static str> {
    ["docker", "podman"].into_iter().find(|r| {
        Command::new(r)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    })
}

/// O binder do kernel está disponível? (módulo binder_linux ou binderfs)
pub fn binder_ok() -> bool {
    Path::new("/dev/binder").exists()
        || Path::new("/dev/binderfs").exists()
        || std::fs::read_to_string("/proc/modules").is_ok_and(|m| m.contains("binder_linux"))
}

pub fn load(data: &Path) -> Vec<Leve> {
    let Ok(dirs) = std::fs::read_dir(home(data)) else {
        return vec![];
    };
    let mut v: Vec<Leve> = dirs
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let t = std::fs::read_to_string(e.path().join("leve.json")).ok()?;
            let mut l: Leve = serde_json::from_str(&t).ok()?;
            l.name = name;
            Some(l)
        })
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

/// Cria a pasta e o leve.json; o contêiner nasce na primeira vez que liga.
pub fn create(data: &Path, name: &str, kind: Kind) -> Result<Leve, String> {
    if !crate::devices::valid_name(name) {
        return Err("use só letras, números, ponto, hífen e sublinhado".into());
    }
    let dir = home(data).join(name);
    if dir.exists() {
        return Err("já existe um aparelho com esse nome".into());
    }
    let usadas: Vec<u16> = load(data).iter().map(|l| l.port).collect();
    let port = (PORTA0..PORTA0 + 500)
        .find(|p| !usadas.contains(p))
        .ok_or("sem porta livre")?;
    let (width, height, dpi) = screen(kind);
    let l = Leve {
        name: name.into(),
        kind: kind.label().into(),
        image: IMAGE.into(),
        width,
        height,
        dpi,
        port,
    };
    std::fs::create_dir_all(dir.join("data")).map_err(|e| e.to_string())?;
    std::fs::write(
        dir.join("leve.json"),
        serde_json::to_string_pretty(&l).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(l)
}

fn run(rt: &str, args: &[&str]) -> Result<String, String> {
    let o = Command::new(rt)
        .args(args)
        .output()
        .map_err(|e| format!("{rt}: {e}"))?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

/// Os argumentos do Redroid: tela, quadros e memória compartilhada por memfd
/// (os kernels novos não têm ashmem).
pub fn boot_args(l: &Leve) -> Vec<String> {
    vec![
        format!("androidboot.redroid_width={}", l.width),
        format!("androidboot.redroid_height={}", l.height),
        format!("androidboot.redroid_dpi={}", l.dpi),
        "androidboot.redroid_fps=30".into(),
        "androidboot.redroid_gpu_mode=guest".into(),
        "androidboot.use_memfd=true".into(),
    ]
}

/// Liga: cria o contêiner na primeira vez, depois só o inicia.
pub fn start(rt: &str, data: &Path, l: &Leve) -> Result<(), String> {
    let c = l.container();
    if run(rt, &["container", "inspect", &c]).is_ok() {
        return run(rt, &["start", &c]).map(|_| ());
    }
    let vol = format!("{}:/data", home(data).join(&l.name).join("data").display());
    let porta = format!("127.0.0.1:{}:5555", l.port);
    let mut args: Vec<String> = [
        "run",
        "-d",
        "--privileged",
        "--name",
        &c,
        "-v",
        &vol,
        "-p",
        &porta,
        &l.image,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    args.extend(boot_args(l));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run(rt, &refs).map(|_| ())
}

/// Desliga (o /data fica na pasta do aparelho).
pub fn stop(rt: &str, name: &str) -> Result<(), String> {
    run(rt, &["stop", "-t", "15", &container(name)]).map(|_| ())
}

/// Pausa e retoma o contêiner inteiro (cgroup freezer).
pub fn pause(rt: &str, name: &str, pausar: bool) -> Result<(), String> {
    run(
        rt,
        &[if pausar { "pause" } else { "unpause" }, &container(name)],
    )
    .map(|_| ())
}

/// Apaga o contêiner e a pasta do aparelho.
pub fn delete(rt: Option<&str>, data: &Path, name: &str) -> Result<(), String> {
    if let Some(rt) = rt {
        let _ = run(rt, &["rm", "-f", &container(name)]);
    }
    let dir = home(data).join(name);
    // O /data do Android é do usuário root do contêiner: apaga por dentro dele se precisar.
    if std::fs::remove_dir_all(&dir).is_err()
        && let Some(rt) = rt
    {
        // Com a própria imagem do Redroid, que já está baixada (o rm é o do Android).
        let d = format!("{}:/x", dir.display());
        let _ = run(
            rt,
            &[
                "run",
                "--rm",
                "-v",
                &d,
                "--entrypoint",
                "/system/bin/rm",
                IMAGE,
                "-rf",
                "/x/data",
            ],
        );
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Nomes dos aparelhos leves com contêiner rodando (e se está pausado).
pub fn running(rt: &str) -> Vec<(String, bool)> {
    run(
        rt,
        &[
            "ps",
            "--filter",
            "name=multidroid-",
            "--format",
            "{{.Names}} {{.Status}}",
        ],
    )
    .unwrap_or_default()
    .lines()
    .filter_map(|l| {
        let (n, st) = l.split_once(' ')?;
        Some((
            n.strip_prefix("multidroid-")?.to_string(),
            st.contains("Paused"),
        ))
    })
    .collect()
}

/// Memória de cada contêiner, em bytes (docker stats).
pub fn memory(rt: &str) -> Vec<(String, u64)> {
    run(
        rt,
        &[
            "stats",
            "--no-stream",
            "--format",
            "{{.Name}} {{.MemUsage}}",
        ],
    )
    .unwrap_or_default()
    .lines()
    .filter_map(|l| {
        let mut p = l.split_whitespace();
        let n = p.next()?.strip_prefix("multidroid-")?.to_string();
        Some((n, parse_size(p.next()?)?))
    })
    .collect()
}

/// "970.2MiB" → bytes.
pub fn parse_size(s: &str) -> Option<u64> {
    let i = s.find(|c: char| c.is_ascii_alphabetic())?;
    let (n, u) = s.split_at(i);
    let n: f64 = n.parse().ok()?;
    let m = match u {
        "B" => 1.0,
        "kB" | "KB" => 1e3,
        "KiB" => 1024.0,
        "MB" => 1e6,
        "MiB" => 1024.0 * 1024.0,
        "GB" => 1e9,
        "GiB" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((n * m) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cria_e_le_aparelhos_leves() {
        let t = tempfile::tempdir().unwrap();
        let a = create(t.path(), "Loja", Kind::Celular).unwrap();
        let b = create(t.path(), "Relo", Kind::Relogio).unwrap();
        assert!(create(t.path(), "Loja", Kind::Celular).is_err());
        assert_ne!(a.port, b.port);
        let v = load(t.path());
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].name, "Loja");
        assert_eq!(v[0].kind(), Kind::Celular);
        assert_eq!(v[1].kind(), Kind::Relogio);
        assert_eq!((v[0].width, v[0].height), (720, 1600));
        assert_eq!(v[0].container(), "multidroid-Loja");
        assert!(boot_args(&v[0]).contains(&"androidboot.use_memfd=true".to_string()));
        delete(None, t.path(), "Loja").unwrap();
        assert_eq!(load(t.path()).len(), 1);
    }

    #[test]
    fn tamanhos_do_docker_stats() {
        assert_eq!(parse_size("970.2MiB"), Some((970.2 * 1048576.0) as u64));
        assert_eq!(parse_size("1.104GiB"), Some((1.104 * 1073741824.0) as u64));
        assert_eq!(parse_size("12kB"), Some(12_000));
        assert_eq!(parse_size("x"), None);
    }
}
