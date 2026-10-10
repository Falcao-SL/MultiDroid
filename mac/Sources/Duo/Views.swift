import SwiftUI

struct RootView: View {
    @Environment(AppModel.self) private var model
    @Environment(DuoWindow.self) private var win
    @Environment(\.colorScheme) private var scheme

    var body: some View {
        let p = DuoPalette.current(scheme)
        HStack(spacing: 0) {
            if !win.panelHidden {
                PanelView()
                    .frame(width: 340)
                    .transition(.move(edge: .leading))
                Rectangle().fill(p.borda).frame(width: Duo.borda)
            }
            GridArea()
        }
        .background(p.fundo)
        .sheet(isPresented: Bindable(win).creating) { NewDeviceSheet().environment(win) }
        .sheet(isPresented: Bindable(win).showImages) { ImagesSheet() }
        .overlay(alignment: .bottom) {
            if let b = model.banner {
                Text(b)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(Duo.tintaSobreAcento)
                    .padding(12)
                    .brutal(fill: Duo.amarelo)
                    .padding(20)
                    .onTapGesture { model.banner = nil }
                    .task(id: b) { try? await Task.sleep(for: .seconds(5)); model.banner = nil }
            }
        }
    }
}

// MARK: - Painel

struct PanelView: View {
    @Environment(AppModel.self) private var model
    @Environment(DuoWindow.self) private var win
    @Environment(\.colorScheme) private var scheme
    @State private var selection = Set<String>()
    @State private var confirmDelete: Device?
    @State private var newGroupFor: [String]?
    @State private var groupName = ""

    var body: some View {
        @Bindable var model = model
        let p = DuoPalette.current(scheme)
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                Text("Duo").font(.system(size: 26, weight: .black))
                Spacer()
                Button { win.showImages = true } label: { Image(systemName: "square.stack.3d.down.right").frame(width: 16, height: 16) }
                    .buttonStyle(BrutalIconStyle(fill: model.images.ready ? .white : Duo.amarelo))
                    .help("Imagens e emulador")
                Button { win.creating = true } label: { Label("Novo aparelho", systemImage: "plus") }
                    .buttonStyle(BrutalButtonStyle())
            }
            MemoryCard()
            if !selection.isEmpty { batchBar }
            ScrollView {
                VStack(spacing: 12) {
                    ForEach(win.devices) { d in
                        DeviceRow(device: d, selected: selection.contains(d.id))
                            .onTapGesture {
                                if NSEvent.modifierFlags.contains(.command) {
                                    if selection.contains(d.id) { selection.remove(d.id) } else { selection.insert(d.id) }
                                } else { selection = [d.id] }
                            }
                            .contextMenu { menu(for: d) }
                    }
                }
                .padding(.trailing, 6).padding(.bottom, 8)
            }
            HStack {
                Text("Teto de memória").font(.system(size: 12, weight: .bold))
                Slider(value: $model.ceilingGB, in: 4...Metrics.physicalGB.rounded(), step: 1) { _ in model.save() }
                Text("\(Int(model.ceilingGB)) GB").font(.system(size: 12, weight: .bold).monospacedDigit())
            }
        }
        .padding(18)
        .foregroundStyle(p.tinta)
        .alert("Apagar \(confirmDelete?.id ?? "")?", isPresented: .constant(confirmDelete != nil), presenting: confirmDelete) { d in
            Button("Cancelar", role: .cancel) { confirmDelete = nil }
                .keyboardShortcut(.defaultAction)
            Button("Apagar", role: .destructive) { model.delete(d); confirmDelete = nil }
        } message: { _ in Text("O aparelho e tudo o que está nele (contas, apps, fotos) somem deste Mac.") }
        .alert("Novo grupo", isPresented: .constant(newGroupFor != nil)) {
            TextField("Nome do grupo", text: $groupName)
            Button("Criar") { model.setGroup(groupName, for: newGroupFor ?? []); newGroupFor = nil; groupName = "" }
            Button("Cancelar", role: .cancel) { newGroupFor = nil; groupName = "" }
        }
    }

    private var batchBar: some View {
        HStack(spacing: 8) {
            Text("\(selection.count) selecionados").font(.system(size: 12, weight: .bold))
            Spacer()
            Button("Ligar") { for n in selection { if let d = model.device(n) { model.start(d) } } }
            Button("Pausar") { for n in selection { if let d = model.device(n) { model.pause(d) } } }
            Button("Hibernar") { for n in selection { if let d = model.device(n) { model.hibernate(d) } } }
            Button("Desligar") { for n in selection { if let d = model.device(n) { model.stop(d) } } }
        }
        .buttonStyle(BrutalButtonStyle(fill: Duo.ciano))
    }

    @ViewBuilder
    private func menu(for d: Device) -> some View {
        let names = selection.contains(d.id) ? Array(selection) : [d.id]
        Button("Duplicar") { model.duplicate(d) }.disabled(d.isRunning)
        Menu("Grupo") {
            ForEach(win.groupNames, id: \.self) { g in Button(g) { model.setGroup(g, for: names) } }
            Divider()
            Button("Novo grupo…") { newGroupFor = names }
            Button("Sem grupo") { model.setGroup(nil, for: names) }
        }
        Divider()
        Button("Apagar…", role: .destructive) { confirmDelete = d }.disabled(d.isRunning)
    }
}

