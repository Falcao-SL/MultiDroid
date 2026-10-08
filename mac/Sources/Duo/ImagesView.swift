import AppKit
import SwiftUI

/// Imagens e emulador (MAC.md §2.7): o que está instalado na pasta do Duo, o
/// que o Google oferece, baixar, atualizar, remover. A licença do Google
/// aparece antes do primeiro download de cada pacote.
struct ImagesSheet: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State private var license: (path: String, pkg: RemotePackage)?
    @State private var studioDevices: [String] = []
    @State private var importError: String?
    @State private var confirm: (path: String, pkg: RemotePackage, names: [String])?

    var body: some View {
        if let l = license {
            // Licença na própria folha: folha sobre folha falha no macOS.
            LicenseSheet(title: Images.title(l.path), text: l.pkg.licenseText) {
                model.images.accept(l.pkg)
                license = nil
                start(l.path, l.pkg)
            } onDecline: { license = nil }
        } else {
            list
        }
    }

    private var list: some View {
        let store = model.images
        return VStack(alignment: .leading, spacing: 16) {
            HStack(alignment: .firstTextBaseline) {
                Text("Imagens").font(.system(size: 22, weight: .black))
                Spacer()
                if store.loading { ProgressView().controlSize(.small) }
                Button { Task { await store.refresh() } } label: { Image(systemName: "arrow.clockwise").frame(width: 14, height: 14) }
                    .buttonStyle(BrutalIconStyle()).help("Procurar versões novas")
            }
            Text("O Duo baixa o emulador e as imagens do Android direto do Google (dl.google.com) para a própria pasta. Não precisa de Android Studio, Java nem adb.")
                .font(.system(size: 12, weight: .medium)).opacity(0.75)
                .fixedSize(horizontal: false, vertical: true)

            VStack(spacing: 10) {
                ForEach(Images.all, id: \.self) { path in row(path) }
            }

            if !studioDevices.isEmpty {
                Text("APARELHOS DO ANDROID STUDIO").font(.system(size: 12, weight: .black)).kerning(1).padding(.top, 4)
                Text("Trazer move o aparelho para a pasta do Duo, com apps e dados. Ele some do Android Studio.")
                    .font(.system(size: 11, weight: .medium)).opacity(0.7)
                ForEach(studioDevices, id: \.self) { name in studioRow(name) }
            }

            if let e = store.error ?? importError {
                Text(e).font(.system(size: 12, weight: .semibold)).foregroundStyle(Duo.vermelho)
                    .fixedSize(horizontal: false, vertical: true)
            }
            HStack {
                Button("Abrir pasta") { NSWorkspace.shared.open(SDK.home) }.buttonStyle(BrutalButtonStyle(fill: .white))
                Text(diskUse()).font(.system(size: 11, weight: .semibold)).opacity(0.6)
                Spacer()
                Button("Fechar") { dismiss() }.buttonStyle(BrutalButtonStyle())
            }
        }
        .padding(24)
        .frame(width: 560)
        .task {
            store.scan()
            studioDevices = Catalog.studioDevices()
            if store.remote.isEmpty { await store.refresh() }
        }
        .alert("Desligar para atualizar?", isPresented: Binding(get: { confirm != nil }, set: { if !$0 { confirm = nil } })) {
            Button("Cancelar", role: .cancel) { confirm = nil }
            Button("Atualizar") {
                if let c = confirm { model.images.install(c.path) }
                confirm = nil
            }
        } message: {
            Text("O download corre com tudo ligado. Na hora de trocar, o Duo desliga \(confirm?.names.joined(separator: ", ") ?? "") com o estado salvo e liga de novo ao terminar.")
        }
    }

    private func row(_ path: String) -> some View {
        let store = model.images
        let have = store.installed[path]
        let pkg = store.remote[path]
        let busy = store.progress[path] != nil
        let inUse = model.devices.contains { $0.info.kind.image == path } || (path == Images.emulator && !model.devices.isEmpty)
        return VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 3) {
                    Text(Images.title(path)).font(.system(size: 14, weight: .heavy)).fixedSize(horizontal: false, vertical: true)
                    Text(Images.detail(path)).font(.system(size: 11, weight: .medium)).opacity(0.7)
                    Text(status(have: have, pkg: pkg)).font(.system(size: 11, weight: .bold).monospacedDigit()).opacity(0.85)
                        .fixedSize(horizontal: false, vertical: true)
                }
                .layoutPriority(1)
                Spacer()
                if busy {
                    Button("Cancelar") { store.cancel(path) }.buttonStyle(BrutalButtonStyle(fill: .white))
                } else {
                    if have == nil, store.studio[path] != nil {
                        Button("Copiar do Studio") { store.importFromStudio(path) }
                            .buttonStyle(BrutalButtonStyle(fill: .white))
                            .help("Cópia instantânea (clonefile) do que o Android Studio já baixou; não ocupa disco extra")
                    }
                    if let pkg, have == nil || newer(pkg.revision, than: have!) {
                        Button(have == nil ? "Baixar" : "Atualizar") { download(path, pkg) }
                            .buttonStyle(BrutalButtonStyle(fill: Duo.verde))
                    }
                    if have != nil {
                        Button { store.remove(path) } label: { Image(systemName: "trash").frame(width: 14, height: 14) }
                            .buttonStyle(BrutalIconStyle(fill: .white))
                            .disabled(inUse)
                            .help(inUse ? "Em uso por aparelhos do Duo" : "Remover")
                    }
                }
            }
            if let f = store.progress[path] {
                HStack(spacing: 8) {
                    ProgressView(value: store.phase[path] == "baixando" ? f : nil)
                    Text(store.phase[path] == "baixando" ? "\(Int(f * 100))%" : (store.phase[path] ?? ""))
                        .font(.system(size: 11, weight: .bold).monospacedDigit()).frame(width: 80, alignment: .trailing)
                }
            }
        }
        .padding(12)
        .brutal(fill: Color(nsColor: .controlBackgroundColor))
    }

    private func studioRow(_ name: String) -> some View {
        let img = Catalog.studioImage(name)
        let supported = img.map { Images.all.contains($0) } ?? false
        let ready = img.map { model.images.installed[$0] != nil } ?? false
        return HStack {
            Text(name).font(.system(size: 13, weight: .bold))
            Text(img.map { Images.title($0) } ?? "?").font(.system(size: 11, weight: .medium)).opacity(0.7)
            Spacer()
            Button("Trazer") {
                if Catalog.isRunning(name) { importError = "Desligue \(name) antes de trazer"; return }
                importError = Catalog.importFromStudio(name)
                studioDevices = Catalog.studioDevices()
                model.refresh()
            }
            .buttonStyle(BrutalButtonStyle(fill: .white))
            .disabled(!supported || !ready)
            .help(!supported ? "Imagem que o Duo ainda não gerencia" : !ready ? "Instale antes a imagem que ele usa" : "")
        }
    }

    private func status(have: String?, pkg: RemotePackage?) -> String {
        let size = pkg.map { String(format: " · %.1f GB", Double($0.size) / 1e9) } ?? ""
        switch (have, pkg) {
        case let (h?, p?) where newer(p.revision, than: h): return "instalado \(h) · nova \(p.revision)\(size)"
        case let (h?, _): return "instalado \(h)"
        case let (nil, p?): return "não instalado · \(p.revision)\(size)"
        default: return "não instalado"
        }
    }

    private func newer(_ a: String, than b: String) -> Bool {
        let x = a.split(separator: ".").compactMap { Int($0) }, y = b.split(separator: ".").compactMap { Int($0) }
        for i in 0..<max(x.count, y.count) {
            let p = i < x.count ? x[i] : 0, q = i < y.count ? y[i] : 0
            if p != q { return p > q }
        }
        return false
    }

    private func download(_ path: String, _ pkg: RemotePackage) {
        if model.images.accepted(pkg) { start(path, pkg) } else { license = (path, pkg) }
    }

    /// Se há aparelhos usando o pacote, avisa antes que eles vão ser desligados.
    private func start(_ path: String, _ pkg: RemotePackage) {
        let names = model.users(of: path).map(\.id)
        if names.isEmpty { model.images.install(path) } else { confirm = (path, pkg, names) }
    }

    private func diskUse() -> String {
        let fm = FileManager.default
        var total: Int64 = 0
        if let e = fm.enumerator(at: SDK.root, includingPropertiesForKeys: [.totalFileAllocatedSizeKey]) {
            for case let u as URL in e {
                total += Int64((try? u.resourceValues(forKeys: [.totalFileAllocatedSizeKey]).totalFileAllocatedSize) ?? 0)
            }
        }
        return String(format: "%.1f GB na pasta", Double(total) / 1e9)
    }
}

struct LicenseSheet: View {
    let title: String
    let text: String
    let onAccept: () -> Void
    let onDecline: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Licença do Google").font(.system(size: 20, weight: .black))
            Text("Para baixar \(title), aceite os termos do Google. O Duo só baixa depois do seu aceite e guarda o registro na pasta dele.")
                .font(.system(size: 12, weight: .medium)).opacity(0.8)
                .fixedSize(horizontal: false, vertical: true)
            ScrollView {
                Text(text).font(.system(size: 11)).textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading)
                    .padding(10)
            }
            .frame(height: 300)
            .brutal(fill: Color(nsColor: .textBackgroundColor))
            HStack {
                Spacer()
                Button("Recusar", action: onDecline).buttonStyle(BrutalButtonStyle(fill: .white))
                Button("Aceito", action: onAccept).buttonStyle(BrutalButtonStyle(fill: Duo.verde))
            }
        }
        .padding(24)
        .frame(width: 600)
    }
}
