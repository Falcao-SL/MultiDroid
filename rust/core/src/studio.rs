//! Como o Android Studio acha um aparelho do MultiDroid.
//!
//! O Studio não recebe um AVD nosso. Ele usa o `adb` da porta 5037, a mesma
//! do Android Studio. O motor completo aparece como `emulator-<console>`
//! (o emulador oficial se anuncia ao servidor ao subir). O motor leve, no
//! Linux, aparece como `127.0.0.1:<porta>` depois do `adb connect`.
//!
//! O script `rust/scripts/studio-adb.sh` sobe esse servidor e lista os
//! seriais. Aqui o `launch` do emulador faz o mesmo antes de abrir o
//! processo, para o anúncio ter para quem ir.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn serial(console: u16) -> String {
    format!("emulator-{console}")
}

/// Porta em que o emulador escuta o adb: a de console mais um.
pub fn adb_port(console: u16) -> u16 {
    console.saturating_add(1)
}

fn adb_name() -> &'static str {
    if cfg!(windows) { "adb.exe" } else { "adb" }
}

/// Onde procurar um `adb` de verdade, nesta ordem: o SDK do MultiDroid, as
/// variáveis do Studio, o caminho padrão do Studio, e por fim o `PATH`.
pub fn find_adb(sdk: &Path) -> Option<PathBuf> {
    let name = adb_name();
    let mut cands = vec![sdk.join("platform-tools").join(name)];
    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(root) = std::env::var_os(var) {
            cands.push(PathBuf::from(root).join("platform-tools").join(name));
        }
    }
    let home = crate::platform::home();
    if cfg!(target_os = "macos") {
        cands.push(home.join("Library/Android/sdk/platform-tools").join(name));
    } else if cfg!(windows) {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            cands.push(
                PathBuf::from(local)
                    .join("Android/Sdk/platform-tools")
                    .join(name),
            );
        }
    } else {
        cands.push(home.join("Android/Sdk/platform-tools").join(name));
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            cands.push(dir.join(name));
        }
    }
    cands.into_iter().find(|p| p.is_file())
}

/// Sobe o servidor adb na porta 5037, se houver um binário. Não falha o
/// ligamento do aparelho quando o adb não está instalado: o emulador abre
/// do mesmo jeito e o Studio só passa a vê-lo quando o servidor existir.
pub fn prepare_adb_server(sdk: &Path) -> Option<PathBuf> {
    let exe = find_adb(sdk)?;
    let _ = Command::new(&exe)
        .args(["start-server"])
        .env_remove("ANDROID_ADB_SERVER_PORT")
        .env_remove("ADB_SERVER_SOCKET")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    Some(exe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_e_porta_que_o_studio_lista() {
        assert_eq!(serial(5600), "emulator-5600");
        assert_eq!(adb_port(5600), 5601);
        assert_eq!(adb_port(u16::MAX), u16::MAX);
    }

    #[test]
    fn portas_do_emulador_nao_cruzam_as_do_motor_leve() {
        use crate::emulator::CONSOLE_LAST;
        use crate::leve::PORTA0;
        // O intervalo do console é exclusivo no fim: a última par é LAST-2.
        assert!(adb_port(CONSOLE_LAST - 2) < PORTA0);
    }

    #[test]
    fn acha_o_adb_do_sdk_antes_do_resto() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("platform-tools").join(adb_name());
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, b"nao-e-um-binario").unwrap();
        assert_eq!(find_adb(dir.path()).as_deref(), Some(exe.as_path()));
    }
}