struct MemoryCard: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        let total = Metrics.physicalGB
        let used = max(0, total - model.availableGB)
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("Memória do Mac").font(.system(size: 12, weight: .bold))
                Spacer()
                Text(String(format: "%.1f de %.0f GB", used, total)).font(.system(size: 12, weight: .bold).monospacedDigit())
            }
            GeometryReader { geo in
                let w = geo.size.width
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 4).fill(Color.gray.opacity(0.25))
                    RoundedRectangle(cornerRadius: 4).fill(used / total > 0.85 ? Duo.vermelho : Duo.verde)
                        .frame(width: w * min(1, used / total))
                    RoundedRectangle(cornerRadius: 4).fill(Duo.azul)
                        .frame(width: w * min(1, model.runningGB / total))
                    Rectangle().fill(Duo.tintaSobreAcento).frame(width: 2).offset(x: w * min(1, model.ceilingGB / total))
                }
            }
            .frame(height: 14)
            Text(String(format: "Aparelhos: %.1f GB · teto %.0f GB · swap %.1f GB", model.runningGB, model.ceilingGB, model.swapGB)
                 + (model.asleepCount > 0 ? " · \(model.asleepCount) hibernando" : ""))
                .font(.system(size: 11, weight: .semibold)).opacity(0.7)
        }
        .padding(12)
        .brutal()
    }
}

struct StatusDot: View {
    let state: Device.State
    var body: some View {
        let (c, t): (Color, String) = switch state {
        case .off: (.gray, "desligado")
        case .starting: (Duo.amarelo, "ligando")
        case .on: (Duo.verde, "ligado")
        case .paused: (Duo.azul, "pausado")
        case .stopping: (Duo.laranja, "desligando")
        case .external: (Duo.roxo, "aberto fora do Duo")
        case .asleep: (Duo.ciano, "hibernando")
        }
        HStack(spacing: 5) {
            Circle().fill(c).frame(width: 9, height: 9).overlay(Circle().stroke(.black.opacity(0.6), lineWidth: 1))
            Text(t).font(.system(size: 11, weight: .semibold))
        }
    }
}

struct DeviceRow: View {
    @Environment(AppModel.self) private var model
    let device: Device
    let selected: Bool

