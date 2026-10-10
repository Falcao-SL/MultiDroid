import SwiftUI

struct RootView: View {
    @Environment(AppModel.self) private var model
    @Environment(DeskWindow.self) private var win
    @Environment(\.colorScheme) private var scheme

    var body: some View {
        let p = Paleta.current(scheme)
        HStack(spacing: 0) {
            if !win.panelHidden {
                PanelView()
                    .frame(width: 340)
                    .transition(.move(edge: .leading))
                Rectangle().fill(p.borda).frame(width: Tema.borda)
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
                    .foregroundStyle(Tema.tintaSobreAcento)
                    .padding(12)
                    .brutal(fill: Tema.amarelo)
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
    @Environment(DeskWindow.self) private var win
    @Environment(\.colorScheme) private var scheme
    @State private var selection = Set<String>()
    @State private var confirmDelete: Device?
    @State private var newGroupFor: [String]?
    @State private var groupName = ""

    var body: some View {
        @Bindable var model = model
        let p = Paleta.current(scheme)
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                Text("MultiDroid").font(.system(size: 26, weight: .black))
                Spacer()
                Button { win.showImages = true } label: { Image(systemName: "square.stack.3d.down.right").frame(width: 16, height: 16) }
                    .buttonStyle(BrutalIconStyle(fill: model.images.ready ? .white : Tema.amarelo))
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
        .buttonStyle(BrutalButtonStyle(fill: Tema.ciano))
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
        @Bindable var model = model
        let total = Metrics.physicalGB
        let used = max(0, total - model.availableGB)
        let max = total.rounded()
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
                    RoundedRectangle(cornerRadius: 4).fill(used / total > 0.85 ? Tema.vermelho : Tema.verde)
                        .frame(width: w * min(1, used / total))
                    RoundedRectangle(cornerRadius: 4).fill(Tema.azul)
                        .frame(width: w * min(1, model.runningGB / total))
                    Rectangle().fill(Tema.tintaSobreAcento).frame(width: 2).offset(x: w * min(1, model.ceilingGB / total))
                }
            }
            .frame(height: 14)
            Text(String(format: "Aparelhos: %.1f GB · swap %.1f GB", model.runningGB, model.swapGB)
                 + (model.asleepCount > 0 ? " · \(model.asleepCount) hibernando" : ""))
                .font(.system(size: 11, weight: .semibold)).opacity(0.7)
            Divider().padding(.vertical, 2)
            // O teto: até quanto os aparelhos ligados podem gastar. Acima dele,
            // o MultiDroid hiberna quem está há mais tempo sem uso (MAC.md §2.3).
            HStack(spacing: 6) {
                Text("Teto dos aparelhos").font(.system(size: 12, weight: .bold))
                Spacer()
                Button { setCeiling(model.ceilingGB - 1) } label: { Image(systemName: "minus").frame(width: 10, height: 10) }
                    .buttonStyle(BrutalIconStyle()).disabled(model.ceilingGB <= 4).help("Diminuir o teto")
                Text("\(Int(model.ceilingGB)) GB").font(.system(size: 14, weight: .black).monospacedDigit())
                    .frame(minWidth: 46)
                Button { setCeiling(model.ceilingGB + 1) } label: { Image(systemName: "plus").frame(width: 10, height: 10) }
                    .buttonStyle(BrutalIconStyle()).disabled(model.ceilingGB >= max).help("Aumentar o teto")
            }
            Slider(value: $model.ceilingGB, in: 4...max, step: 1) { editing in if !editing { setCeiling(model.ceilingGB) } }
            Text(capacity).font(.system(size: 11, weight: .semibold))
                .foregroundStyle(model.runningGB > model.ceilingGB ? Tema.vermelho : Color.primary.opacity(0.7))
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(12)
        .brutal()
    }

    /// Quantos celulares cabem acordados com este teto, e o que acontece além dele.
    private var capacity: String {
        let fit = Int(model.ceilingGB / DeviceKind.celular.estimateGB)
        if model.runningGB > model.ceilingGB {
            return String(format: "Os ligados já passam do teto (%.1f GB). O próximo a ligar hiberna os sem uso.", model.runningGB)
        }
        return fit == 0
            ? "Nenhum celular cabe neste teto. Aumente para ligar."
            : "Cabem ~\(fit) celular\(fit == 1 ? "" : "es") acordado\(fit == 1 ? "" : "s"). Acima disso, o MultiDroid hiberna quem está sem uso."
    }

    private func setCeiling(_ gb: Double) {
        let v = min(Metrics.physicalGB.rounded(), Swift.max(4, gb.rounded()))
        model.ceilingGB = v
        model.save()
        let fit = Int(v / DeviceKind.celular.estimateGB)
        model.banner = "Teto em \(Int(v)) GB: cabem ~\(fit) celular\(fit == 1 ? "" : "es") acordado\(fit == 1 ? "" : "s")"
    }
}

struct StatusDot: View {
    let state: Device.State
    var body: some View {
        let (c, t): (Color, String) = switch state {
        case .off: (.gray, "desligado")
        case .starting: (Tema.amarelo, "ligando")
        case .on: (Tema.verde, "ligado")
        case .paused: (Tema.azul, "pausado")
        case .stopping: (Tema.laranja, "desligando")
        case .external: (Tema.roxo, "aberto fora do MultiDroid")
        case .asleep: (Tema.ciano, "hibernando")
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
                .foregroundStyle(Tema.tintaSobreAcento)
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
                        .brutal(fill: Tema.amarelo, shadow: false)
                        .foregroundStyle(Tema.tintaSobreAcento)
                }
            }
            Spacer()
            controls(d)
        }
        .padding(10)
        .brutal(fill: selected ? Tema.azul.opacity(0.35) : nil)
    }

    private func color(for d: Device) -> Color {
        let i = model.order.firstIndex(of: d.id) ?? 0
        return Tema.ciclo[i % Tema.ciclo.count]
    }

    @ViewBuilder
    private func controls(_ d: Device) -> some View {
        HStack(spacing: 6) {
            switch d.state {
            case .off:
                iconButton("play.fill", Tema.verde) { model.start(d) }
            case .asleep:
                if d.waking { ProgressView().controlSize(.small) } else { iconButton("play.fill", Tema.verde) { model.start(d) } }
            case .on:
                iconButton("pause.fill", Tema.azul) { model.pause(d) }
                iconButton("moon.zzz.fill", Tema.ciano) { model.hibernate(d) }
                iconButton("stop.fill", Tema.vermelho) { model.stop(d) }
            case .paused:
                iconButton("play.fill", Tema.verde) { model.resume(d) }
                iconButton("moon.zzz.fill", Tema.ciano) { model.hibernate(d) }
                iconButton("stop.fill", Tema.vermelho) { model.stop(d) }
            case .external:
                iconButton("stop.fill", Tema.vermelho) { model.stop(d) }
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
    @Environment(DeskWindow.self) private var win
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
                        .buttonStyle(BrutalButtonStyle(fill: Tema.amarelo))
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
        let p = Paleta.current(scheme)
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
                                Button("Ligar") { model.start(d) }.buttonStyle(BrutalButtonStyle(fill: Tema.verde))
                            }
                            if d.state == .asleep, !d.waking {
                                Button("Acordar") { model.start(d) }.buttonStyle(BrutalButtonStyle(fill: Tema.ciano))
                            }
                        }
                        .padding()
                    }
                    if d.state == .paused {
                        Button("Retomar") { model.resume(d) }.buttonStyle(BrutalButtonStyle(fill: Tema.verde))
                    }
                    if d.state == .asleep, !d.waking, d.lastFrame != nil {
                        Button("Acordar") { model.start(d) }.buttonStyle(BrutalButtonStyle(fill: Tema.ciano))
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
        case .external: "Aberto fora do MultiDroid.\nDesligue e ligue por aqui para ver a tela."
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
                .buttonStyle(BrutalIconStyle(fill: Tema.azul)).help("Pausar")
        } else {
            Button { model.resume(d) } label: { Image(systemName: "play.fill").frame(width: 14, height: 14) }
                .buttonStyle(BrutalIconStyle(fill: Tema.verde)).help("Retomar")
        }
        Button { model.hibernate(d) } label: { Image(systemName: "moon.zzz.fill").frame(width: 14, height: 14) }
            .buttonStyle(BrutalIconStyle(fill: Tema.ciano)).help("Hibernar: libera a memória e guarda o estado")
        Button { model.stop(d) } label: { Image(systemName: "stop.fill").frame(width: 14, height: 14) }
            .buttonStyle(BrutalIconStyle(fill: Tema.vermelho)).help("Desligar")
    }

    private func rotate(_ link: EmulatorLink) async {
        let d = device
        let landscape = await MainActor.run { d.landscape.toggle(); return d.landscape }
        await link.rotate(z: landscape ? -90 : 0)
    }

    private func capture(_ link: EmulatorLink) async {
        guard let png = await link.screenshotPNG() else { return }
        let dir = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Pictures/MultiDroid")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let stamp = ISO8601DateFormatter().string(from: Date()).replacingOccurrences(of: ":", with: "-")
        try? png.write(to: dir.appendingPathComponent("\(device.id)-\(stamp).png"))
        await MainActor.run { model.banner = "Captura salva em Imagens/MultiDroid" }
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
    @Environment(DeskWindow.self) private var win
    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var kind: DeviceKind = .celular
    @State private var image: String?
    @State private var group = ""
    @State private var busy = false
    @State private var error: String?
    @State private var license: (path: String, pkg: RemotePackage)?

    var body: some View {
        if let l = license {
            // Licença na própria folha: folha sobre folha falha no macOS.
            LicenseSheet(title: Images.title(l.path), text: l.pkg.licenseText) {
                model.images.accept(l.pkg)
                license = nil
                create()
            } onDecline: { license = nil; busy = false }
        } else {
            form
        }
    }

    private var form: some View {
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
                    .buttonStyle(BrutalButtonStyle(fill: kind == k ? Tema.azul : Color.white))
                }
            }
            let store = model.images
            let installed = store.installedImages(for: kind)
            let available = store.images.filter { Images.suits($0, kind) && store.installed[$0] == nil && store.remote[$0] != nil }
            if installed.isEmpty && available.isEmpty {
                Text(store.loading ? "Procurando imagens no Google…" : (store.error ?? "Nenhuma imagem encontrada para \(kind.label.lowercased())."))
                    .font(.system(size: 12, weight: .bold)).fixedSize(horizontal: false, vertical: true)
            } else {
                Picker("Imagem", selection: $image) {
                    if !installed.isEmpty {
                        Section("Instaladas") {
                            ForEach(installed, id: \.self) { Text(label($0)).tag(Optional($0)) }
                        }
                    }
                    if !available.isEmpty {
                        Section("Baixar do Google") {
                            ForEach(available, id: \.self) { p in
                                Text(label(p) + String(format: " · %.1f GB", Double(store.remote[p]?.size ?? 0) / 1e9)).tag(Optional(p))
                            }
                        }
                    }
                }
                .font(.system(size: 13, weight: .semibold))
                .disabled(busy)
                if let image {
                    Text([Images.detail(image), Images.note(image), downloadNote(image)].compactMap { $0 }.joined(separator: "\n"))
                        .font(.system(size: 11, weight: .semibold)).opacity(0.7)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            TextField("Nome (ex.: Conta Trabalho)", text: $name).textFieldStyle(.roundedBorder)
            TextField("Grupo (opcional)", text: $group).textFieldStyle(.roundedBorder)
            Text(String(format: "Custa cerca de %.1f GB de memória ligado", kind.estimateGB))
                .font(.system(size: 12, weight: .semibold)).opacity(0.7)
                .fixedSize(horizontal: false, vertical: true)
            if let fit = ceilingNote {
                Text(fit).font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(kind.estimateGB > model.ceilingGB ? Tema.vermelho : Color.primary.opacity(0.7))
                    .fixedSize(horizontal: false, vertical: true)
            }
            if let hint = nameHint {
                Text(hint).font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(valid ? Color.primary.opacity(0.7) : Tema.vermelho)
                    .fixedSize(horizontal: false, vertical: true)
            }
            if let error {
                Text(error).font(.system(size: 12)).foregroundStyle(Tema.vermelho)
                    .fixedSize(horizontal: false, vertical: true)
            }
            HStack {
                Spacer()
                Button("Cancelar") { dismiss() }.buttonStyle(BrutalButtonStyle(fill: .white))
                if busy, let image, let f = model.images.progress[image] {
                    Button("Cancelar download") { model.images.cancel(image) }.buttonStyle(BrutalButtonStyle(fill: .white))
                    Text(model.images.phase[image] == "baixando" ? "Baixando \(Int(f * 100))%" : (model.images.phase[image] ?? "…"))
                        .font(.system(size: 12, weight: .bold).monospacedDigit())
                } else {
                    Button("Cancelar") { dismiss() }.buttonStyle(BrutalButtonStyle(fill: .white))
                }
                Button(busy ? "Criando…" : needsDownload ? "Baixar e criar" : "Criar") { create() }
                    .buttonStyle(BrutalButtonStyle())
                    .disabled(busy || !valid || image == nil)
            }
        }
        .padding(24)
        .frame(width: 500)
        .onAppear { pickImage() }
        .onChange(of: kind) { pickImage() }
        .onChange(of: model.images.remote.count) { if image == nil { pickImage() } }
        .task { if model.images.remote.isEmpty { await model.images.refresh() } }
    }

    private func label(_ path: String) -> String {
        Images.title(path) + (Images.recommended[path] != nil ? "  ★ recomendada" : "")
    }

    private var needsDownload: Bool { image.map { model.images.installed[$0] == nil } ?? false }

    private func downloadNote(_ path: String) -> String? {
        guard model.images.installed[path] == nil, let pkg = model.images.remote[path] else { return nil }
        return String(format: "Não instalada: o MultiDroid baixa %.1f GB do Google antes de criar.", Double(pkg.size) / 1e9)
    }

    /// Ao abrir e ao trocar o tipo: a imagem sugerida; senão uma recomendada;
    /// senão a mais nova instalada; senão a mais nova do Google. Instaladas
    /// têm preferência a cada passo.
    private func pickImage() {
        let store = model.images
        let installed = store.installedImages(for: kind)
        let all = installed + store.images.filter { Images.suits($0, kind) && store.installed[$0] == nil && store.remote[$0] != nil }
        image = installed.contains(kind.defaultImage) ? kind.defaultImage
            : installed.first { Images.recommended[$0] != nil }
            ?? installed.first
            ?? (all.contains(kind.defaultImage) ? kind.defaultImage : nil)
            ?? all.first { Images.recommended[$0] != nil }
            ?? all.first
    }

    /// Cria o aparelho; se a imagem (ou o emulador) não está instalada, baixa
    /// antes, passando pela licença do Google na primeira vez.
    private func create() {
        guard let image else { return }
        let store = model.images
        busy = true
        error = nil
        let missing = [Images.emulator, image].filter { store.installed[$0] == nil }
        for path in missing {
            guard let pkg = store.remote[path] else {
                error = "\(Images.title(path)) não está instalada e o Google não respondeu: tente de novo em Imagens."
                busy = false
                return
            }
            if !store.accepted(pkg) { license = (path, pkg); return }
        }
        Task {
            for path in missing {
                store.install(path)
                while store.progress[path] != nil { try? await Task.sleep(for: .milliseconds(300)) }
                if store.installed[path] == nil {
                    error = store.error ?? "\(Images.title(path)) não foi instalada."
                    busy = false
                    return
                }
            }
            error = await model.create(name: avdName, kind: kind, image: image, group: group, desk: win.desk)
            busy = false
            if error == nil { dismiss() }
        }
    }

    /// O emulador só aceita letras sem acento, números, ponto, _ e - no nome do
    /// aparelho: "Conta 2" vira "Conta-2", "Ação" vira "Acao".
    private var avdName: String {
        let plain = name.folding(options: [.diacriticInsensitive, .widthInsensitive], locale: .current)
        let dashed = plain.replacingOccurrences(of: "\\s+", with: "-", options: .regularExpression)
        let kept = dashed.replacingOccurrences(of: "[^A-Za-z0-9._-]", with: "", options: .regularExpression)
        return kept.replacingOccurrences(of: "-{2,}", with: "-", options: .regularExpression)
                   .trimmingCharacters(in: CharacterSet(charactersIn: "-."))
    }

    private var valid: Bool { !avdName.isEmpty && model.device(avdName) == nil }

    /// Como o teto de memória afeta este aparelho quando ele for ligado.
    private var ceilingNote: String? {
        let teto = Int(model.ceilingGB)
        if kind.estimateGB > model.ceilingGB {
            return "Sozinho ele já passa do teto de \(teto) GB: aumente o teto no painel para conseguir ligar."
        }
        let ligados = model.devices.filter(\.isRunning).reduce(0) { $0 + Swift.max($1.footprintGB, $1.info.kind.estimateGB) }
        if ligados + kind.estimateGB > model.ceilingGB {
            return "Com o teto de \(teto) GB, ligar este vai hibernar os aparelhos sem uso há mais tempo."
        }
        return nil
    }

    /// Diz por que o Criar está desligado, ou como o nome vai ficar.
    private var nameHint: String? {
        let typed = name.trimmingCharacters(in: .whitespaces)
        if typed.isEmpty { return nil }
        if avdName.isEmpty { return "Use letras ou números no nome." }
        if model.device(avdName) != nil { return "Já existe um aparelho chamado \(avdName)." }
        if avdName != typed { return "Vai se chamar \(avdName) (o emulador não aceita espaço nem acento)." }
        return nil
    }
}

/// Primeiro uso: sem aparelho nenhum, mostra o caminho (imagens → aparelho).
struct EmptyDesk: View {
    @Environment(AppModel.self) private var model
    @Environment(DeskWindow.self) private var win

    var body: some View {
        VStack(spacing: 14) {
            Image(systemName: "iphone.and.arrow.forward").font(.system(size: 40, weight: .bold)).opacity(0.5)
            Text(model.images.ready ? "Mesa vazia" : "Comece pelas imagens").font(.system(size: 20, weight: .black))
            Text(model.images.ready
                 ? "Os aparelhos criados aqui ficam nesta janela."
                 : "O MultiDroid baixa do Google o emulador e o Android que os aparelhos usam: de 1 a 2 GB por imagem, uma vez só.")
                .font(.system(size: 13, weight: .medium)).opacity(0.7).multilineTextAlignment(.center).frame(maxWidth: 380)
            if model.images.ready {
                Button { win.creating = true } label: { Label("Novo aparelho", systemImage: "plus") }.buttonStyle(BrutalButtonStyle())
            } else {
                Button("Abrir Imagens") { win.showImages = true }.buttonStyle(BrutalButtonStyle(fill: Tema.amarelo))
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
