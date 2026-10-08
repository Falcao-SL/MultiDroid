import SwiftUI

struct RootView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.colorScheme) private var scheme
    @State private var creating = false

    var body: some View {
        let p = DuoPalette.current(scheme)
        HStack(spacing: 0) {
            PanelView(creating: $creating)
                .frame(width: 340)
            Rectangle().fill(p.borda).frame(width: Duo.borda)
            GridArea()
        }
        .background(p.fundo)
        .sheet(isPresented: $creating) { NewDeviceSheet() }
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
    @Environment(\.colorScheme) private var scheme
    @Binding var creating: Bool
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
                Button { creating = true } label: { Label("Novo aparelho", systemImage: "plus") }
                    .buttonStyle(BrutalButtonStyle())
            }
            MemoryCard()
            if !selection.isEmpty { batchBar }
            ScrollView {
                VStack(spacing: 12) {
                    ForEach(model.ordered) { d in
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
            Button("Desligar") { for n in selection { if let d = model.device(n) { model.stop(d) } } }
        }
        .buttonStyle(BrutalButtonStyle(fill: Duo.ciano))
    }

    @ViewBuilder
    private func menu(for d: Device) -> some View {
        let names = selection.contains(d.id) ? Array(selection) : [d.id]
        Button("Duplicar") { model.duplicate(d) }.disabled(d.isRunning)
        Menu("Grupo") {
            ForEach(model.groupNames, id: \.self) { g in Button(g) { model.setGroup(g, for: names) } }
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
            Text(String(format: "Aparelhos: %.1f GB · teto %.0f GB · swap %.1f GB", model.runningGB, model.ceilingGB, model.swapGB))
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
            case .on:
                iconButton("pause.fill", Duo.azul) { model.pause(d) }
                iconButton("stop.fill", Duo.vermelho) { model.stop(d) }
            case .paused:
                iconButton("play.fill", Duo.verde) { model.resume(d) }
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

    var body: some View {
        @Bindable var model = model
        VStack(spacing: 0) {
            HStack {
                Picker("", selection: $model.layout) {
                    ForEach(AppModel.Layout.allCases) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented)
                .frame(width: 220)
                if model.focused != nil {
                    Button { model.focused = nil } label: { Label("Voltar à grade", systemImage: "square.grid.2x2") }
                        .buttonStyle(BrutalButtonStyle(fill: Duo.amarelo))
                }
                Spacer()
                Text("Dois cliques amplia · arraste para reordenar").font(.system(size: 11, weight: .semibold)).opacity(0.6)
            }
            .padding(14)
            ScrollView {
                if let f = model.focused, let d = model.device(f) {
                    DeviceTile(device: d, large: true).padding(20)
                } else if model.layout == .grade {
                    tiles(model.ordered)
                } else {
                    VStack(alignment: .leading, spacing: 22) {
                        ForEach(model.groupNames + [""], id: \.self) { g in
                            let members = model.ordered.filter { (model.groups[$0.id] ?? "") == g }
                            if !members.isEmpty {
                                Text(g.isEmpty ? "SEM GRUPO" : g.uppercased()).font(.system(size: 13, weight: .black)).kerning(1)
                                    .padding(.horizontal, 20)
                                tiles(members)
                            }
                        }
                    }
                }
            }
        }
    }

    private func tiles(_ list: [Device]) -> some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 230, maximum: 560), spacing: 22, alignment: .top)], spacing: 22) {
            ForEach(list) { d in
                DeviceTile(device: d, large: false)
                    .draggable(d.id)
                    .dropDestination(for: String.self) { items, _ in
                        guard let name = items.first else { return false }
                        withAnimation { model.move(name, before: d.id) }
                        return true
                    }
                    .onTapGesture(count: 2) { model.focused = d.id }
            }
        }
        .padding(20)
    }
}

struct DeviceTile: View {
    @Environment(AppModel.self) private var model
    @Environment(\.colorScheme) private var scheme
    let device: Device
    let large: Bool

