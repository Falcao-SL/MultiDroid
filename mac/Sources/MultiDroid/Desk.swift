import SwiftUI

/// O que é de cada janela do MultiDroid: a mesa (quais aparelhos ela mostra), o
/// arranjo, o aparelho ampliado, o painel recolhido e as folhas abertas.
/// Os aparelhos e o resto do estado continuam no AppModel, um só para o app.
@MainActor
@Observable
final class DeskWindow {
    let desk: String
    let model: AppModel
    var focused: String?
    var panelHidden = false
    var showImages = false
    var creating = false

    init(desk: String, model: AppModel) {
        self.desk = desk
        self.model = model
    }

    var devices: [Device] { model.devices(in: desk) }
    var groupNames: [String] { Array(Set(devices.compactMap { model.groups[$0.id] })).sorted() }

    var layout: AppModel.Layout {
        get { model.layouts[desk] ?? .grade }
        set { model.layouts[desk] = newValue; model.save() }
    }
}

/// Raiz de cada janela: cria o estado da janela e avisa o modelo quando ela
/// abre e fecha (mesa vazia fechada some).
struct WindowRoot: View {
    @Environment(AppModel.self) private var model
    let desk: String
    @State private var win: DeskWindow?

    var body: some View {
        Group {
            if let win { RootView().environment(win) } else { Color.clear }
        }
        .navigationTitle(model.title(of: desk))
        .onAppear {
            if win == nil { win = DeskWindow(desk: desk, model: model) }
            model.windowOpened(desk)
        }
        .onDisappear { model.windowClosed(desk) }
    }
}

/// Menu Mesas: reabre uma mesa fechada que ainda tem aparelhos.
struct DesksMenu: View {
    @Environment(\.openWindow) private var openWindow
    let model: AppModel

    var body: some View {
        let closed = model.workspaces.filter { !model.openWindows.contains($0) }
        if closed.isEmpty {
            Text("Todas as mesas estão abertas")
        } else {
            ForEach(closed, id: \.self) { desk in
                Button("\(model.title(of: desk)) (\(model.devices(in: desk).count) aparelhos)") { openWindow(value: desk) }
            }
        }
    }
}
