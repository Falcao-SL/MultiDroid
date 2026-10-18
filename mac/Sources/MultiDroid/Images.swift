import CryptoKit
import Darwin
import Foundation

/// Os pacotes que o MultiDroid instala sozinho: o emulador e as imagens de sistema.
/// Vêm do repositório público do Google (os mesmos arquivos que o sdkmanager
/// baixa), conferidos pelo SHA-1 publicado e extraídos na pasta do MultiDroid.
/// O MultiDroid nunca embute nem redistribui imagem: cada usuário aceita a licença
/// do Google e baixa direto de dl.google.com.
enum Images {
    static let emulator = "emulator"
    /// As imagens padrão (as mais leves que a fase 0 mediu): Android 13 com
    /// Play para celular, tablet e dobrável, Wear OS 4 para o relógio.
    static let play33 = "system-images;android-33;google_apis_playstore;arm64-v8a"
    static let wear33 = "system-images;android-33;android-wear;arm64-v8a"

    static let repo = URL(string: "https://dl.google.com/android/repository/")!

    /// Os tipos de imagem que o MultiDroid oferece, cada um com a sua lista no
    /// repositório do Google. Só arm64-v8a (nativas no Apple Silicon), canal
    /// estável, do Android 11 (API 30) em diante.
    static let tags = ["google_apis_playstore", "google_apis", "default", "android-wear"]
    /// Pasta de cada lista no repositório: a das imagens sem Google (tag
    /// "default") fica em sys-img/android, não em sys-img/default.
    static func listDir(_ tag: String) -> String { tag == "default" ? "android" : tag }
    static let minAPI = 30

    /// "system-images;android-34;google_apis;arm64-v8a" → ("34", "google_apis");
    /// nil para o que não é imagem de sistema arm64 que o MultiDroid gerencia.
    static func parts(_ path: String) -> (api: String, tag: String)? {
        let p = path.split(separator: ";").map(String.init)
        guard p.count == 4, p[0] == "system-images", p[1].hasPrefix("android-"), p[3] == "arm64-v8a",
              tags.contains(where: { p[2] == $0 || p[2].hasPrefix($0 + "-") }) else { return nil }
        return (String(p[1].dropFirst(8)), p[2])
    }

    static func isWear(_ path: String) -> Bool { parts(path)?.tag.hasPrefix("android-wear") ?? false }

    /// A imagem serve para esse tipo de aparelho? Wear só no relógio, e vice-versa.
    static func suits(_ path: String, _ kind: DeviceKind) -> Bool { isWear(path) == (kind == .relogio) }

    /// Número da API como número, para ordenar ("36.1" → 36.1).
    static func apiNumber(_ path: String) -> Double { Double(parts(path)?.api ?? "") ?? 0 }

    /// Versão do Android que o usuário reconhece: API 33 é o Android 13.
    /// Uma versão intermediária (API 36.1) leva o número da API junto, para não
    /// aparecer igual à 36: "16 (API 36.1)".
    static func androidVersion(_ api: String) -> String {
        let nums = api.split(separator: ".")
        guard let n = Int(nums.first ?? "") else { return api }
        let v = switch n {
        case 30: "11"
        case 31: "12"
        case 32: "12L"
        default: n >= 33 ? String(n - 20) : api
        }
        let minor = nums.count > 1 ? Int(nums[1]) ?? 0 : 0
        return minor == 0 ? v : "\(v) (API \(api))"
    }

    static func title(_ path: String) -> String {
        if path == emulator { return "Emulador do Android" }
        guard let (api, tag) = parts(path) else { return path }
        let v = androidVersion(api)
        if tag.hasPrefix("android-wear") { return "Wear OS (Android \(v))" }
        return switch tag {
        case "google_apis_playstore": "Android \(v) com Google Play"
        case "google_apis": "Android \(v) com APIs do Google"
        default: "Android \(v) sem Google (AOSP)"
        }
    }

    static func detail(_ path: String) -> String {
        if path == emulator { return "Roda os aparelhos. Obrigatório." }
        guard let (api, tag) = parts(path) else { return "" }
        let uso = tag.hasPrefix("android-wear") ? "Relógio" : "Celular, tablet e dobrável"
        let extra = switch tag {
        case "google_apis_playstore": " · com Play Store"
        case "google_apis": " · sem Play Store; apps por APK"
        case "default": " · sem nenhum app do Google; apps por APK"
        default: ""
        }
        return "\(uso) · API \(api)\(extra)"
    }

