import CryptoKit
import Darwin
import Foundation

/// Os pacotes que o Duo instala sozinho: o emulador e as imagens de sistema.
/// Vêm do repositório público do Google (os mesmos arquivos que o sdkmanager
/// baixa), conferidos pelo SHA-1 publicado e extraídos na pasta do Duo.
/// O Duo nunca embute nem redistribui imagem: cada usuário aceita a licença
/// do Google e baixa direto de dl.google.com.
enum Images {
    static let emulator = "emulator"
    static let play33 = "system-images;android-33;google_apis_playstore;arm64-v8a"
    static let wear33 = "system-images;android-33;android-wear;arm64-v8a"
    static let all = [emulator, play33, wear33]

    static let repo = URL(string: "https://dl.google.com/android/repository/")!

    static func title(_ path: String) -> String {
        switch path {
        case emulator: "Emulador do Android"
        case play33: "Android 13 com Google Play"
        case wear33: "Wear OS 4 (Android 13)"
        default: path
        }
    }

    static func detail(_ path: String) -> String {
        switch path {
        case emulator: "Roda os aparelhos. Obrigatório."
        case play33: "Celular, tablet e dobrável."
        case wear33: "Relógio."
        default: ""
        }
    }

    /// Lista XML onde o pacote é publicado, e base das URLs dos arquivos.
    static func listURL(_ path: String) -> URL {
        if path == emulator { return repo.appendingPathComponent("repository2-3.xml") }
        return base(path).appendingPathComponent("sys-img2-3.xml")
    }

    static func base(_ path: String) -> URL {
        if path == emulator { return repo }
        let tag = path.split(separator: ";")[2]
        return repo.appendingPathComponent("sys-img/\(tag)/")
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
    func hasImage(for kind: DeviceKind) -> Bool { installed[kind.image] != nil }

    func scan() {
        for p in Images.all {
            installed[p] = Images.installed(p)
            studio[p] = Images.installed(p, root: SDK.studioRoot)
        }
    }

    /// Lê as listas do Google (duas ou três requisições pequenas).
    func refresh() async {
        loading = true
        defer { loading = false }
        do {
            var out: [String: RemotePackage] = [:]
            var docs: [URL: XMLDocument] = [:]
            for p in Images.all {
                let u = Images.listURL(p)
                if docs[u] == nil {
                    let (data, _) = try await URLSession.shared.data(from: u)
                    docs[u] = try XMLDocument(data: data)
                }
                if let pkg = try Self.parse(docs[u]!, path: p) { out[p] = pkg }
            }
            remote = out
            error = nil
        } catch {
            self.error = "Sem acesso ao repositório do Google: \(error.localizedDescription)"
        }
    }

    nonisolated static func parse(_ doc: XMLDocument, path: String) throws -> RemotePackage? {
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
                                     url: Images.base(path).appendingPathComponent(file),
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
            error = String(format: "Falta espaço: precisa de %.1f GB livres", Double(needed) / 1e9)
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
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/ditto")
        p.arguments = ["-x", "-k", zip.path, tmp.path]
        try p.run()
        p.waitUntilExit()
        guard p.terminationStatus == 0 else { throw InstallError("não extraiu o arquivo") }
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
        // O arquivo some quando este método volta: move para a pasta do Duo.
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