    var body: some View {
        let d = device
        HStack(spacing: 10) {
            Image(systemName: d.info.kind.symbol)
                .font(.system(size: 18, weight: .bold))
                .frame(width: 34, height: 34)
                .brutal(fill: color(for: d), shadow: false)
                .foregroundStyle(Duo.tintaSobreAcento)
            VStack(alignment: .leading, spacing: 3) {
                Text(d.id).font(.system(size: 14, weight: .heavy)).lineLimit(1)
                HStack(spacing: 6) {
                    StatusDot(state: d.state)
                    if d.isRunning {
                        Text(String(format: "%.1f GB · %.0f%%", d.footprintGB, d.cpu))
                            .font(.system(size: 11, weight: .semibold).monospacedDigit()).opacity(0.7)
                    } else {
                        Text("\(d.info.kind.label) · \(d.info.android)").font(.system(size: 11)).opacity(0.6)
                    }
                }
                if let g = model.groups[d.id] {
                    Text(g.uppercased()).font(.system(size: 9, weight: .black)).kerning(0.8)
                        .padding(.horizontal, 5).padding(.vertical, 2)
                        .brutal(fill: Duo.amarelo, shadow: false)
                        .foregroundStyle(Duo.tintaSobreAcento)
                }
            }
            Spacer()
            controls(d)
        }
        .padding(10)
        .brutal(fill: selected ? Duo.azul.opacity(0.35) : nil)
    }

    private func color(for d: Device) -> Color {
        let i = model.order.firstIndex(of: d.id) ?? 0
        return Duo.ciclo[i % Duo.ciclo.count]
    }

    @ViewBuilder
    private func controls(_ d: Device) -> some View {
        HStack(spacing: 6) {
            switch d.state {
            case .off:
                iconButton("play.fill", Duo.verde) { model.start(d) }
            case .asleep:
                if d.waking { ProgressView().controlSize(.small) } else { iconButton("play.fill", Duo.verde) { model.start(d) } }
            case .on:
                iconButton("pause.fill", Duo.azul) { model.pause(d) }
                iconButton("moon.zzz.fill", Duo.ciano) { model.hibernate(d) }
                iconButton("stop.fill", Duo.vermelho) { model.stop(d) }
            case .paused:
                iconButton("play.fill", Duo.verde) { model.resume(d) }
                iconButton("moon.zzz.fill", Duo.ciano) { model.hibernate(d) }
                iconButton("stop.fill", Duo.vermelho) { model.stop(d) }
            case .external:
                iconButton("stop.fill", Duo.vermelho) { model.stop(d) }
            case .starting, .stopping:
                ProgressView().controlSize(.small)
            }
        }
    }

    private func iconButton(_ symbol: String, _ fill: Color, _ action: @escaping () -> Void) -> some View {
        Button(action: action) { Image(systemName: symbol).frame(width: 14, height: 14) }
            .buttonStyle(BrutalButtonStyle(fill: fill))
    }
}

// MARK: - Grade

struct GridArea: View {
    @Environment(AppModel.self) private var model
    @Environment(DuoWindow.self) private var win
    @State private var hovered: String?

    var body: some View {
        @Bindable var win = win
        VStack(spacing: 0) {
            HStack {
                Button { withAnimation(.snappy) { win.panelHidden.toggle() } } label: {
                    Image(systemName: "sidebar.left").frame(width: 14, height: 14)
                }
                .buttonStyle(BrutalIconStyle(fill: .white))
                .keyboardShortcut("0")
                .help(win.panelHidden ? "Mostrar painel (⌘0)" : "Esconder painel (⌘0)")
                Picker("", selection: $win.layout) {
                    ForEach(AppModel.Layout.allCases) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented)
                .frame(width: 300)
                if win.layout == .grade && win.focused == nil {
                    Button { withAnimation(.snappy) { model.resetSpots(win.devices) } } label: {
                        Image(systemName: "rectangle.3.group").frame(width: 14, height: 14)
                    }
                    .buttonStyle(BrutalIconStyle(fill: .white))
                    .help("Reorganizar a grade")
                }
                if win.focused != nil {
                    Button { win.focused = nil } label: { Label("Voltar à mesa", systemImage: "square.grid.2x2") }
                        .buttonStyle(BrutalButtonStyle(fill: Duo.amarelo))
                }
                Spacer()
                Text(win.layout == .grade ? "Arraste pelo título · redimensione pelo canto" : "Dois cliques amplia · arraste para reordenar").font(.system(size: 11, weight: .semibold)).opacity(0.6)
                    .lineLimit(1)
            }
            .padding(14)
            if win.devices.isEmpty {
                EmptyDesk()
            } else if let f = win.focused, let d = model.device(f) {
                mesa([d])
            } else if win.layout == .grade {
                GridBoard(list: win.devices)
            } else if win.layout == .auto {
                mesa(win.devices)
            } else {
                groups
            }
        }
    }

