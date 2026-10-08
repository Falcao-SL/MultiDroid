import Foundation

/// Onde moram o SDK do Android e os aparelhos (AVDs).
enum SDK {
    static let root: URL = {
        if let env = ProcessInfo.processInfo.environment["ANDROID_SDK_ROOT"] { return URL(fileURLWithPath: env) }
        return FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Android/sdk")
    }()
    static let emulator = root.appendingPathComponent("emulator/emulator")
    static let adb = root.appendingPathComponent("platform-tools/adb")
    static let avdmanager = root.appendingPathComponent("cmdline-tools/latest/bin/avdmanager")
    static let avdHome = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".android/avd")
    /// Onde o emulador escreve porta e token gRPC de cada instância (pid_<pid>.ini).
    static let discovery = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Library/Caches/TemporaryItems/avd/running")

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
