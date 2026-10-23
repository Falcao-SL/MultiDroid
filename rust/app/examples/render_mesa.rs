//! Desenha a janela fora da tela, com o renderizador por software (o mesmo do
//! Mac), e grava um PNG: serve para conferir a mesa sem permissão de gravação
//! de tela.
//!   cargo run --release -p multidroid --example render_mesa -- <saida.png> [linhas] [tela-celular.png] [tela-relogio.png]
//! Sem `linhas`, os quadros ficam no tamanho padrão; com `linhas`, como depois
//! do Organizar numa mesa com essa altura.

slint::include_modules!();

use multidroid_core::devices::Kind;
use multidroid_core::state::grid;
use multidroid_core::tokens;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, WindowAdapter};
use slint::{Image, Model, ModelRc, Rgb8Pixel, Rgba8Pixel, SharedPixelBuffer, VecModel};
use std::rc::Rc;

struct Fora(Rc<MinimalSoftwareWindow>);

impl Platform for Fora {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}

fn imagem(caminho: Option<&String>) -> (Image, bool) {
    let Some(c) = caminho else {
        return (Image::default(), false);
    };
    let dec = png::Decoder::new(std::fs::File::open(c).expect("tela não abriu"));
    let mut r = dec.read_info().unwrap();
    let mut buf = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).unwrap();
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        _ => buf[..info.buffer_size()]
            .chunks(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
    };
    let px = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&rgba, info.width, info.height);
    (Image::from_rgba8(px), true)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let saida = args.get(1).expect("caminho do PNG");
    let linhas: Option<i32> = args.get(2).and_then(|v| v.parse().ok());
    let (w, h) = (
        1400u32,
        std::env::var("ALTURA")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(860u32),
    );

    let janela = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    janela.set_size(slint::PhysicalSize::new(w, h));
    slint::platform::set_platform(Box::new(Fora(janela.clone()))).unwrap();

    let ui = AppWindow::new().unwrap();
    // MULTIDROID_LANG=pt desenha em português, como o app num sistema em português.
    let pt =
        std::env::var("MULTIDROID_LANG").is_ok_and(|l| multidroid_core::idioma::eh_portugues(&l));
    multidroid_core::idioma::usar_portugues(pt);
    let _ = slint::select_bundled_translation(if pt { "pt_BR" } else { "en" });
    // O tema escuro dos tokens, como no main.rs.
    let cor = |hex: &str| {
        let (r, g, b) = tokens::rgb(hex);
        slint::Color::from_rgb_u8(r, g, b)
    };
    let t = tokens::load();
    let tema = ui.global::<Tema>();
    let (e, a) = (&t.cor.escuro, &t.cor.acento);
    tema.set_fundo(cor(&e.fundo));
    tema.set_superficie(cor(&e.superficie));
    tema.set_borda(cor(&e.borda));
    tema.set_sombra(cor(&e.sombra));
    tema.set_tinta(cor(&e.tinta));
    tema.set_tinta_sobre_acento(cor(&e.tinta_sobre_acento));
    tema.set_azul(cor(&a.azul));
    tema.set_verde(cor(&a.verde));
    tema.set_ciano(cor(&a.ciano));
    tema.set_amarelo(cor(&a.amarelo));
    tema.set_vermelho(cor(&a.vermelho));
    tema.set_rosa(cor(&a.rosa));
    tema.set_roxo(cor(&a.roxo));
    tema.set_largura_borda(t.forma.largura_borda);
    tema.set_raio(t.forma.raio);
    tema.set_sombra_x(t.forma.sombra.x);
    tema.set_sombra_y(t.forma.sombra.y);
    let (tela_cel, tem_cel) = imagem(args.get(3));
    let (tela_rel, tem_rel) = imagem(args.get(4));
    let tipos = [
        Kind::Celular,
        Kind::Celular,
        Kind::Celular,
        Kind::Celular,
        Kind::Relogio,
    ];
    let padroes: Vec<(i32, i32)> = tipos.iter().map(|k| k.grid_size()).collect();
    let lugares = match linhas {
        Some(l) => grid::encaixar(&padroes, l),
        None => {
            let mut v = vec![];
            for &(gw, gh) in &padroes {
                let s = grid::first_free(gw, gh, &v);
                v.push(s);
            }
            v
        }
    };
    let aparelhos: Vec<Aparelho> = tipos
        .iter()
        .zip(&lugares)
        .enumerate()
        .map(|(i, (k, s))| {
            let relogio = *k == Kind::Relogio;
            Aparelho {
                nome: if relogio {
                    "Relogio1".into()
                } else {
                    format!("Celular{}", i + 1).into()
                },
                sigla: if relogio { "Re".into() } else { "Ce".into() },
                tipo: k.display().into(),
                android: "13".into(),
                estado: Estado::Ligado,
                mensagem: "".into(),
                metricas: "".into(),
                grupo: "".into(),
                x: s.x,
                y: s.y,
                w: s.w,
                h: s.h,
                proporcao: if relogio { 1.0 } else { 1080.0 / 2424.0 },
                tela: if relogio {
                    tela_rel.clone()
                } else {
                    tela_cel.clone()
                },
                tem_tela: if relogio { tem_rel } else { tem_cel },
                relogio,
                selecionado: false,
            }
        })
        .collect();
    let linhas_ocupadas = aparelhos.iter().map(|a| a.y + a.h).max().unwrap_or(0);
    ui.set_aparelhos(ModelRc::new(VecModel::from(aparelhos)));
    ui.set_linhas(linhas_ocupadas);
    ui.set_versao("render".into());
    ui.show().unwrap();

    // Duas passadas: a primeira assenta o layout.
    let mut buffer = vec![Rgb8Pixel::default(); (w * h) as usize];
    for _ in 0..2 {
        slint::platform::update_timers_and_animations();
        janela.request_redraw();
        janela.draw_if_needed(|r| {
            r.render(&mut buffer, w as usize);
        });
    }
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for p in &buffer {
        rgb.extend_from_slice(&[p.r, p.g, p.b]);
    }
    let f = std::fs::File::create(saida).unwrap();
    let mut enc = png::Encoder::new(f, w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.write_header().unwrap().write_image_data(&rgb).unwrap();
    let _ = ui.get_aparelhos().row_count();
    eprintln!(
        "gravado {saida} ({} aparelhos, linhas {linhas:?})",
        lugares.len()
    );
}
