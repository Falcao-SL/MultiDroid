import CryptoKit
import Foundation
import Observation

/// Atualização do MultiDroid estável (canal `stable` do feed do site).
/// Só baixa e instala quando o usuário aperta o botão.
@MainActor
@Observable
final class AppUpdate {
    enum Phase: Equatable {
        case idle
        case checking
        case available(version: String, tag: String)
        case downloading(version: String, fraction: Double)
        case installing(version: String)
        case failed(String)
    }

    private(set) var phase: Phase = .idle
    private var pending: Pending?
    private var task: Task<Void, Never>?

    private struct Pending {
        var version: String
        var tag: String
        var assetURL: URL
        var assetName: String
        var size: Int64
        var sha256: String?
    }

    private struct Feed: Decodable {
        var stable: Channel?
        var preview: Channel?
    }

    private struct Channel: Decodable {
        var tag: String
        var url: String
        var assets: [Asset]
    }

    private struct Asset: Decodable {
        var name: String
        var url: String
        var size: Int64?
    }

    static let feedURL = URL(string: "https://multidroid.falcaosl.com/releases.json")!
    static let feedFallback = URL(string: "https://raw.githubusercontent.com/Falcao-SL/MultiDroid/main/site/releases.json")!

    var bannerText: String? {
        switch phase {
        case .available(let v, _): return "Atualização \(v) disponível"
        case .downloading(let v, let f):
            return "Baixando \(v)… \(Int((f * 100).rounded()))%"
        case .installing(let v): return "Instalando \(v)…"
        case .failed(let e): return e
        default: return nil
        }
    }

    var showButton: Bool {
        switch phase {
        case .available, .failed: return true
        default: return false
        }
    }

    var buttonLabel: String {
        if case .failed = phase { return "Tentar de novo" }
        return "Atualizar"
    }

    var isBusy: Bool {
        switch phase {
        case .checking, .downloading, .installing: return true
        default: return false
        }
    }

