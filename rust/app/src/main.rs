// MultiDroid em Rust e Slint: um código para Mac, Windows e Linux
// (docs/spec/MULTIPLATAFORMA.md). A interface fica na thread principal; o
// emulador, o gRPC e os quadros rodam num runtime tokio e avisam a interface
// por `invoke_from_event_loop`.

slint::include_modules!();

use multidroid_core::adb::Adb;
use multidroid_core::catalog::{Host, RemotePackage};
use multidroid_core::devices::{self, Device, Kind};
use multidroid_core::emulator::{self, LaunchOptions, Launched, Link, Processes};
use multidroid_core::idioma;
use multidroid_core::install;
use multidroid_core::state::{MAIN_DESK, State, grid};
use multidroid_core::{catalog, platform, screen, tokens};
use multidroid_core::{leve, scrcpy};
use slint::{Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use std::collections::{BTreeMap, HashMap};
use std::io::Write as _;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn cor(hex: &str) -> slint::Color {
    let (r, g, b) = tokens::rgb(hex);
    slint::Color::from_rgb_u8(r, g, b)
}

/// Toque, texto e teclas de um aparelho leve: o socket de controle do scrcpy.
#[derive(Clone)]
struct CanalLeve {
    ctl: Arc<Mutex<TcpStream>>,
    w: u16,
    h: u16,
    adb: Adb,
    serial: String,
}

impl CanalLeve {
    fn enviar(&self, msg: &[u8]) {
        if let Ok(mut s) = self.ctl.lock() {
            let _ = s.write_all(msg);
        }
    }

    fn tecla(&self, w3c: &str) {
        if let Some(k) = scrcpy::keycode(w3c) {
            self.enviar(&scrcpy::msg_keycode(false, k));
            self.enviar(&scrcpy::msg_keycode(true, k));
        }
    }
}

/// Por onde falar com um aparelho ligado: gRPC no emulador, scrcpy no leve.
#[derive(Clone)]
enum Canal {
    Emu(Link),
    Leve(CanalLeve),
}

impl Canal {
    async fn tecla(&self, w3c: &str) {
        match self {
            Canal::Emu(l) => l.key(w3c).await,
            Canal::Leve(l) => l.tecla(w3c),
        }
    }

    async fn texto(&self, t: &str) {
        match self {
            Canal::Emu(l) => l.type_text(t).await,
            Canal::Leve(l) => l.enviar(&scrcpy::msg_text(t)),
        }
    }

    /// Toque em frações da tela (0 a 1); `tipo` é down, move ou up.
    async fn toque(&self, fx: f32, fy: f32, tipo: &str, tela: &str) {
        let (fx, fy) = (fx.clamp(0.0, 1.0), fy.clamp(0.0, 1.0));
        match self {
            Canal::Emu(l) => {
                let Some((w, h)) = tela
                    .split_once('×')
                    .and_then(|(a, b)| Some((a.parse::<f32>().ok()?, b.parse::<f32>().ok()?)))
                else {
                    return;
                };
                l.touch((fx * w) as i32, (fy * h) as i32, tipo != "up")
                    .await
            }
            Canal::Leve(l) => {
                let acao = match tipo {
                    "down" => scrcpy::TOUCH_DOWN,
                    "up" => scrcpy::TOUCH_UP,
                    _ => scrcpy::TOUCH_MOVE,
                };
                let (x, y) = (fx * l.w as f32, fy * l.h as f32);
                l.enviar(&scrcpy::msg_touch(acao, x as u32, y as u32, l.w, l.h));
            }
        }
    }

    async fn captura(&self) -> Option<Vec<u8>> {
        match self {
            Canal::Emu(l) => l.screenshot_png().await,
            Canal::Leve(l) => {
                let (a, s) = (l.adb.clone(), l.serial.clone());
                tokio::task::spawn_blocking(move || a.screencap(&s).ok())
                    .await
                    .ok()
                    .flatten()
            }
        }
    }
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
    selecionado: bool,
    /// Aparelho do motor leve (Redroid), só no Linux.
    leve: Option<leve::Leve>,
    canal_leve: Option<CanalLeve>,
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
            selecionado: false,
            leve: None,
            canal_leve: None,
        }
    }

    fn de_leve(l: leve::Leve) -> Dev {
        let info = Device {
            name: l.name.clone(),
            kind: l.kind(),
            api: "33".into(),
            screen: format!("{}×{}", l.width, l.height),
            dir: leve::home(&platform::data_dir()).join(&l.name),
            image: l.image.clone(),
        };
        let mut d = Dev::new(info);
        d.leve = Some(l);
        d
    }

    fn rodando(&self) -> bool {
        !matches!(self.estado, Estado::Desligado | Estado::Hibernado)
    }
}

/// Um aparelho a criar quando a imagem dele terminar de baixar.
#[derive(Clone)]
struct Criar {
    leve: bool,
    kind: Kind,
    image: String,
    nome: String,
    grupo: String,
}

/// A tela de Imagens: o que o repositório oferece, o que está instalado e os downloads.
/// Um download em andamento: a fração, a fase ("baixando" ou a extração), o
/// tamanho e quando começou, para a velocidade e o tempo que falta.
struct Progresso {
    fracao: f64,
    fase: String,
    total: u64,
    inicio: Instant,
}

impl Progresso {
    fn baixando(&self) -> bool {
        self.fase == "baixando"
    }

    /// "Downloading 45% · 120 of 270 MB · 4.3 MB/s · about 1 min left"
    fn situacao(&self) -> String {
        if !self.baixando() {
            let fase = if self.fase == "Verifying and extracting" { idioma::t("Verifying and extracting") } else { self.fase.as_str() };
            return format!("{fase}…");
        }
        let pct = idioma::tf("Downloading {pct}%", &[("pct", &format!("{:.0}", self.fracao * 100.0))]);
        if self.total == 0 {
            return pct;
        }
        let feito = (self.fracao * self.total as f64) as u64;
        let mut s = format!(
            "{pct} · {}",
            idioma::tf("{feito} of {total}", &[("feito", &(feito >> 20)), ("total", &mb(self.total))])
        );
        let seg = self.inicio.elapsed().as_secs_f64();
        // A velocidade só diz algo depois de uns segundos de download.
        if seg < 2.0 || feito == 0 {
            return format!("{s} · {}", idioma::t("estimating time…"));
        }
        let vel = feito as f64 / seg;
        let falta = (self.total - feito) as f64 / vel;
        s = format!("{s} · {} MB/s · {}", idioma::num(vel / (1u64 << 20) as f64, 1), tempo_restante(falta));
        s
    }
}

/// "about 40 s left", "about 3 min left", "about 1 h 20 min left"
fn tempo_restante(seg: f64) -> String {
    let seg = seg.ceil() as u64;
    if seg < 60 {
        idioma::tf("about {n} s left", &[("n", &seg.max(1))])
    } else if seg < 3600 {
        idioma::tf("about {n} min left", &[("n", &seg.div_ceil(60))])
    } else {
        idioma::tf("about {h} h {m} min left", &[("h", &(seg / 3600)), ("m", &(seg % 3600 / 60))])
    }
}

