//! Ligar, pausar, hibernar e desligar o emulador oficial, sem janela, e falar
//! com ele por gRPC. Porte de `AppModel.launch/stopAndWait`, `SDK.console` e
//! `EmulatorLink` do app SwiftUI.
//!
//! Desligar e hibernar são o mesmo comando ("kill" no console), que salva o
//! estado: a próxima partida leva segundos pelo Quick Boot. A diferença é só o
//! que a interface mostra depois.

use crate::platform;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Código gerado do .proto do Google: os avisos do clippy são dele, não daqui.
#[allow(clippy::all, clippy::pedantic)]
pub mod proto {
    tonic::include_proto!("android.emulation.control");
}

use proto::emulator_controller_client::EmulatorControllerClient;

/// Portas do console: pares a partir de 5600 (o emulador usa a par para o console e
/// a ímpar para o adb). O gRPC fica em 8600 + metade da distância.
pub const CONSOLE_FIRST: u16 = 5600;
pub const CONSOLE_LAST: u16 = 5680;

pub fn executable(sdk: &Path) -> PathBuf {
    let exe = if cfg!(windows) {
        "emulator.exe"
    } else {
        "emulator"
    };
    sdk.join("emulator").join(exe)
}

/// Ambiente do emulador: só as pastas do MultiDroid, nunca as do Android Studio.
pub fn environment(sdk: &Path, avd_home: &Path, home: &Path) -> Vec<(String, String)> {
    vec![
        ("ANDROID_SDK_ROOT".into(), sdk.display().to_string()),
        ("ANDROID_AVD_HOME".into(), avd_home.display().to_string()),
        ("ANDROID_EMULATOR_HOME".into(), home.display().to_string()),
    ]
}

/// GPU: a do computador no Mac e no PC; MULTIDROID_GPU troca (o CI usa swiftshader_indirect).
pub fn gpu_mode() -> String {
    std::env::var("MULTIDROID_GPU").unwrap_or_else(|_| "host".into())
}

pub fn args(name: &str, cores: u32, console: u16, grpc: u16, gpu: &str) -> Vec<String> {
    [
        "-avd",
        name,
        "-no-window",
        "-no-audio",
        "-no-boot-anim",
        "-no-metrics",
        "-gpu",
        gpu,
        "-cores",
        &cores.to_string(),
        "-grpc",
        &grpc.to_string(),
        "-grpc-use-token",
        "-port",
        &console.to_string(),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Primeira porta de console livre: fora das que outros emuladores já usam e
/// que o sistema deixa abrir agora (console, adb e gRPC).
pub fn free_ports(used: &[u16]) -> Option<(u16, u16)> {
    (CONSOLE_FIRST..CONSOLE_LAST).step_by(2).find_map(|c| {
        let g = 8600 + (c - CONSOLE_FIRST) / 2;
        let free = |p: u16| TcpListener::bind(("127.0.0.1", p)).is_ok();
        (!used.contains(&c) && free(c) && free(c + 1) && free(g)).then_some((c, g))
    })
}

/// Um emulador que este processo abriu.
pub struct Launched {
    pub child: Child,
    pub console: u16,
    pub grpc: u16,
}

impl Launched {
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

pub struct LaunchOptions<'a> {
    pub sdk: &'a Path,
    pub avd_home: &'a Path,
    /// ANDROID_EMULATOR_HOME: a pasta de dados do MultiDroid.
    pub home: &'a Path,
    pub log_dir: &'a Path,
    pub cores: u32,
    pub gpu: String,
}

/// Abre o emulador sem janela. A saída vai para <log_dir>/<aparelho>.log.
pub fn launch(name: &str, used_ports: &[u16], o: &LaunchOptions) -> std::io::Result<Launched> {
    let (console, grpc) = free_ports(used_ports).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::AddrInUse,
            "sem porta livre para o emulador",
        )
    })?;
    std::fs::create_dir_all(o.log_dir)?;
    // Sem platform-tools (mesmo vazia) o emulador não reconhece a pasta como
    // SDK e fecha com "Broken AVD system path".
    std::fs::create_dir_all(o.sdk.join("platform-tools"))?;
    let log = std::fs::File::create(o.log_dir.join(format!("{name}.log")))?;
    let mut cmd = Command::new(executable(o.sdk));
    cmd.args(args(name, o.cores, console, grpc, &o.gpu))
        .env_remove("ANDROID_HOME")
        .envs(environment(o.sdk, o.avd_home, o.home))
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);
    Ok(Launched {
        child: cmd.spawn()?,
        console,
        grpc,
    })
}

