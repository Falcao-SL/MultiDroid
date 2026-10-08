import CoreGraphics
import Darwin
import Foundation
import GRPCCore
import GRPCNIOTransportHTTP2
import SwiftProtobuf

typealias EC = Android_Emulation_Control_EmulatorController

/// Conexão gRPC com um emulador (localhost, com token). Tudo o que o Duo pede
/// ao aparelho passa por aqui: status, tela, toque, pausa.
final class EmulatorLink: Sendable {
    let client: GRPCClient<HTTP2ClientTransport.Posix>
    let api: EC.Client<HTTP2ClientTransport.Posix>
    let metadata: Metadata
    private let runner: Task<Void, Never>

    init(port: Int, token: String) throws {
        let transport = try HTTP2ClientTransport.Posix(
            target: .ipv4(address: "127.0.0.1", port: port), transportSecurity: .plaintext)
        let client = GRPCClient(transport: transport)
        self.client = client
        self.api = EC.Client(wrapping: client)
        var md = Metadata()
        md.addString("Bearer \(token)", forKey: "authorization")
        self.metadata = md
        self.runner = Task { try? await client.runConnections() }
    }

    func close() {
        client.beginGracefulShutdown()
        runner.cancel()
    }

    func booted() async -> Bool {
        (try? await api.getStatus(Google_Protobuf_Empty(), metadata: metadata).booted) ?? false
    }

    func setPaused(_ paused: Bool) async {
        var s = Android_Emulation_Control_VmRunState()
        s.state = paused ? .paused : .running
        _ = try? await api.setVmState(s, metadata: metadata)
    }

    /// Toque em coordenadas do aparelho; pressão 0 = dedo levantado.
    func touch(x: Int, y: Int, down: Bool) async {
        var t = Android_Emulation_Control_Touch()
        t.x = Int32(x); t.y = Int32(y); t.pressure = down ? 1 : 0; t.identifier = 0
        var e = Android_Emulation_Control_TouchEvent()
        e.touches = [t]
        _ = try? await api.sendTouch(e, metadata: metadata)
    }
}

/// A tela de um aparelho no tamanho do quadro da grade, entregue por memória
/// compartilhada (MMAP): o emulador escala e escreve os pixels direto num
/// arquivo mapeado; a resposta gRPC vem vazia (MAC.md §2.2).
@MainActor
@Observable
final class ScreenStream {
    private(set) var image: CGImage?
    private(set) var fps: Double = 0
    let width: Int
    let height: Int
    private var task: Task<Void, Never>?
    private let path: String

    init(width: Int, height: Int, tag: String) {
        self.width = width
        self.height = height
        self.path = NSTemporaryDirectory() + "duo-screen-\(tag).raw"
    }

    func start(_ link: EmulatorLink) {
        stop()
        let w = width, h = height, path = path, bytes = width * height * 4
        // Arquivo do tamanho exato de um quadro, mapeado só para leitura aqui.
        FileManager.default.createFile(atPath: path, contents: nil)
        if let fh = FileHandle(forWritingAtPath: path) { try? fh.truncate(atOffset: UInt64(bytes)); try? fh.close() }
        task = Task.detached { [weak self] in
            guard let frame = MappedFrame(path: path, bytes: bytes) else { return }
            var fmt = Android_Emulation_Control_ImageFormat()
            fmt.format = .rgba8888
            fmt.width = UInt32(w)    // largura E altura: só assim o emulador escala
            fmt.height = UInt32(h)
            fmt.transport.channel = .mmap
            fmt.transport.handle = "file://" + path
            _ = try? await link.api.streamScreenshot(fmt, metadata: link.metadata) { response in
                var frames = 0, window = Date()
                for try await img in response.messages {
                    if Task.isCancelled { break }
                    let fw = Int(img.format.width), fh = Int(img.format.height)
                    guard fw > 0, fh > 0, fw * fh * 4 <= bytes else { continue }
                    let data = frame.copy(count: fw * fh * 4)  // cópia: evita rasgar o quadro
                    guard let provider = CGDataProvider(data: data as CFData),
                          let cg = CGImage(width: fw, height: fh, bitsPerComponent: 8, bitsPerPixel: 32,
                                           bytesPerRow: fw * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                           bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipLast.rawValue),
                                           provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent)
                    else { continue }
                    frames += 1
                    let elapsed = Date().timeIntervalSince(window)
                    let rate: Double? = elapsed >= 1 ? Double(frames) / elapsed : nil
                    if elapsed >= 1 { frames = 0; window = Date() }
                    await MainActor.run {
                        self?.image = cg
                        if let rate { self?.fps = rate }
                    }
                }
                return ()
            }
        }
    }

    func stop() {
        task?.cancel()
        task = nil
    }
}

/// O arquivo de quadros mapeado em memória. Só leitura; o emulador escreve.
final class MappedFrame: @unchecked Sendable {
    private let base: UnsafeMutableRawPointer
    private let bytes: Int
    private let fd: Int32

    init?(path: String, bytes: Int) {
        let fd = open(path, O_RDONLY)
        guard fd >= 0 else { return nil }
        guard let base = mmap(nil, bytes, PROT_READ, MAP_SHARED, fd, 0), base != MAP_FAILED else { close(fd); return nil }
        self.base = base; self.bytes = bytes; self.fd = fd
    }

    func copy(count: Int) -> Data { Data(bytes: base, count: min(count, bytes)) }

    deinit { munmap(base, bytes); close(fd) }
}