#[derive(Default)]
struct Imagens {
    remoto: Vec<RemotePackage>,
    instalado: BTreeMap<String, String>,
    progresso: HashMap<String, Progresso>,
    cancelar: HashMap<String, Arc<AtomicBool>>,
    erro: String,
    carregando: bool,
    lido: bool,
    /// Pacote esperando o aceite da licença, e o aparelho a criar depois.
    licenca: Option<(RemotePackage, Option<Criar>)>,
    /// Aparelhos esperando a imagem baixar.
    criar_depois: Vec<Criar>,
    /// Aparelhos a ligar quando o emulador terminar de baixar.
    ligar_depois: Vec<String>,
}

struct App {
    devs: Vec<Dev>,
    state: State,
    aviso: String,
    memoria: String,
    img: Imagens,
    novo_erro: String,
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

/// Os aparelhos do emulador e, no Linux, os leves.
fn carregar() -> Vec<Dev> {
    let mut v: Vec<Dev> = devices::load(&devices::avd_home())
        .into_iter()
        .map(Dev::new)
        .collect();
    for l in leve::load(&platform::data_dir()) {
        if !v.iter().any(|d| d.info.name == l.name) {
            v.push(Dev::de_leve(l));
        }
    }
    v.sort_by(|a, b| a.info.name.cmp(&b.info.name));
    v
}

fn state_path() -> std::path::PathBuf {
    platform::data_dir().join("state.json")
}

fn gb(bytes: u64) -> String {
    format!("{} GB", idioma::num(bytes as f64 / (1u64 << 30) as f64, 1))
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
        tipo: if d.leve.is_some() {
            idioma::tf("{kind} light", &[("kind", &d.info.kind.display())]).into()
        } else {
            d.info.kind.display().into()
        },
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
        selecionado: d.selecionado,
    }
}

fn host() -> Host {
    Host::current()
}

fn mb(bytes: u64) -> String {
    if bytes >= 1 << 30 {
        gb(bytes)
    } else {
        format!("{} MB", bytes >> 20)
    }
}

/// Em uso: o emulador por qualquer aparelho ligado; uma imagem pelos que a usam.
fn em_uso(app: &App, path: &str) -> bool {
    app.devs
        .iter()
        .any(|d| d.rodando() && (path == catalog::EMULATOR || d.info.image == path))
}

/// As linhas da tela de Imagens: o emulador primeiro, depois as imagens da mais nova à mais antiga.
fn pacotes(app: &App) -> Vec<Pacote> {
    let h = host();
    let mut paths: Vec<String> = app.img.remoto.iter().map(|p| p.path.clone()).collect();
    paths.extend(app.img.instalado.keys().cloned());
    paths.sort();
    paths.dedup();
    paths.sort_by(|a, b| {
        let k = |p: &str| (p != catalog::EMULATOR, -catalog::api_number(p, h));
        k(a).partial_cmp(&k(b))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.cmp(a))
    });
    paths
        .iter()
        .map(|p| {
            let remoto = app.img.remoto.iter().find(|r| &r.path == p);
            let inst = app.img.instalado.get(p);
            let prog = app.img.progresso.get(p);
            let situacao = match (prog, inst, remoto) {
                (Some(pr), _, _) => pr.situacao(),
                (None, Some(v), Some(r)) if &r.revision != v && !r.revision.is_empty() => {
                    idioma::tf("Installed · r{v} · new r{nova} ({mb})", &[("v", v), ("nova", &r.revision), ("mb", &mb(r.size))])
                }
                (None, Some(v), _) => idioma::tf("Installed · r{v}", &[("v", v)]),
                (None, None, Some(r)) => mb(r.size),
                (None, None, None) => String::new(),
            };
            Pacote {
                path: p.as_str().into(),
                titulo: catalog::title(p, h).into(),
                detalhe: catalog::detail(p, h).into(),
                nota: catalog::note(p, h).unwrap_or_default().into(),
                situacao: situacao.into(),
                instalado: inst.is_some(),
                atualizar: matches!((inst, remoto), (Some(v), Some(r)) if &r.revision != v && !r.revision.is_empty()),
                ocupado: prog.is_some(),
                progresso: prog.map_or(-1.0, |pr| if pr.baixando() { pr.fracao as f32 } else { 2.0 }),
                em_uso: em_uso(app, p),
            }
        })
        .collect()
}

fn tipo_de(i: i32) -> Kind {
    Kind::ALL
        .get(i.max(0) as usize)
        .copied()
        .unwrap_or(Kind::Celular)
}