    /// Por grupo: cada grupo tem a sua mesa, dividindo a altura; com muitos
    /// grupos, cada um fica com pelo menos 320 pt e a área rola.
    private var groups: some View {
        let sections = (win.groupNames + [""]).compactMap { g -> (String, [Device])? in
            let members = win.devices.filter { (model.groups[$0.id] ?? "") == g }
            return members.isEmpty ? nil : (g, members)
        }
        return GeometryReader { geo in
            ScrollView {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(sections, id: \.0) { g, members in
                        Text(g.isEmpty ? "SEM GRUPO" : g.uppercased()).font(.system(size: 13, weight: .black)).kerning(1)
                            .padding(.horizontal, 20)
                        mesa(members)
                            .frame(height: max(320, geo.size.height / CGFloat(sections.count) - 24))
                    }
                }
            }
        }
    }

    private func mesa(_ list: [Device]) -> some View {
        MesaLayout {
            ForEach(list) { d in
                DeviceTile(device: d)
                    .tile(base: d.tileBase, toolbar: d.state == .on || d.state == .paused)
                    .zIndex(hovered == d.id ? 1 : 0)
                    .onHover { inside in
                        if inside { hovered = d.id } else if hovered == d.id { hovered = nil }
                    }
                    .draggable(d.id)
                    .dropDestination(for: String.self) { items, _ in
                        guard let name = items.first else { return false }
                        withAnimation { model.move(name, before: d.id) }
                        return true
                    }
                    .onTapGesture(count: 2) {
                        model.used(d)
                        withAnimation(.snappy) { win.focused = win.focused == d.id ? nil : d.id }
                    }
            }
        }
        .animation(.snappy, value: list.map(\.id))
    }
}

struct DeviceTile: View {
    @Environment(AppModel.self) private var model
    @Environment(\.colorScheme) private var scheme
    let device: Device

