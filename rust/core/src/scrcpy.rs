//! A tela e o toque dos aparelhos leves pelo protocolo do scrcpy
//! (doc/develop.md do scrcpy, versão fixada em `VERSION`): o servidor oficial roda
//! dentro do Android, codifica a tela em H.264 e recebe toque, texto e teclas.
//! Cliente e servidor precisam ser da mesma versão; trocar a versão é trocar
//! `VERSION` e `SHA256`.
//!
//! Sockets, abertos nesta ordem pelo túnel `adb forward`: vídeo, controle.
//! No primeiro chegam um byte de teste e 64 bytes com o nome do aparelho; no vídeo,
//! 12 bytes de codec (id, largura, altura) e depois pacotes com cabeçalho de 12 bytes.

use crate::adb::Adb;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant};

pub const VERSION: &str = "3.3.4";
pub const SHA256: &str = "8588238c9a5a00aa542906b6ec7e6d5541d9ffb9b5d0f6e1bc0e365e2303079e";
pub const URL: &str =
    "https://github.com/Genymobile/scrcpy/releases/download/v3.3.4/scrcpy-server-v3.3.4";
/// Onde o servidor fica no Android.
pub const REMOTE: &str = "/data/local/tmp/multidroid-scrcpy.jar";

/// Baixa o servidor (90 KB) uma vez e confere o SHA-256.
pub fn ensure_server(dir: &Path) -> Result<PathBuf, String> {
    let p = dir.join(format!("scrcpy-server-v{VERSION}"));
    if let Ok(b) = std::fs::read(&p)
        && sha256_hex(&b) == SHA256
    {
        return Ok(p);
    }
    let mut b = vec![];
    ureq::get(URL)
        .call()
        .map_err(|e| e.to_string())?
        .into_reader()
        .take(10 << 20)
        .read_to_end(&mut b)
        .map_err(|e| e.to_string())?;
    if sha256_hex(&b) != SHA256 {
        return Err(crate::idioma::t("the downloaded scrcpy server doesn't match the pinned SHA-256").into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(&p, b).map_err(|e| e.to_string())?;
    Ok(p)
}

fn sha256_hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

// MARK: mensagens de controle (test_control_msg_serialize.c do scrcpy)

pub const TOUCH_DOWN: u8 = 0;
pub const TOUCH_UP: u8 = 1;
pub const TOUCH_MOVE: u8 = 2;
const POINTER_FINGER: u64 = u64::MAX - 1; // SC_POINTER_ID_GENERIC_FINGER

pub fn msg_keycode(action_up: bool, keycode: u32) -> Vec<u8> {
    let mut m = vec![0u8, action_up as u8];
    m.extend(keycode.to_be_bytes());
    m.extend(0u32.to_be_bytes()); // repeat
    m.extend(0u32.to_be_bytes()); // metastate
    m
}

pub fn msg_text(text: &str) -> Vec<u8> {
    // O servidor aceita até 300 bytes por mensagem; corta num limite de caractere.
    let mut end = text.len().min(300);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let t = &text.as_bytes()[..end];
    let mut m = vec![1u8];
    m.extend((t.len() as u32).to_be_bytes());
    m.extend(t);
    m
}

pub fn msg_touch(action: u8, x: u32, y: u32, w: u16, h: u16) -> Vec<u8> {
    let mut m = vec![2u8, action];
    m.extend(POINTER_FINGER.to_be_bytes());
    m.extend(x.to_be_bytes());
    m.extend(y.to_be_bytes());
    m.extend(w.to_be_bytes());
    m.extend(h.to_be_bytes());
    m.extend((if action == TOUCH_UP { 0u16 } else { 0xffff }).to_be_bytes());
    m.extend(0u32.to_be_bytes()); // action button (dedo: nenhum)
    m.extend(0u32.to_be_bytes()); // buttons
    m
}

/// Gira o aparelho 90° (SC_CONTROL_MSG_TYPE_ROTATE_DEVICE).
pub fn msg_rotate() -> Vec<u8> {
    vec![11u8]
}

/// Voltar (ou ligar a tela, se estiver apagada).
pub fn msg_back() -> Vec<u8> {
    vec![4u8, 0]
}

/// Códigos de tecla do Android para os botões do quadro.
pub const KEY_HOME: u32 = 3;
pub const KEY_BACK: u32 = 4;
pub const KEY_VOLUME_UP: u32 = 24;
pub const KEY_VOLUME_DOWN: u32 = 25;
pub const KEY_ENTER: u32 = 66;
pub const KEY_DEL: u32 = 67;
pub const KEY_TAB: u32 = 61;
pub const KEY_APP_SWITCH: u32 = 187;
pub const KEY_DPAD_UP: u32 = 19;
pub const KEY_DPAD_DOWN: u32 = 20;
pub const KEY_DPAD_LEFT: u32 = 21;
pub const KEY_DPAD_RIGHT: u32 = 22;
pub const KEY_FORWARD_DEL: u32 = 112;

/// Nome W3C (o mesmo que o app usa para o emulador) → código do Android.
pub fn keycode(w3c: &str) -> Option<u32> {
    Some(match w3c {
        "GoHome" => KEY_HOME,
        "GoBack" => KEY_BACK,
        "AppSwitch" => KEY_APP_SWITCH,
        "AudioVolumeUp" => KEY_VOLUME_UP,
        "AudioVolumeDown" => KEY_VOLUME_DOWN,
        "Enter" => KEY_ENTER,
        "Backspace" => KEY_DEL,
        "Delete" => KEY_FORWARD_DEL,
        "Tab" => KEY_TAB,
        "ArrowUp" => KEY_DPAD_UP,
        "ArrowDown" => KEY_DPAD_DOWN,
        "ArrowLeft" => KEY_DPAD_LEFT,
        "ArrowRight" => KEY_DPAD_RIGHT,
        _ => return None,
    })
}

// MARK: sessão

/// Um pacote do vídeo.
pub struct Packet {
    pub config: bool,
    pub key: bool,
    pub data: Vec<u8>,
}

/// Uma sessão do scrcpy com um aparelho: o servidor no Android e os dois sockets.
pub struct Session {
    server: Child,
    pub video: TcpStream,
    pub control: TcpStream,
    pub device: String,
    pub width: u32,
    pub height: u32,
    adb: Adb,
    serial: String,
    port: u16,
}

fn read_exact(s: &mut TcpStream, n: usize) -> std::io::Result<Vec<u8>> {
    let mut b = vec![0u8; n];
    s.read_exact(&mut b)?;
    Ok(b)
}

impl Session {
    /// Sobe o servidor e conecta. `max_size` limita o lado maior da tela.
    pub fn start(
        adb: &Adb,
        serial: &str,
        jar: &Path,
        local_port: u16,
        max_size: u32,
    ) -> Result<Session, String> {
        adb.push(serial, jar, REMOTE)?;
        let scid = (std::process::id() ^ local_port as u32 ^ 0x2d5a_17c3) & 0x7fff_ffff;
        adb.forward(
            serial,
            local_port,
            &format!("localabstract:scrcpy_{scid:08x}"),
        )?;
        let cmd = format!(
            "CLASSPATH={REMOTE} app_process / com.genymobile.scrcpy.Server {VERSION} \
             scid={scid:08x} log_level=warn tunnel_forward=true audio=false control=true \
             cleanup=true video_codec=h264 max_size={max_size} max_fps=30 \
             video_bit_rate=4000000 clipboard_autosync=false"
        );
        let mut server = adb.spawn_shell(serial, &cmd).map_err(|e| e.to_string())?;
        // O primeiro socket só vale quando chega o byte de teste: até o servidor
        // subir, a conexão abre (é o adb que escuta) e fecha em seguida.
        let t0 = Instant::now();
        let mut video = loop {
            if let Ok(Some(st)) = server.try_wait() {
                let mut err = String::new();
                if let Some(e) = server.stderr.as_mut() {
                    let _ = e.read_to_string(&mut err);
                }
                return Err(format!("o servidor do scrcpy saiu ({st}): {err}"));
            }
            if t0.elapsed() > Duration::from_secs(15) {
                let _ = server.kill();
                return Err(crate::idioma::t("the scrcpy server didn't respond").into());
            }
            if let Ok(mut s) = TcpStream::connect(("127.0.0.1", local_port)) {
                let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
                let mut b = [0u8; 1];
                if s.read_exact(&mut b).is_ok() {
                    break s;
                }
            }
            std::thread::sleep(Duration::from_millis(150));
        };
        let control = TcpStream::connect(("127.0.0.1", local_port)).map_err(|e| e.to_string())?;
        let _ = video.set_read_timeout(Some(Duration::from_secs(10)));
        let meta = read_exact(&mut video, 64).map_err(|e| e.to_string())?;
        let device = String::from_utf8_lossy(&meta)
            .trim_end_matches('\0')
            .to_string();
        let codec = read_exact(&mut video, 12).map_err(|e| e.to_string())?;
        if &codec[0..4] != b"h264" {
            return Err(format!("codec inesperado: {:?}", &codec[0..4]));
        }
        let width = u32::from_be_bytes(codec[4..8].try_into().unwrap());
        let height = u32::from_be_bytes(codec[8..12].try_into().unwrap());
        let _ = video.set_read_timeout(None);
        let _ = control.set_nodelay(true);
        Ok(Session {
            server,
            video,
            control,
            device,
            width,
            height,
            adb: adb.clone(),
            serial: serial.into(),
            port: local_port,
        })
    }

    pub fn read_packet(&mut self) -> std::io::Result<Packet> {
        read_packet(&mut self.video)
    }

    pub fn send(&mut self, msg: &[u8]) -> std::io::Result<()> {
        self.control.write_all(msg)
    }

    /// Um canal de controle separado, para outra thread mandar toque e texto.
    pub fn controller(&self) -> std::io::Result<TcpStream> {
        self.control.try_clone()
    }
}

/// Lê um pacote: 8 bytes de PTS com as marcas, 4 de tamanho, depois os dados.
pub fn read_packet(video: &mut TcpStream) -> std::io::Result<Packet> {
    let h = read_exact(video, 12)?;
    let pts = u64::from_be_bytes(h[0..8].try_into().unwrap());
    let n = u32::from_be_bytes(h[8..12].try_into().unwrap()) as usize;
    if n > 32 << 20 {
        return Err(std::io::Error::other("pacote grande demais"));
    }
    Ok(Packet {
        config: pts >> 63 == 1,
        key: (pts >> 62) & 1 == 1,
        data: read_exact(video, n)?,
    })
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.video.shutdown(std::net::Shutdown::Both);
        let _ = self.control.shutdown(std::net::Shutdown::Both);
        let _ = self.server.kill();
        let _ = self.server.wait();
        self.adb.forward_remove(&self.serial, self.port);
    }
}

// MARK: decodificação

/// Decodifica o H.264 do scrcpy em RGBA com alfa 255.
pub struct Decoder {
    d: openh264::decoder::Decoder,
}

impl Decoder {
    pub fn new() -> Result<Decoder, String> {
        Ok(Decoder {
            d: openh264::decoder::Decoder::new().map_err(|e| e.to_string())?,
        })
    }

    /// Um pacote; devolve o quadro se ele completou um.
    pub fn decode(&mut self, data: &[u8]) -> Result<Option<crate::screen::Frame>, String> {
        use openh264::formats::YUVSource;
        let Some(yuv) = self.d.decode(data).map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let (w, h) = yuv.dimensions();
        let mut rgba = vec![0u8; w * h * 4];
        yuv.write_rgba8(&mut rgba);
        for px in rgba.as_chunks_mut::<4>().0 {
            px[3] = 255;
        }
        Ok(Some(crate::screen::Frame {
            width: w as u32,
            height: h as u32,
            rgba,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Os bytes esperados são os do teste oficial do scrcpy (test_control_msg_serialize.c).
    #[test]
    fn mensagens_como_as_do_scrcpy() {
        assert_eq!(
            msg_keycode(true, 66),
            [0, 1, 0, 0, 0, 0x42, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        let t = msg_text("hello, world!");
        assert_eq!(&t[..5], [1, 0, 0, 0, 0x0d]);
        assert_eq!(&t[5..], b"hello, world!");
        let m = msg_touch(TOUCH_DOWN, 100, 200, 1080, 1920);
        assert_eq!(m.len(), 32);
        assert_eq!(&m[..2], [2, 0]);
        assert_eq!(
            &m[10..22],
            [0, 0, 0, 0x64, 0, 0, 0, 0xc8, 0x04, 0x38, 0x07, 0x80]
        );
        assert_eq!(&m[22..24], [0xff, 0xff]);
        assert_eq!(&msg_touch(TOUCH_UP, 1, 1, 2, 2)[22..24], [0, 0]);
        assert_eq!(msg_text(&"é".repeat(200)).len(), 5 + 300);
        assert_eq!(keycode("GoHome"), Some(3));
        assert_eq!(keycode("x"), None);
    }

    #[test]
    fn cabecalho_do_pacote() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let pts: u64 = (1 << 63) | 42;
            s.write_all(&pts.to_be_bytes()).unwrap();
            s.write_all(&3u32.to_be_bytes()).unwrap();
            s.write_all(b"abc").unwrap();
        });
        let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let p = read_packet(&mut c).unwrap();
        assert!(p.config && !p.key);
        assert_eq!(p.data, b"abc");
    }
}
