import Darwin
import Foundation

/// Os tipos de aparelho do Duo (MAC.md §2.5). A imagem padrão é o Android 13
/// (API 33): a mais leve que a fase 0 mediu e a única com Wear OS que liga.
enum DeviceKind: String, CaseIterable, Identifiable, Codable {
    case celular, tablet, dobravel, relogio
    var id: String { rawValue }

    var label: String {
        switch self {
        case .celular: "Celular"
        case .tablet: "Tablet"
        case .dobravel: "Dobrável"
        case .relogio: "Relógio"
        }
    }

    var symbol: String {
        switch self {
        case .celular: "iphone"
        case .tablet: "ipad.landscape"
        case .dobravel: "rectangle.split.2x1"
        case .relogio: "applewatch"
        }
    }

    var image: String {
        self == .relogio ? "system-images;android-33;android-wear;arm64-v8a"
                         : "system-images;android-33;google_apis_playstore;arm64-v8a"
    }

    var hardware: String {
        switch self {
        case .celular: "pixel_9"
        case .tablet: "pixel_tablet"
        case .dobravel: "pixel_9_pro_fold"
        case .relogio: "wearos_large_round"
        }
    }

    /// RAM pedida ao Android e núcleos.
    var ramMB: Int { 2048 }
    var cores: Int { self == .relogio ? 1 : 2 }

    /// Tela reduzida para o tablet: a fase 0 mostrou que a memória cresce com
    /// a resolução (8,5 GB a 2560×1600).
    var lcd: (Int, Int, Int)? { self == .tablet ? (1280, 800, 160) : nil }

    /// Memória estimada do processo, em GB (medido na fase 0; tablet e
    /// dobrável estimados).
    var estimateGB: Double {
        switch self {
        case .celular: 4.5
        case .tablet: 5.5
        case .dobravel: 5.0
        case .relogio: 4.1
        }
    }

    static func guess(device: String, image: String) -> DeviceKind {
        if image.contains("wear") || device.contains("wear") { return .relogio }
        if device.contains("tablet") { return .tablet }
        if device.contains("fold") { return .dobravel }
        return .celular
    }
}

struct DeviceInfo: Identifiable, Hashable {
    var id: String { name }
    let name: String
    let kind: DeviceKind
    let android: String
    let screen: String
    let dir: URL
}

/// Os aparelhos em ~/.android/avd, e as operações que mudam essa pasta.
enum Catalog {
    static func load() -> [DeviceInfo] {
        let fm = FileManager.default
        guard let files = try? fm.contentsOfDirectory(at: SDK.avdHome, includingPropertiesForKeys: nil) else { return [] }
        return files.filter { $0.pathExtension == "ini" }.compactMap { ini in
            let name = ini.deletingPathExtension().lastPathComponent
            let top = readIni(ini)
            let dir = URL(fileURLWithPath: top["path"] ?? SDK.avdHome.appendingPathComponent("\(name).avd").path)
            let cfg = readIni(dir.appendingPathComponent("config.ini"))
            guard !cfg.isEmpty else { return nil }
            let image = cfg["image.sysdir.1"] ?? ""
            let api = image.split(separator: "/").first(where: { $0.hasPrefix("android-") })?.dropFirst(8) ?? "?"
            return DeviceInfo(
                name: name,
                kind: .guess(device: cfg["hw.device.name"] ?? "", image: image),
                android: "API \(api)",
                screen: "\(cfg["hw.lcd.width"] ?? "?")×\(cfg["hw.lcd.height"] ?? "?")",
                dir: dir
            )
        }.sorted { $0.name < $1.name }
    }

    /// Cria um aparelho do tipo pedido. Bloqueante.
    static func create(name: String, kind: DeviceKind) -> String? {
        let r = SDK.run(SDK.avdmanager, ["create", "avd", "-n", name, "-k", kind.image, "-d", kind.hardware], stdin: "no\n")
        guard r.status == 0 else { return r.output.split(separator: "\n").last.map(String.init) ?? "falhou" }
        let cfgURL = SDK.avdHome.appendingPathComponent("\(name).avd/config.ini")
        var text = (try? String(contentsOf: cfgURL, encoding: .utf8)) ?? ""
        func set(_ k: String, _ v: String) {
            if let r = text.range(of: "(?m)^\(NSRegularExpression.escapedPattern(for: k))=.*$", options: .regularExpression) {
                text.replaceSubrange(r, with: "\(k)=\(v)")
            } else { text += "\n\(k)=\(v)" }
        }
        set("hw.ramSize", "\(kind.ramMB)")
        set("hw.cpu.ncore", "\(kind.cores)")
        set("hw.keyboard", "yes")
        if let (w, h, dpi) = kind.lcd { set("hw.lcd.width", "\(w)"); set("hw.lcd.height", "\(h)"); set("hw.lcd.density", "\(dpi)") }
        try? text.write(to: cfgURL, atomically: true, encoding: .utf8)
        return nil
    }

    /// Duplica com clonefile do APFS: instantâneo e sem custo de disco até divergir.
    static func duplicate(_ d: DeviceInfo, as newName: String) -> String? {
        let newDir = SDK.avdHome.appendingPathComponent("\(newName).avd")
        guard clonefile(d.dir.path, newDir.path, 0) == 0 else { return String(cString: strerror(errno)) }
        // Locks e snapshot de execução não podem ir junto.
        for junk in ["hardware-qemu.ini.lock", "multiinstance.lock", "userdata-qemu.img.lock"] {
            try? FileManager.default.removeItem(at: newDir.appendingPathComponent(junk))
        }
        let cfgURL = newDir.appendingPathComponent("config.ini")
        if var cfg = try? String(contentsOf: cfgURL, encoding: .utf8) {
            cfg = cfg.replacingOccurrences(of: "AvdId=\(d.name)", with: "AvdId=\(newName)")
                     .replacingOccurrences(of: "avd.ini.displayname=\(d.name)", with: "avd.ini.displayname=\(newName)")
            try? cfg.write(to: cfgURL, atomically: true, encoding: .utf8)
        }
        var top = (try? String(contentsOf: SDK.avdHome.appendingPathComponent("\(d.name).ini"), encoding: .utf8)) ?? ""
        top = top.replacingOccurrences(of: d.dir.path, with: newDir.path)
                 .replacingOccurrences(of: "avd/\(d.name).avd", with: "avd/\(newName).avd")
        try? top.write(to: SDK.avdHome.appendingPathComponent("\(newName).ini"), atomically: true, encoding: .utf8)
        return nil
    }

    static func delete(_ d: DeviceInfo) -> String? {
        let r = SDK.run(SDK.avdmanager, ["delete", "avd", "-n", d.name])
        return r.status == 0 ? nil : r.output
    }
}
