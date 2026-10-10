import CoreGraphics
import Darwin
import Foundation
import Observation

/// Um aparelho do painel: o AVD e o que está rodando dele agora.
@MainActor
@Observable
final class Device: Identifiable {
    enum State: Equatable {
        case off, starting, on, paused, stopping
        /// Rodando fora do MultiDroid (aberto pelo Android Studio ou em janela própria).
        case external
        /// Hibernado: desligado com o estado salvo, para dar memória a outro.
        /// Não gasta nada; volta em segundos pelo Quick Boot.
        case asleep
    }

    var info: DeviceInfo
    var state: State = .off
    var pid: pid_t?
    var consolePort: Int?
    var link: EmulatorLink?
    var screen: ScreenStream?
    var footprintGB = 0.0
    var cpu = 0.0
    var message: String?
    var landscape = false
    /// Último uso (toque, tecla, ampliar, ligar): quem está há mais tempo sem
    /// uso é o primeiro a hibernar.
    var lastUsed = Date.distantPast
    /// A última tela antes de hibernar, mostrada esmaecida no quadro.
    var lastFrame: CGImage?
    /// Esperando outro hibernar para poder ligar.
    var waking = false
    fileprivate var lastCPU: (nanos: UInt64, at: Date)?

    nonisolated let id: String
    /// Qualquer estado com processo vivo (ou ainda morrendo): bloqueia duplicar e apagar.
    var isRunning: Bool { state != .off && state != .asleep }

    init(info: DeviceInfo) { self.info = info; self.id = info.name }

    /// Tamanho do quadro pedido ao emulador: o do quadro na grade, em 2× para tela Retina.
    var frameSize: (Int, Int) {
        let parts = info.screen.split(separator: "×").compactMap { Int($0) }
        let (w, h) = parts.count == 2 ? (parts[0], parts[1]) : (1080, 2400)
        let base: Int = switch info.kind {
        case .celular: 300
        case .dobravel: 380
        case .tablet: 520
        case .relogio: 220
        }
        let fw = base * 2
        return (fw, max(1, fw * h / w))
    }

    /// Tamanho-base do quadro na mesa, em pontos.
    var tileBase: CGSize {
        let (w, h) = frameSize
        return CGSize(width: CGFloat(w) / 2, height: CGFloat(h) / 2)
    }
}

/// Estado do app: aparelhos, grupos, ordem da grade, teto de memória.
@MainActor
@Observable
final class AppModel {
    static let shared = AppModel()
    static let mainDesk = "main"

    enum Layout: String, CaseIterable, Identifiable, Codable {
        case grade = "Grade", auto = "Automática", grupos = "Por grupo"
        var id: String { rawValue }
    }

    var devices: [Device] = []
    var order: [String] = []
    var groups: [String: String] = [:]          // aparelho → grupo
    var spots: [String: GridSpot] = [:]         // aparelho → lugar na grade livre
    var layouts: [String: Layout] = [:]           // mesa → arranjo
    var workspaceOf: [String: String] = [:]       // aparelho → mesa (sem entrada = principal)
    var workspaces: [String] = [AppModel.mainDesk] // mesas, na ordem de criação
    var openWindows: Set<String> = []
    var ceilingGB: Double
    var availableGB = 0.0
    var swapGB = 0.0
    var banner: String?
    let images = ImageStore()

    private var metricsTask: Task<Void, Never>?
    private let stateURL: URL

