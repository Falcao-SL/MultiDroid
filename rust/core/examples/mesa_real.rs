//! Critério 4 e 5 do MULTIPLATAFORMA.md §5, no Mac: 4 celulares e 1 relógio
//! ligados ao mesmo tempo, com tela, toque, teclas e texto, e a memória medida.
//!   cargo run --release -p multidroid-core --example mesa_real
//! Usa a pasta real do MultiDroid (ou MULTIDROID_DATA), instala o emulador e as
//! imagens padrão se faltarem, cria os cinco aparelhos se não existirem e os põe
//! na mesa principal do state.json. Com DEIXAR=1 os aparelhos ficam ligados, para
//! um app (0.2 ou Rust) adotá-los e a memória dos dois ser comparada.
//! CAPTURAS=<pasta> grava a tela de cada etapa; SO_DESLIGAR=1 só desliga os cinco.

use multidroid_core::catalog::{self, EMULATOR, Host, REPO};
use multidroid_core::devices::{self, Kind};
use multidroid_core::emulator::{self, LaunchOptions, Link, Processes};
use multidroid_core::{install, platform, screen, state};
use std::path::Path;
use std::time::{Duration, Instant};

const APARELHOS: [(&str, Kind); 5] = [
    ("Celular1", Kind::Celular),
    ("Celular2", Kind::Celular),
    ("Celular3", Kind::Celular),
    ("Celular4", Kind::Celular),
    ("Relogio1", Kind::Relogio),
];

fn garantir(path: &str, lista: &str, sdk: &Path, host: Host) {
    if catalog::installed(path, sdk).is_some() {
        return;
    }
    let xml = ureq::get(lista).call().expect("sem rede").into_string().unwrap();
    let base = lista.rsplit_once('/').unwrap().0.to_string() + "/";
    let pkg = catalog::parse(&xml, path, &base, host)
        .unwrap()
        .unwrap_or_else(|| panic!("{path} não está no repositório"));
    eprintln!("baixando {path} ({} MB)", pkg.size >> 20);
    install::accept(&pkg, sdk).unwrap();
    install::install(&pkg, sdk, &sdk.join(".work"), &Default::default(), |_| {}).unwrap();
}

async fn um_quadro(link: &Link, kind: Kind, tela: &str, nome: &str) -> (u32, u32, Vec<u8>) {
    let (w, h) = screen::frame_size(kind, tela);
    let t = screen::Transport::for_device(nome);
    // O emulador só manda quadro quando a tela muda; o primeiro chega na hora.
    let mut ultimo = (0, 0, vec![]);
    let fluxo = screen::stream(link, w, h, &t, |f| {
        ultimo = (f.width, f.height, f.rgba);
        false
    });
    tokio::time::timeout(Duration::from_secs(30), fluxo)
        .await
        .unwrap_or_else(|_| panic!("{nome}: nenhum quadro em 30 s"))
        .unwrap();
    ultimo
}

fn gravar(dir: &Option<std::path::PathBuf>, nome: &str, etapa: &str, q: &(u32, u32, Vec<u8>)) {
    let Some(dir) = dir else { return };
    let (w, h, rgba) = (q.0, q.1, &q.2);
    std::fs::create_dir_all(dir).unwrap();
    // PPM: sem dependência para gravar; o sips converte.
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for p in rgba.chunks(4) {
        ppm.extend_from_slice(&p[..3]);
    }
    std::fs::write(dir.join(format!("{nome}-{etapa}.ppm")), ppm).unwrap();
}

/// Captura pedida na hora pelo gRPC, decodificada para RGBA (só para conferir
/// toque, tecla e texto: o fluxo não manda quadro com a tela parada).
async fn captura(link: &Link, nome: &str) -> (u32, u32, Vec<u8>) {
    let png = link.screenshot_png().await.unwrap_or_else(|| panic!("{nome}: sem captura"));
    let dec = png::Decoder::new(std::io::Cursor::new(png));
    let mut r = dec.read_info().unwrap();
    let mut buf = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        c => panic!("{nome}: captura em {c:?}"),
    };
    (info.width, info.height, rgba)
}

