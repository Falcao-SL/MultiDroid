import SwiftUI

/// Falcao Neo-Brutalism (docs/design/tokens.json): bordas grossas, sombra dura,
/// cor chapada. Os valores repetem tokens.json; mudou lá, muda aqui.
enum Duo {
    static func hex(_ v: UInt32) -> Color {
        Color(red: Double((v >> 16) & 0xFF) / 255, green: Double((v >> 8) & 0xFF) / 255, blue: Double(v & 0xFF) / 255)
    }

    static let azul = hex(0x7AA7FF)
    static let rosa = hex(0xFF90E8)
    static let amarelo = hex(0xFFDE59)
    static let verde = hex(0x4ADE80)
    static let ciano = hex(0x5CE1E6)
    static let roxo = hex(0xA78BFA)
    static let laranja = hex(0xFF9F45)
    static let vermelho = hex(0xFF5757)
    static let tintaSobreAcento = hex(0x141414)
    static let ciclo: [Color] = [azul, rosa, amarelo, verde, ciano, roxo, laranja]

    static let borda: CGFloat = 2.5
    static let raio: CGFloat = 10
    static let sombra: CGFloat = 4
}

/// Cores que mudam com o tema (claro/escuro).
struct DuoPalette {
    let fundo, superficie, borda, sombra, tinta: Color

    static func current(_ scheme: ColorScheme) -> DuoPalette {
        scheme == .dark
            ? DuoPalette(fundo: Duo.hex(0x17171C), superficie: Duo.hex(0x23232B), borda: Duo.hex(0xEFEDE6),
                         sombra: Duo.hex(0x000000), tinta: Duo.hex(0xEFEDE6))
            : DuoPalette(fundo: Duo.hex(0xF2EFE9), superficie: Duo.hex(0xFFFFFF), borda: Duo.hex(0x141414),
                         sombra: Duo.hex(0x141414), tinta: Duo.hex(0x141414))
    }
}

/// BrutalBox: superfície com borda grossa e sombra dura (4, 4) sem desfoque.
struct BrutalBox: ViewModifier {
    @Environment(\.colorScheme) private var scheme
    var fill: Color? = nil
    var shadow = true

    func body(content: Content) -> some View {
        let p = DuoPalette.current(scheme)
        content
            .background(
                RoundedRectangle(cornerRadius: Duo.raio)
                    .fill(fill ?? p.superficie)
                    .overlay(RoundedRectangle(cornerRadius: Duo.raio).stroke(p.borda, lineWidth: Duo.borda))
            )
            .background(
                RoundedRectangle(cornerRadius: Duo.raio)
                    .fill(p.sombra)
                    .offset(x: shadow ? Duo.sombra : 0, y: shadow ? Duo.sombra : 0)
            )
    }
}

extension View {
    func brutal(fill: Color? = nil, shadow: Bool = true) -> some View {
        modifier(BrutalBox(fill: fill, shadow: shadow))
    }
}

/// Botão que afunda na própria sombra ao ser pressionado (70 ms).
struct BrutalButtonStyle: ButtonStyle {
    var fill: Color = Duo.azul

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 13, weight: .bold))
            .foregroundStyle(Duo.tintaSobreAcento)
            .padding(.horizontal, 12)
            .padding(.vertical, 7)
            .brutal(fill: fill, shadow: !configuration.isPressed)
            .offset(x: configuration.isPressed ? Duo.sombra : 0, y: configuration.isPressed ? Duo.sombra : 0)
            .animation(.linear(duration: 0.07), value: configuration.isPressed)
    }
}

/// Botão quadrado só com ícone, para a coluna de botões do aparelho.
struct BrutalIconStyle: ButtonStyle {
    var fill: Color = .white

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 12, weight: .bold))
            .foregroundStyle(Duo.tintaSobreAcento)
            .padding(6)
            .brutal(fill: fill, shadow: !configuration.isPressed)
            .offset(x: configuration.isPressed ? 2 : 0, y: configuration.isPressed ? 2 : 0)
            .animation(.linear(duration: 0.07), value: configuration.isPressed)
    }
}
