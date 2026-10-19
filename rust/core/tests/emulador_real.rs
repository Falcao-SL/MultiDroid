//! Liga um aparelho de verdade (etapa R-3): cria numa pasta temporária, liga sem
//! janela, espera o boot por gRPC, pausa, retoma, captura a tela e desliga
//! salvando o estado. Não roda por padrão; rode com
//!   cargo test -p multidroid-core --test emulador_real -- --ignored --nocapture
//! Usa o SDK de MULTIDROID_SDK ou o do MultiDroid; baixa o emulador e a imagem
//! AOSP 33 se faltarem. No CI do Linux roda com KVM e MULTIDROID_GPU=swiftshader_indirect.

use multidroid_core::catalog::{self, EMULATOR, Host, REPO};
use multidroid_core::devices::{self, Kind};
use multidroid_core::emulator::{self, LaunchOptions, Link, Processes};
use multidroid_core::install;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn get(url: &str) -> String {
    ureq::get(url)
        .call()
        .expect("sem rede")
        .into_string()
        .unwrap()
}

fn ensure(path: &str, list_url: &str, sdk: &Path, host: Host) {
    if catalog::installed(path, sdk).is_some() {
        return;
    }
    let base = list_url.rsplit_once('/').unwrap().0.to_string() + "/";
    let pkg = catalog::parse(&get(list_url), path, &base, host)
        .unwrap()
        .unwrap_or_else(|| panic!("{path} não está no repositório"));
    eprintln!("baixando {path} ({} MB)", pkg.size >> 20);
    install::accept(&pkg, sdk).unwrap();
    install::install(&pkg, sdk, &sdk.join(".work"), &Default::default(), |_| {}).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn liga_pausa_e_desliga_um_aparelho() {
    let host = Host::current();
    let sdk = std::env::var_os("MULTIDROID_SDK")
        .map(PathBuf::from)
        .unwrap_or_else(devices::sdk_root);
    std::fs::create_dir_all(sdk.join("platform-tools")).unwrap();
    let image = format!("system-images;android-33;default;{}", host.abi);
    ensure(EMULATOR, &format!("{REPO}repository2-3.xml"), &sdk, host);
    ensure(&image, &catalog::tag_list_url("default"), &sdk, host);

    let t = tempfile::tempdir().unwrap();
    let avd = t.path().join("avd");
    devices::create(&avd, &sdk, "R3Teste", Kind::Celular, &image, host).unwrap();

    let mut used: Vec<u16> = Processes::new()
        .emulators()
        .iter()
        .map(|r| r.console)
        .collect();
    used.sort();
    let mut em = emulator::launch(
        "R3Teste",
        &used,
        &LaunchOptions {
            sdk: &sdk,
            avd_home: &avd,
            home: t.path(),
            log_dir: t.path(),
            cores: 2,
            gpu: emulator::gpu_mode(),
        },
    )
    .unwrap();
    let pid = em.pid();
    let log = t.path().join("R3Teste.log");
    eprintln!("pid {pid}, console {}, gRPC {}", em.console, em.grpc);

    let start = Instant::now();
    let token = loop {
        if let Some(t) = emulator::grpc_token(pid) {
            break t;
        }
        assert!(
            em.alive(),
            "o emulador fechou:\n{}",
            std::fs::read_to_string(&log).unwrap_or_default()
        );
        assert!(start.elapsed() < Duration::from_secs(120), "sem token gRPC");
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    let link = Link::connect(em.grpc, &token).await.unwrap();
    while !link.booted().await {
        assert!(em.alive(), "o emulador fechou no boot");
        assert!(
            start.elapsed() < Duration::from_secs(900),
            "não terminou o boot"
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    eprintln!("ligou em {:?}", start.elapsed());

    let running = Processes::new().emulators();
    assert!(
        running.iter().any(|r| r.pid == pid && r.avd == "R3Teste"),
        "{running:?}"
    );

    // A troca de estado da VM é assíncrona: espera chegar, com prazo.
    for want in [true, false] {
        assert!(link.set_paused(want).await);
        let t0 = Instant::now();
        while link.paused().await != Some(want) {
            assert!(
                t0.elapsed() < Duration::from_secs(15),
                "não ficou paused={want}"
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        eprintln!("paused={want} em {:?}", t0.elapsed());
    }
    link.key("GoHome").await;
    // Etapa R-4: quadros escalados chegando pelo transporte do sistema (MMAP no Linux).
    let transporte = multidroid_core::screen::Transport::for_device("R3Teste");
    let (fw, fh) = multidroid_core::screen::frame_size(Kind::Celular, "1080×2424");
    let mut quadros = 0;
    let mut tamanho = (0, 0);
    let fluxo = multidroid_core::screen::stream(&link, fw, fh, &transporte, |f| {
        tamanho = (f.width, f.height);
        assert_eq!(f.rgba.len(), (f.width * f.height * 4) as usize);
        assert!(f.rgba.chunks(4).all(|p| p[3] == 255));
        quadros += 1;
        quadros < 3
    });
    tokio::time::timeout(Duration::from_secs(60), fluxo)
        .await
        .expect("nenhum quadro em 60 s")
        .unwrap();
    eprintln!("{quadros} quadros de {tamanho:?} por {transporte:?}");
    assert_eq!(tamanho, (fw, fh));
    link.touch(540, 1200, true).await;
    link.touch(540, 1200, false).await;
    link.type_text("ok").await;

    let png = link.screenshot_png().await.expect("sem captura");
    assert_eq!(&png[1..4], b"PNG");

    let console = em.console;
    let saved = tokio::task::spawn_blocking(move || emulator::shutdown(pid, console))
        .await
        .unwrap();
    assert!(saved, "o console não aceitou o kill");
    assert!(!em.alive());
    assert!(!emulator::alive(pid));
    eprintln!("desligou salvando, total {:?}", start.elapsed());
}