    static let play36 = "system-images;android-36;google_apis_playstore;arm64-v8a"

    /// As que levam o selo de recomendada, com o motivo: leves (o emulador
    /// pede pouca RAM), completas (com Play Store) e as mais novas nessas
    /// condições. Medido no MAC.md §3: o emulador força 2 GB no Android 13,
    /// 2,5 GB do 14 ao 16 e 4 GB no 17.
    static let recommended: [String: String] = [
        play33: "Recomendada: a mais leve, com Play Store (o emulador usa 2 GB de RAM; ~4,6 GB por celular).",
        play36: "Recomendada: a mais nova que ainda é leve, com Play Store (2,5 GB de RAM; ~5,3 GB por celular).",
        wear33: "Recomendada: a única versão do Wear OS que liga neste emulador.",
    ]

    /// O que a fase 0 mediu (MAC.md §3) e vale avisar antes de baixar.
    static func note(_ path: String) -> String? {
        if let why = recommended[path] { return why }
        guard let (api, tag) = parts(path), let n = Int(api.split(separator: ".").first ?? "") else { return nil }
        if tag.hasPrefix("android-wear"), n > 33 { return "Não ligou neste emulador nos testes do MultiDroid." }
        if n >= 37 { return "O emulador força 4 GB de RAM nesta versão: ~6,5 GB por celular." }
        if n >= 34 { return "O emulador força 2,5 GB de RAM nesta versão: ~0,7 GB a mais por celular que o Android 13." }
        return nil
    }

    /// Nome curto do tipo, para o config.ini do aparelho (tag.display).
    static func tagDisplay(_ path: String) -> String {
        guard let tag = parts(path)?.tag else { return "" }
        if tag.hasPrefix("android-wear") { return "Wear OS" }
        return switch tag {
        case "google_apis_playstore": "Google Play"
        case "google_apis": "Google APIs"
        default: "Default Android System Image"
        }
    }

    static func tagListURL(_ tag: String) -> URL { repo.appendingPathComponent("sys-img/\(listDir(tag))/sys-img2-3.xml") }

    /// Imagens instaladas numa pasta de SDK (a do MultiDroid ou a do Android Studio).
    static func scanInstalled(root: URL = SDK.root) -> [String: String] {
        let fm = FileManager.default
        var out: [String: String] = [:]
        let base = root.appendingPathComponent("system-images", isDirectory: true)
        for api in (try? fm.contentsOfDirectory(atPath: base.path)) ?? [] {
            for tag in (try? fm.contentsOfDirectory(atPath: base.appendingPathComponent(api).path)) ?? [] {
                let path = "system-images;\(api);\(tag);arm64-v8a"
                if parts(path) != nil, let rev = installed(path, root: root) { out[path] = rev }
            }
        }
        return out
    }

    /// "system-images;android-33;android-wear;arm64-v8a" → sdk/system-images/android-33/android-wear/arm64-v8a
    static func dir(_ path: String, root: URL = SDK.root) -> URL {
        path.split(separator: ";").reduce(root) { $0.appendingPathComponent(String($1), isDirectory: true) }
    }

    static func sysdir(_ path: String) -> String {
        path.split(separator: ";").joined(separator: "/") + "/"
    }

    /// Versão instalada (Pkg.Revision do source.properties), ou nil.
    static func installed(_ path: String, root: URL = SDK.root) -> String? {
        let props = readIni(dir(path, root: root).appendingPathComponent("source.properties"))
        if path == emulator, !FileManager.default.isExecutableFile(atPath: dir(path, root: root).appendingPathComponent("emulator").path) { return nil }
        return props["Pkg.Revision"]
    }
}

/// O que o repositório do Google oferece de um pacote.
struct RemotePackage: Sendable {
    let revision: String
    let size: Int64
    let sha1: String
    let url: URL
    let license: String
    let licenseText: String
}

@MainActor
@Observable
final class ImageStore {
    var remote: [String: RemotePackage] = [:]
    var installed: [String: String] = [:]
    var studio: [String: String] = [:]          // versões no SDK do Android Studio
    var progress: [String: Double] = [:]
    var phase: [String: String] = [:]
    var loading = false
    var error: String?
    private var tasks: [String: Task<Void, Never>] = [:]
    /// Ligados pelo AppModel: desligar quem usa o pacote antes da troca
    /// (devolve quem foi desligado) e religar depois.
    var beforeSwap: (@MainActor (String) async -> [String])?
    var afterSwap: (@MainActor ([String]) -> Void)?

