//! Capturas do README e do site a partir do estado real: os aparelhos da pasta
//! do MultiDroid, a tela de cada um que está ligado (gRPC), a memória e a CPU de
//! cada processo e o painel de memória com o texto do app. Desenha com o
//! renderizador por software, o mesmo do app no Mac, fora da tela.
//!   cargo run --release -p multidroid --example capturas -- <pasta>
//! Grava <pasta>/mesa-ligada.png, mesa-desligada.png e painel.png. Não mexe nos
//! aparelhos: só lê. MULTIDROID_LANG=pt desenha em português; APENAS=a,b limita a mesa.

slint::include_modules!();

use multidroid_core::emulator::{self, Link, Processes};
use multidroid_core::state::{State, grid};
use multidroid_core::{catalog, devices, idioma, platform, tokens};
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, WindowAdapter};
use slint::{Image, ModelRc, Rgb8Pixel, Rgba8Pixel, SharedPixelBuffer, VecModel};
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

const LARGURA: u32 = 1710;
const ALTURA: u32 = 890;

struct Fora(Rc<MinimalSoftwareWindow>);

impl Platform for Fora {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}

fn gb(bytes: u64) -> String {
    format!("{} GB", idioma::num(bytes as f64 / (1u64 << 30) as f64, 1))
}

fn png_para_imagem(png: &[u8]) -> Option<(Image, f32)> {
    let mut r = png::Decoder::new(std::io::Cursor::new(png)).read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).ok()?;
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => buf[..info.buffer_size()].chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        _ => return None,
    };
    let px = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&rgba, info.width, info.height);
    Some((Image::from_rgba8(px), info.width as f32 / info.height.max(1) as f32))
}

fn tema(ui: &AppWindow) {
    let cor = |hex: &str| {
        let (r, g, b) = tokens::rgb(hex);
        slint::Color::from_rgb_u8(r, g, b)
    };
    let t = tokens::load();
    let tm = ui.global::<Tema>();
    let (e, a) = (&t.cor.escuro, &t.cor.acento);
    tm.set_fundo(cor(&e.fundo));
    tm.set_superficie(cor(&e.superficie));
    tm.set_borda(cor(&e.borda));
    tm.set_sombra(cor(&e.sombra));
    tm.set_tinta(cor(&e.tinta));
    tm.set_tinta_sobre_acento(cor(&e.tinta_sobre_acento));
    tm.set_azul(cor(&a.azul));
    tm.set_verde(cor(&a.verde));
    tm.set_ciano(cor(&a.ciano));
    tm.set_amarelo(cor(&a.amarelo));
    tm.set_vermelho(cor(&a.vermelho));
    tm.set_rosa(cor(&a.rosa));
    tm.set_roxo(cor(&a.roxo));
    tm.set_largura_borda(t.forma.largura_borda);
    tm.set_raio(t.forma.raio);
    tm.set_sombra_x(t.forma.sombra.x);
    tm.set_sombra_y(t.forma.sombra.y);
}

fn desenhar(janela: &MinimalSoftwareWindow, saida: &Path, recorte: Option<(u32, u32)>) {
    let mut buffer = vec![Rgb8Pixel::default(); (LARGURA * ALTURA) as usize];
    for _ in 0..2 {
        slint::platform::update_timers_and_animations();
        janela.request_redraw();
        janela.draw_if_needed(|r| {
            r.render(&mut buffer, LARGURA as usize);
        });
    }
    let (w, h) = recorte.unwrap_or((LARGURA, ALTURA));
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let p = buffer[(y * LARGURA + x) as usize];
            rgb.extend_from_slice(&[p.r, p.g, p.b]);
        }
    }
    let mut enc = png::Encoder::new(std::fs::File::create(saida).unwrap(), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.write_header().unwrap().write_image_data(&rgb).unwrap();
    eprintln!("gravado {} ({w}×{h})", saida.display());
}

