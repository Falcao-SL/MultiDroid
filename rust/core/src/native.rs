//! Ponte para o núcleo C (`rust/native`). O processo que carrega o código é
//! este, o hospedeiro. A cópia do quadro corre numa thread própria; a thread
//! da interface Slint só recebe o buffer já opaco.

use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Apple,
    Windows,
    Linux,
    Unknown,
}

unsafe extern "C" {
    fn md_host() -> i32;
    fn md_frame_opaque(dst: *mut u8, src: *const u8, bytes: usize) -> usize;
    fn md_frame_last_blit_ns() -> u64;
    fn md_pump_new() -> *mut PumpRaw;
    fn md_pump_free(pump: *mut PumpRaw);
    fn md_pump_blit(
        pump: *mut PumpRaw,
        dst: *mut u8,
        src: *const u8,
        bytes: usize,
        out_ns: *mut u64,
    ) -> i32;
    fn md_pump_caller_is_worker(pump: *const PumpRaw) -> i32;
}

enum PumpRaw {}

pub fn host() -> Host {
    match unsafe { md_host() } {
        1 => Host::Apple,
        2 => Host::Windows,
        3 => Host::Linux,
        _ => Host::Unknown,
    }
}

/// Força o alfa em 255 no próprio buffer. Pixels incompletos no fim ficam.
pub fn stamp_inplace(buf: &mut [u8]) {
    if buf.is_empty() {
        return;
    }
    let p = buf.as_mut_ptr();
    // SAFETY: `p` aponta para `buf.len()` bytes; a função C só escreve esse
    // intervalo, e dst e src são o mesmo ponteiro de propósito.
    unsafe {
        md_frame_opaque(p, p, buf.len());
    }
}

pub fn last_blit_ns() -> u64 {
    unsafe { md_frame_last_blit_ns() }
}

/// Thread dedicada ao blit. Não é `Sync`: um fluxo de tela usa uma bomba.
pub struct Pump {
    raw: *mut PumpRaw,
    last_ns: Cell<u64>,
}

// SAFETY: a bomba C tem o seu mutex; este dono não é compartilhado entre
// threads Rust ao mesmo tempo (não implementamos Sync). Pode mudar de thread
// com o future do tokio.
unsafe impl Send for Pump {}

impl Pump {
    pub fn start() -> Option<Pump> {
        let raw = unsafe { md_pump_new() };
        if raw.is_null() {
            None
        } else {
            Some(Pump {
                raw,
                last_ns: Cell::new(0),
            })
        }
    }

    /// Um quadro opaco, numa cópia só, feita na thread da bomba.
    /// `src` tem de viver até a função voltar — ela espera o worker.
    pub fn copy_frame(&self, src: &[u8]) -> Option<Vec<u8>> {
        let n = src.len() & !3;
        if n == 0 {
            self.last_ns.set(0);
            return Some(Vec::new());
        }
        let mut v = Vec::with_capacity(n);
        let mut ns = 0u64;
        let dst = v.spare_capacity_mut().as_mut_ptr().cast::<u8>();
        // SAFETY: `dst` tem capacidade para `n` bytes. `md_pump_blit` escreve
        // esses bytes na thread da bomba e só volta depois. `set_len` corre
        // só nesse caso. `src` permanece emprestado até o retorno.
        let ok = unsafe { md_pump_blit(self.raw, dst, src.as_ptr(), n, &mut ns) };
        if ok == 0 {
            return None;
        }
        unsafe { v.set_len(n) };
        self.last_ns.set(ns);
        Some(v)
    }

    pub fn last_blit_ns(&self) -> u64 {
        self.last_ns.get()
    }

    pub fn caller_is_worker(&self) -> bool {
        unsafe { md_pump_caller_is_worker(self.raw) != 0 }
    }
}

impl Drop for Pump {
    fn drop(&mut self) {
        unsafe { md_pump_free(self.raw) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_e_o_desta_maquina() {
        let h = host();
        if cfg!(target_os = "macos") {
            assert_eq!(h, Host::Apple);
        } else if cfg!(windows) {
            assert_eq!(h, Host::Windows);
        } else if cfg!(target_os = "linux") {
            assert_eq!(h, Host::Linux);
        } else {
            assert_eq!(h, Host::Unknown);
        }
    }

    #[test]
    fn carimbo_no_lugar_bate_com_o_escalar() {
        let mut buf = vec![0u8; 512 * 512 * 4];
        for (i, b) in buf.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }
        let mut esperado = buf.clone();
        for px in esperado.as_chunks_mut::<4>().0 {
            px[3] = 255;
        }
        stamp_inplace(&mut buf);
        assert_eq!(buf, esperado);
        assert!(last_blit_ns() > 0, "o relógio do blit ficou em zero");
    }

    #[test]
    fn a_bomba_nao_corre_na_thread_de_quem_chama() {
        let pump = Pump::start().expect("thread do blit");
        assert!(!pump.caller_is_worker());
        let src = [7u8, 8, 9, 1].repeat(256 * 256);
        let quadro = pump.copy_frame(&src).expect("blit");
        assert_eq!(quadro.len(), src.len());
        assert!(
            quadro
                .as_chunks::<4>()
                .0
                .iter()
                .all(|px| *px == [7, 8, 9, 255])
        );
        assert!(!pump.caller_is_worker());
        assert!(pump.last_blit_ns() > 0);
    }
}
