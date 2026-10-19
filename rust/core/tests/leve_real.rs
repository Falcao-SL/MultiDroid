//! Motor leve de verdade (etapa R-6): liga um Android em Redroid, espera o boot
//! pelo adb, abre a sessão do scrcpy, decodifica quadros, manda toque, texto e
//! tecla, e confere que o /data sobrevive a desligar. Só no Linux, com Docker ou
//! Podman e o binder no kernel. Não roda por padrão:
//!   cargo test -p multidroid-core --test leve_real -- --ignored --nocapture

#![cfg(target_os = "linux")]

use multidroid_core::adb::{Adb, PLATFORM_TOOLS};
use multidroid_core::catalog::{self, Host, REPO};
use multidroid_core::devices::Kind;
use multidroid_core::{install, leve, scrcpy};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn adb() -> Adb {
    let sdk = std::env::var_os("MULTIDROID_SDK")
        .map(PathBuf::from)
        .unwrap_or_else(multidroid_core::devices::sdk_root);
    if Adb::from_sdk(&sdk).is_none() {
        let url = format!("{REPO}repository2-3.xml");
        let xml = ureq::get(&url).call().unwrap().into_string().unwrap();
        let pkg = catalog::parse(&xml, PLATFORM_TOOLS, REPO, Host::current())
            .unwrap()
            .expect("platform-tools no repositório");
        install::accept(&pkg, &sdk).unwrap();
        install::install(&pkg, &sdk, &sdk.join(".work"), &Default::default(), |_| {}).unwrap();
    }
    Adb::from_sdk(&sdk).expect("adb instalado")
}

/// Contêineres, registro do Redroid e aparelhos do adb, para entender um boot que falhou.
fn diagnostico(rt: &str, adb: &Adb, l: &leve::Leve) -> String {
    let run = |exe: &std::path::Path, args: &[&str]| {
        std::process::Command::new(exe)
            .args(args)
            .output()
            .map(|o| {
                format!(
                    "{}{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                )
            })
            .unwrap_or_default()
    };
    let rt = std::path::Path::new(rt);
    format!(
        "--- ps\n{}--- logs\n{}--- adb\n{}",
        run(rt, &["ps", "-a"]),
        run(rt, &["logs", "--tail", "80", &l.container()]),
        run(&adb.exe, &["devices"])
    )
}

#[test]
#[ignore]
fn liga_mostra_a_tela_e_guarda_os_dados() {
    let rt = leve::runtime().expect("sem Docker nem Podman");
    assert!(leve::binder_ok(), "o binder não está no kernel");
    let adb = adb();
    let t = tempfile::tempdir().unwrap();
    let data = t.path();
    let l = leve::create(data, "R6Teste", Kind::Celular).unwrap();
    let t0 = Instant::now();
    leve::start(rt, data, &l).unwrap();
    if let Err(e) = adb.wait_boot(&l.serial(), Duration::from_secs(240)) {
        panic!("{e}\n{}", diagnostico(rt, &adb, &l));
    }
    eprintln!("ligou em {:?}", t0.elapsed());

    let jar = scrcpy::ensure_server(&data.join("scrcpy")).unwrap();
    let mut s = scrcpy::Session::start(&adb, &l.serial(), &jar, 27183, 800).unwrap();
    eprintln!("scrcpy: {} {}×{}", s.device, s.width, s.height);
    assert_eq!(s.height, 800);

    let mut ctl = s.controller().unwrap();
    let (w, h) = (s.width as u16, s.height as u16);
    // Toques e uma tecla para a tela mudar e o codificador mandar quadros novos.
    std::thread::spawn(move || {
        use std::io::Write;
        for i in 0..20 {
            std::thread::sleep(Duration::from_millis(400));
            let y = 200 + (i % 5) * 80;
            let _ = ctl.write_all(&scrcpy::msg_touch(scrcpy::TOUCH_DOWN, 180, y, w, h));
            let _ = ctl.write_all(&scrcpy::msg_touch(scrcpy::TOUCH_UP, 180, y, w, h));
            if i == 5 {
                let _ = ctl.write_all(&scrcpy::msg_keycode(false, scrcpy::KEY_HOME));
                let _ = ctl.write_all(&scrcpy::msg_keycode(true, scrcpy::KEY_HOME));
                let _ = ctl.write_all(&scrcpy::msg_text("ok"));
            }
        }
    });
    let mut dec = scrcpy::Decoder::new().unwrap();
    let (mut quadros, mut tamanho, t1) = (0, (0, 0), Instant::now());
    while quadros < 5 && t1.elapsed() < Duration::from_secs(60) {
        let p = s.read_packet().unwrap();
        if let Some(f) = dec.decode(&p.data).unwrap() {
            assert_eq!(f.rgba.len(), (f.width * f.height * 4) as usize);
            tamanho = (f.width, f.height);
            quadros += 1;
        }
    }
    eprintln!("{quadros} quadros de {tamanho:?} em {:?}", t1.elapsed());
    assert!(quadros >= 1, "nenhum quadro decodificado");
    assert_eq!(tamanho, (s.width, s.height));
    drop(s);

    // O /data fica na pasta do aparelho: sobrevive a desligar e ligar.
    adb.shell(
        &l.serial(),
        "echo guardado > /data/local/tmp/multidroid-teste",
    )
    .unwrap();
    leve::stop(rt, &l.name).unwrap();
    assert!(leve::running(rt).iter().all(|(n, _)| n != &l.name));
    leve::start(rt, data, &l).unwrap();
    adb.wait_boot(&l.serial(), Duration::from_secs(240))
        .unwrap();
    assert_eq!(
        adb.shell(&l.serial(), "cat /data/local/tmp/multidroid-teste")
            .unwrap(),
        "guardado"
    );
    leve::pause(rt, &l.name, true).unwrap();
    assert!(leve::running(rt).iter().any(|(n, p)| n == &l.name && *p));
    leve::pause(rt, &l.name, false).unwrap();
    let mem = leve::memory(rt);
    eprintln!("memória: {mem:?}");
    assert!(mem.iter().any(|(n, b)| n == &l.name && *b > 100 << 20));
    adb.disconnect(&l.serial());
    leve::delete(Some(rt), data, &l.name).unwrap();
    assert!(leve::load(data).is_empty());
}