/// As imagens que servem para o tipo: instaladas primeiro, depois as do repositório.
fn opcoes(app: &App, kind: Kind) -> Vec<Opcao> {
    let h = host();
    let wear = kind == Kind::Relogio;
    let mut v: Vec<(bool, String)> = app
        .img
        .instalado
        .keys()
        .filter(|p| *p != catalog::EMULATOR && catalog::is_wear(p, h) == wear)
        .map(|p| (true, p.clone()))
        .collect();
    for r in &app.img.remoto {
        if r.path != catalog::EMULATOR
            && catalog::is_wear(&r.path, h) == wear
            && !app.img.instalado.contains_key(&r.path)
        {
            v.push((false, r.path.clone()));
        }
    }
    v.sort_by(|a, b| {
        b.0.cmp(&a.0).then(
            catalog::api_number(&b.1, h)
                .partial_cmp(&catalog::api_number(&a.1, h))
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });
    v.into_iter()
        .map(|(inst, p)| Opcao {
            path: p.as_str().into(),
            titulo: catalog::title(&p, h).into(),
            detalhe: catalog::detail(&p, h).into(),
            nota: catalog::note(&p, h).unwrap_or_default().into(),
            instalada: inst,
        })
        .collect()
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
    ui.set_teto(app.state.ceiling_gb as f32);
    ui.set_linhas(
        app.state
            .spots
            .values()
            .map(|s| s.y + s.h)
            .max()
            .unwrap_or(0),
    );
    ui.set_selecionados(app.devs.iter().filter(|d| d.selecionado).count() as i32);
    ui.set_sem_emulador(!app.img.instalado.contains_key(catalog::EMULATOR));
    ui.set_pacotes(ModelRc::new(VecModel::from(pacotes(app))));
    ui.set_imagens_erro(app.img.erro.as_str().into());
    ui.set_imagens_carregando(app.img.carregando);
    let kind = tipo_de(ui.get_novo_tipo());
    let ops = opcoes(app, kind);
    if ui.get_novo_imagem().is_empty() || !ops.iter().any(|o| o.path == ui.get_novo_imagem()) {
        let pref = catalog::default_image(kind == Kind::Relogio, host());
        let escolha = ops
            .iter()
            .find(|o| o.path == pref.as_str() && o.instalada)
            .or(ops.iter().find(|o| o.instalada))
            .or(ops.iter().find(|o| o.path == pref.as_str()))
            .or(ops.first())
            .map(|o| o.path.clone())
            .unwrap_or_default();
        ui.set_novo_imagem(escolha);
    }
    let img = ui.get_novo_imagem().to_string();
    let mut nota = catalog::note(&img, host()).unwrap_or_default();
    if let Some(r) = app.img.remoto.iter().find(|r| r.path == img)
        && !app.img.instalado.contains_key(&img)
    {
        nota = format!("{nota}\n{}", idioma::tf("Downloads {mb} before creating.", &[("mb", &mb(r.size))]))
            .trim()
            .to_string();
    }
    let custo = idioma::num(kind.estimate_gb(), 1);
    nota = format!("{nota}\n{}", idioma::tf("Uses about {custo} GB of memory while on.", &[("custo", &custo)]))
        .trim()
        .to_string();
    ui.set_novo_nota(nota.as_str().into());
    ui.set_opcoes(ModelRc::new(VecModel::from(ops)));
    ui.set_novo_erro(app.novo_erro.as_str().into());
    let prog = app.img.progresso.get(&img);
    ui.set_novo_ocupado(prog.is_some());
    ui.set_novo_progresso(prog.map_or(-1.0, |pr| if pr.baixando() { pr.fracao as f32 } else { 2.0 }));
    ui.set_novo_situacao(prog.map(Progresso::situacao).unwrap_or_default().into());
    if let Some((p, _)) = &app.img.licenca {
        ui.set_licenca_titulo(catalog::title(&p.path, host()).into());
        ui.set_licenca_texto(p.license_text.as_str().into());
    }
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

    /// Memória que os ligados vão usar com `nome` também ligado, em GB.
    fn projetado(app: &App, nome: &str) -> f64 {
        let gib = (1u64 << 30) as f64;
        app.devs
            .iter()
            .filter(|d| d.rodando() || d.info.name == nome)
            .map(|d| (d.memoria as f64 / gib).max(d.info.kind.estimate_gb()))
            .sum()
    }

    /// Liga (ou acorda) um aparelho. Se não couber no teto de memória, os aparelhos
    /// sem uso há mais tempo hibernam antes, até caber.
    fn ligar(&self, nome: &str) {
        let (cabe, teto) = {
            let app = self.app.lock().unwrap();
            (
                Self::projetado(&app, nome) <= app.state.ceiling_gb,
                app.state.ceiling_gb,
            )
        };
        if cabe {
            return self.abrir_emulador(nome);
        }
        self.com(nome, |d, _| d.mensagem = idioma::t("making room…").into());
        self.avisar();
        let c = self.clone();
        let nome = nome.to_string();
        self.rt.spawn(async move {
            loop {
                let vitima = {
                    let app = c.app.lock().unwrap();
                    if Self::projetado(&app, &nome) <= app.state.ceiling_gb {
                        None
                    } else {
                        Some(
                            app.devs
                                .iter()
                                .filter(|d| {
                                    d.info.name != nome
                                        && matches!(d.estado, Estado::Ligado | Estado::Pausado)
                                })
                                .min_by_key(|d| d.ultimo_uso)
                                .map(|d| d.info.name.clone()),
                        )
                    }
                };
                match vitima {
                    None => break,
                    Some(None) => {
                        c.com(&nome, |d, _| d.mensagem.clear());
                        c.aviso(idioma::tf(
                            "{nome} can't start: it doesn't fit under the {teto} GB ceiling even after hibernating the others",
                            &[("nome", &nome), ("teto", &format!("{teto:.0}"))],
                        ));
                        return;
                    }
                    Some(Some(v)) => {
                        c.aviso(idioma::tf("{v} hibernated so {nome} could start", &[("v", &v), ("nome", &nome)]));
                        c.desligar_e_esperar(&v, true).await;
                    }
                }
            }
            c.com(&nome, |d, _| d.mensagem.clear());
            let c2 = c.clone();
            let _ = slint::invoke_from_event_loop(move || c2.abrir_emulador(&nome));
        });
    }

    fn abrir_emulador(&self, nome: &str) {
        if self.com(nome, |d, _| d.leve.is_some()) == Some(true) {
            return self.ligar_leve(nome);
        }
        let sdk = devices::sdk_root();
        if catalog::installed(catalog::EMULATOR, &sdk).is_none() {
            return self.baixar_emulador_e_ligar(nome);
        }
        let pronto = self.com(nome, |d, app| {
            if d.rodando() {
                return Err(String::new());
            }
            if catalog::installed(&d.info.image, &sdk).is_none() {
                return Err(idioma::tf(
                    "{nome} uses {imagem}, which isn't installed",
                    &[("nome", &d.info.name), ("imagem", &catalog::title(&d.info.image, catalog::Host::current()))],
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
                .map_err(|e| idioma::tf("the emulator didn't open: {e}", &[("e", &e)]))?;
            let r = (l.pid(), l.grpc);
            d.pid = Some(l.pid());
            d.console = Some(l.console);
            d.child = Some(l);
            d.estado = Estado::Ligando;
            d.mensagem = idioma::t("starting…").into();
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
                // No Windows o processo que conta (métricas, adotar, desligar) é o
                // qemu filho do emulator.exe.
                let qemu = emulator::engine_pid(pid);
                self.com(nome, |d, _| d.pid = Some(qemu));
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
            d.mensagem = idioma::t("the emulator closed while starting").into();
        });
        self.avisar();
    }

    async fn conectar(&self, nome: &str, grpc: u16, token: &str) {
        let link = match Link::connect(grpc, token).await {
            Ok(l) => l,
            Err(e) => {
                self.com(nome, |d, _| d.mensagem = idioma::tf("no gRPC connection: {e}", &[("e", &e)]));
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
                c.entregar(&nome, &pendente, f);
                true
            })
            .await;
        });
    }

    /// Guarda o quadro e pede à interface para mostrar (sem empilhar quadros).
    fn entregar(&self, nome: &str, pendente: &Arc<AtomicBool>, f: screen::Frame) {
        let buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&f.rgba, f.width, f.height);
        self.com(nome, |d, _| d.quadro = Some(buf));
        if pendente.swap(true, Ordering::AcqRel) {
            return;
        }
        let (app, ui, nome, pendente) = (
            self.app.clone(),
            self.ui.clone(),
            nome.to_string(),
            pendente.clone(),
        );
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

    // MARK: motor leve (Redroid)

    /// Liga um aparelho leve: contêiner, boot pelo adb, sessão do scrcpy e quadros.
    fn ligar_leve(&self, nome: &str) {
        let Some(rt) = leve::runtime() else {
            return self.aviso(idioma::t("The light engine needs Docker installed"));
        };
        if !leve::binder_ok() {
            return self.aviso(idioma::t(
                "The light engine needs binder in the kernel: sudo modprobe binder_linux devices=binder,hwbinder,vndbinder",
            ));
        }
        let Some(adb) = Adb::from_sdk(&devices::sdk_root()) else {
            return self
                .aviso(idioma::t("Download the platform tools (adb) in Images before starting"));
        };
        let Some(Some(l)) = self.com(nome, |d, _| {
            if d.rodando() && d.estado != Estado::Externo {
                return None;
            }
            d.estado = Estado::Ligando;
            d.mensagem = idioma::t("starting…").into();
            d.ultimo_uso = Instant::now();
            d.leve.clone()
        }) else {
            return;
        };
        self.avisar();
        let c = self.clone();
        let nome = nome.to_string();
        let pendente = self
            .com(&nome, |d, _| d.quadro_pendente.clone())
            .unwrap_or_default();
        let parar = Arc::new(AtomicBool::new(false));
        self.com(&nome, |d, _| d.parar_tela = Some(parar.clone()));
        std::thread::spawn(move || {
            let data = platform::data_dir();
            let r = (|| -> Result<scrcpy::Session, String> {
                leve::start(rt, &data, &l)?;
                adb.wait_boot(&l.serial(), Duration::from_secs(240))?;
                let jar = scrcpy::ensure_server(&data.join("scrcpy"))?;
                let max = l.width.max(l.height).min(1600);
                // Uma porta local por aparelho leve, para as sessões não se cruzarem.
                let porta = 27183 + (l.port - leve::PORTA0);
                scrcpy::Session::start(&adb, &l.serial(), &jar, porta, max)
            })();
            let mut s = match r {
                Ok(s) => s,
                Err(e) => {
                    c.com(&nome, |d, _| {
                        d.estado = Estado::Desligado;
                        d.mensagem = String::new();
                    });
                    return c.aviso(idioma::tf("{nome} didn't start: {e}", &[("nome", &nome), ("e", &e)]));
                }
            };
            let canal = CanalLeve {
                ctl: Arc::new(Mutex::new(s.controller().expect("controle"))),
                w: s.width as u16,
                h: s.height as u16,
                adb: adb.clone(),
                serial: l.serial(),
            };
            c.com(&nome, |d, _| {
                d.canal_leve = Some(canal);
                d.estado = Estado::Ligado;
                d.mensagem.clear();
            });
            c.avisar();
            let Ok(mut dec) = scrcpy::Decoder::new() else {
                return c.aviso(idioma::t("the video decoder didn't open"));
            };
            while !parar.load(Ordering::Relaxed) {
                let Ok(p) = s.read_packet() else { break };
                if let Ok(Some(f)) = dec.decode(&p.data) {
                    c.entregar(&nome, &pendente, f);
                }
            }
        });
    }

    fn pausar_leve(&self, nome: &str, pausar: bool) {
        let ok = self.com(nome, |d, _| {
            let ok = if pausar {
                d.estado == Estado::Ligado
            } else {
                d.estado == Estado::Pausado
            };
            if ok {
                d.estado = if pausar {
                    Estado::Pausado
                } else {
                    Estado::Ligado
                };
                d.ultimo_uso = Instant::now();
            }
            ok
        });
        if ok != Some(true) {
            return;
        }
        self.avisar();
        let (c, nome) = (self.clone(), nome.to_string());
        self.rt.spawn_blocking(move || {
            if let Some(rt) = leve::runtime()
                && let Err(e) = leve::pause(rt, &nome, pausar)
            {
                c.aviso(format!("{nome}: {e}"));
            }
        });
    }

    /// Desliga o contêiner (o /data fica guardado). Hibernar guarda a última tela; ao
    /// acordar, o Android liga de novo com os apps e as contas como estavam no disco.
    async fn terminar_leve(&self, nome: &str, hibernar: bool) {
        let n = nome.to_string();
        let r = tokio::task::spawn_blocking(move || match leve::runtime() {
            Some(rt) => leve::stop(rt, &n),
            None => Err(idioma::t("no Podman or Docker").into()),
        })
        .await
        .unwrap_or_else(|e| Err(e.to_string()));
        self.com(nome, |d, _| {
            if let Some(p) = d.parar_tela.take() {
                p.store(true, Ordering::Relaxed);
            }
            d.canal_leve = None;
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
        if let Err(e) = r {
            self.aviso(format!("{nome}: {e}"));
        }
        self.avisar();
    }

    // MARK: pausar, hibernar, desligar

    fn pausar(&self, nome: &str, pausar: bool) {
        if self.com(nome, |d, _| d.leve.is_some()) == Some(true) {
            return self.pausar_leve(nome, pausar);
        }
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
        if self.com(nome, |d, _| {
            d.leve.is_some()
                && d.rodando()
                && d.estado != Estado::Desligando
                && (!hibernar || matches!(d.estado, Estado::Ligado | Estado::Pausado))
        }) == Some(true)
        {
            self.com(nome, |d, _| d.estado = Estado::Desligando);
            self.avisar();
            let (c, nome) = (self.clone(), nome.to_string());
            self.rt
                .spawn(async move { c.terminar_leve(&nome, hibernar).await });
            return;
        }
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
        self.rt
            .spawn(async move { c.terminar(&nome, pid, console, hibernar).await });
    }

    /// Hibernar ou desligar e só voltar quando o processo saiu (para abrir espaço).
    async fn desligar_e_esperar(&self, nome: &str, hibernar: bool) {
        if self.com(nome, |d, _| d.leve.is_some()) == Some(true) {
            self.com(nome, |d, _| d.estado = Estado::Desligando);
            self.avisar();
            return self.terminar_leve(nome, hibernar).await;
        }
        let alvo = self.com(nome, |d, _| {
            if !matches!(d.estado, Estado::Ligado | Estado::Pausado) {
                return None;
            }
            d.estado = Estado::Desligando;
            if let Some(p) = d.parar_tela.take() {
                p.store(true, Ordering::Relaxed);
            }
            Some((d.pid?, d.console?))
        });
        self.avisar();
        if let Some(Some((pid, console))) = alvo {
            self.terminar(nome, pid, console, hibernar).await;
        }
    }

    async fn terminar(&self, nome: &str, pid: u32, console: u16, hibernar: bool) {
        tokio::task::spawn_blocking(move || emulator::shutdown(pid, console))
            .await
            .ok();
        self.com(nome, |d, _| {
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
        self.avisar();
    }

    // MARK: botões, toque e teclado

    fn com_canal(&self, nome: &str) -> Option<Canal> {
        self.com(nome, |d, _| {
            d.ultimo_uso = Instant::now();
            if d.estado != Estado::Ligado {
                return None;
            }
            match (&d.canal_leve, &d.link) {
                (Some(l), _) => Some(Canal::Leve(l.clone())),
                (None, Some(l)) => Some(Canal::Emu(l.clone())),
                _ => None,
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
            "selecionar" => {
                self.com(nome, |d, _| d.selecionado = !d.selecionado);
                return self.avisar();
            }
            "duplicar" => return self.duplicar(nome),
            _ => {}
        }
        let Some(canal) = self.com_canal(nome) else {
            return;
        };
        let c = self.clone();
        let (nome, acao) = (nome.to_string(), acao.to_string());
        self.rt.spawn(async move {
            match acao.as_str() {
                "voltar" => canal.tecla("GoBack").await,
                "inicio" => canal.tecla("GoHome").await,
                "recentes" => canal.tecla("AppSwitch").await,
                "volume+" => canal.tecla("AudioVolumeUp").await,
                "volume-" => canal.tecla("AudioVolumeDown").await,
                "girar" => {
                    let paisagem = c
                        .com(&nome, |d, _| {
                            d.paisagem = !d.paisagem;
                            d.paisagem
                        })
                        .unwrap_or(false);
                    match &canal {
                        Canal::Emu(l) => l.rotate(if paisagem { -90.0 } else { 0.0 }).await,
                        Canal::Leve(l) => l.enviar(&scrcpy::msg_rotate()),
                    }
                }
                "captura" => {
                    if let Some(png) = canal.captura().await {
                        let dir = platform::pictures_dir();
                        let stamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let ok = std::fs::create_dir_all(&dir).is_ok()
                            && std::fs::write(dir.join(format!("{nome}-{stamp}.png")), png).is_ok();
                        c.aviso(if ok {
                            idioma::tf("Screenshot saved to {pasta}", &[("pasta", &dir.display())])
                        } else {
                            idioma::t("The screenshot wasn't saved").into()
                        });
                    }
                }
                _ => {}
            }
        });
    }

    fn toque(&self, nome: &str, fx: f32, fy: f32, tipo: &str) {
        let Some(canal) = self.com_canal(nome) else {
            return;
        };
        let tela = self
            .com(nome, |d, _| d.info.screen.clone())
            .unwrap_or_default();
        let tipo = tipo.to_string();
        self.rt
            .spawn(async move { canal.toque(fx, fy, &tipo, &tela).await });
    }

    fn tecla(&self, nome: &str, texto: &str) {
        let Some(canal) = self.com_canal(nome) else {
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
                Some(k) => canal.tecla(k).await,
                None => canal.texto(&texto).await,
            }
        });
    }

    // MARK: lote, grade, teto

    fn lote(&self, acao: &str) {
        let nomes: Vec<String> = {
            let mut app = self.app.lock().unwrap();
            if acao == "limpar" {
                app.devs.iter_mut().for_each(|d| d.selecionado = false);
                drop(app);
                return self.avisar();
            }
            app.devs
                .iter()
                .filter(|d| d.selecionado)
                .map(|d| d.info.name.clone())
                .collect()
        };
        for n in nomes {
            self.acao(&n, acao);
        }
    }

    fn salvar(app: &App) {
        if let Err(e) = app.state.save(&state_path()) {
            eprintln!("não salvou o state.json: {e}");
        }
    }

    /// Arrasta (`crescer = false`) ou redimensiona um quadro, em células, e empurra para
    /// baixo quem ficar embaixo.
    fn mover(&self, nome: &str, dx: i32, dy: i32) {
        if dx == 0 && dy == 0 {
            return self.avisar();
        }
        let Some(mut s) = self.app.lock().unwrap().state.spots.get(nome).copied() else {
            return;
        };
        s.x = (s.x + dx).clamp(0, grid::COLS - s.w);
        s.y = (s.y + dy).max(0);
        self.colocar(nome, s);
    }

    /// Lugar novo depois de redimensionar por uma borda ou canto.
    fn posicionar(&self, nome: &str, x: i32, y: i32, w: i32, h: i32) {
        let w = w.clamp(grid::MIN_W, grid::COLS);
        let s = multidroid_core::state::Spot {
            x: x.clamp(0, grid::COLS - w),
            y: y.max(0),
            w,
            h: h.max(grid::MIN_H),
        };
        self.colocar(nome, s);
    }

    /// A altura que mostra o aparelho inteiro mudou (largura ou janela nova):
    /// guarda e empurra para baixo quem passou a colidir.
    fn altura(&self, nome: &str, h: i32) {
        let Some(mut s) = self.app.lock().unwrap().state.spots.get(nome).copied() else {
            return;
        };
        let h = h.max(grid::MIN_H);
        if s.h == h {
            return;
        }
        s.h = h;
        self.colocar(nome, s);
    }

    /// Põe `nome` em `s` na sua mesa, empurrando quem colidir, e salva.
    fn colocar(&self, nome: &str, s: multidroid_core::state::Spot) {
        let mut app = self.app.lock().unwrap();
        if app.state.spots.get(nome) == Some(&s) {
            drop(app);
            return self.avisar();
        }
        let mesa = app.state.desk_of(nome).to_string();
        let mesmos: BTreeMap<String, _> = app
            .state
            .spots
            .iter()
            .filter(|(n, _)| app.state.desk_of(n) == mesa)
            .map(|(n, v)| (n.clone(), *v))
            .collect();
        let novo = grid::place(nome, s, &mesmos);
        app.state.spots.extend(novo);
        Self::salvar(&app);
        drop(app);
        self.avisar();
    }

    /// Organizar: encaixa os aparelhos da mesa principal em `linhas` linhas visíveis.
    fn organizar(&self, linhas: i32) {
        let mut app = self.app.lock().unwrap();
        let infos: Vec<Device> = app.devs.iter().map(|d| d.info.clone()).collect();
        let mut st = app.state.clone();
        let mesa: Vec<&Device> = st
            .ordered(&infos)
            .into_iter()
            .filter(|d| app.state.desk_of(&d.name) == MAIN_DESK)
            .collect();
        st.fit_spots(&mesa, linhas);
        app.state = st;
        Self::salvar(&app);
        drop(app);
        self.avisar();
    }

    fn teto(&self, passo: i32) {
        let mut app = self.app.lock().unwrap();
        let max = platform::physical_gb().floor();
        app.state.ceiling_gb = (app.state.ceiling_gb + passo as f64).clamp(4.0, max);
        Self::salvar(&app);
        drop(app);
        self.avisar();
    }

    // MARK: criar, duplicar, apagar, grupo

    /// Relê a pasta de aparelhos sem perder o que está rodando.
    fn recarregar(app: &mut App) {
        let mut velhos: Vec<Dev> = std::mem::take(&mut app.devs);
        app.devs = carregar()
            .into_iter()
            .map(
                |n| match velhos.iter().position(|d| d.info.name == n.info.name) {
                    Some(k) => {
                        let mut d = velhos.remove(k);
                        d.info = n.info;
                        d.leve = n.leve;
                        d
                    }
                    None => n,
                },
            )
            .collect();
        let infos: Vec<Device> = app.devs.iter().map(|d| d.info.clone()).collect();
        let nomes: Vec<String> = infos.iter().map(|d| d.name.clone()).collect();
        app.state.order.retain(|n| nomes.contains(n));
        for n in &nomes {
            if !app.state.order.contains(n) {
                app.state.order.push(n.clone());
            }
        }
        let mut st = app.state.clone();
        let mesa: Vec<&Device> = st
            .ordered(&infos)
            .into_iter()
            .filter(|d| app.state.desk_of(&d.name) == MAIN_DESK)
            .collect();
        st.ensure_spots(&mesa);
        app.state = st;
        Self::salvar(app);
    }

    fn duplicar(&self, nome: &str) {
        let mut app = self.app.lock().unwrap();
        let Some(d) = app.devs.iter().find(|d| d.info.name == nome) else {
            return;
        };
        if d.rodando() {
            app.aviso = idioma::tf("Shut down {nome} before duplicating", &[("nome", &nome)]);
            drop(app);
            return self.avisar();
        }
        if d.leve.is_some() {
            app.aviso = idioma::t("Duplicating doesn't work for light devices yet").into();
            drop(app);
            return self.avisar();
        }
        let info = d.info.clone();
        let mut n = 2;
        while app
            .devs
            .iter()
            .any(|d| d.info.name == format!("{nome}-{n}"))
        {
            n += 1;
        }
        let novo = format!("{nome}-{n}");
        match devices::duplicate(&devices::avd_home(), &info, &novo) {
            Ok(()) => {
                app.state.copy_from(nome, &novo);
                Self::recarregar(&mut app);
                app.aviso = idioma::tf("{novo} created from {nome}", &[("novo", &novo), ("nome", &nome)]);
            }
            Err(e) => app.aviso = idioma::tf("Couldn't duplicate: {e}", &[("e", &e)]),
        }
        drop(app);
        self.avisar();
    }

    fn apagar(&self, nome: &str) {
        let mut app = self.app.lock().unwrap();
        let Some(d) = app.devs.iter().find(|d| d.info.name == nome) else {
            return;
        };
        if d.rodando() {
            app.aviso = idioma::tf("Shut down {nome} before deleting", &[("nome", &nome)]);
        } else {
            let info = d.info.clone();
            let r = if d.leve.is_some() {
                leve::delete(leve::runtime(), &platform::data_dir(), nome)
            } else {
                devices::delete(&devices::avd_home(), &info)
            };
            match r {
                Ok(()) => {
                    app.state.forget(nome);
                    Self::recarregar(&mut app);
                    app.aviso = idioma::tf("{nome} was deleted", &[("nome", &nome)]);
                }
                Err(e) => app.aviso = idioma::tf("Couldn't delete: {e}", &[("e", &e)]),
            }
        }
        drop(app);
        self.avisar();
    }

    fn grupo(&self, nome: &str, grupo: &str) {
        let mut app = self.app.lock().unwrap();
        let g = grupo.trim();
        if g.is_empty() {
            app.state.groups.remove(nome);
        } else {
            app.state.groups.insert(nome.into(), g.into());
        }
        Self::salvar(&app);
        drop(app);
        self.avisar();
    }

    fn criar(&self, c: Criar) {
        let sdk = devices::sdk_root();
        let mut app = self.app.lock().unwrap();
        app.novo_erro.clear();
        let nome = c.nome.trim().to_string();
        if !devices::valid_name(&nome) {
            app.novo_erro =
                idioma::t("Use a name with letters, numbers, dot, hyphen or underscore (no spaces).").into();
        } else if app.devs.iter().any(|d| d.info.name == nome) {
            app.novo_erro = idioma::t("A device with that name already exists.").into();
        } else if c.leve {
            match leve::create(&platform::data_dir(), &nome, c.kind) {
                Ok(_) => {
                    if !c.grupo.trim().is_empty() {
                        app.state.groups.insert(nome.clone(), c.grupo.trim().into());
                    }
                    Self::recarregar(&mut app);
                    app.aviso = idioma::tf("{nome} created on the light engine", &[("nome", &nome)]);
                    drop(app);
                    self.mostrar("");
                    return self.avisar();
                }
                Err(e) => app.novo_erro = e,
            }
        } else if c.image.is_empty() {
            app.novo_erro = idioma::t("Choose an image.").into();
        } else if catalog::installed(&c.image, &sdk).is_none() {
            // Baixa a imagem antes (com a licença) e cria quando terminar.
            drop(app);
            let image = c.image.clone();
            return self.baixar(&image, Some(Criar { nome, ..c }));
        } else {
            match devices::create(&devices::avd_home(), &sdk, &nome, c.kind, &c.image, host()) {
                Ok(()) => {
                    if !c.grupo.trim().is_empty() {
                        app.state.groups.insert(nome.clone(), c.grupo.trim().into());
                    }
                    Self::recarregar(&mut app);
                    app.aviso = idioma::tf("{nome} created", &[("nome", &nome)]);
                    drop(app);
                    self.mostrar("");
                    return self.avisar();
                }
                Err(e) => app.novo_erro = e,
            }
        }
        drop(app);
        self.avisar();
    }

    /// Troca a tela sobreposta ("" fecha).
    fn mostrar(&self, tela: &'static str) {
        let ui = self.ui.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui.upgrade() {
                ui.set_tela(tela.into());
            }
        });
    }

    // MARK: imagens

    fn ler_instalados(app: &mut App) {
        let sdk = devices::sdk_root();
        app.img.instalado = catalog::scan_installed(&sdk, host());
        if let Some(v) = catalog::installed(catalog::EMULATOR, &sdk) {
            app.img.instalado.insert(catalog::EMULATOR.into(), v);
        }
    }

    /// Lê o repositório (uma vez por abertura do app, ou de novo se falhou).
    fn ler_repositorio(&self) {
        {
            let mut app = self.app.lock().unwrap();
            Self::ler_instalados(&mut app);
            if app.img.carregando || (app.img.lido && app.img.erro.is_empty()) {
                drop(app);
                return self.avisar();
            }
            app.img.carregando = true;
        }
        self.avisar();
        let c = self.clone();
        self.rt.spawn(async move {
            let (remoto, falhas) = tokio::task::spawn_blocking(|| install::fetch_remote(host()))
                .await
                .unwrap_or_default();
            let mut app = c.app.lock().unwrap();
            app.img.remoto = remoto;
            app.img.erro = if falhas.is_empty() {
                String::new()
            } else {
                idioma::tf("Couldn't read the repository: {falhas}", &[("falhas", &falhas.join("; "))])
            };
            app.img.carregando = false;
            app.img.lido = true;
            drop(app);
            c.avisar();
        });
    }

    fn pacote(&self, path: &str, acao: &str) {
        match acao {
            "baixar" => self.baixar(path, None),
            "cancelar" => {
                if let Some(f) = self.app.lock().unwrap().img.cancelar.get(path) {
                    f.store(true, Ordering::Relaxed);
                }
            }
            "remover" => {
                let mut app = self.app.lock().unwrap();
                if em_uso(&app, path) {
                    app.aviso = idioma::t("Shut down the devices that use this image first").into();
                } else if let Err(e) = install::remove(path, &devices::sdk_root()) {
                    app.aviso = idioma::tf("Couldn't remove: {e}", &[("e", &e)]);
                }
                Self::ler_instalados(&mut app);
                drop(app);
                self.avisar();
            }
            _ => {}
        }
    }

    /// Pede o aceite da licença, se faltar, e baixa.
    fn baixar(&self, path: &str, depois: Option<Criar>) {
        let sdk = devices::sdk_root();
        let mut app = self.app.lock().unwrap();
        let Some(pkg) = app.img.remoto.iter().find(|r| r.path == path).cloned() else {
            app.novo_erro =
                idioma::t("This image isn't in the repository right now; try again in Images.").into();
            drop(app);
            return self.avisar();
        };
        if em_uso(&app, path) {
            app.aviso = idioma::t("Shut down the devices that use this image before updating").into();
            drop(app);
            return self.avisar();
        }
        if !install::accepted(&pkg, &sdk) {
            app.img.licenca = Some((pkg, depois));
            drop(app);
            self.avisar();
            return self.mostrar("licenca");
        }
        drop(app);
        self.instalar(pkg, depois);
    }

    /// Sem o emulador, ligar baixa ele primeiro (lendo o repositório se
    /// preciso) e liga o aparelho quando terminar, em vez de só avisar.
    fn baixar_emulador_e_ligar(&self, nome: &str) {
        let tem = {
            let mut app = self.app.lock().unwrap();
            if !app.img.ligar_depois.iter().any(|n| n == nome) {
                app.img.ligar_depois.push(nome.into());
            }
            if let Some(d) = app.devs.iter_mut().find(|d| d.info.name == nome) {
                d.mensagem = idioma::t("downloading the emulator…").into();
            }
            app.img.remoto.iter().any(|r| r.path == catalog::EMULATOR)
        };
        self.avisar();
        if tem {
            return self.baixar(catalog::EMULATOR, None);
        }
        let c = self.clone();
        self.rt.spawn(async move {
            let (remoto, falhas) = tokio::task::spawn_blocking(|| install::fetch_remote(host()))
                .await
                .unwrap_or_default();
            {
                let mut app = c.app.lock().unwrap();
                if !remoto.is_empty() {
                    app.img.remoto = remoto;
                    app.img.lido = falhas.is_empty();
                }
            }
            let _ = slint::invoke_from_event_loop(move || {
                let tem = c.app.lock().unwrap().img.remoto.iter().any(|r| r.path == catalog::EMULATOR);
                if tem {
                    c.baixar(catalog::EMULATOR, None);
                } else {
                    c.desistir_de_ligar();
                    c.aviso(idioma::tf(
                        "Couldn't read the repository: {falhas}",
                        &[("falhas", &falhas.join("; "))],
                    ));
                }
            });
        });
    }

    /// Limpa os aparelhos que esperavam o emulador (download cancelado ou recusado).
    fn desistir_de_ligar(&self) {
        let mut app = self.app.lock().unwrap();
        let nomes = std::mem::take(&mut app.img.ligar_depois);
        for d in app.devs.iter_mut().filter(|d| nomes.contains(&d.info.name)) {
            d.mensagem.clear();
        }
        drop(app);
        self.avisar();
    }

    fn licenca(&self, aceitou: bool) {
        let pend = self.app.lock().unwrap().img.licenca.take();
        let Some((pkg, depois)) = pend else {
            return self.mostrar("");
        };
        let volta = if depois.is_some() { "novo" } else { "imagens" };
        if !aceitou {
            if pkg.path == catalog::EMULATOR {
                self.desistir_de_ligar();
            }
            return self.mostrar(volta);
        }
        if let Err(e) = install::accept(&pkg, &devices::sdk_root()) {
            self.aviso(idioma::tf("Couldn't record the acceptance: {e}", &[("e", &e)]));
            return self.mostrar("");
        }
        self.mostrar(volta);
        self.instalar(pkg, depois);
    }

    fn instalar(&self, pkg: RemotePackage, depois: Option<Criar>) {
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut app = self.app.lock().unwrap();
            if let Some(d) = depois {
                app.img.criar_depois.push(d);
            }
            if app.img.progresso.contains_key(&pkg.path) {
                return;
            }
            app.img.progresso.insert(
                pkg.path.clone(),
                Progresso { fracao: 0.0, fase: "baixando".into(), total: pkg.size, inicio: Instant::now() },
            );
            app.img.cancelar.insert(pkg.path.clone(), cancel.clone());
        }
        self.avisar();
        let c = self.clone();
        self.rt.spawn(async move {
            let path = pkg.path.clone();
            let c2 = c.clone();
            let r = tokio::task::spawn_blocking(move || {
                let sdk = devices::sdk_root();
                let mut ultimo = -1.0;
                let mut ultima_vez = Instant::now();
                let p = pkg.path.clone();
                let work = platform::data_dir().join("downloads");
                install::install(&pkg, &sdk, &work, &cancel, |f| {
                    // A cada 1% ou a cada segundo, para o tempo que falta andar
                    // mesmo numa conexão lenta.
                    if f - ultimo >= 0.01 || f >= 1.0 || ultima_vez.elapsed() >= Duration::from_secs(1) {
                        ultimo = f;
                        ultima_vez = Instant::now();
                        let fase = if f >= 1.0 {
                            "Verifying and extracting"
                        } else {
                            "baixando"
                        };
                        let mut g = c2.app.lock().unwrap();
                        let app: &mut App = &mut g;
                        if let Some(pr) = app.img.progresso.get_mut(&p) {
                            pr.fracao = f;
                            pr.fase = fase.into();
                            if p == catalog::EMULATOR {
                                let s = idioma::tf("Emulator: {s}", &[("s", &pr.situacao())]);
                                for d in app.devs.iter_mut().filter(|d| app.img.ligar_depois.contains(&d.info.name)) {
                                    d.mensagem = s.clone();
                                }
                            }
                        }
                        drop(g);
                        c2.avisar();
                    }
                })
            })
            .await;
            let erro = match r {
                Ok(Ok(_)) => None,
                Ok(Err(install::Error::Cancelled)) => Some(String::new()),
                Ok(Err(e)) => Some(e.to_string()),
                Err(e) => Some(e.to_string()),
            };
            let mut ligar = vec![];
            let criar: Vec<Criar> = {
                let mut app = c.app.lock().unwrap();
                app.img.progresso.remove(&path);
                app.img.cancelar.remove(&path);
                Self::ler_instalados(&mut app);
                let (prontos, resto): (Vec<Criar>, Vec<Criar>) =
                    std::mem::take(&mut app.img.criar_depois)
                        .into_iter()
                        .partition(|d| d.image == path);
                app.img.criar_depois = resto;
                if path == catalog::EMULATOR {
                    let nomes = std::mem::take(&mut app.img.ligar_depois);
                    for d in app.devs.iter_mut().filter(|d| nomes.contains(&d.info.name)) {
                        d.mensagem.clear();
                    }
                    if erro.is_none() {
                        ligar = nomes;
                    }
                }
                match &erro {
                    Some(e) if !e.is_empty() => {
                        app.aviso = format!("{}: {e}", catalog::title(&path, host()));
                        app.novo_erro = app.aviso.clone();
                        vec![]
                    }
                    Some(_) => vec![],
                    None => prontos,
                }
            };
            c.avisar();
            for d in criar {
                let c3 = c.clone();
                let _ = slint::invoke_from_event_loop(move || c3.criar(d));
            }
            for nome in ligar {
                let c3 = c.clone();
                let _ = slint::invoke_from_event_loop(move || c3.ligar(&nome));
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
        // Motor leve: memória de cada contêiner e quem já estava ligado ao abrir o app.
        if let Some(rt) = leve::runtime().filter(|_| cfg!(target_os = "linux")) {
            let mem = leve::memory(rt);
            let ligados = leve::running(rt);
            let mut adotar = vec![];
            {
                let mut app = self.app.lock().unwrap();
                for d in app.devs.iter_mut().filter(|d| d.leve.is_some()) {
                    d.memoria = mem
                        .iter()
                        .find(|(n, _)| n == &d.info.name)
                        .map_or(0, |(_, b)| *b);
                    let aceso = ligados.iter().any(|(n, _)| n == &d.info.name);
                    if aceso && d.estado == Estado::Desligado && d.canal_leve.is_none() {
                        d.estado = Estado::Externo;
                        adotar.push(d.info.name.clone());
                    }
                }
            }
            for n in adotar {
                self.ligar_leve(&n);
            }
        }
        let rodando = procs.emulators();
        self.adotar(&rodando);
        let (usada, swap) = platform::memory_use_gb();
        let total = platform::physical_gb();
        let mut app = self.app.lock().unwrap();
        let mut soma: u64 = app
            .devs
            .iter()
            .filter(|d| d.leve.is_some())
            .map(|d| d.memoria)
            .sum();
        for d in app.devs.iter_mut().filter(|d| d.leve.is_none()) {
            match d.pid.and_then(|p| rodando.iter().find(|r| r.pid == p)) {
                Some(r) => {
                    d.memoria = r.memory;
                    d.cpu = r.cpu;
                    soma += r.memory;
                }
                None if d.pid.is_some() && !matches!(d.estado, Estado::Desligando | Estado::Ligando) => {
                    // Fechou por fora, ou o emulador que este app abriu morreu
                    // (no Windows d.pid é o qemu e child o emulator.exe).
                    let vivo = d.child.as_mut().is_some_and(|c| c.alive())
                        && d.pid.is_some_and(emulator::alive);
                    if !vivo {
                        d.pid = None;
                        d.console = None;
                        d.link = None;
                        d.child = None;
                        d.estado = Estado::Desligado;
                    }
                }
                None => {}
            }
        }
        app.memoria = idioma::tf(
            "Computer memory: {usada} of {total} GB\nDevices {aparelhos} · swap {swap} GB · ceiling {teto} GB",
            &[
                ("usada", &idioma::num(usada, 1)),
                ("total", &idioma::num(total, 0)),
                ("aparelhos", &gb(soma)),
                ("swap", &idioma::num(swap, 1)),
                ("teto", &idioma::num(app.state.ceiling_gb, 0)),
            ],
        )
        .replace('.', ",");
    }
}

/// Abre a janela no meio do monitor em que ela nasceu, com no máximo 90% dele
/// (1400×860 em escala de 125% não cabe num monitor de 1080 linhas). O Slint só
/// cria a janela nativa com o loop de eventos rodando, então tenta a cada 50 ms
/// até ela existir. No Wayland quem decide é o compositor e o pedido é ignorado.
fn centralizar(ui: &AppWindow) {
    use slint::winit_030::{WinitWindowAccessor, winit};
    let ui = ui.as_weak();
    // O fecho guarda uma cópia do Rc, o que mantém o timer vivo até ele parar.
    let timer = std::rc::Rc::new(slint::Timer::default());
    let t = timer.clone();
    let mut tentativas = 0;
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(50), move || {
        tentativas += 1;
        let Some(ui) = ui.upgrade() else { return t.stop() };
        let feito = ui.window().with_winit_window(|w: &winit::window::Window| {
            let Some(m) = w.current_monitor().or_else(|| w.primary_monitor()) else {
                return;
            };
            let (mp, ms) = (m.position(), m.size());
            let (inner, outer) = (w.inner_size(), w.outer_size());
            // Bordas e barra de título: o que a janela tem fora da área do app.
            let (bw, bh) = (outer.width - inner.width, outer.height - inner.height);
            let (ow, oh) = (outer.width.min(ms.width * 9 / 10), outer.height.min(ms.height * 9 / 10));
            if (ow, oh) != (outer.width, outer.height) {
                let _ = w.request_inner_size(winit::dpi::PhysicalSize::new(ow - bw, oh - bh));
            }
            let x = mp.x + ((ms.width - ow) / 2) as i32;
            let y = mp.y + ((ms.height - oh) / 2) as i32;
            w.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
        });
        if feito.is_some() || tentativas >= 40 {
            t.stop();
        }
    });
}

fn main() -> Result<(), slint::PlatformError> {
    let t = tokens::load();
    // No Mac, o renderizador por software: com 5 aparelhos na mesa ocupa ~148 MB
    // contra ~303 MB do femtovg e ~363 MB do Skia, e gasta menos CPU que a 0.2
    // (medido em 2026-10-21, MULTIPLATAFORMA.md §3). SLINT_BACKEND continua mandando.
    #[cfg(target_os = "macos")]
    if std::env::var_os("SLINT_BACKEND").is_none() {
        slint::BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name("software".into())
            .select()?;
    }
    let ui = AppWindow::new()?;
    // Inglês é a língua principal; português quando o sistema está em português.
    // MULTIDROID_LANG=en|pt força um dos dois. O Slint (@tr) e as mensagens do
    // Rust (idioma::t) seguem a mesma escolha.
    let pt = match std::env::var("MULTIDROID_LANG") {
        Ok(l) => idioma::eh_portugues(&l),
        Err(_) => sys_locale::get_locale().is_some_and(|l| idioma::eh_portugues(&l)),
    };
    idioma::usar_portugues(pt);
    let _ = slint::select_bundled_translation(if pt { "pt_BR" } else { "en" });
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
    // O motor leve só existe no Linux.
    ui.set_tem_leve(cfg!(target_os = "linux"));
    ui.set_novo_motor(if cfg!(target_os = "linux") { 1 } else { 0 });

    let runtime = tokio::runtime::Runtime::new().expect("sem runtime tokio");
    let infos = devices::load(&devices::avd_home());
    let mut state = State::load(&state_path(), platform::physical_gb());
    let ordem: Vec<&Device> = state.ordered(&infos);
    let mudou = state.ensure_spots(&ordem.clone());
    if mudou {
        let _ = state.save(&state_path());
    }
    let app = Arc::new(Mutex::new(App {
        devs: carregar(),
        state,
        aviso: String::new(),
        memoria: String::new(),
        img: Imagens::default(),
        novo_erro: String::new(),
    }));
    Ctx::ler_instalados(&mut app.lock().unwrap());
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
    let c = ctx.clone();
    ui.on_lote(move |a| c.lote(&a));
    let c = ctx.clone();
    ui.on_mover(move |n, dx, dy| c.mover(&n, dx, dy));
    let c = ctx.clone();
    ui.on_posicionar(move |n, x, y, w, h| c.posicionar(&n, x, y, w, h));
    let c = ctx.clone();
    ui.on_altura(move |n, h| c.altura(&n, h));
    let c = ctx.clone();
    ui.on_organizar(move |linhas| c.organizar(linhas));
    let c = ctx.clone();
    ui.on_teto_mudou(move |p| c.teto(p));
    let c = ctx.clone();
    let w = ui.as_weak();
    ui.on_abrir_imagens(move || {
        if let Some(ui) = w.upgrade() {
            ui.set_tela("imagens".into());
        }
        c.ler_repositorio();
    });
    let c = ctx.clone();
    ui.on_pacote(move |p, a| c.pacote(&p, &a));
    let c = ctx.clone();
    ui.on_licenca(move |ok| c.licenca(ok));
    let c = ctx.clone();
    let w = ui.as_weak();
    ui.on_abrir_novo(move || {
        if let Some(ui) = w.upgrade() {
            c.app.lock().unwrap().novo_erro.clear();
            ui.set_novo_imagem("".into());
            ui.set_tela("novo".into());
        }
        c.ler_repositorio();
    });
    let c = ctx.clone();
    let w = ui.as_weak();
    ui.on_novo_tipo_mudou(move |_| {
        if let Some(ui) = w.upgrade() {
            ui.set_novo_imagem("".into());
        }
        c.avisar();
    });
    let c = ctx.clone();
    ui.on_criar(move |motor, t, img, nome, grupo| {
        c.criar(Criar {
            leve: motor == 1,
            kind: tipo_de(t),
            image: img.to_string(),
            nome: nome.to_string(),
            grupo: grupo.to_string(),
        })
    });
    let c = ctx.clone();
    ui.on_grupo(move |n, g| c.grupo(&n, &g));
    let c = ctx.clone();
    ui.on_apagar(move |n| c.apagar(&n));

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

    centralizar(&ui);
    let r = ui.run();
    // Ao fechar, os aparelhos continuam ligados, como na 0.2: a próxima abertura os adota.
    runtime.shutdown_timeout(Duration::from_secs(1));
    r
}
