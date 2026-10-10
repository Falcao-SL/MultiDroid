import Darwin
import Foundation

/// Os tipos de aparelho do MultiDroid (MAC.md §2.5). A imagem padrão é o Android 13
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

    /// Tela (largura, altura, dpi). O tablet sai reduzido: a fase 0 mostrou
    /// que a memória cresce com a resolução (8,5 GB a 2560×1600).
    var lcd: (Int, Int, Int) {
        switch self {
        case .celular: (1080, 2424, 420)
        case .tablet: (1280, 800, 160)
        case .dobravel: (2076, 2152, 390)
        case .relogio: (454, 454, 320)
        }
    }

    var tag: String { self == .relogio ? "android-wear" : "google_apis_playstore" }
    var tagDisplay: String { self == .relogio ? "Wear OS 4" : "Google Play" }
    var imageLabel: String { self == .relogio ? "Wear OS 4 (Android 13)" : "Android 13 com Google Play" }

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

    /// Cria um aparelho escrevendo os dois arquivos que o emulador lê
    /// (<nome>.ini e <nome>.avd/config.ini), com os mesmos campos que o
    /// avdmanager gravaria para esse hardware. O emulador cria o disco do
    /// usuário a partir da imagem no primeiro boot.
    static func create(name: String, kind: DeviceKind) -> String? {
        let fm = FileManager.default
        let dir = SDK.avdHome.appendingPathComponent("\(name).avd")
        let ini = SDK.avdHome.appendingPathComponent("\(name).ini")
        guard !fm.fileExists(atPath: dir.path), !fm.fileExists(atPath: ini.path) else { return "já existe um aparelho com esse nome" }
        guard Images.installed(kind.image) != nil else { return "baixe antes a imagem \(kind.imageLabel) em Imagens" }
        do {
            try fm.createDirectory(at: dir, withIntermediateDirectories: true)
            try """
            avd.ini.encoding=UTF-8
            path=\(dir.path)
            path.rel=avd/\(name).avd
            target=android-33

            """.write(to: ini, atomically: true, encoding: .utf8)
            try config(name: name, kind: kind).write(to: dir.appendingPathComponent("config.ini"), atomically: true, encoding: .utf8)
        } catch {
            try? fm.removeItem(at: dir); try? fm.removeItem(at: ini)
            return error.localizedDescription
        }
        return nil
    }

    static func config(name: String, kind: DeviceKind) -> String {
        let (w, h, dpi) = kind.lcd
        let wear = kind == .relogio
        var c: [(String, String)] = [
            ("avd.ini.encoding", "UTF-8"), ("AvdId", name), ("avd.ini.displayname", name),
            ("abi.type", "arm64-v8a"), ("hw.cpu.arch", "arm64"), ("hw.cpu.ncore", "\(kind.cores)"),
            ("hw.ramSize", "\(kind.ramMB)"), ("vm.heapSize", wear ? "48M" : "228M"),
            ("hw.device.manufacturer", "Google"), ("hw.device.name", kind.hardware),
            ("hw.lcd.width", "\(w)"), ("hw.lcd.height", "\(h)"), ("hw.lcd.density", "\(dpi)"),
            ("hw.lcd.depth", "32"), ("hw.lcd.circular", wear ? "true" : "false"),
            ("hw.initialOrientation", "portrait"), ("hw.keyboard", "yes"), ("hw.mainKeys", wear ? "yes" : "no"),
            ("hw.gpu.enabled", "yes"), ("hw.gpu.mode", "host"),
            ("hw.accelerometer", "yes"), ("hw.gyroscope", "yes"), ("hw.battery", "yes"), ("hw.gps", "yes"),
            ("hw.sensors.proximity", "yes"), ("hw.sensors.light", "yes"), ("hw.sensors.orientation", "yes"),
            ("hw.camera.back", "emulated"), ("hw.camera.front", "none"),
            ("hw.audioInput", "no"), ("hw.audioOutput", "no"), ("hw.sdCard", "no"),
            ("disk.dataPartition.size", wear ? "2G" : "6G"), ("userdata.useQcow2", "no"),
            ("image.sysdir.1", Images.sysdir(kind.image)),
            ("tag.id", kind.tag), ("tag.ids", kind.tag), ("tag.display", kind.tagDisplay), ("tag.displaynames", kind.tagDisplay),
            ("PlayStore.enabled", "no"), ("showDeviceFrame", "no"),
            ("fastboot.forceColdBoot", "no"), ("fastboot.forceFastBoot", "yes"),
            ("firstboot.bootFromLocalSnapshot", "yes"), ("firstboot.saveToLocalSnapshot", "yes"),
        ]
        if kind == .dobravel {
            // Dobradiça do Pixel 9 Pro Fold, como o avdmanager descreve.
            c += [("hw.displayRegion.0.1.width", "1080"), ("hw.displayRegion.0.1.height", "2424"),
                  ("hw.sensor.hinge", "yes"), ("hw.sensor.hinge.count", "1"), ("hw.sensor.hinge.type", "1"),
                  ("hw.sensor.hinge.sub_type", "1"), ("hw.sensor.hinge.ranges", "0-180"),
                  ("hw.sensor.hinge.defaults", "180"), ("hw.sensor.hinge.areas", "1038-0-0-2152"),
                  ("hw.sensor.hinge_angles_posture_definitions", "0-30, 30-150, 150-180"),
                  ("hw.sensor.posture_list", "1, 2, 3"),
                  ("hw.sensor.hinge.fold_to_displayRegion.0.1_at_posture", "1"),
                  ("hw.sensor.hinge.resizable.config", "1")]
        }
        return c.map { "\($0.0)=\($0.1)" }.joined(separator: "\n") + "\n"
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
        do {
            try FileManager.default.removeItem(at: d.dir)
            try? FileManager.default.removeItem(at: SDK.avdHome.appendingPathComponent("\(d.name).ini"))
            return nil
        } catch { return error.localizedDescription }
    }
}