    var body: some View {
        let d = device
        let p = DuoPalette.current(scheme)
        let (fw, fh) = d.frameSize
        let round = d.info.kind == .relogio
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 6) {
                Image(systemName: d.info.kind.symbol).font(.system(size: 12, weight: .bold))
                Text(d.id).font(.system(size: 13, weight: .heavy)).lineLimit(1)
                Spacer(minLength: 4)
                if d.state == .on, let s = d.screen { Text(String(format: "%.0f fps", s.fps)).font(.system(size: 10, weight: .bold).monospacedDigit()).opacity(0.6) }
                StatusDot(state: d.state)
            }
            .frame(height: 20)
            HStack(alignment: .top, spacing: 8) {
                ZStack {
                    RoundedRectangle(cornerRadius: round ? 999 : 14).fill(Color.black)
                    if let img = d.screen?.image, d.state == .on || d.state == .paused {
                        ScreenView(device: d, image: img)
                            .clipShape(RoundedRectangle(cornerRadius: round ? 999 : 12))
                            .opacity(d.state == .paused ? 0.45 : 1)
                    } else if let img = d.lastFrame, d.state == .asleep || d.state == .starting {
                        // Hibernado (ou acordando): a última tela, esmaecida; um clique acorda.
                        Image(decorative: img, scale: 1).resizable().interpolation(.high)
                            .clipShape(RoundedRectangle(cornerRadius: round ? 999 : 12))
                            .opacity(0.35)
                            .contentShape(Rectangle())
                            .onTapGesture { model.start(d) }
                        if d.state == .starting || d.waking { ProgressView().controlSize(.large).tint(.white) }
                    } else {
                        VStack(spacing: 10) {
                            Text(placeholder(d)).font(.system(size: 12, weight: .bold)).foregroundStyle(.white.opacity(0.75))
                                .multilineTextAlignment(.center)
                            if d.state == .off {
                                Button("Ligar") { model.start(d) }.buttonStyle(BrutalButtonStyle(fill: Duo.verde))
                            }
                            if d.state == .asleep, !d.waking {
                                Button("Acordar") { model.start(d) }.buttonStyle(BrutalButtonStyle(fill: Duo.ciano))
                            }
                        }
                        .padding()
                    }
                    if d.state == .paused {
                        Button("Retomar") { model.resume(d) }.buttonStyle(BrutalButtonStyle(fill: Duo.verde))
                    }
                    if d.state == .asleep, !d.waking, d.lastFrame != nil {
                        Button("Acordar") { model.start(d) }.buttonStyle(BrutalButtonStyle(fill: Duo.ciano))
                    }
                }
                .aspectRatio(CGFloat(fw) / CGFloat(fh), contentMode: .fit)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                if d.state == .on || d.state == .paused {
                    DeviceToolbar(device: d).frame(width: MesaLayout.toolbarWidth)
                }
            }
        }
        .padding(MesaLayout.pad)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .brutal()
        .foregroundStyle(p.tinta)
    }

    private func placeholder(_ d: Device) -> String {
        switch d.state {
        case .off: "\(d.info.kind.label) · \(d.info.android)\n\(d.info.screen)"
        case .starting: d.message ?? "ligando…"
        case .stopping: "desligando…"
        case .external: "Aberto fora do Duo.\nDesligue e ligue por aqui para ver a tela."
        case .asleep: d.message ?? "Hibernando: não gasta memória.\nVolta em segundos."
        default: ""
        }
    }
}

/// Os botões do aparelho, numa coluna à direita da tela. Cabendo, mostra todos;
/// em quadro baixo, os extras vão para o menu "⋯".
struct DeviceToolbar: View {
    @Environment(AppModel.self) private var model
    let device: Device
    @State private var typing = false
    @State private var text = ""

    var body: some View {
        ViewThatFits(in: .vertical) {
            VStack(spacing: 6) {
                nav
                Spacer(minLength: 4).frame(maxHeight: 10)
                tool("speaker.plus", "Volume +") { await $0.key("AudioVolumeUp") }
                tool("speaker.minus", "Volume −") { await $0.key("AudioVolumeDown") }
                if device.info.kind != .relogio { tool("rotate.right", "Girar") { await rotate($0) } }
                tool("keyboard", "Digitar") { _ in await MainActor.run { typing = true } }
                tool("camera", "Captura de tela") { await capture($0) }
                Spacer(minLength: 4).frame(maxHeight: 10)
                power
            }
            VStack(spacing: 6) {
                nav
                Menu {
                    Button("Volume +", systemImage: "speaker.plus") { run { await $0.key("AudioVolumeUp") } }
                    Button("Volume −", systemImage: "speaker.minus") { run { await $0.key("AudioVolumeDown") } }
                    if device.info.kind != .relogio { Button("Girar", systemImage: "rotate.right") { run { await rotate($0) } } }
                    Button("Digitar…", systemImage: "keyboard") { typing = true }
                    Button("Captura de tela", systemImage: "camera") { run { await capture($0) } }
                } label: {
                    Image(systemName: "ellipsis").font(.system(size: 12, weight: .bold)).frame(width: 14, height: 14)
                }
                .menuIndicator(.hidden)
                .buttonStyle(BrutalIconStyle(fill: .white))
                .disabled(device.state != .on)
                .help("Mais")
                power
            }
        }
        .popover(isPresented: $typing, arrowEdge: .trailing) {
            HStack {
                TextField("Texto para o aparelho", text: $text).frame(width: 240)
                    .onSubmit { send() }
                Button("Enviar") { send() }.buttonStyle(BrutalButtonStyle())
            }
            .padding(12)
        }
    }

