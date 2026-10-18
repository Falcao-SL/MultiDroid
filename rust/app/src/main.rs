// MultiDroid em Rust e Slint: um código para Mac, Windows e Linux
// (docs/spec/MULTIPLATAFORMA.md). A interface fica na thread principal; o
// emulador, o gRPC e os quadros rodam num runtime tokio e avisam a interface
// por `invoke_from_event_loop`.

slint::include_modules!();

use multidroid_core::devices::{self, Device, Kind};
use multidroid_core::emulator::{self, LaunchOptions, Launched, Link, Processes};
use multidroid_core::state::{MAIN_DESK, State};
use multidroid_core::{catalog, platform, screen, tokens};
use slint::{Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn cor(hex: &str) -> slint::Color {
    let (r, g, b) = tokens::rgb(hex);
    slint::Color::from_rgb_u8(r, g, b)
}

/// O que está rodando de um aparelho agora.
struct Dev {
    info: Device,
    estado: Estado,
    pid: Option<u32>,
    console: Option<u16>,
    child: Option<Launched>,
    link: Option<Link>,
    /// Para o fluxo de quadros atual.
    parar_tela: Option<Arc<AtomicBool>>,
    /// Último quadro (também a tela esmaecida de quem hibernou).
    quadro: Option<SharedPixelBuffer<Rgba8Pixel>>,
    /// Já há um quadro esperando a interface: não empilha outro.
    quadro_pendente: Arc<AtomicBool>,
    mensagem: String,
    paisagem: bool,
    memoria: u64,
    cpu: f32,
    ultimo_uso: Instant,
}

impl Dev {
    fn new(info: Device) -> Dev {
        Dev {
            info,
            estado: Estado::Desligado,
            pid: None,
            console: None,
            child: None,
            link: None,
            parar_tela: None,
            quadro: None,
            quadro_pendente: Arc::new(AtomicBool::new(false)),
            mensagem: String::new(),
            paisagem: false,
            memoria: 0,
            cpu: 0.0,
            ultimo_uso: Instant::now() - Duration::from_secs(3600),
        }
    }

    fn rodando(&self) -> bool {
        !matches!(self.estado, Estado::Desligado | Estado::Hibernado)
    }
}

struct App {
    devs: Vec<Dev>,
    state: State,
    aviso: String,
    memoria: String,
}

type Shared = Arc<Mutex<App>>;

struct Ctx {
    app: Shared,
    ui: slint::Weak<AppWindow>,
    rt: tokio::runtime::Handle,
}

impl Clone for Ctx {
    fn clone(&self) -> Self {
        Ctx {
            app: self.app.clone(),
            ui: self.ui.clone(),
            rt: self.rt.clone(),
        }
    }
}

fn state_path() -> std::path::PathBuf {
    platform::data_dir().join("state.json")
}

fn gb(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / (1u64 << 30) as f64).replace('.', ",")
}

/// Proporção da tela (largura / altura), girada se estiver em paisagem.
fn proporcao(d: &Dev) -> f32 {
    if let Some(q) = &d.quadro {
        return q.width() as f32 / q.height().max(1) as f32;
    }
    let (w, h) = screen::frame_size(d.info.kind, &d.info.screen);
    let r = w as f32 / h.max(1) as f32;
    if d.paisagem { 1.0 / r } else { r }
}

fn linha(d: &Dev, st: &State) -> Aparelho {
    let spot = st.spots.get(&d.info.name).copied();
    let (w, h) = d.info.kind.grid_size();
    let s = spot.unwrap_or(multidroid_core::state::Spot { x: 0, y: 0, w, h });
    Aparelho {
        nome: d.info.name.as_str().into(),
        tipo: d.info.kind.label().into(),
        android: catalog::android_version(&d.info.api).into(),
        estado: d.estado,
        mensagem: d.mensagem.as_str().into(),
        metricas: if d.rodando() && d.memoria > 0 {
            format!("{} · {:.0}%", gb(d.memoria), d.cpu).into()
        } else {
            SharedString::new()
        },
        grupo: st
            .groups
            .get(&d.info.name)
            .cloned()
            .unwrap_or_default()
            .into(),
        x: s.x,
        y: s.y,
        w: s.w,
        h: s.h,
        proporcao: proporcao(d),
        tela: d
            .quadro
            .clone()
            .map(slint::Image::from_rgba8)
            .unwrap_or_default(),
        tem_tela: d.quadro.is_some(),
        relogio: d.info.kind == Kind::Relogio,
    }
}

