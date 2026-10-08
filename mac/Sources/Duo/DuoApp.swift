import SwiftUI

@main
struct DuoApp: App {
    @State private var model = AppModel()

    var body: some Scene {
        WindowGroup("Duo") {
            RootView()
                .environment(model)
                .frame(minWidth: 1000, minHeight: 640)
        }
        .defaultSize(width: 1440, height: 900)
        .commands {
            CommandGroup(after: .newItem) {
                Button("Atualizar lista") { model.refresh() }.keyboardShortcut("r")
                Button(model.panelHidden ? "Mostrar painel" : "Esconder painel") {
                    withAnimation(.snappy) { model.panelHidden.toggle() }
                }.keyboardShortcut("0")
            }
        }
    }
}