    init() {
        SDK.migrateLegacyHome()
        try? FileManager.default.createDirectory(at: SDK.home, withIntermediateDirectories: true)
        stateURL = SDK.home.appendingPathComponent("state.json")
        ceilingGB = (Metrics.physicalGB * 0.75).rounded()
        SDK.prepare()
        migrateFromStudio()
        wireImageSwap()
        loadState()
        refresh()
        // --start A,B: liga aparelhos ao abrir (testes e atalhos), um de cada
        // vez: o próximo só começa quando o anterior terminou o boot.
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "--start"), i + 1 < args.count {
            let names = args[i + 1].split(separator: ",").map(String.init)
            Task { [weak self] in
                for name in names {
                    guard let self, let d = self.device(name) else { continue }
                    self.start(d)
                    for _ in 0..<480 {
                        try? await Task.sleep(for: .milliseconds(500))
                        if d.state == .on || (!d.waking && (d.state == .off || d.state == .asleep)) { break }
                    }
                }
            }
        }
        metricsTask = Task { [weak self] in
            while !Task.isCancelled {
                self?.sampleMetrics()
                try? await Task.sleep(for: .seconds(2))
            }
        }
    }

    /// Uma vez só: os aparelhos que versões antigas (ainda chamadas Duo) criaram no Android
    /// Studio (nome "Duo-…") vêm para a pasta do MultiDroid, com a imagem que usam
    /// e o emulador copiados por clonefile (sem baixar, sem ocupar disco).
    private func migrateFromStudio() {
        guard Catalog.load().isEmpty else { return }
        let names = Catalog.studioDevices().filter { $0.hasPrefix("Duo-") && !Catalog.isRunning($0) }
        guard !names.isEmpty else { return }
        if images.installed[Images.emulator] == nil, images.studio[Images.emulator] != nil { images.importFromStudio(Images.emulator) }
        for n in names {
            guard let img = Catalog.studioImage(n), Images.all.contains(img) else { continue }
            if images.installed[img] == nil, images.studio[img] != nil { images.importFromStudio(img) }
            if images.installed[img] != nil { _ = Catalog.importFromStudio(n) }
        }
    }

    // MARK: - Mesas (uma por janela)

    func desk(of id: String) -> String { workspaceOf[id] ?? Self.mainDesk }
    func devices(in desk: String) -> [Device] { ordered.filter { self.desk(of: $0.id) == desk } }

    /// Id da janela nova: a principal, se estiver fechada; senão uma mesa vazia.
    func newWindowID() -> String {
        if !openWindows.contains(Self.mainDesk) { return Self.mainDesk }
        var n = 2
        while workspaces.contains("mesa-\(n)") { n += 1 }
        let id = "mesa-\(n)"
        workspaces.append(id)
        save()
        return id
    }

    func title(of desk: String) -> String {
        desk == Self.mainDesk ? "MultiDroid" : "MultiDroid · Mesa \(desk.replacingOccurrences(of: "mesa-", with: ""))"
    }

    func windowOpened(_ desk: String) {
        openWindows.insert(desk)
        if !workspaces.contains(desk) { workspaces.append(desk); save() }
    }

    /// Fechar a janela não desliga os aparelhos. Mesa vazia fechada some.
    func windowClosed(_ desk: String) {
        openWindows.remove(desk)
        if desk != Self.mainDesk, devices(in: desk).isEmpty {
            workspaces.removeAll { $0 == desk }
            layouts[desk] = nil
            save()
        }
    }

    // MARK: - Estado salvo (grupos, ordem, teto)

    private struct Saved: Codable {
        var order: [String]; var groups: [String: String]; var ceilingGB: Double
        var spots: [String: GridSpot]?; var layout: Layout?
        var layouts: [String: Layout]?; var workspaceOf: [String: String]?; var workspaces: [String]?
    }

    private func loadState() {
        guard let data = try? Data(contentsOf: stateURL), let s = try? JSONDecoder().decode(Saved.self, from: data) else { return }
        order = s.order; groups = s.groups; ceilingGB = s.ceilingGB
        spots = s.spots ?? [:]
        layouts = s.layouts ?? [Self.mainDesk: s.layout ?? .grade]
        workspaceOf = s.workspaceOf ?? [:]
        workspaces = s.workspaces ?? [Self.mainDesk]
    }

    func save() {
        let s = Saved(order: order, groups: groups, ceilingGB: ceilingGB, spots: spots, layout: nil,
                      layouts: layouts, workspaceOf: workspaceOf, workspaces: workspaces)
        try? JSONEncoder().encode(s).write(to: stateURL)
    }

    // MARK: - Catálogo e processos

    func device(_ name: String) -> Device? { devices.first { $0.id == name } }

    var ordered: [Device] {
        devices.sorted { (order.firstIndex(of: $0.id) ?? .max, $0.id) < (order.firstIndex(of: $1.id) ?? .max, $1.id) }
    }

    var groupNames: [String] { Array(Set(groups.values)).sorted() }

    func refresh() {
        let infos = Catalog.load()
        var next: [Device] = []
        for info in infos {
            if let d = device(info.name) { d.info = info; next.append(d) } else { next.append(Device(info: info)) }
        }
        devices = next
        for d in devices where !order.contains(d.id) { order.append(d.id) }
        order.removeAll { name in !devices.contains { $0.id == name } }
        adoptRunning()
        save()
    }

    /// Aparelhos já rodando (abertos antes, ou pelo Android Studio): o MultiDroid os
    /// mostra e consegue desligar; se tiverem gRPC com token, também a tela.
    private func adoptRunning() {
        let ps = SDK.run(URL(fileURLWithPath: "/bin/ps"), ["-axo", "pid=,args="]).output
        for line in ps.split(separator: "\n") where line.contains("qemu-system-aarch64") {
            let parts = line.split(separator: " ", omittingEmptySubsequences: true).map(String.init)
            guard let pid = pid_t(parts.first ?? ""), let i = parts.firstIndex(of: "-avd"), i + 1 < parts.count,
                  let d = device(parts[i + 1]), d.pid == nil || d.pid == pid else { continue }
            d.pid = pid
            if let p = parts.firstIndex(of: "-port"), p + 1 < parts.count { d.consolePort = Int(parts[p + 1]) }
            if d.consolePort == nil { d.consolePort = 5554 }
            if d.state == .off { d.state = .external }
            if d.link == nil, parts.contains("-grpc"), let g = parts.firstIndex(of: "-grpc"), let port = Int(parts[g + 1]),
               let token = readIni(SDK.discovery.appendingPathComponent("pid_\(pid).ini"))["grpc.token"] {
                attach(d, grpcPort: port, token: token)
            }
        }
    }

    // MARK: - Ações

    func projectedGB(adding d: Device) -> Double {
        devices.filter(\.isRunning).reduce(0) { $0 + max($1.footprintGB, $1.info.kind.estimateGB) } + d.info.kind.estimateGB
    }

    func used(_ d: Device) { d.lastUsed = Date() }

    /// Liga (ou acorda) um aparelho. Se ele não couber no teto de memória, os
    /// aparelhos sem uso há mais tempo hibernam antes, até caber.
    func start(_ d: Device) {
        guard d.state == .off || d.state == .asleep, !d.waking else { return }
        guard images.ready else { banner = "Baixe o emulador em Imagens antes de ligar"; return }
        used(d)
        guard projectedGB(adding: d) > ceilingGB else { launch(d); return }
        d.waking = true
        d.message = "abrindo espaço…"
        Task {
            let fits = await makeRoom(for: d)
            d.waking = false
            d.message = nil
            if fits { launch(d) }
        }
    }

    private func makeRoom(for d: Device) async -> Bool {
        while projectedGB(adding: d) > ceilingGB {
            let awake = devices.filter { ($0.state == .on || $0.state == .paused) && $0.id != d.id }
            guard let victim = awake.min(by: { $0.lastUsed < $1.lastUsed }) else {
                banner = String(format: "%@ não liga: nem hibernando os outros cabe no teto de %.0f GB", d.id, ceilingGB)
                return false
            }
            banner = "\(victim.id) hibernou para \(d.id) ligar"
            await hibernateAndWait(victim)
        }
        return true
    }

    private func launch(_ d: Device) {
        let used = Set(devices.compactMap(\.consolePort))
        let console = stride(from: 5600, to: 5680, by: 2).first { !used.contains($0) } ?? 5600
        let grpcPort = 8600 + (console - 5600) / 2
        let p = Process()
        p.executableURL = SDK.emulator
        p.environment = SDK.environment
        p.arguments = ["-avd", d.id, "-no-window", "-no-audio", "-no-boot-anim", "-no-metrics", "-gpu", "host",
                       "-cores", "\(d.info.kind.cores)", "-grpc", "\(grpcPort)", "-grpc-use-token", "-port", "\(console)"]
        // Saída do emulador em ~/Library/Logs/MultiDroid/<aparelho>.log, para entender falhas.
        let logs = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Logs/MultiDroid")
        try? FileManager.default.createDirectory(at: logs, withIntermediateDirectories: true)
        let logURL = logs.appendingPathComponent("\(d.id).log")
        FileManager.default.createFile(atPath: logURL.path, contents: nil)
        let log = try? FileHandle(forWritingTo: logURL)
        p.standardOutput = log ?? FileHandle.nullDevice
        p.standardError = log ?? FileHandle.nullDevice
        do { try p.run() } catch { d.message = "não abriu o emulador: \(error.localizedDescription)"; return }
        d.pid = p.processIdentifier
        d.consolePort = console
        d.state = .starting
        d.message = "ligando…"
        let pid = p.processIdentifier
        Task {
            // O emulador escreve porta e token gRPC em pid_<pid>.ini ao subir.
            var token: String?
            for _ in 0..<120 {
                token = readIni(SDK.discovery.appendingPathComponent("pid_\(pid).ini"))["grpc.token"]
                if token != nil || !p.isRunning { break }
                try? await Task.sleep(for: .milliseconds(500))
            }
            guard let token, p.isRunning else { d.state = .off; d.pid = nil; d.message = "o emulador fechou ao ligar"; return }
            attach(d, grpcPort: grpcPort, token: token)
        }
    }

    private func attach(_ d: Device, grpcPort: Int, token: String) {
        guard let link = try? EmulatorLink(port: grpcPort, token: token) else { d.message = "sem conexão gRPC"; return }
        d.link = link
        Task {
            for _ in 0..<240 {
                if await link.booted() { break }
                try? await Task.sleep(for: .seconds(1))
            }
            if d.state == .starting || d.state == .external { d.state = .on }
            d.message = nil
            let (w, h) = d.frameSize
            let s = ScreenStream(width: w, height: h, tag: d.id)
            d.screen = s
            s.start(link)
        }
    }

    func pause(_ d: Device) {
        guard d.state == .on, let link = d.link else { return }
        d.screen?.stop()
        d.state = .paused
        Task { await link.setPaused(true) }
    }

    func resume(_ d: Device) {
        guard d.state == .paused, let link = d.link else { return }
        used(d)
        d.state = .on
        Task {
            await link.setPaused(false)
            d.screen?.start(link)
        }
    }

    func stop(_ d: Device) {
        if d.state == .asleep { d.state = .off; d.lastFrame = nil; return }
        Task { await stopAndWait(d) }
    }

    /// Hibernar é desligar salvando o estado (como o Desligar) e guardar a
    /// última tela; o aparelho acorda sozinho ao ser usado de novo.
    func hibernate(_ d: Device) {
        Task { await hibernateAndWait(d) }
    }

    private func hibernateAndWait(_ d: Device) async {
        guard d.state == .on || d.state == .paused else { return }
        d.lastFrame = d.screen?.image
        await stopAndWait(d, to: .asleep)
    }

    func stopAndWait(_ d: Device, to final: Device.State = .off) async {
        guard d.isRunning, d.state != .stopping, let console = d.consolePort else { return }
        d.state = .stopping
        d.screen?.stop()
        let pid = d.pid
        // "kill" pelo console salva o estado: a próxima partida leva segundos.
        // Se o console não responder, SIGTERM (desliga limpo, sem salvar).
        let asked = await SDK.console(port: console, "kill")
        if !asked, let pid { kill(pid, SIGTERM) }
        for i in 0..<60 {
            if let pid, kill(pid, 0) != 0 { break }
            if i == 40, let pid { kill(pid, SIGTERM) }
            try? await Task.sleep(for: .milliseconds(500))
        }
        d.link?.close()
        d.link = nil; d.screen = nil; d.pid = nil; d.consolePort = nil
        d.footprintGB = 0; d.cpu = 0; d.state = final; d.message = nil
        if final == .off { d.lastFrame = nil }
    }

    /// Aparelhos que usam um pacote: todos, para o emulador; os do tipo, para uma imagem.
    func users(of path: String) -> [Device] {
        devices.filter { $0.isRunning && (path == Images.emulator || $0.info.kind.image == path) }
    }

    /// Antes de trocar um pacote em uso: desliga quem o usa (estado salvo).
    /// Depois: liga de novo os mesmos aparelhos.
    private func wireImageSwap() {
        images.beforeSwap = { [weak self] path in
            guard let self else { return [] }
            let list = users(of: path)
            for d in list { d.message = "atualizando…" }
            await withTaskGroup(of: Void.self) { g in for d in list { g.addTask { await self.stopAndWait(d) } } }
            return list.map(\.id)
        }
        images.afterSwap = { [weak self] names in
            guard let self else { return }
            for n in names { if let d = device(n) { start(d) } }
        }
    }

    func duplicate(_ d: Device) {
        guard !d.isRunning else { banner = "desligue \(d.id) antes de duplicar"; return }
        var n = 2
        while device("\(d.id)-\(n)") != nil { n += 1 }
        let name = "\(d.id)-\(n)"
        if let err = Catalog.duplicate(d.info, as: name) { banner = "não duplicou: \(err)"; return }
        if let g = groups[d.id] { groups[name] = g }
        if let w = workspaceOf[d.id] { workspaceOf[name] = w }
        if let i = order.firstIndex(of: d.id) { order.insert(name, at: i + 1) }
        refresh()
    }

    func delete(_ d: Device) {
        guard !d.isRunning else { banner = "desligue \(d.id) antes de apagar"; return }
        Task {
            let info = d.info
            let err = await Task.detached { Catalog.delete(info) }.value
            if let err { banner = "não apagou: \(err)" }
            groups[d.id] = nil
            spots[d.id] = nil
            workspaceOf[d.id] = nil
            refresh()
        }
    }

    func create(name: String, kind: DeviceKind, group: String?, desk: String) async -> String? {
        let err = await Task.detached { Catalog.create(name: name, kind: kind) }.value
        if err == nil, let group, !group.isEmpty { groups[name] = group }
        if err == nil, desk != Self.mainDesk { workspaceOf[name] = desk }
        refresh()
        return err
    }

    func setGroup(_ group: String?, for names: [String]) {
        for n in names { groups[n] = (group?.isEmpty ?? true) ? nil : group }
        save()
    }

    func move(_ name: String, before target: String) {
        guard name != target, let from = order.firstIndex(of: name) else { return }
        order.remove(at: from)
        let to = order.firstIndex(of: target) ?? order.endIndex
        order.insert(name, at: to)
        save()
    }

    // MARK: - Métricas

    private func sampleMetrics() {
        availableGB = Metrics.availableGB()
        swapGB = Metrics.swapUsedGB()
        let now = Date()
        for d in devices {
            guard let pid = d.pid, let u = Metrics.usage(pid) else {
                if d.state == .external || d.state == .on { adoptCheck(d) }
                continue
            }
            d.footprintGB = Double(u.footprint) / Metrics.gb
            if let last = d.lastCPU {
                let dt = now.timeIntervalSince(last.at)
                if dt > 0 { d.cpu = Double(u.cpuNanos &- last.nanos) / 1e9 / dt * 100 }
            }
            d.lastCPU = (u.cpuNanos, now)
        }
    }

    /// Processo sumiu sem passar pelo MultiDroid (fechado por fora): volta a desligado.
    private func adoptCheck(_ d: Device) {
        if let pid = d.pid, kill(pid, 0) == 0 { return }
        d.screen?.stop(); d.link?.close()
        d.link = nil; d.screen = nil; d.pid = nil; d.consolePort = nil
        d.footprintGB = 0; d.cpu = 0; d.state = .off
    }

    var runningGB: Double { devices.filter(\.isRunning).reduce(0) { $0 + $1.footprintGB } }
    var asleepCount: Int { devices.filter { $0.state == .asleep }.count }
}