    var body: some View {
        let d = device
        let p = DuoPalette.current(scheme)
        let (fw, fh) = d.frameSize
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 6) {
                Image(systemName: d.info.kind.symbol).font(.system(size: 12, weight: .bold))
                Text(d.id).font(.system(size: 13, weight: .heavy)).lineLimit(1)
                Spacer()
                if d.state == .on, let s = d.screen { Text(String(format: "%.0f fps", s.fps)).font(.system(size: 10, weight: .bold).monospacedDigit()).opacity(0.6) }
                StatusDot(state: d.state)
            }
            ZStack {
                RoundedRectangle(cornerRadius: d.info.kind == .relogio ? 999 : 14).fill(Color.black)
                if let img = d.screen?.image, d.state == .on || d.state == .paused {
                    ScreenView(device: d, image: img)
                        .clipShape(RoundedRectangle(cornerRadius: d.info.kind == .relogio ? 999 : 12))
                        .opacity(d.state == .paused ? 0.45 : 1)
                } else {
                    VStack(spacing: 10) {
                        Text(placeholder(d)).font(.system(size: 12, weight: .bold)).foregroundStyle(.white.opacity(0.75))
                            .multilineTextAlignment(.center)
                        if d.state == .off {
                            Button("Ligar") { model.start(d) }.buttonStyle(BrutalButtonStyle(fill: Duo.verde))
                        }
                    }
                    .padding()
                }
                if d.state == .paused {
                    Button("Retomar") { model.resume(d) }.buttonStyle(BrutalButtonStyle(fill: Duo.verde))
                }
            }
            .aspectRatio(CGFloat(fw) / CGFloat(fh), contentMode: .fit)
            .frame(maxWidth: large ? min(860 * CGFloat(fw) / CGFloat(fh), 1200) : .infinity)
        }
        .padding(10)
        .fixedSize(horizontal: false, vertical: true)  // a grade não estica o quadro na altura
        .brutal()
        .foregroundStyle(p.tinta)
    }

    private func placeholder(_ d: Device) -> String {
        switch d.state {
        case .off: "\(d.info.kind.label) · \(d.info.android)\n\(d.info.screen)"
        case .starting: d.message ?? "ligando…"
        case .stopping: "desligando…"
        case .external: "Aberto fora do Duo.\nDesligue e ligue por aqui para ver a tela."
        default: ""
        }
    }
}

/// A tela do aparelho; clique e arraste viram toque no Android.
struct ScreenView: View {
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
                        .frame(width: 90, height: 70)
                    }
                    .buttonStyle(BrutalButtonStyle(fill: kind == k ? Duo.azul : Color.white))
                }
            }
            TextField("Nome (ex.: Conta-Trabalho)", text: $name).textFieldStyle(.roundedBorder)
            TextField("Grupo (opcional)", text: $group).textFieldStyle(.roundedBorder)
            Text(String(format: "Android 13 · custa cerca de %.1f GB de memória ligado", kind.estimateGB))
                .font(.system(size: 12, weight: .semibold)).opacity(0.7)
            if let error { Text(error).font(.system(size: 12)).foregroundStyle(Duo.vermelho) }
            HStack {
                Spacer()
                Button("Cancelar") { dismiss() }.buttonStyle(BrutalButtonStyle(fill: .white))
                Button(busy ? "Criando…" : "Criar") {
                    busy = true
                    Task {
                        error = await model.create(name: name, kind: kind, group: group)
                        busy = false
                        if error == nil { dismiss() }
                    }
                }
                .buttonStyle(BrutalButtonStyle())
                .disabled(busy || !valid)
            }
        }
        .padding(24)
        .frame(width: 460)
    }

    private var valid: Bool {
        !name.isEmpty && name.range(of: "^[A-Za-z0-9._-]+$", options: .regularExpression) != nil && model.device(name) == nil
    }
}