// MARK: Descoberta (porta e token gRPC que o emulador escreve ao subir)

/// Onde o emulador escreve pid_<pid>.ini com a porta e o token gRPC de cada instância.
pub fn discovery_dirs() -> Vec<PathBuf> {
    let home = platform::home();
    let mut v = vec![];
    if cfg!(target_os = "macos") {
        v.push(home.join("Library/Caches/TemporaryItems/avd/running"));
    } else if cfg!(windows) {
        if let Some(l) = std::env::var_os("LOCALAPPDATA") {
            v.push(PathBuf::from(l).join("Temp").join("avd").join("running"));
        }
    } else {
        if let Some(x) = std::env::var_os("XDG_RUNTIME_DIR") {
            v.push(PathBuf::from(x).join("avd/running"));
        }
        let user = std::env::var("USER").unwrap_or_default();
        v.push(PathBuf::from(format!("/tmp/android-{user}/avd/running")));
    }
    v.push(std::env::temp_dir().join("avd").join("running"));
    v
}

/// Token gRPC do emulador `pid`. No Windows, emulator.exe é só um lançador e o
/// pid_<pid>.ini sai com o pid do qemu filho; procura nos dois.
pub fn grpc_token(pid: u32) -> Option<String> {
    let ler = |pid: u32| {
        discovery_dirs().iter().find_map(|d| {
            crate::ini::read(&d.join(format!("pid_{pid}.ini")))
                .remove("grpc.token")
                .filter(|t| !t.is_empty())
        })
    };
    ler(pid).or_else(|| {
        let q = engine_pid(pid);
        (q != pid).then(|| ler(q)).flatten()
    })
}

/// O processo qemu de um emulador: o próprio `pid` no Mac e no Linux, o filho
/// qemu-system do emulator.exe no Windows. Sem filho, devolve `pid`.
pub fn engine_pid(pid: u32) -> u32 {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let pai = Pid::from_u32(pid);
    sys.processes()
        .iter()
        .find(|(_, p)| {
            p.parent() == Some(pai) && p.name().to_string_lossy().contains("qemu-system")
        })
        .map_or(pid, |(q, _)| q.as_u32())
}

// MARK: Processos

/// Um emulador rodando, aberto por este app, pela 0.2 ou pelo Android Studio.
#[derive(Clone, Debug, PartialEq)]
pub struct Running {
    pub pid: u32,
    pub avd: String,
    pub console: u16,
    pub grpc: Option<u16>,
    /// Memória residente, em bytes.
    pub memory: u64,
    /// CPU em % de um núcleo, desde a última leitura do mesmo `Processes`.
    pub cpu: f32,
}

/// Lê os argumentos de um processo qemu do emulador.
pub fn parse_cmdline(pid: u32, cmd: &[String]) -> Option<Running> {
    let exe = cmd.first()?;
    if !exe.contains("qemu-system") {
        return None;
    }
    let after = |flag: &str| {
        cmd.iter()
            .position(|a| a == flag)
            .and_then(|i| cmd.get(i + 1))
    };
    Some(Running {
        pid,
        avd: after("-avd")?.clone(),
        console: after("-port").and_then(|p| p.parse().ok()).unwrap_or(5554),
        grpc: after("-grpc").and_then(|p| p.parse().ok()),
        memory: 0,
        cpu: 0.0,
    })
}

/// Lista de processos que guarda a leitura anterior, para a CPU sair certa.
pub struct Processes {
    sys: System,
}

impl Default for Processes {
    fn default() -> Self {
        Self::new()
    }
}

impl Processes {
    pub fn new() -> Self {
        Processes { sys: System::new() }
    }

    pub fn emulators(&mut self) -> Vec<Running> {
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cmd(UpdateKind::OnlyIfNotSet)
                .with_memory()
                .with_cpu(),
        );
        let mut out: Vec<Running> = self
            .sys
            .processes()
            .iter()
            .filter_map(|(pid, p)| {
                let cmd: Vec<String> = p
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy().into_owned())
                    .collect();
                let mut r = parse_cmdline(pid.as_u32(), &cmd)?;
                r.memory = p.memory();
                r.cpu = p.cpu_usage();
                Some(r)
            })
            .collect();
        out.sort_by_key(|r| r.pid);
        out
    }
}

pub fn alive(pid: u32) -> bool {
    let mut sys = System::new();
    let p = Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[p]), true);
    // Um filho que já saiu e ainda não foi recolhido aparece como zumbi: está desligado.
    sys.process(p).is_some_and(|p| {
        !matches!(
            p.status(),
            sysinfo::ProcessStatus::Zombie | sysinfo::ProcessStatus::Dead
        )
    })
}