extension Catalog {
    /// Aparelhos na pasta do Android Studio (~/.android/avd).
    static func studioDevices() -> [String] {
        let files = (try? FileManager.default.contentsOfDirectory(at: SDK.studioAvdHome, includingPropertiesForKeys: nil)) ?? []
        return files.filter { $0.pathExtension == "ini" }.map { $0.deletingPathExtension().lastPathComponent }.sorted()
    }

    /// A imagem que um aparelho do Android Studio usa, no formato de pacote.
    static func studioImage(_ name: String) -> String? {
        let top = readIni(SDK.studioAvdHome.appendingPathComponent("\(name).ini"))
        let dir = URL(fileURLWithPath: top["path"] ?? SDK.studioAvdHome.appendingPathComponent("\(name).avd").path)
        guard let sys = readIni(dir.appendingPathComponent("config.ini"))["image.sysdir.1"] else { return nil }
        return sys.split(separator: "/").joined(separator: ";")
    }

    /// Traz um aparelho do Android Studio para a pasta do MultiDroid (move, mesmo
    /// volume: instantâneo). Leva junto o estado salvo e os apps instalados.
    static func importFromStudio(_ name: String) -> String? {
        let fm = FileManager.default
        let top = readIni(SDK.studioAvdHome.appendingPathComponent("\(name).ini"))
        let src = URL(fileURLWithPath: top["path"] ?? SDK.studioAvdHome.appendingPathComponent("\(name).avd").path)
        let dst = SDK.avdHome.appendingPathComponent("\(name).avd")
        guard !fm.fileExists(atPath: dst.path) else { return "já existe \(name) no MultiDroid" }
        do {
            try fm.moveItem(at: src, to: dst)
            for junk in ["hardware-qemu.ini.lock", "multiinstance.lock", "userdata-qemu.img.lock"] {
                try? fm.removeItem(at: dst.appendingPathComponent(junk))
            }
            try """
            avd.ini.encoding=UTF-8
            path=\(dst.path)
            path.rel=avd/\(name).avd
            target=\(top["target"] ?? "android-33")

            """.write(to: SDK.avdHome.appendingPathComponent("\(name).ini"), atomically: true, encoding: .utf8)
            try? fm.removeItem(at: SDK.studioAvdHome.appendingPathComponent("\(name).ini"))
            return nil
        } catch { return error.localizedDescription }
    }

    /// Está rodando fora do MultiDroid? (processo do emulador com -avd <nome>)
    static func isRunning(_ name: String) -> Bool {
        let ps = SDK.run(URL(fileURLWithPath: "/bin/ps"), ["-axo", "args="]).output
        return ps.split(separator: "\n").contains { $0.contains("qemu-system") && $0.contains("-avd \(name) ") }
    }
}