    private var nav: some View {
        VStack(spacing: 6) {
            tool("chevron.backward", "Voltar") { await $0.key("GoBack") }
            tool("circle", "Início") { await $0.key("GoHome") }
            tool("square.on.square", "Apps recentes") { await $0.key("AppSwitch") }
        }
    }

    @ViewBuilder private var power: some View {
        let d = device
        if d.state == .on {
            Button { model.pause(d) } label: { Image(systemName: "pause.fill").frame(width: 14, height: 14) }
                .buttonStyle(BrutalIconStyle(fill: Duo.azul)).help("Pausar")
        } else {
            Button { model.resume(d) } label: { Image(systemName: "play.fill").frame(width: 14, height: 14) }
                .buttonStyle(BrutalIconStyle(fill: Duo.verde)).help("Retomar")
        }
        Button { model.hibernate(d) } label: { Image(systemName: "moon.zzz.fill").frame(width: 14, height: 14) }
            .buttonStyle(BrutalIconStyle(fill: Duo.ciano)).help("Hibernar: libera a memória e guarda o estado")
        Button { model.stop(d) } label: { Image(systemName: "stop.fill").frame(width: 14, height: 14) }
            .buttonStyle(BrutalIconStyle(fill: Duo.vermelho)).help("Desligar")
    }

    private func rotate(_ link: EmulatorLink) async {
        let d = device
        let landscape = await MainActor.run { d.landscape.toggle(); return d.landscape }
        await link.rotate(z: landscape ? -90 : 0)
    }

    private func capture(_ link: EmulatorLink) async {
        guard let png = await link.screenshotPNG() else { return }
        let dir = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Pictures/Duo")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let stamp = ISO8601DateFormatter().string(from: Date()).replacingOccurrences(of: ":", with: "-")
        try? png.write(to: dir.appendingPathComponent("\(device.id)-\(stamp).png"))
        await MainActor.run { model.banner = "Captura salva em Imagens/Duo" }
    }

    private func run(_ action: @escaping @Sendable (EmulatorLink) async -> Void) {
        guard let link = device.link else { return }
        model.used(device)
        Task { await action(link) }
    }

    private func send() {
        guard let link = device.link, !text.isEmpty else { return }
        model.used(device)
        let t = text
        text = ""
        Task { await link.type(t) }
    }

    private func tool(_ symbol: String, _ help: String, _ action: @escaping @Sendable (EmulatorLink) async -> Void) -> some View {
        Button {
            run(action)
        } label: {
            Image(systemName: symbol).font(.system(size: 12, weight: .bold)).frame(width: 14, height: 14)
        }
        .buttonStyle(BrutalIconStyle(fill: .white))
        .help(help)
        .disabled(device.state != .on)
    }
}

/// A tela do aparelho; clique e arraste viram toque no Android.
struct ScreenView: View {
    @Environment(AppModel.self) private var model
    let device: Device
    let image: CGImage

    var body: some View {
        GeometryReader { geo in
            Image(decorative: image, scale: 1)
                .resizable()
                .interpolation(.high)
                .frame(width: geo.size.width, height: geo.size.height)
                .contentShape(Rectangle())
                .gesture(
                    DragGesture(minimumDistance: 0)
                        .onChanged { send($0.location, geo.size, down: true) }
                        .onEnded { send($0.location, geo.size, down: false) }
                )
        }
    }

    private func send(_ point: CGPoint, _ size: CGSize, down: Bool) {
        guard let link = device.link else { return }
        if down { model.used(device) }
        let parts = device.info.screen.split(separator: "×").compactMap { Int($0) }
        guard parts.count == 2, size.width > 0, size.height > 0 else { return }
        let x = Int(max(0, min(1, point.x / size.width)) * Double(parts[0]))
        let y = Int(max(0, min(1, point.y / size.height)) * Double(parts[1]))
        Task { await link.touch(x: x, y: y, down: down) }
    }
}

