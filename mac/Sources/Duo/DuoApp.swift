import SwiftUI

@main
struct DuoApp: App {
    @State private var model = AppModel.shared

    var body: some Scene {
        // Cada janela é uma mesa. ⌘N abre uma mesa vazia; a principal guarda
        // os aparelhos que já existiam.
        WindowGroup("Duo", for: String.self) { $desk in
            WindowRoot(desk: desk ?? AppModel.mainDesk)
                .environment(model)
                .frame(minWidth: 1000, minHeight: 640)
        } defaultValue: {
            model.newWindowID()
        }
        .defaultSize(width: 1440, height: 900)
        .commands {
            CommandGroup(after: .newItem) {
                Button("Atualizar lista") { model.refresh() }.keyboardShortcut("r")
            }
            CommandMenu("Mesas") { DesksMenu(model: model) }
        }
    }
}
