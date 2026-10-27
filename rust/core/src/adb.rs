//! O adb do pacote oficial platform-tools, instalado pelo MultiDroid como o
//! emulador (`catalog::PLATFORM_TOOLS`). O motor leve usa para tela e toque.
//! O emulador oficial fala gRPC com o MultiDroid e, à parte, se anuncia a este
//! mesmo adb para o Android Studio (ADR-001). Sempre a porta 5037.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const PLATFORM_TOOLS: &str = "platform-tools";

#[derive(Clone, Debug)]
pub struct Adb {
    pub exe: PathBuf,
}

impl Adb {
    /// O adb do SDK do MultiDroid, se instalado.
    pub fn from_sdk(sdk: &Path) -> Option<Adb> {
        let exe = sdk
            .join(PLATFORM_TOOLS)
            .join(if cfg!(windows) { "adb.exe" } else { "adb" });
        exe.is_file().then_some(Adb { exe })
    }

    fn cmd(&self, serial: Option<&str>) -> Command {
        let mut c = Command::new(&self.exe);
        // A porta do Android Studio. Outra porta deixa o Run sem este aparelho.
        c.env_remove("ANDROID_ADB_SERVER_PORT");
        c.env_remove("ADB_SERVER_SOCKET");
        if let Some(s) = serial {
            c.args(["-s", s]);
        }
        c.stdin(Stdio::null());
        c
    }

    fn out(&self, serial: Option<&str>, args: &[&str]) -> Result<String, String> {
        let o = self
            .cmd(serial)
            .args(args)
            .output()
            .map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if o.status.success() {
            Ok(text)
        } else {
            Err(format!(
                "{text} {}",
                String::from_utf8_lossy(&o.stderr).trim()
            ))
        }
    }

    pub fn connect(&self, serial: &str) -> bool {
        self.out(None, &["connect", serial])
            .is_ok_and(|t| t.contains("connected"))
    }

    pub fn disconnect(&self, serial: &str) {
        let _ = self.out(None, &["disconnect", serial]);
    }

    pub fn shell(&self, serial: &str, cmd: &str) -> Result<String, String> {
        self.out(Some(serial), &["shell", cmd])
    }

    pub fn booted(&self, serial: &str) -> bool {
        self.shell(serial, "getprop sys.boot_completed")
            .is_ok_and(|v| v.trim() == "1")
    }

    /// Conecta e espera o boot terminar.
    pub fn wait_boot(&self, serial: &str, timeout: Duration) -> Result<(), String> {
        let t0 = Instant::now();
        while t0.elapsed() < timeout {
            if self.connect(serial) && self.booted(serial) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        Err(crate::idioma::tf(
            "{serial} didn't finish booting in {timeout}",
            &[("serial", &serial), ("timeout", &format!("{timeout:?}"))],
        ))
    }

    pub fn push(&self, serial: &str, local: &Path, remote: &str) -> Result<(), String> {
        let l = local.display().to_string();
        self.out(Some(serial), &["push", &l, remote]).map(|_| ())
    }

    pub fn forward(&self, serial: &str, local_port: u16, remote: &str) -> Result<(), String> {
        let l = format!("tcp:{local_port}");
        self.out(Some(serial), &["forward", &l, remote]).map(|_| ())
    }

    pub fn forward_remove(&self, serial: &str, local_port: u16) {
        let l = format!("tcp:{local_port}");
        let _ = self.out(Some(serial), &["forward", "--remove", &l]);
    }

    /// Um comando do shell que fica rodando (o servidor do scrcpy).
    pub fn spawn_shell(&self, serial: &str, cmd: &str) -> std::io::Result<Child> {
        self.cmd(Some(serial))
            .args(["shell", cmd])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
    }

    /// Captura em PNG pelo screencap do Android.
    pub fn screencap(&self, serial: &str) -> Result<Vec<u8>, String> {
        let o = self
            .cmd(Some(serial))
            .args(["exec-out", "screencap", "-p"])
            .output()
            .map_err(|e| e.to_string())?;
        if o.status.success() && o.stdout.starts_with(b"\x89PNG") {
            Ok(o.stdout)
        } else {
            Err("sem captura".into())
        }
    }

    /// Tecla do Android pelo código (KEYCODE_*), pelo shell.
    pub fn keyevent(&self, serial: &str, code: u32) {
        let _ = self.shell(serial, &format!("input keyevent {code}"));
    }
}