/// Desliga sem salvar (SIGTERM no Mac e no Linux, TerminateProcess no Windows).
pub fn terminate(pid: u32) -> bool {
    let mut sys = System::new();
    let p = Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[p]), true);
    sys.process(p).is_some_and(|proc_| {
        proc_
            .kill_with(sysinfo::Signal::Term)
            .unwrap_or_else(|| proc_.kill())
    })
}

// MARK: Console (o que "adb emu" faz por baixo)

/// Token do console, em ~/.emulator_console_auth_token.
fn console_token() -> Option<String> {
    std::fs::read_to_string(platform::home().join(".emulator_console_auth_token"))
        .ok()
        .map(|t| t.trim().to_string())
}

/// Manda um comando ao console do emulador (TCP local). Diz se ele respondeu OK.
pub fn console(port: u16, command: &str) -> bool {
    console_with_token(port, command, console_token().as_deref())
}

fn console_with_token(port: u16, command: &str, token: Option<&str>) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_secs(3)) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    let Ok(reader) = s.try_clone() else {
        return false;
    };
    let mut r = BufReader::new(reader);
    let mut read_until_ok = || {
        let mut text = String::new();
        let mut line = String::new();
        loop {
            line.clear();
            match r.read_line(&mut line) {
                Ok(0) | Err(_) => return text,
                Ok(_) => {
                    text.push_str(&line);
                    if line.starts_with("OK") || line.starts_with("KO") {
                        return text;
                    }
                }
            }
        }
    };
    let banner = read_until_ok();
    if banner.contains("Authentication required") {
        let Some(t) = token else { return false };
        if s.write_all(format!("auth {t}\r\n").as_bytes()).is_err()
            || !read_until_ok().contains("OK")
        {
            return false;
        }
    }
    if s.write_all(format!("{command}\r\n").as_bytes()).is_err() {
        return false;
    }
    let answer = read_until_ok();
    // "kill" responde "OK: killing emulator, bye bye" e fecha; às vezes só fecha.
    answer.contains("OK")
        || (command == "kill" && answer.is_empty() && {
            let mut rest = [0u8; 1];
            !matches!(s.read(&mut rest), Ok(n) if n > 0)
        })
}