    init() { scan() }

    var ready: Bool { installed[Images.emulator] != nil }
    /// Imagens instaladas que servem para esse tipo de aparelho, da mais nova à mais antiga.
    func installedImages(for kind: DeviceKind) -> [String] { images.filter { installed[$0] != nil && Images.suits($0, kind) } }

    func scan() {
        installed = Images.scanInstalled()
        installed[Images.emulator] = Images.installed(Images.emulator)
        studio = Images.scanInstalled(root: SDK.studioRoot)
        studio[Images.emulator] = Images.installed(Images.emulator, root: SDK.studioRoot)
    }

    /// Imagens de sistema conhecidas (instaladas ou oferecidas pelo Google),
    /// da mais nova para a mais antiga.
    var images: [String] {
        Set(installed.keys).union(remote.keys).subtracting([Images.emulator])
            .sorted { (Images.apiNumber($0), $0) > (Images.apiNumber($1), $1) }
    }

    /// Lê as listas do Google: a do emulador e uma por tipo de imagem.
    func refresh() async {
        loading = true
        defer { loading = false }
        var out: [String: RemotePackage] = [:]
        var failures: [String] = []
        // Cada lista é independente: uma que falhe não esconde as outras.
        let main = Images.repo.appendingPathComponent("repository2-3.xml")
        do {
            if let pkg = try Self.parse(try await Self.xml(main), path: Images.emulator, base: Images.repo) {
                out[Images.emulator] = pkg
            }
        } catch { failures.append("emulador (\(error.localizedDescription))") }
        for tag in Images.tags {
            let u = Images.tagListURL(tag)
            do {
                let doc = try await Self.xml(u)
                let paths = try doc.nodes(forXPath: "//remotePackage/@path").compactMap(\.stringValue)
                for p in Set(paths) where Images.parts(p) != nil && Images.apiNumber(p) >= Double(Images.minAPI) {
                    if let pkg = try Self.parse(doc, path: p, base: u.deletingLastPathComponent()) { out[p] = pkg }
                }
            } catch { failures.append("\(tag) (\(error.localizedDescription))") }
        }
        remote = out
        error = failures.isEmpty ? nil : "Não li do Google: " + failures.joined(separator: "; ")
    }

