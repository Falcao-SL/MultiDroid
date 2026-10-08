import Darwin
import Foundation
import Observation

/// Um aparelho do painel: o AVD e o que está rodando dele agora.
@MainActor
@Observable
final class Device: Identifiable {
    enum State: Equatable {
        case off, starting, on, paused, stopping
        /// Rodando fora do Duo (aberto pelo Android Studio ou em janela própria).
        case external
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
    fileprivate var lastCPU: (nanos: UInt64, at: Date)?

    nonisolated let id: String
    /// Qualquer estado com processo vivo (ou ainda morrendo): bloqueia duplicar e apagar.
    var isRunning: Bool { state != .off }

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
}

/// Estado do app: aparelhos, grupos, ordem da grade, teto de memória.
@MainActor
@Observable
final class AppModel {
    enum Layout: String, CaseIterable, Identifiable { case grade = "Grade", grupos = "Por grupo"; var id: String { rawValue } }

    var devices: [Device] = []
    var order: [String] = []
    var groups: [String: String] = [:]          // aparelho → grupo
    var layout: Layout = .grade
    var focused: String?
    var ceilingGB: Double
    var availableGB = 0.0
    var swapGB = 0.0
    var banner: String?

    private var metricsTask: Task<Void, Never>?
    private let stateURL: URL

    init() {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Duo", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        stateURL = dir.appendingPathComponent("state.json")
        ceilingGB = (Metrics.physicalGB * 0.75).rounded()
        loadState()
        refresh()
        // --start A,B: liga aparelhos ao abrir (testes e atalhos).
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "--start"), i + 1 < args.count {
            for name in args[i + 1].split(separator: ",") { if let d = device(String(name)) { start(d) } }
        }
        metricsTask = Task { [weak self] in
            while !Task.isCancelled {
                self?.sampleMetrics()
                try? await Task.sleep(for: .seconds(2))
            }
        }
    }

    // MARK: - Estado salvo (grupos, ordem, teto)

    private struct Saved: Codable { var order: [String]; var groups: [String: String]; var ceilingGB: Double }

    private func loadState() {
        guard let data = try? Data(contentsOf: stateURL), let s = try? JSONDecoder().decode(Saved.self, from: data) else { return }
        order = s.order; groups = s.groups; ceilingGB = s.ceilingGB
    }

    func save() {
        let s = Saved(order: order, groups: groups, ceilingGB: ceilingGB)
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

    /// Aparelhos já rodando (abertos antes, ou pelo Android Studio): o Duo os
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

    func start(_ d: Device) {
        guard d.state == .off else { return }
        let projected = projectedGB(adding: d)
        guard projected <= ceilingGB else {
            banner = String(format: "%@ não liga: os aparelhos passariam de %.0f GB (teto de %.0f GB)", d.id, projected, ceilingGB)
            return
        }
        let used = Set(devices.compactMap(\.consolePort))
        let console = stride(from: 5600, to: 5680, by: 2).first { !used.contains($0) } ?? 5600
        let grpcPort = 8600 + (console - 5600) / 2
        let p = Process()
        p.executableURL = SDK.emulator
        p.arguments = ["-avd", d.id, "-no-window", "-no-audio", "-no-boot-anim", "-gpu", "host",
                       "-cores", "\(d.info.kind.cores)", "-grpc", "\(grpcPort)", "-grpc-use-token", "-port", "\(console)"]
        // Saída do emulador em ~/Library/Logs/Duo/<aparelho>.log, para entender falhas.
        let logs = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Logs/Duo")
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
        d.state = .on
        Task {
            await link.setPaused(false)
            d.screen?.start(link)
        }
    }

    func stop(_ d: Device) {
        guard d.isRunning, let console = d.consolePort else { return }
        d.state = .stopping
        d.screen?.stop()
        let pid = d.pid
        Task {
            // "emu kill" salva o snapshot: a próxima partida leva segundos.
            _ = await Task.detached { SDK.run(SDK.adb, ["-s", "emulator-\(console)", "emu", "kill"]) }.value
            for _ in 0..<60 {
                if let pid, kill(pid, 0) != 0 { break }
                try? await Task.sleep(for: .milliseconds(500))
            }
            d.link?.close()
            d.link = nil; d.screen = nil; d.pid = nil; d.consolePort = nil
            d.footprintGB = 0; d.cpu = 0; d.state = .off; d.message = nil
        }
    }

    func duplicate(_ d: Device) {
        guard !d.isRunning else { banner = "desligue \(d.id) antes de duplicar"; return }
        var n = 2
        while device("\(d.id)-\(n)") != nil { n += 1 }
        let name = "\(d.id)-\(n)"
        if let err = Catalog.duplicate(d.info, as: name) { banner = "não duplicou: \(err)"; return }
        if let g = groups[d.id] { groups[name] = g }
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
            refresh()
        }
    }

    func create(name: String, kind: DeviceKind, group: String?) async -> String? {
        let err = await Task.detached { Catalog.create(name: name, kind: kind) }.value
        if err == nil, let group, !group.isEmpty { groups[name] = group }
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

    /// Processo sumiu sem passar pelo Duo (fechado por fora): volta a desligado.
    private func adoptCheck(_ d: Device) {
        if let pid = d.pid, kill(pid, 0) == 0 { return }
        d.screen?.stop(); d.link?.close()
        d.link = nil; d.screen = nil; d.pid = nil; d.consolePort = nil
        d.footprintGB = 0; d.cpu = 0; d.state = .off
    }

    var runningGB: Double { devices.filter(\.isRunning).reduce(0) { $0 + $1.footprintGB } }
}