/// Desliga salvando o estado e espera o processo sair. Se o console não responder,
/// ou o processo não sair em 150 s, encerra sem salvar. Diz se salvou.
///
/// Salvar grava a RAM inteira (4 GB no Android 37) em disco: num Windows com
/// pouca memória livre passa de 20 s, e encerrar no meio deixa um estado
/// quebrado que o emulador descarta no próximo boot (que sai a frio).
pub fn shutdown(pid: u32, console_port: u16) -> bool {
    let saved = console(console_port, "kill");
    if !saved {
        terminate(pid);
    }
    for i in 0..320 {
        if !alive(pid) {
            return saved;
        }
        if i == 300 {
            terminate(pid);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    false
}

// MARK: gRPC

#[derive(Clone)]
pub struct Auth(tonic::metadata::MetadataValue<tonic::metadata::Ascii>);

impl tonic::service::Interceptor for Auth {
    fn call(&mut self, mut req: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        req.metadata_mut().insert("authorization", self.0.clone());
        Ok(req)
    }
}

type Client = EmulatorControllerClient<
    tonic::service::interceptor::InterceptedService<tonic::transport::Channel, Auth>,
>;

/// Conexão gRPC com um emulador (localhost, com token): status, pausa, teclas, toque, tela.
#[derive(Clone)]
pub struct Link {
    pub api: Client,
}

impl Link {
    pub async fn connect(port: u16, token: &str) -> Result<Link, String> {
        let channel = tonic::transport::Endpoint::from_shared(format!("http://127.0.0.1:{port}"))
            .map_err(|e| e.to_string())?
            .connect_timeout(Duration::from_secs(3))
            .connect()
            .await
            .map_err(|e| e.to_string())?;
        let auth = Auth(
            format!("Bearer {token}")
                .parse()
                .map_err(|_| crate::idioma::t("invalid token").to_string())?,
        );
        let api = EmulatorControllerClient::with_interceptor(channel, auth)
            // PNG de tela cheia passa dos 4 MB padrão.
            .max_decoding_message_size(64 << 20);
        Ok(Link { api })
    }

    pub async fn booted(&self) -> bool {
        self.api
            .clone()
            .get_status(())
            .await
            .map(|r| r.into_inner().booted)
            .unwrap_or(false)
    }

    pub async fn set_paused(&self, paused: bool) -> bool {
        use proto::vm_run_state::RunState;
        let state = if paused {
            RunState::Paused
        } else {
            RunState::Running
        };
        self.api
            .clone()
            .set_vm_state(proto::VmRunState {
                state: state as i32,
            })
            .await
            .is_ok()
    }

    pub async fn paused(&self) -> Option<bool> {
        use proto::vm_run_state::RunState;
        let s = self.api.clone().get_vm_state(()).await.ok()?.into_inner();
        Some(s.state == RunState::Paused as i32)
    }

    /// Tecla do Android pelo nome W3C: GoBack, GoHome, AppSwitch, Power,
    /// AudioVolumeUp, AudioVolumeDown.
    pub async fn key(&self, name: &str) {
        let k = proto::KeyboardEvent {
            event_type: proto::keyboard_event::KeyEventType::Keypress as i32,
            key: name.into(),
            ..Default::default()
        };
        let _ = self.api.clone().send_key(k).await;
    }

    /// Texto digitado no computador vai para o campo em foco no Android.
    pub async fn type_text(&self, text: &str) {
        let k = proto::KeyboardEvent {
            text: text.into(),
            ..Default::default()
        };
        let _ = self.api.clone().send_key(k).await;
    }

    /// Toque em coordenadas do aparelho; `down = false` levanta o dedo.
    pub async fn touch(&self, x: i32, y: i32, down: bool) {
        let e = proto::TouchEvent {
            touches: vec![proto::Touch {
                x,
                y,
                pressure: if down { 1 } else { 0 },
                identifier: 0,
                ..Default::default()
            }],
            ..Default::default()
        };
        let _ = self.api.clone().send_touch(e).await;
    }

    /// Gira o aparelho (graus em torno de z: 0 retrato, -90 paisagem).
    pub async fn rotate(&self, z: f32) {
        let m = proto::PhysicalModelValue {
            target: proto::physical_model_value::PhysicalType::Rotation as i32,
            value: Some(proto::ParameterValue {
                data: vec![0.0, 0.0, z],
            }),
            ..Default::default()
        };
        let _ = self.api.clone().set_physical_model(m).await;
    }

    /// Captura em resolução cheia, em PNG.
    pub async fn screenshot_png(&self) -> Option<Vec<u8>> {
        let f = proto::ImageFormat {
            format: proto::image_format::ImgFormat::Png as i32,
            ..Default::default()
        };
        Some(
            self.api
                .clone()
                .get_screenshot(f)
                .await
                .ok()?
                .into_inner()
                .image,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argumentos_como_na_02() {
        let a = args("Loja", 2, 5600, 8600, "host");
        assert_eq!(
            a.join(" "),
            "-avd Loja -no-window -no-audio -no-boot-anim -no-metrics -gpu host -cores 2 -grpc 8600 -grpc-use-token -port 5600"
        );
    }

    #[test]
    fn le_a_linha_de_comando_do_qemu() {
        let cmd: Vec<String> = "/x/emulator/qemu/darwin-aarch64/qemu-system-aarch64 -avd Shop -no-window -grpc 8601 -grpc-use-token -port 5602"
            .split(' ')
            .map(String::from)
            .collect();
        let r = parse_cmdline(42, &cmd).unwrap();
        assert_eq!(
            (r.avd.as_str(), r.console, r.grpc),
            ("Shop", 5602, Some(8601))
        );
        let studio: Vec<String> = ["qemu-system-x86_64", "-avd", "P", "-netdelay", "none"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let r = parse_cmdline(7, &studio).unwrap();
        assert_eq!((r.console, r.grpc), (5554, None));
        assert!(parse_cmdline(1, &["bash".into()]).is_none());
    }

    #[test]
    fn portas_livres_pulam_as_usadas() {
        let (c, g) = free_ports(&[5600, 5602]).unwrap();
        assert!(c >= 5604 && c % 2 == 0);
        assert_eq!(g, 8600 + (c - 5600) / 2);
    }

    /// Um console falso, com autenticação, como o do emulador.
    #[test]
    fn console_autentica_e_manda_o_comando() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            s.write_all(b"Android Console: Authentication required\r\nOK\r\n")
                .unwrap();
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut got = vec![];
            for _ in 0..2 {
                let mut line = String::new();
                r.read_line(&mut line).unwrap();
                got.push(line.trim().to_string());
                s.write_all(b"OK\r\n").unwrap();
            }
            got
        });
        assert!(console_with_token(port, "avd status", Some("segredo")));
        assert_eq!(server.join().unwrap(), ["auth segredo", "avd status"]);
        assert!(!console_with_token(1, "kill", None));
    }
}