// MARK: - Novo aparelho

struct NewDeviceSheet: View {
    @Environment(AppModel.self) private var model
    @Environment(DuoWindow.self) private var win
    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var kind: DeviceKind = .celular
    @State private var group = ""
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Novo aparelho").font(.system(size: 22, weight: .black))
            HStack(spacing: 10) {
                ForEach(DeviceKind.allCases) { k in
                    Button { kind = k } label: {
                        VStack(spacing: 6) {
                            Image(systemName: k.symbol).font(.system(size: 22, weight: .bold))
                            Text(k.label).font(.system(size: 12, weight: .bold))
                        }
                        .frame(maxWidth: .infinity, minHeight: 64)
                    }
                    .buttonStyle(BrutalButtonStyle(fill: kind == k ? Duo.azul : Color.white))
                }
            }
            TextField("Nome (ex.: Conta-Trabalho)", text: $name).textFieldStyle(.roundedBorder)
            TextField("Grupo (opcional)", text: $group).textFieldStyle(.roundedBorder)
            Text(String(format: "Android 13 · custa cerca de %.1f GB de memória ligado", kind.estimateGB))
                .font(.system(size: 12, weight: .semibold)).opacity(0.7)
                .fixedSize(horizontal: false, vertical: true)
            if !model.images.hasImage(for: kind) {
                HStack {
                    Text("Falta a imagem \(kind.imageLabel).").font(.system(size: 12, weight: .bold))
                        .fixedSize(horizontal: false, vertical: true)
                    Spacer()
                    Button("Abrir Imagens") { dismiss(); win.showImages = true }.buttonStyle(BrutalButtonStyle(fill: Duo.amarelo))
                }
            }
            if let error {
                Text(error).font(.system(size: 12)).foregroundStyle(Duo.vermelho)
                    .fixedSize(horizontal: false, vertical: true)
            }
            HStack {
                Spacer()
                Button("Cancelar") { dismiss() }.buttonStyle(BrutalButtonStyle(fill: .white))
                Button(busy ? "Criando…" : "Criar") {
                    busy = true
                    Task {
                        error = await model.create(name: name, kind: kind, group: group, desk: win.desk)
                        busy = false
                        if error == nil { dismiss() }
                    }
                }
                .buttonStyle(BrutalButtonStyle())
                .disabled(busy || !valid || !model.images.hasImage(for: kind))
            }
        }
        .padding(24)
        .frame(width: 500)
    }

    private var valid: Bool {
        !name.isEmpty && name.range(of: "^[A-Za-z0-9._-]+$", options: .regularExpression) != nil && model.device(name) == nil
    }
}

/// Primeiro uso: sem aparelho nenhum, mostra o caminho (imagens → aparelho).
struct EmptyDesk: View {
    @Environment(AppModel.self) private var model
    @Environment(DuoWindow.self) private var win

    var body: some View {
        VStack(spacing: 14) {
            Image(systemName: "iphone.and.arrow.forward").font(.system(size: 40, weight: .bold)).opacity(0.5)
            Text(model.images.ready ? "Mesa vazia" : "Comece pelas imagens").font(.system(size: 20, weight: .black))
            Text(model.images.ready
                 ? "Os aparelhos criados aqui ficam nesta janela."
                 : "O Duo baixa do Google o emulador e o Android que os aparelhos usam: de 1 a 2 GB por imagem, uma vez só.")
                .font(.system(size: 13, weight: .medium)).opacity(0.7).multilineTextAlignment(.center).frame(maxWidth: 380)
            if model.images.ready {
                Button { win.creating = true } label: { Label("Novo aparelho", systemImage: "plus") }.buttonStyle(BrutalButtonStyle())
            } else {
                Button("Abrir Imagens") { win.showImages = true }.buttonStyle(BrutalButtonStyle(fill: Duo.amarelo))
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