/// Atualiza a interface a partir do estado (só na thread da interface).
fn sincronizar(app: &App, ui: &AppWindow) {
    let lista: Vec<&Dev> = {
        let infos: Vec<Device> = app.devs.iter().map(|d| d.info.clone()).collect();
        let ordem: Vec<String> = app
            .state
            .ordered(&infos)
            .iter()
            .filter(|d| app.state.desk_of(&d.name) == MAIN_DESK)
            .map(|d| d.name.clone())
            .collect();
        ordem
            .iter()
            .filter_map(|n| app.devs.iter().find(|d| &d.info.name == n))
            .collect()
    };
    let rows: Vec<Aparelho> = lista.iter().map(|d| linha(d, &app.state)).collect();
    let model = ui.get_aparelhos();
    match model.as_any().downcast_ref::<VecModel<Aparelho>>() {
        Some(vm) if vm.row_count() == rows.len() => {
            for (i, r) in rows.into_iter().enumerate() {
                vm.set_row_data(i, r);
            }
        }
        _ => ui.set_aparelhos(ModelRc::new(VecModel::from(rows))),
    }
    ui.set_aviso(app.aviso.as_str().into());
    ui.set_memoria(app.memoria.as_str().into());
}

impl Ctx {
    /// Pede à interface para se redesenhar a partir do estado.
    fn avisar(&self) {
        let app = self.app.clone();
        let ui = self.ui.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui.upgrade() {
                sincronizar(&app.lock().unwrap(), &ui);
            }
        });
    }

    fn com<R>(&self, nome: &str, f: impl FnOnce(&mut Dev, &mut App) -> R) -> Option<R> {
        let mut g = self.app.lock().unwrap();
        let app: &mut App = &mut g;
        let i = app.devs.iter().position(|d| d.info.name == nome)?;
        // Tira o aparelho da lista para emprestar os dois sem conflito.
        let mut d = app.devs.remove(i);
        let r = f(&mut d, app);
        app.devs.insert(i, d);
        Some(r)
    }

    fn aviso(&self, texto: impl Into<String>) {
        self.app.lock().unwrap().aviso = texto.into();
        self.avisar();
    }

    // MARK: ligar e conectar

    fn ligar(&self, nome: &str) {
        let sdk = devices::sdk_root();
        if catalog::installed(catalog::EMULATOR, &sdk).is_none() {
            self.aviso("Falta o emulador: instale pela versão 0.2 por enquanto");
            return;
        }
        let pronto = self.com(nome, |d, app| {
            if d.rodando() {
                return Err(String::new());
            }
            if catalog::installed(&d.info.image, &sdk).is_none() {
                return Err(format!(
                    "{} usa {}, que não está instalada",
                    d.info.name,
                    catalog::title(&d.info.image, catalog::Host::current())
                ));
            }
            let usadas: Vec<u16> = app.devs.iter().filter_map(|o| o.console).collect();
            let mut fora: Vec<u16> = Processes::new()
                .emulators()
                .iter()
                .map(|r| r.console)
                .collect();
            fora.extend(usadas);
            let opts = LaunchOptions {
                sdk: &sdk,
                avd_home: &devices::avd_home(),
                home: &platform::data_dir(),
                log_dir: &platform::logs_dir(),
                cores: d.info.kind.cores(),
                gpu: emulator::gpu_mode(),
            };
            let l = emulator::launch(&d.info.name, &fora, &opts)
                .map_err(|e| format!("não abriu o emulador: {e}"))?;
            let r = (l.pid(), l.grpc);
            d.pid = Some(l.pid());
            d.console = Some(l.console);
            d.child = Some(l);
            d.estado = Estado::Ligando;
            d.mensagem = "ligando…".into();
            d.ultimo_uso = Instant::now();
            Ok(r)
        });
        match pronto {
            Some(Ok((pid, grpc))) => {
                self.avisar();
                let c = self.clone();
                let nome = nome.to_string();
                self.rt
                    .spawn(async move { c.esperar_token(&nome, pid, grpc).await });
            }
            Some(Err(e)) if !e.is_empty() => self.aviso(e),
            _ => {}
        }
    }

    async fn esperar_token(&self, nome: &str, pid: u32, grpc: u16) {
        for _ in 0..240 {
            if let Some(t) = emulator::grpc_token(pid) {
                return self.conectar(nome, grpc, &t).await;
            }
            let vivo = self
                .com(nome, |d, _| d.child.as_mut().is_some_and(|c| c.alive()))
                .unwrap_or(false);
            if !vivo {
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        self.com(nome, |d, _| {
            d.estado = Estado::Desligado;
            d.pid = None;
            d.console = None;
            d.child = None;
            d.mensagem = "o emulador fechou ao ligar".into();
        });
        self.avisar();
    }

    async fn conectar(&self, nome: &str, grpc: u16, token: &str) {
        let link = match Link::connect(grpc, token).await {
            Ok(l) => l,
            Err(e) => {
                self.com(nome, |d, _| d.mensagem = format!("sem conexão gRPC: {e}"));
                return self.avisar();
            }
        };
        self.com(nome, |d, _| d.link = Some(link.clone()));
        for _ in 0..600 {
            if link.booted().await {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        self.com(nome, |d, _| {
            if matches!(d.estado, Estado::Ligando | Estado::Externo) {
                d.estado = Estado::Ligado;
            }
            d.mensagem.clear();
        });
        self.avisar();
        self.abrir_tela(nome);
    }

    fn abrir_tela(&self, nome: &str) {
        let Some((link, kind, tela, parar, pendente)) = self
            .com(nome, |d, _| {
                if let Some(p) = d.parar_tela.take() {
                    p.store(true, Ordering::Relaxed);
                }
                let parar = Arc::new(AtomicBool::new(false));
                d.parar_tela = Some(parar.clone());
                Some((
                    d.link.clone()?,
                    d.info.kind,
                    d.info.screen.clone(),
                    parar,
                    d.quadro_pendente.clone(),
                ))
            })
            .flatten()
        else {
            return;
        };
        let c = self.clone();
        let nome = nome.to_string();
        self.rt.spawn(async move {
            let (w, h) = screen::frame_size(kind, &tela);
            let transporte = screen::Transport::for_device(&nome);
            let _ = screen::stream(&link, w, h, &transporte, |f| {
                if parar.load(Ordering::Relaxed) {
                    return false;
                }
                let buf =
                    SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&f.rgba, f.width, f.height);
                c.com(&nome, |d, _| d.quadro = Some(buf));
                if !pendente.swap(true, Ordering::AcqRel) {
                    let (app, ui, nome, pendente) =
                        (c.app.clone(), c.ui.clone(), nome.clone(), pendente.clone());
                    let _ = slint::invoke_from_event_loop(move || {
                        pendente.store(false, Ordering::Release);
                        let Some(ui) = ui.upgrade() else { return };
                        let app = app.lock().unwrap();
                        let model = ui.get_aparelhos();
                        let Some(vm) = model.as_any().downcast_ref::<VecModel<Aparelho>>() else {
                            return;
                        };
                        for i in 0..vm.row_count() {
                            let mut r = vm.row_data(i).unwrap();
                            if r.nome == nome.as_str() {
                                if let Some(d) = app.devs.iter().find(|d| d.info.name == nome) {
                                    r.tela = d
                                        .quadro
                                        .clone()
                                        .map(slint::Image::from_rgba8)
                                        .unwrap_or_default();
                                    r.tem_tela = d.quadro.is_some();
                                    r.proporcao = proporcao(d);
                                }
                                vm.set_row_data(i, r);
                            }
                        }
                    });
                }
                true
            })
            .await;
        });
    }

    // MARK: pausar, hibernar, desligar

    fn pausar(&self, nome: &str, pausar: bool) {
        let Some(Some(link)) = self.com(nome, |d, _| {
            let ok = if pausar {
                d.estado == Estado::Ligado
            } else {
                d.estado == Estado::Pausado
            };
            if !ok {
                return None;
            }
            d.estado = if pausar {
                Estado::Pausado
            } else {
                Estado::Ligado
            };
            if pausar {
                if let Some(p) = d.parar_tela.take() {
                    p.store(true, Ordering::Relaxed);
                }
            } else {
                d.ultimo_uso = Instant::now();
            }
            d.link.clone()
        }) else {
            return;
        };
        self.avisar();
        let c = self.clone();
        let nome = nome.to_string();
        self.rt.spawn(async move {
            link.set_paused(pausar).await;
            if !pausar {
                c.abrir_tela(&nome);
            }
        });
    }

    /// Desligar e hibernar salvam o estado; hibernar guarda a última tela.
    fn desligar(&self, nome: &str, hibernar: bool) {
        let alvo = self.com(nome, |d, _| {
            if d.estado == Estado::Hibernado && !hibernar {
                d.estado = Estado::Desligado;
                d.quadro = None;
                return None;
            }
            if !d.rodando() || d.estado == Estado::Desligando {
                return None;
            }
            if hibernar && !matches!(d.estado, Estado::Ligado | Estado::Pausado) {
                return None;
            }
            d.estado = Estado::Desligando;
            if let Some(p) = d.parar_tela.take() {
                p.store(true, Ordering::Relaxed);
            }
            Some((d.pid?, d.console?))
        });
        self.avisar();
        let Some(Some((pid, console))) = alvo else {
            return;
        };
        let c = self.clone();
        let nome = nome.to_string();
        self.rt.spawn(async move {
            tokio::task::spawn_blocking(move || emulator::shutdown(pid, console))
                .await
                .ok();
            c.com(&nome, |d, _| {
                if let Some(ch) = d.child.as_mut() {
                    ch.alive();
                }
                d.child = None;
                d.link = None;
                d.pid = None;
                d.console = None;
                d.memoria = 0;
                d.cpu = 0.0;
                d.mensagem.clear();
                d.estado = if hibernar {
                    Estado::Hibernado
                } else {
                    Estado::Desligado
                };
                if !hibernar {
                    d.quadro = None;
                }
            });
            c.avisar();
        });
    }

    // MARK: botões, toque e teclado

    fn com_link(&self, nome: &str) -> Option<Link> {
        self.com(nome, |d, _| {
            d.ultimo_uso = Instant::now();
            if d.estado == Estado::Ligado {
                d.link.clone()
            } else {
                None
            }
        })
        .flatten()
    }

    fn acao(&self, nome: &str, acao: &str) {
        match acao {
            "ligar" => return self.ligar(nome),
            "pausar" => return self.pausar(nome, true),
            "retomar" => return self.pausar(nome, false),
            "hibernar" => return self.desligar(nome, true),
            "desligar" => return self.desligar(nome, false),
            _ => {}
        }
        let Some(link) = self.com_link(nome) else {
            return;
        };
        let c = self.clone();
        let (nome, acao) = (nome.to_string(), acao.to_string());
        self.rt.spawn(async move {
            match acao.as_str() {
                "voltar" => link.key("GoBack").await,
                "inicio" => link.key("GoHome").await,
                "recentes" => link.key("AppSwitch").await,
                "volume+" => link.key("AudioVolumeUp").await,
                "volume-" => link.key("AudioVolumeDown").await,
                "girar" => {
                    let paisagem = c
                        .com(&nome, |d, _| {
                            d.paisagem = !d.paisagem;
                            d.paisagem
                        })
                        .unwrap_or(false);
                    link.rotate(if paisagem { -90.0 } else { 0.0 }).await;
                }
                "captura" => {
                    if let Some(png) = link.screenshot_png().await {
                        let dir = platform::pictures_dir();
                        let stamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let ok = std::fs::create_dir_all(&dir).is_ok()
                            && std::fs::write(dir.join(format!("{nome}-{stamp}.png")), png).is_ok();
                        c.aviso(if ok {
                            format!("Captura salva em {}", dir.display())
                        } else {
                            "A captura não foi salva".into()
                        });
                    }
                }
                _ => {}
            }
        });
    }

    fn toque(&self, nome: &str, fx: f32, fy: f32, tipo: &str) {
        let Some(link) = self.com_link(nome) else {
            return;
        };
        let tela = self
            .com(nome, |d, _| d.info.screen.clone())
            .unwrap_or_default();
        let Some((w, h)) = tela
            .split_once('×')
            .and_then(|(a, b)| Some((a.parse::<f32>().ok()?, b.parse::<f32>().ok()?)))
        else {
            return;
        };
        let x = (fx.clamp(0.0, 1.0) * w) as i32;
        let y = (fy.clamp(0.0, 1.0) * h) as i32;
        let down = tipo != "up";
        self.rt.spawn(async move { link.touch(x, y, down).await });
    }

    fn tecla(&self, nome: &str, texto: &str) {
        let Some(link) = self.com_link(nome) else {
            return;
        };
        // Teclas especiais do Slint viram os nomes W3C que o emulador entende.
        let especial = match texto {
            "\u{8}" => Some("Backspace"),
            "\u{7f}" => Some("Delete"),
            "\n" | "\r" => Some("Enter"),
            "\t" => Some("Tab"),
            "\u{1b}" => Some("GoBack"),
            "\u{f700}" => Some("ArrowUp"),
            "\u{f701}" => Some("ArrowDown"),
            "\u{f702}" => Some("ArrowLeft"),
            "\u{f703}" => Some("ArrowRight"),
            _ => None,
        };
        let texto = texto.to_string();
        if especial.is_none()
            && (texto.is_empty()
                || texto
                    .chars()
                    .any(|c| ('\u{f700}'..='\u{f8ff}').contains(&c)))
        {
            return;
        }
        self.rt.spawn(async move {
            match especial {
                Some(k) => link.key(k).await,
                None => link.type_text(&texto).await,
            }
        });
    }

    // MARK: aparelhos rodando e métricas

    /// Mostra os aparelhos já ligados (pela 0.2 ou antes) e, com gRPC e token, a tela deles.
    fn adotar(&self, rodando: &[emulator::Running]) {
        let mut conectar = vec![];
        {
            let mut app = self.app.lock().unwrap();
            for d in app.devs.iter_mut() {
                let Some(r) = rodando.iter().find(|r| r.avd == d.info.name) else {
                    continue;
                };
                if d.pid.is_some_and(|p| p != r.pid) {
                    continue;
                }
                if d.pid.is_none() {
                    d.pid = Some(r.pid);
                    d.console = Some(r.console);
                    d.estado = Estado::Externo;
                    if let (Some(g), Some(t)) = (r.grpc, emulator::grpc_token(r.pid)) {
                        conectar.push((d.info.name.clone(), g, t));
                    }
                }
            }
        }
        for (n, g, t) in conectar {
            let c = self.clone();
            self.rt.spawn(async move { c.conectar(&n, g, &t).await });
        }
    }

    fn medir(&self, procs: &mut Processes) {
        let rodando = procs.emulators();
        self.adotar(&rodando);
        let (usada, swap) = platform::memory_use_gb();
        let total = platform::physical_gb();
        let mut app = self.app.lock().unwrap();
        let mut soma = 0u64;
        for d in app.devs.iter_mut() {
            match d.pid.and_then(|p| rodando.iter().find(|r| r.pid == p)) {
                Some(r) => {
                    d.memoria = r.memory;
                    d.cpu = r.cpu;
                    soma += r.memory;
                }
                None if d.pid.is_some() && d.child.is_none() && d.estado != Estado::Desligando => {
                    // Fechou por fora.
                    d.pid = None;
                    d.console = None;
                    d.link = None;
                    d.estado = Estado::Desligado;
                }
                None => {}
            }
        }
        app.memoria = format!(
            "Memória do computador: {:.1} de {:.0} GB\nAparelhos {} · swap {:.1} GB · teto {:.0} GB",
            usada,
            total,
            gb(soma),
            swap,
            app.state.ceiling_gb
        )
        .replace('.', ",");
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let t = tokens::load();
    let ui = AppWindow::new()?;
    let tema = ui.global::<Tema>();
    // Tema escuro dos tokens, o mesmo das capturas da 0.2.
    let e = &t.cor.escuro;
    let a = &t.cor.acento;
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
    ui.set_versao(env!("CARGO_PKG_VERSION").into());

    let runtime = tokio::runtime::Runtime::new().expect("sem runtime tokio");
    let infos = devices::load(&devices::avd_home());
    let mut state = State::load(&state_path(), platform::physical_gb());
    let ordem: Vec<&Device> = state.ordered(&infos);
    let mudou = state.ensure_spots(&ordem.clone());
    if mudou {
        let _ = state.save(&state_path());
    }
    let app = Arc::new(Mutex::new(App {
        devs: infos.into_iter().map(Dev::new).collect(),
        state,
        aviso: String::new(),
        memoria: String::new(),
    }));
    ui.set_sem_emulador(catalog::installed(catalog::EMULATOR, &devices::sdk_root()).is_none());
    let ctx = Ctx {
        app: app.clone(),
        ui: ui.as_weak(),
        rt: runtime.handle().clone(),
    };
    sincronizar(&app.lock().unwrap(), &ui);

    let c = ctx.clone();
    ui.on_acao(move |n, a| c.acao(&n, &a));
    let c = ctx.clone();
    ui.on_toque(move |n, x, y, k| c.toque(&n, x, y, &k));
    let c = ctx.clone();
    ui.on_tecla(move |n, t| c.tecla(&n, &t));

    // Métricas e aparelhos abertos por fora, a cada 2 s, fora da interface.
    let c = ctx.clone();
    runtime.spawn(async move {
        let mut procs = Processes::new();
        loop {
            let c2 = c.clone();
            procs = tokio::task::spawn_blocking(move || {
                c2.medir(&mut procs);
                procs
            })
            .await
            .expect("métricas");
            c.avisar();
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    let r = ui.run();
    // Ao fechar, os aparelhos continuam ligados, como na 0.2: a próxima abertura os adota.
    runtime.shutdown_timeout(Duration::from_secs(1));
    r
}
