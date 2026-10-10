import Darwin
import Foundation

/// Onde moram o emulador, as imagens e os aparelhos. Tudo numa pasta do MultiDroid
/// (~/Library/Application Support/MultiDroid): o MultiDroid baixa o que precisa direto do
/// repositório do Google e não usa Android Studio, Java, avdmanager nem adb.
/// O emulador só exige que exista a pasta platform-tools, mesmo vazia.
enum SDK {
    static let home: URL = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("MultiDroid", isDirectory: true)
    /// A pasta das versões chamadas Duo (até 2026-10-10).
    static let legacyHome: URL = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("Duo", isDirectory: true)
    static let root = home.appendingPathComponent("sdk", isDirectory: true)
    static let avdHome = home.appendingPathComponent("avd", isDirectory: true)
    static let emulator = root.appendingPathComponent("emulator/emulator")
    static let licenses = root.appendingPathComponent("licenses", isDirectory: true)

    /// O SDK do Android Studio, se houver: só para importar imagens (clonefile,
    /// sem baixar nem ocupar disco) e aparelhos antigos. O MultiDroid não depende dele.
    static let studioRoot = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Android/sdk")
    static let studioAvdHome = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".android/avd")

    /// Onde o emulador escreve porta e token gRPC de cada instância (pid_<pid>.ini).
    static let discovery = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Library/Caches/TemporaryItems/avd/running")

    /// Uma vez só: a pasta da época do nome Duo vira a do MultiDroid (mesmo
    /// volume: instantâneo, sem copiar o SDK) e os <nome>.ini dos aparelhos
    /// passam a apontar para o caminho novo. Não mexe em nada se algum
    /// emulador ainda roda a partir da pasta antiga.
    static func migrateLegacyHome() {
        let fm = FileManager.default
        guard fm.fileExists(atPath: legacyHome.path), !fm.fileExists(atPath: home.path) else { return }
        guard !run(URL(fileURLWithPath: "/bin/ps"), ["-axo", "args="]).output.contains(legacyHome.path) else { return }
        do { try fm.moveItem(at: legacyHome, to: home) } catch { return }
        let avd = home.appendingPathComponent("avd", isDirectory: true)
        for ini in (try? fm.contentsOfDirectory(at: avd, includingPropertiesForKeys: nil)) ?? [] where ini.pathExtension == "ini" {
            guard let text = try? String(contentsOf: ini, encoding: .utf8) else { continue }
            try? text.replacingOccurrences(of: legacyHome.path, with: home.path).write(to: ini, atomically: true, encoding: .utf8)
        }
    }

    static func prepare() {
        let fm = FileManager.default
        for dir in [root, avdHome, licenses, root.appendingPathComponent("platform-tools")] {
            try? fm.createDirectory(at: dir, withIntermediateDirectories: true)
        }
    }

    /// Ambiente do emulador: aponta para as pastas do MultiDroid e ignora as do Android Studio.
    static var environment: [String: String] {
        var env = ProcessInfo.processInfo.environment
        env["ANDROID_HOME"] = nil
        env["ANDROID_SDK_ROOT"] = root.path
        env["ANDROID_AVD_HOME"] = avdHome.path
        env["ANDROID_EMULATOR_HOME"] = home.path
        return env
    }

    /// Manda um comando ao console do emulador (TCP local, com o token de
    /// ~/.emulator_console_auth_token). É o que "adb emu" faz por baixo.
    /// "kill" desliga salvando o estado: a próxima partida leva segundos.
    static func console(port: Int, _ command: String) async -> Bool {
        await Task.detached {
            let tokenURL = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".emulator_console_auth_token")
            let token = (try? String(contentsOf: tokenURL, encoding: .utf8))?.trimmingCharacters(in: .whitespacesAndNewlines)
            let fd = socket(AF_INET, SOCK_STREAM, 0)
            guard fd >= 0 else { return false }
            defer { close(fd) }
            var tv = timeval(tv_sec: 3, tv_usec: 0)
            setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, socklen_t(MemoryLayout<timeval>.size))
            var addr = sockaddr_in()
            addr.sin_family = sa_family_t(AF_INET)
            addr.sin_port = in_port_t(UInt16(port).bigEndian)
            addr.sin_addr.s_addr = inet_addr("127.0.0.1")
            let ok = withUnsafePointer(to: &addr) {
                $0.withMemoryRebound(to: sockaddr.self, capacity: 1) { connect(fd, $0, socklen_t(MemoryLayout<sockaddr_in>.size)) }
            }
            guard ok == 0 else { return false }
            func readUntilOK() -> String {
                var text = "", buf = [UInt8](repeating: 0, count: 1024)
                while !text.contains("OK\r\n") && !text.contains("KO") {
                    let n = read(fd, &buf, buf.count)
                    if n <= 0 { break }
                    text += String(decoding: buf[0..<n], as: UTF8.self)
                }
                return text
            }
            func send(_ line: String) { _ = line.withCString { write(fd, $0, strlen($0)) } }
            let banner = readUntilOK()
            if banner.contains("Authentication required"), let token {
                send("auth \(token)\r\n")
                guard readUntilOK().contains("OK") else { return false }
            }
            send("\(command)\r\n")
            return readUntilOK().contains("OK")
        }.value
    }

    /// Roda um comando e devolve a saída (stdout + stderr). Bloqueante: chame fora da main.
    @discardableResult
    static func run(_ tool: URL, _ args: [String], stdin: String? = nil) -> (status: Int32, output: String) {
        let p = Process()
        p.executableURL = tool
        p.arguments = args
        let out = Pipe()
        p.standardOutput = out
        p.standardError = out
        if let stdin {
            let inPipe = Pipe()
            p.standardInput = inPipe
            inPipe.fileHandleForWriting.write(Data(stdin.utf8))
            try? inPipe.fileHandleForWriting.close()
        }
        do { try p.run() } catch { return (-1, "\(error)") }
        let data = out.fileHandleForReading.readDataToEndOfFile()
        p.waitUntilExit()
        return (p.terminationStatus, String(decoding: data, as: UTF8.self))
    }
}

/// Lê um .ini do emulador (chave=valor ou chave = valor).
func readIni(_ url: URL) -> [String: String] {
    guard let text = try? String(contentsOf: url, encoding: .utf8) else { return [:] }
    var out: [String: String] = [:]
    for line in text.split(separator: "\n") {
        guard let eq = line.firstIndex(of: "=") else { continue }
        let k = line[..<eq].trimmingCharacters(in: .whitespaces)
        let v = line[line.index(after: eq)...].trimmingCharacters(in: .whitespaces)
        out[k] = v
    }
    return out
}
