//! A tela de um aparelho no tamanho do quadro da mesa. Porte de `ScreenStream`:
//! o emulador escala e escreve os pixels direto num arquivo mapeado em memória
//! (MMAP) e a resposta gRPC vem vazia (MAC.md §2.2). Onde o MMAP não está
//! medido (Windows, risco 1 da MULTIPLATAFORMA.md §7), os pixels vêm dentro da
//! própria resposta gRPC.

use crate::emulator::{Link, proto};
use std::path::{Path, PathBuf};

/// Um quadro RGBA com alfa 255, pronto para a interface.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transport {
    /// Arquivo mapeado: o emulador escreve, nós lemos.
    Mmap(PathBuf),
    /// Pixels dentro da mensagem gRPC.
    Inline,
}

impl Transport {
    /// MMAP no Mac e no Linux; no Windows, por mensagem até a medição de R-7.
    /// MULTIDROID_FRAMES=inline ou mmap troca.
    pub fn for_device(tag: &str) -> Transport {
        let mmap =
            Transport::Mmap(std::env::temp_dir().join(format!("multidroid-screen-{tag}.raw")));
        match std::env::var("MULTIDROID_FRAMES").as_deref() {
            Ok("inline") => Transport::Inline,
            Ok("mmap") => mmap,
            _ if cfg!(windows) => Transport::Inline,
            _ => mmap,
        }
    }
}

/// Copia `count` bytes RGBA e põe alfa 255 (o emulador manda o quarto byte sem uso).
pub fn opaque(src: &[u8]) -> Vec<u8> {
    let mut v = src.to_vec();
    for px in v.as_chunks_mut::<4>().0 {
        px[3] = 255;
    }
    v
}

fn file_url(path: &Path) -> String {
    let s = path.display().to_string().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

/// Pede ao emulador quadros de `width`×`height` e chama `on_frame` a cada um, até
/// `on_frame` devolver false ou o emulador fechar o fluxo.
pub async fn stream(
    link: &Link,
    width: u32,
    height: u32,
    transport: &Transport,
    mut on_frame: impl FnMut(Frame) -> bool,
) -> Result<(), String> {
    let bytes = (width * height * 4) as usize;
    let mut fmt = proto::ImageFormat {
        format: proto::image_format::ImgFormat::Rgba8888 as i32,
        // Largura E altura: só assim o emulador escala.
        width,
        height,
        ..Default::default()
    };
    let map = if let Transport::Mmap(path) = transport {
        // Arquivo do tamanho exato de um quadro.
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|e| e.to_string())?;
        f.set_len(bytes as u64).map_err(|e| e.to_string())?;
        fmt.transport = Some(proto::ImageTransport {
            channel: proto::image_transport::TransportChannel::Mmap as i32,
            handle: file_url(path),
        });
        // SAFETY: só lemos; o emulador escreve no arquivo. Um quadro rasgado no meio
        // de uma escrita é aceitável (é copiado a cada aviso do emulador).
        Some(unsafe { memmap2::Mmap::map(&f) }.map_err(|e| e.to_string())?)
    } else {
        None
    };
    let mut s = link
        .api
        .clone()
        .stream_screenshot(fmt)
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    while let Some(img) = s.message().await.map_err(|e| e.to_string())? {
        let Some(f) = img.format else { continue };
        let (fw, fh) = (f.width, f.height);
        let n = (fw * fh * 4) as usize;
        if fw == 0 || fh == 0 {
            continue;
        }
        let rgba = match &map {
            Some(m) if n <= m.len() => opaque(&m[..n]),
            Some(_) => continue,
            None if img.image.len() >= n => opaque(&img.image[..n]),
            None => continue,
        };
        if !on_frame(Frame {
            width: fw,
            height: fh,
            rgba,
        }) {
            break;
        }
    }
    Ok(())
}

/// Tamanho pedido ao emulador: a largura do quadro na mesa em 2× (tela de alta
/// densidade), com a proporção da tela do aparelho.
pub fn frame_size(kind: crate::devices::Kind, screen: &str) -> (u32, u32) {
    use crate::devices::Kind;
    let (w, h) = screen
        .split_once('×')
        .and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)))
        .unwrap_or((1080, 2400));
    let base = match kind {
        Kind::Celular => 300,
        Kind::Dobravel => 380,
        Kind::Tablet => 520,
        Kind::Relogio => 220,
    };
    let fw = base * 2;
    (fw, (fw * h / w.max(1)).max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::Kind;

    #[test]
    fn alfa_sempre_opaco() {
        assert_eq!(
            opaque(&[1, 2, 3, 0, 4, 5, 6, 9]),
            [1, 2, 3, 255, 4, 5, 6, 255]
        );
    }

    #[test]
    fn tamanho_do_quadro_como_na_02() {
        assert_eq!(frame_size(Kind::Celular, "1080×2424"), (600, 1346));
        assert_eq!(frame_size(Kind::Relogio, "454×454"), (440, 440));
        assert_eq!(frame_size(Kind::Tablet, "?×?"), (1040, 2311));
    }

    #[test]
    fn url_do_arquivo_nos_dois_formatos() {
        assert_eq!(file_url(Path::new("/tmp/a.raw")), "file:///tmp/a.raw");
        assert_eq!(file_url(Path::new(r"C:\T\a.raw")), "file:///C:/T/a.raw");
    }
}