    private static func xml(_ url: URL) async throws -> XMLDocument {
        let (data, response) = try await URLSession.shared.data(from: url)
        if let http = response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
            throw InstallError("o Google respondeu \(http.statusCode)")
        }
        return try XMLDocument(data: data)
    }

    nonisolated static func parse(_ doc: XMLDocument, path: String, base: URL) throws -> RemotePackage? {
        let candidates = try doc.nodes(forXPath: "//remotePackage[@path='\(path)']").compactMap { $0 as? XMLElement }
        for el in candidates {
            // Só o canal estável (channel-0).
            let channel = try el.nodes(forXPath: "channelRef/@ref").first?.stringValue ?? "channel-0"
            guard channel == "channel-0" else { continue }
            for archive in try el.nodes(forXPath: "archives/archive").compactMap({ $0 as? XMLElement }) {
                let os = try archive.nodes(forXPath: "host-os").first?.stringValue
                let arch = try archive.nodes(forXPath: "host-arch").first?.stringValue
                if let os, os != "macosx" { continue }
                if let arch, arch != "aarch64" { continue }
                guard let file = try archive.nodes(forXPath: "complete/url").first?.stringValue,
                      let size = try archive.nodes(forXPath: "complete/size").first?.stringValue.flatMap({ Int64($0) }),
                      let sha1 = try archive.nodes(forXPath: "complete/checksum").first?.stringValue else { continue }
                let rev = try ["major", "minor", "micro"].compactMap { try el.nodes(forXPath: "revision/\($0)").first?.stringValue }
                let license = try el.nodes(forXPath: "uses-license/@ref").first?.stringValue ?? "android-sdk-license"
                let text = try doc.nodes(forXPath: "//license[@id='\(license)']").first?.stringValue ?? ""
                return RemotePackage(revision: rev.joined(separator: "."), size: size, sha1: sha1,
                                     url: base.appendingPathComponent(file),
                                     license: license, licenseText: text)
            }
        }
        return nil
    }

    // MARK: - Licenças

    /// Aceite gravado em sdk/licenses/<id> com o SHA-1 do texto, no mesmo
    /// formato do sdkmanager: se o Google mudar o texto, pede de novo.
    func accepted(_ pkg: RemotePackage) -> Bool {
        let file = SDK.licenses.appendingPathComponent(pkg.license)
        let hashes = (try? String(contentsOf: file, encoding: .utf8))?.split(whereSeparator: \.isNewline).map(String.init) ?? []
        return hashes.contains(Self.sha1(Data(pkg.licenseText.utf8)))
    }

    func accept(_ pkg: RemotePackage) {
        let file = SDK.licenses.appendingPathComponent(pkg.license)
        let old = (try? String(contentsOf: file, encoding: .utf8)) ?? ""
        try? (old + "\n" + Self.sha1(Data(pkg.licenseText.utf8))).write(to: file, atomically: true, encoding: .utf8)
    }

    nonisolated static func sha1(_ data: Data) -> String {
        Insecure.SHA1.hash(data: data).map { String(format: "%02x", $0) }.joined()
    }

    // MARK: - Instalar, importar, remover

    func install(_ path: String) {
        guard let pkg = remote[path], tasks[path] == nil else { return }
        let needed = pkg.size * 3  // zip + extraído, com folga
        if let free = try? SDK.home.resourceValues(forKeys: [.volumeAvailableCapacityForImportantUsageKey])
            .volumeAvailableCapacityForImportantUsage, free < needed {
            error = "Falta espaço: precisa de \(gb(Double(needed) / 1e9)) livres"
            return
        }
        progress[path] = 0
        phase[path] = "baixando"
        tasks[path] = Task {
            defer { tasks[path] = nil; progress[path] = nil; phase[path] = nil; scan() }
            do {
                let zip = try await Downloader.fetch(pkg.url) { [weak self] f in
                    Task { @MainActor in self?.progress[path] = f }
                }
                defer { try? FileManager.default.removeItem(at: zip) }
                phase[path] = "conferindo"
                let ok = await Task.detached { (try? Self.fileSHA1(zip)) == pkg.sha1 }.value
                guard ok else { throw InstallError("o arquivo baixado não confere com o SHA-1 do Google") }
                // O download corre com os aparelhos ligados; só a troca pede desligar.
                phase[path] = "desligando aparelhos"
                let paused = await beforeSwap?(path) ?? []
                defer { scan(); afterSwap?(paused) }
                phase[path] = "instalando"
                try await Task.detached { try Self.unpack(zip, as: path) }.value
                error = nil
            } catch is CancellationError {
            } catch {
                self.error = "\(Images.title(path)): \(error.localizedDescription)"
            }
        }
    }

    func cancel(_ path: String) {
        tasks[path]?.cancel()
    }

    /// Copia do Android Studio com clonefile: instantâneo e sem ocupar disco
    /// até um dos lados mudar.
    func importFromStudio(_ path: String) {
        let src = Images.dir(path, root: SDK.studioRoot), dst = Images.dir(path)
        try? FileManager.default.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
        try? FileManager.default.removeItem(at: dst)
        if clonefile(src.path, dst.path, 0) != 0 {
            error = "não copiou: \(String(cString: strerror(errno)))"
        }
        scan()
    }

    func remove(_ path: String) {
        try? FileManager.default.removeItem(at: Images.dir(path))
        scan()
    }

    nonisolated static func fileSHA1(_ url: URL) throws -> String {
        let h = try FileHandle(forReadingFrom: url)
        defer { try? h.close() }
        var sha = Insecure.SHA1()
        while let chunk = try h.read(upToCount: 8 << 20), !chunk.isEmpty { sha.update(data: chunk) }
        return sha.finalize().map { String(format: "%02x", $0) }.joined()
    }

    /// Extrai com o ditto do macOS (mantém links e permissões) numa pasta
    /// temporária e troca a instalação de uma vez.
    nonisolated static func unpack(_ zip: URL, as path: String) throws {
        let fm = FileManager.default
        let tmp = SDK.home.appendingPathComponent("tmp-\(UUID().uuidString)", isDirectory: true)
        try fm.createDirectory(at: tmp, withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: tmp) }
        // O ditto não lê alguns zips do Google (o do Wear OS 33 r04: "Couldn't read
        // pkzip signature"); o unzip do sistema lê. Tenta um, depois o outro.
        func run(_ tool: String, _ args: [String]) throws -> Bool {
            let p = Process()
            p.executableURL = URL(fileURLWithPath: tool)
            p.arguments = args
            try p.run()
            p.waitUntilExit()
            return p.terminationStatus == 0
        }
        if try !run("/usr/bin/ditto", ["-x", "-k", zip.path, tmp.path]) {
            for item in (try? fm.contentsOfDirectory(at: tmp, includingPropertiesForKeys: nil)) ?? [] { try? fm.removeItem(at: item) }
            guard try run("/usr/bin/unzip", ["-q", zip.path, "-d", tmp.path]) else { throw InstallError("não extraiu o arquivo") }
        }
        // O zip tem uma única pasta no topo (emulator/ ou arm64-v8a/).
        guard let top = try fm.contentsOfDirectory(at: tmp, includingPropertiesForKeys: nil)
            .first(where: { !$0.lastPathComponent.hasPrefix(".") }) else { throw InstallError("arquivo vazio") }
        let dst = Images.dir(path)
        try fm.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
        if fm.fileExists(atPath: dst.path) { try fm.removeItem(at: dst) }
        try fm.moveItem(at: top, to: dst)
    }
}