fn main() {
    let pasta = std::path::PathBuf::from(std::env::args().nth(1).expect("pasta de saída"));
    std::fs::create_dir_all(&pasta).unwrap();
    let pt = std::env::var("MULTIDROID_LANG").is_ok_and(|l| idioma::eh_portugues(&l));
    idioma::usar_portugues(pt);

    // Estado real.
    let infos = devices::load(&devices::avd_home());
    let st = State::load(&platform::data_dir().join("state.json"), platform::physical_gb());
    // APENAS=Celular1,Relogio1 mostra só esses na mesa.
    let apenas: Option<Vec<String>> = std::env::var("APENAS").ok().map(|v| v.split(',').map(str::to_string).collect());
    let ordem: Vec<_> = st
        .ordered(&infos)
        .into_iter()
        .filter(|d| apenas.as_ref().is_none_or(|a| a.contains(&d.name)))
        .collect();
    // A CPU de um processo só vale a partir da terceira leitura (sysinfo).
    let mut procs = Processes::new();
    for _ in 0..2 {
        procs.emulators();
        std::thread::sleep(Duration::from_secs(1));
    }
    let rodando = procs.emulators();

    // Telas pelo gRPC, só lendo.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let telas: Vec<Option<(Image, f32)>> = ordem
        .iter()
        .map(|d| {
            let r = rodando.iter().find(|r| r.avd == d.name)?;
            let token = emulator::grpc_token(r.pid)?;
            let png = rt.block_on(async {
                let link = Link::connect(r.grpc?, &token).await.ok()?;
                link.screenshot_png().await
            })?;
            png_para_imagem(&png)
        })
        .collect();

    // Grade como depois do Organizar, na altura visível desta janela.
    let celula = (LARGURA as f32 - 16.0 * 3.0 - 330.0) / 24.0;
    let linhas = ((ALTURA as f32 - 32.0 - 44.0) / celula).floor() as i32;
    let padroes: Vec<(i32, i32)> = ordem.iter().map(|d| d.kind.grid_size()).collect();
    let lugares = grid::encaixar(&padroes, linhas);

    let janela = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    janela.set_size(slint::PhysicalSize::new(LARGURA, ALTURA));
    slint::platform::set_platform(Box::new(Fora(janela.clone()))).unwrap();
    let ui = AppWindow::new().unwrap();
    let _ = slint::select_bundled_translation(if pt { "pt_BR" } else { "en" });
    tema(&ui);
    ui.set_versao(env!("CARGO_PKG_VERSION").into());

    let montar = |ligados: bool| -> Vec<Aparelho> {
        ordem
            .iter()
            .zip(&lugares)
            .zip(&telas)
            .map(|((d, s), tela)| {
                let r = rodando.iter().find(|r| r.avd == d.name).filter(|_| ligados);
                let tela = tela.as_ref().filter(|_| ligados);
                Aparelho {
                    nome: d.name.as_str().into(),
                    tipo: d.kind.display().into(),
                    android: catalog::android_version(&d.api).into(),
                    estado: if r.is_some() { Estado::Ligado } else { Estado::Desligado },
                    mensagem: "".into(),
                    metricas: r.map(|r| format!("{} · {:.0}%", gb(r.memory), r.cpu)).unwrap_or_default().into(),
                    grupo: st.groups.get(&d.name).cloned().unwrap_or_default().into(),
                    x: s.x,
                    y: s.y,
                    w: s.w,
                    h: s.h,
                    proporcao: tela.map_or_else(
                        || {
                            let (lw, lh, _) = d.kind.lcd();
                            lw as f32 / lh as f32
                        },
                        |t| t.1,
                    ),
                    tela: tela.map(|t| t.0.clone()).unwrap_or_default(),
                    tem_tela: tela.is_some(),
                    relogio: d.kind == devices::Kind::Relogio,
                    selecionado: false,
                }
            })
            .collect()
    };

    let (usada, swap) = platform::memory_use_gb();
    let soma: u64 = rodando.iter().filter(|r| ordem.iter().any(|d| d.name == r.avd)).map(|r| r.memory).sum();
    let memoria = idioma::tf(
        "Computer memory: {usada} of {total} GB\nDevices {aparelhos} · swap {swap} GB · ceiling {teto} GB",
        &[
            ("usada", &idioma::num(usada, 1)),
            ("total", &idioma::num(platform::physical_gb(), 0)),
            ("aparelhos", &gb(soma)),
            ("swap", &idioma::num(swap, 1)),
            ("teto", &idioma::num(st.ceiling_gb, 0)),
        ],
    );
    ui.set_teto(st.ceiling_gb as f32);
    ui.set_linhas(lugares.iter().map(|s| s.y + s.h).max().unwrap_or(0));
    ui.show().unwrap();

    ui.set_memoria(memoria.as_str().into());
    ui.set_aparelhos(ModelRc::new(VecModel::from(montar(true))));
    desenhar(&janela, &pasta.join("mesa-ligada.png"), None);
    // O painel sozinho: a coluna da esquerda, com a margem.
    desenhar(&janela, &pasta.join("painel.png"), Some((362, 655)));

    let memoria_parada = idioma::tf(
        "Computer memory: {usada} of {total} GB\nDevices {aparelhos} · swap {swap} GB · ceiling {teto} GB",
        &[
            ("usada", &idioma::num(usada, 1)),
            ("total", &idioma::num(platform::physical_gb(), 0)),
            ("aparelhos", &gb(0)),
            ("swap", &idioma::num(swap, 1)),
            ("teto", &idioma::num(st.ceiling_gb, 0)),
        ],
    );
    ui.set_memoria(memoria_parada.as_str().into());
    ui.set_aparelhos(ModelRc::new(VecModel::from(montar(false))));
    desenhar(&janela, &pasta.join("mesa-desligada.png"), None);
    eprintln!(
        "{} aparelhos, {} ligados, linhas {linhas}",
        ordem.len(),
        telas.iter().filter(|t| t.is_some()).count()
    );
}