    /// Versão instalada: Info.plist do .app, ou `mac/VERSION` embutido no debug.
    static var installedVersion: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
            ?? "0.0"
    }

    func start() {
        guard task == nil else { return }
        check()
        task = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(6 * 3600))
                self?.check()
            }
        }
    }

    func check() {
        // Não interrompe download/instalação; recheck em segundo plano
        // preserva o cartão "disponível"/"falhou" se a rede falhar.
        switch phase {
        case .checking, .downloading, .installing: return
        case .idle: phase = .checking
        case .available, .failed: break
        }
        Task { await self.refresh() }
    }

    func install() {
        guard let pending else {
            check()
            return
        }
        Task { await self.downloadAndInstall(pending) }
    }

    private func refresh() async {
        let prior = phase
        do {
            let feed = try await Self.fetchFeed()
            guard let stable = feed.stable else {
                if case .checking = prior { phase = .idle }
                return
            }
            let remote = stable.tag.hasPrefix("v") ? String(stable.tag.dropFirst()) : stable.tag
            guard Self.remoteIsNewer(remote, than: Self.installedVersion) else {
                phase = .idle
                pending = nil
                return
            }
            guard let asset = Self.pickStableDMG(stable.assets),
                  let url = URL(string: asset.url), url.scheme == "https"
            else {
                phase = .failed("Não há instalador Mac nesta release.")
                return
            }
            let sha = try? await Self.fetchSHA256(tag: stable.tag, asset: asset.name)
            pending = Pending(
                version: remote, tag: stable.tag, assetURL: url,
                assetName: asset.name, size: asset.size ?? 0, sha256: sha
            )
            phase = .available(version: remote, tag: stable.tag)
        } catch {
            // Silencioso no arranque; mantém cartão se já havia oferta ou erro.
            switch prior {
            case .available, .failed: phase = prior
            default: phase = .idle
            }
        }
    }

    private func downloadAndInstall(_ pending: Pending) async {
        phase = .downloading(version: pending.version, fraction: 0)
        let dir = SDK.home.appendingPathComponent("updates", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let dest = dir.appendingPathComponent(pending.assetName)
        try? FileManager.default.removeItem(at: dest)

        do {
            try await Self.download(pending.assetURL, to: dest, expected: pending.size) { [weak self] frac in
                Task { @MainActor in
                    self?.phase = .downloading(version: pending.version, fraction: frac)
                }
            }
            if let want = pending.sha256 {
                let got = try Self.fileSHA256(dest)
                guard got == want else {
                    try? FileManager.default.removeItem(at: dest)
                    phase = .failed("O arquivo baixado não confere com o SHA-256 publicado.")
                    return
                }
            }
            phase = .installing(version: pending.version)
            try await Self.installDMG(dest)
            // installDMG relança e encerra; se voltar, algo falhou sem throw.
            phase = .failed("Instalou, mas não conseguiu reabrir. Abra o MultiDroid em Aplicativos.")
        } catch {
            phase = .failed(error.localizedDescription)
        }
    }

    // MARK: - Rede

    private static func fetchFeed() async throws -> Feed {
        for url in [feedURL, feedFallback] {
            do {
                var req = URLRequest(url: url)
                req.timeoutInterval = 20
                let (data, resp) = try await URLSession.shared.data(for: req)
                guard let http = resp as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
                    continue
                }
                return try JSONDecoder().decode(Feed.self, from: data)
            } catch {
                continue
            }
        }
        throw URLError(.cannotConnectToHost)
    }

    private static func pickStableDMG(_ assets: [Asset]) -> Asset? {
        assets.first {
            $0.name.hasPrefix("MultiDroid-")
                && $0.name.hasSuffix(".dmg")
                && !$0.name.contains("Preview")
                && !$0.name.contains("QA")
        }
    }

    private static func fetchSHA256(tag: String, asset: String) async throws -> String? {
        let url = URL(string: "https://github.com/Falcao-SL/MultiDroid/releases/download/\(tag)/SHA256SUMS.txt")!
        let (data, resp) = try await URLSession.shared.data(from: url)
        guard let http = resp as? HTTPURLResponse, (200..<300).contains(http.statusCode),
              let text = String(data: data, encoding: .utf8)
        else { return nil }
        for line in text.split(whereSeparator: \.isNewline) {
            let parts = line.split(whereSeparator: \.isWhitespace)
            guard parts.count >= 2 else { continue }
            let digest = String(parts[0]).lowercased()
            let name = String(parts[1]).trimmingCharacters(in: CharacterSet(charactersIn: "*"))
            if name == asset, digest.count == 64, digest.allSatisfy(\.isHexDigit) {
                return digest
            }
        }
        return nil
    }

    private static func download(
        _ url: URL, to dest: URL, expected: Int64, progress: @escaping @Sendable (Double) -> Void
    ) async throws {
        let (bytes, resp) = try await URLSession.shared.bytes(from: url)
        guard let http = resp as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw URLError(.badServerResponse)
        }
        FileManager.default.createFile(atPath: dest.path, contents: nil)
        let handle = try FileHandle(forWritingTo: dest)
        defer { try? handle.close() }
        let total = expected > 0 ? expected : http.expectedContentLength
        var done: Int64 = 0
        var buf = Data()
        buf.reserveCapacity(1 << 20)
        for try await b in bytes {
            buf.append(b)
            if buf.count >= 1 << 20 {
                try handle.write(contentsOf: buf)
                done += Int64(buf.count)
                buf.removeAll(keepingCapacity: true)
                if total > 0 { progress(min(1, Double(done) / Double(total))) }
            }
        }
        if !buf.isEmpty {
            try handle.write(contentsOf: buf)
            done += Int64(buf.count)
        }
        progress(1)
    }

    private static func fileSHA256(_ url: URL) throws -> String {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        var hasher = SHA256()
        while true {
            let chunk = try handle.read(upToCount: 1 << 20) ?? Data()
            if chunk.isEmpty { break }
            hasher.update(data: chunk)
        }
        return hasher.finalize().map { String(format: "%02x", $0) }.joined()
    }

    // MARK: - Versão

    /// `remote` é mais novo que `current`?
    static func remoteIsNewer(_ remote: String, than current: String) -> Bool {
        guard let c = parse(current), let r = parse(remote) else {
            return current != remote && !remote.isEmpty
        }
        return r > c
    }

    private struct Ver: Comparable {
        var major, minor, patch: Int
        var pre: String?
        static func < (a: Ver, b: Ver) -> Bool {
            if (a.major, a.minor, a.patch) != (b.major, b.minor, b.patch) {
                return (a.major, a.minor, a.patch) < (b.major, b.minor, b.patch)
            }
            switch (a.pre, b.pre) {
            case (nil, nil): return false
            case (nil, _?): return false
            case (_?, nil): return true
            case let (x?, y?): return x.compare(y, options: .numeric) == .orderedAscending
            }
        }
    }

    private static func parse(_ s: String) -> Ver? {
        var s = s.trimmingCharacters(in: .whitespaces)
        if s.hasPrefix("v") { s.removeFirst() }
        let pre: String?
        let num: String
        if let i = s.firstIndex(of: "-") {
            num = String(s[..<i])
            pre = String(s[s.index(after: i)...]).lowercased()
        } else {
            num = s
            pre = nil
        }
        let parts = num.split(separator: ".").compactMap { Int($0) }
        guard let major = parts.first else { return nil }
        return Ver(
            major: major,
            minor: parts.count > 1 ? parts[1] : 0,
            patch: parts.count > 2 ? parts[2] : 0,
            pre: pre
        )
    }

    // MARK: - Instalar DMG

    private static func installDMG(_ dmg: URL) async throws {
        let mount = SDK.home.appendingPathComponent("updates/mnt-\(ProcessInfo.processInfo.processIdentifier)")
        try? FileManager.default.removeItem(at: mount)
        try FileManager.default.createDirectory(at: mount, withIntermediateDirectories: true)

        let attach = Process()
        attach.executableURL = URL(fileURLWithPath: "/usr/bin/hdiutil")
        attach.arguments = ["attach", "-nobrowse", "-readonly", "-mountpoint", mount.path, dmg.path]
        try attach.run()
        attach.waitUntilExit()
        guard attach.terminationStatus == 0 else {
            throw NSError(domain: "MultiDroid.Update", code: 1, userInfo: [
                NSLocalizedDescriptionKey: "Não montou o DMG.",
            ])
        }
        defer {
            let detach = Process()
            detach.executableURL = URL(fileURLWithPath: "/usr/bin/hdiutil")
            detach.arguments = ["detach", "-quiet", mount.path]
            try? detach.run()
            detach.waitUntilExit()
            try? FileManager.default.removeItem(at: mount)
        }

        let bundled = mount.appendingPathComponent("MultiDroid.app", isDirectory: true)
        guard FileManager.default.fileExists(atPath: bundled.path) else {
            throw NSError(domain: "MultiDroid.Update", code: 2, userInfo: [
                NSLocalizedDescriptionKey: "O DMG não tem um .app.",
            ])
        }

        // A checksum delivered by the same feed is not proof of authorship.
        // Verify the complete code signature and the expected Developer ID
        // before copying or asking for elevated installation privileges.
        let verify = Process()
        verify.executableURL = URL(fileURLWithPath: "/usr/bin/codesign")
        verify.arguments = ["--verify", "--deep", "--strict",
                            "-R=identifier \"com.falcaosl.multidroid\" and anchor apple generic and certificate leaf[subject.OU] = \"6EZ89E2F5S\"",
                            bundled.path]
        try verify.run()
        verify.waitUntilExit()
        guard verify.terminationStatus == 0 else {
            throw NSError(domain: "MultiDroid.Update", code: 5, userInfo: [
                NSLocalizedDescriptionKey: "A assinatura da atualização não corresponde ao desenvolvedor do MultiDroid.",
            ])
        }

        let running = Bundle.main.bundleURL
        let dest: URL = {
            if running.pathExtension == "app",
               running.path.hasPrefix("/Applications") || FileManager.default.isWritableFile(atPath: running.deletingLastPathComponent().path)
            {
                return running
            }
            return URL(fileURLWithPath: "/Applications").appendingPathComponent(bundled.lastPathComponent)
        }()

        let staging = dest.deletingLastPathComponent()
            .appendingPathComponent(".\(dest.deletingPathExtension().lastPathComponent)-update-\(ProcessInfo.processInfo.processIdentifier).app")
        try? FileManager.default.removeItem(at: staging)

        func ditto(_ from: URL, _ to: URL) throws {
            let p = Process()
            p.executableURL = URL(fileURLWithPath: "/usr/bin/ditto")
            p.arguments = [from.path, to.path]
            try p.run()
            p.waitUntilExit()
            guard p.terminationStatus == 0 else {
                throw NSError(domain: "MultiDroid.Update", code: 3, userInfo: [
                    NSLocalizedDescriptionKey: "Não copiou o app.",
                ])
            }
        }

        do {
            try ditto(bundled, staging)
        } catch {
            try await elevatedShell(
                "ditto \(shellQuote(bundled.path)) \(shellQuote(staging.path))"
            )
        }

        let backup = dest.deletingLastPathComponent()
            .appendingPathComponent(".MultiDroid-backup-\(ProcessInfo.processInfo.processIdentifier).app")
        let replace = """
        set -e
        test ! -e \(shellQuote(backup.path))
        if [ -e \(shellQuote(dest.path)) ]; then mv \(shellQuote(dest.path)) \(shellQuote(backup.path)); fi
        if ! mv \(shellQuote(staging.path)) \(shellQuote(dest.path)); then
          if [ -e \(shellQuote(backup.path)) ]; then mv \(shellQuote(backup.path)) \(shellQuote(dest.path)); fi
          exit 1
        fi
        """
        let swap = Process()
        swap.executableURL = URL(fileURLWithPath: "/bin/sh")
        swap.arguments = ["-c", replace]
        try swap.run()
        swap.waitUntilExit()
        if swap.terminationStatus != 0 { try await elevatedShell(replace) }

        let open = Process()
        open.executableURL = URL(fileURLWithPath: "/usr/bin/open")
        open.arguments = ["-n", dest.path]
        try open.run()
        open.waitUntilExit()
        guard open.terminationStatus == 0 else {
            throw NSError(domain: "MultiDroid.Update", code: 6, userInfo: [
                NSLocalizedDescriptionKey: "A atualização foi instalada, mas não foi possível reabrir o MultiDroid.",
            ])
        }
        try? FileManager.default.removeItem(at: backup)
        // Encerra para a cópia nova assumir.
        exit(0)
    }

    private static func shellQuote(_ s: String) -> String {
        "'" + s.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }

    private static func elevatedShell(_ script: String) async throws {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
        let appleScriptString = "\"" + script.replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"") + "\""
        p.arguments = ["-e", "do shell script \(appleScriptString) with administrator privileges"]
        try p.run()
        p.waitUntilExit()
        guard p.terminationStatus == 0 else {
            throw NSError(domain: "MultiDroid.Update", code: 4, userInfo: [
                NSLocalizedDescriptionKey: "Precisa de permissão de administrador para instalar em Aplicativos.",
            ])
        }
    }
}