struct InstallError: LocalizedError {
    let message: String
    init(_ m: String) { message = m }
    var errorDescription: String? { message }
}

/// Download com progresso, para arquivo temporário. Se a conexão cair, retoma
/// de onde parou (resume data do URLSession, que usa Range no servidor), até
/// 10 vezes. Cancela junto com a Task.
final class Downloader: NSObject, URLSessionDownloadDelegate, @unchecked Sendable {
    private let onProgress: @Sendable (Double) -> Void
    private var continuation: CheckedContinuation<URL, Error>?
    private var lastReport = Date.distantPast
    private var session: URLSession!
    private var task: URLSessionDownloadTask?
    private var retries = 0
    private var cancelled = false
    private let lock = NSLock()

    private init(onProgress: @escaping @Sendable (Double) -> Void) { self.onProgress = onProgress }

    static func fetch(_ url: URL, onProgress: @escaping @Sendable (Double) -> Void) async throws -> URL {
        let d = Downloader(onProgress: onProgress)
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 60
        let queue = OperationQueue()
        queue.maxConcurrentOperationCount = 1
        d.session = URLSession(configuration: config, delegate: d, delegateQueue: queue)
        defer { d.session.finishTasksAndInvalidate() }
        return try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { c in
                d.lock.withLock {
                    d.continuation = c
                    d.task = d.session.downloadTask(with: url)
                    d.task?.resume()
                }
            }
        } onCancel: {
            d.lock.withLock { d.cancelled = true; d.task?.cancel() }
        }
    }

    func urlSession(_ s: URLSession, downloadTask: URLSessionDownloadTask, didWriteData _: Int64, totalBytesWritten done: Int64, totalBytesExpectedToWrite total: Int64) {
        guard total > 0, Date().timeIntervalSince(lastReport) > 0.25 else { return }
        lastReport = Date()
        onProgress(Double(done) / Double(total))
    }

    func urlSession(_ s: URLSession, downloadTask: URLSessionDownloadTask, didFinishDownloadingTo location: URL) {
        // O arquivo some quando este método volta: move para a pasta do MultiDroid.
        let keep = SDK.home.appendingPathComponent("download-\(UUID().uuidString).zip")
        let result: Result<URL, Error>
        do {
            if let http = downloadTask.response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
                throw InstallError("o Google respondeu \(http.statusCode)")
            }
            try FileManager.default.moveItem(at: location, to: keep)
            result = .success(keep)
        } catch {
            result = .failure(error)
        }
        lock.withLock {
            continuation?.resume(with: result)
            continuation = nil
        }
    }

    func urlSession(_ s: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        guard let error else { return }
        lock.lock()
        defer { lock.unlock() }
        guard let c = continuation else { return }
        if cancelled { continuation = nil; c.resume(throwing: CancellationError()); return }
        let resume = (error as NSError).userInfo[NSURLSessionDownloadTaskResumeData] as? Data
        if let resume, retries < 10 {
            retries += 1
            let delay = Double(min(retries, 5))
            DispatchQueue.global().asyncAfter(deadline: .now() + delay) { [self] in
                lock.withLock {
                    guard !cancelled, continuation != nil else { return }
                    self.task = session.downloadTask(withResumeData: resume)
                    self.task?.resume()
                }
            }
            return
        }
        continuation = nil
        c.resume(throwing: error)
    }
}