fn diferenca(a: &(u32, u32, Vec<u8>), b: &(u32, u32, Vec<u8>)) -> f64 {
    let (a, b) = (&a.2, &b.2);
    if a.len() != b.len() || a.is_empty() {
        return 1.0;
    }
    let mudou = a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p != q).count();
    mudou as f64 / (a.len() / 4) as f64
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let host = Host::current();
    let sdk = devices::sdk_root();
    let avd_home = devices::avd_home();
    std::fs::create_dir_all(sdk.join("platform-tools")).unwrap();
    std::fs::create_dir_all(&avd_home).unwrap();
    let celular = catalog::default_image(false, host);
    let relogio = catalog::default_image(true, host);
    garantir(EMULATOR, &format!("{REPO}repository2-3.xml"), &sdk, host);
    for img in [&celular, &relogio] {
        let tag = img.split(';').nth(2).unwrap();
        garantir(img, &catalog::tag_list_url(tag), &sdk, host);
    }

    let existentes: Vec<String> = devices::load(&avd_home).into_iter().map(|d| d.name).collect();
    for (nome, kind) in APARELHOS {
        if !existentes.iter().any(|n| n == nome) {
            let img = if kind == Kind::Relogio { &relogio } else { &celular };
            devices::create(&avd_home, &sdk, nome, kind, img, host).unwrap();
            eprintln!("criado {nome}");
        }
    }
    let devs = devices::load(&avd_home);
    let caminho = platform::data_dir().join("state.json");
    let mut st = state::State::load(&caminho, platform::physical_gb());
    for (nome, _) in APARELHOS {
        if !st.order.iter().any(|n| n == nome) {
            st.order.push(nome.into());
        }
    }
    let lista: Vec<_> = st.ordered(&devs);
    st.ensure_spots(&lista);
    st.save(&caminho).unwrap();

    // Liga os cinco com as opções do app (app/src/main.rs); os que já estão
    // ligados são adotados, como o app faz.
    let inicio = Instant::now();
    let mut ligados = vec![];
    let mut adotados = vec![];
    let rodando = Processes::new().emulators();
    for (nome, kind) in APARELHOS {
        if let Some(r) = rodando.iter().find(|r| r.avd == nome) {
            eprintln!("{nome}: já ligado (pid {}), adotado", r.pid);
            adotados.push((nome, kind, r.pid, r.grpc.expect("sem porta gRPC"), r.console));
            continue;
        }
        let usadas: Vec<u16> = Processes::new().emulators().iter().map(|r| r.console).collect();
        let fora: Vec<u16> = usadas
            .iter()
            .copied()
            .chain(ligados.iter().map(|(_, _, l): &(_, _, emulator::Launched)| l.console))
            .collect();
        let opts = LaunchOptions {
            sdk: &sdk,
            avd_home: &avd_home,
            home: &platform::data_dir(),
            log_dir: &platform::logs_dir(),
            cores: kind.cores(),
            gpu: emulator::gpu_mode(),
        };
        let l = emulator::launch(nome, &fora, &opts).expect("não abriu o emulador");
        eprintln!("{nome}: pid {} console {} gRPC {}", l.pid(), l.console, l.grpc);
        ligados.push((nome, kind, l));
    }

    // SO_DESLIGAR=1: só desliga os que estão ligados, salvando, e sai.
    if std::env::var_os("SO_DESLIGAR").is_some() {
        for (nome, _, pid, _, console) in &adotados {
            emulator::shutdown(*pid, *console);
            eprintln!("{nome}: desligado");
        }
        return;
    }
    let mut links = vec![];
    for (nome, kind, pid, grpc, _) in &adotados {
        let token = emulator::grpc_token(*pid).expect("sem token gRPC");
        links.push((*nome, *kind, Link::connect(*grpc, &token).await.unwrap()));
    }
    for (nome, kind, l) in ligados.iter_mut() {
        let token = loop {
            if let Some(t) = emulator::grpc_token(l.pid()) {
                break t;
            }
            assert!(l.alive(), "{nome} fechou antes do gRPC");
            tokio::time::sleep(Duration::from_millis(500)).await;
        };
        let link = Link::connect(l.grpc, &token).await.unwrap();
        while !link.booted().await {
            assert!(l.alive(), "{nome} fechou no boot");
            assert!(inicio.elapsed() < Duration::from_secs(900), "{nome} não terminou o boot");
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        eprintln!("{nome}: boot em {:?} desde o início", inicio.elapsed());
        links.push((*nome, *kind, link));
    }
    eprintln!("os cinco ligados em {:?}", inicio.elapsed());

    // Deixa a tela inicial assentar antes de medir mudança.
    if !ligados.is_empty() {
        tokio::time::sleep(Duration::from_secs(20)).await;
    }

    let capturas = std::env::var_os("CAPTURAS").map(std::path::PathBuf::from);
    let telas: Vec<String> = links
        .iter()
        .map(|(n, _, _)| devs.iter().find(|d| d.name == *n).map(|d| d.screen.clone()).unwrap_or_default())
        .collect();
    let mut falhas = 0;
    for ((nome, kind, link), tela) in links.iter().zip(&telas) {
        let (lw, lh, _) = kind.lcd();
        let (lw, lh) = (lw as i32, lh as i32);
        link.key("GoHome").await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        let quadro = um_quadro(link, *kind, tela, nome).await;
        assert!(quadro.0 > 0 && quadro.2.len() == (quadro.0 * quadro.1 * 4) as usize, "{nome}: quadro MMAP inválido");
        gravar(&capturas, nome, "0-mmap", &quadro);
        let antes = captura(link, nome).await;
        gravar(&capturas, nome, "1-antes", &antes);

        // Toque: arrastar de baixo para cima abre a gaveta de apps (no relógio,
        // de cima para baixo abre as configurações rápidas).
        if *kind == Kind::Relogio {
            let (y0, y1) = (lh / 10, lh * 7 / 10);
            link.touch(lw / 2, y0, true).await;
            for i in 1..=8 {
                link.touch(lw / 2, y0 + (y1 - y0) * i / 8, true).await;
                tokio::time::sleep(Duration::from_millis(16)).await;
            }
            link.touch(lw / 2, y1, false).await;
        } else {
            // Toque simples no Chrome, terceiro ícone da barra de baixo do Pixel Launcher.
            let (x, y) = (lw * 62 / 100, lh * 83 / 100);
            link.touch(x, y, true).await;
            tokio::time::sleep(Duration::from_millis(80)).await;
            link.touch(x, y, false).await;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        let depois_toque = captura(link, nome).await;
        gravar(&capturas, nome, "2-toque", &depois_toque);
        let d_toque = diferenca(&antes, &depois_toque);

        // Teclas: GoBack fecha o que o toque abriu.
        link.key("GoBack").await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        let depois_tecla = captura(link, nome).await;
        gravar(&capturas, nome, "3-tecla", &depois_tecla);
        let d_tecla = diferenca(&depois_toque, &depois_tecla);

        // Texto: no celular, o que se digita aparece na busca.
        let mut d_texto = f64::NAN;
        if *kind == Kind::Celular {
            // A barra de busca do Pixel Launcher, no rodapé, abre com o teclado.
            link.key("GoHome").await;
            tokio::time::sleep(Duration::from_secs(2)).await;
            let (x, y) = (lw * 40 / 100, lh * 92 / 100);
            link.touch(x, y, true).await;
            tokio::time::sleep(Duration::from_millis(80)).await;
            link.touch(x, y, false).await;
            tokio::time::sleep(Duration::from_secs(3)).await;
            let gaveta = captura(link, nome).await;
            link.type_text("matiz").await;
            tokio::time::sleep(Duration::from_secs(2)).await;
            let digitado = captura(link, nome).await;
            gravar(&capturas, nome, "4-texto", &digitado);
            d_texto = diferenca(&gaveta, &digitado);
        }
        if let Some(dir) = &capturas {
            std::fs::create_dir_all(dir).unwrap();
            if let Some(png) = link.screenshot_png().await {
                std::fs::write(dir.join(format!("{nome}.png")), png).unwrap();
            }
        }
        link.key("GoHome").await;

        let ok = d_toque > 0.02 && d_tecla > 0.02 && (d_texto.is_nan() || d_texto > 0.001);
        if !ok {
            falhas += 1;
        }
        println!(
            "{nome:<9} toque {:>5.1}%  tecla {:>5.1}%  texto {}  {}",
            d_toque * 100.0,
            d_tecla * 100.0,
            if d_texto.is_nan() { "  n/a".into() } else { format!("{:>5.1}%", d_texto * 100.0) },
            if ok { "ok" } else { "FALHOU" }
        );
    }

    tokio::time::sleep(Duration::from_secs(5)).await;
    let mut p = Processes::new();
    let rodando = p.emulators();
    let mut total = 0u64;
    for r in rodando.iter().filter(|r| APARELHOS.iter().any(|(n, _)| *n == r.avd)) {
        println!("memória {:<9} {:>6.2} GB", r.avd, r.memory as f64 / (1u64 << 30) as f64);
        total += r.memory;
    }
    let (usado, swap) = platform::memory_use_gb();
    println!(
        "memória residente dos 5 emuladores: {:.2} GB · Mac: {usado:.1} GB em uso, {swap:.1} GB de swap, {:.0} GB físicos",
        total as f64 / (1u64 << 30) as f64,
        platform::physical_gb()
    );

    if std::env::var_os("DEIXAR").is_none() {
        for (nome, _, l) in &ligados {
            emulator::shutdown(l.pid(), l.console);
            eprintln!("{nome}: desligado");
        }
        for (nome, _, pid, _, console) in &adotados {
            emulator::shutdown(*pid, *console);
            eprintln!("{nome}: desligado");
        }
    } else {
        // Solta os processos para continuarem depois que este exemplo sair.
        for (_, _, l) in ligados {
            std::mem::forget(l);
        }
        eprintln!("aparelhos deixados ligados (DEIXAR=1)");
    }
    assert_eq!(falhas, 0, "{falhas} aparelho(s) não responderam");
}
