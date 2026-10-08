import SwiftUI

/// Lugar de um aparelho na grade livre, em células.
struct GridSpot: Codable, Equatable {
    var x, y, w, h: Int

    func intersects(_ o: GridSpot) -> Bool {
        x < o.x + o.w && o.x < x + w && y < o.y + o.h && o.y < y + h
    }
}

/// A grade livre (MAC.md §2.6): 24 colunas de células quadradas que crescem
/// e encolhem com a janela. Cada quadro tem lugar e tamanho em células; o
/// usuário arrasta pelo título e redimensiona pelo canto. Quadro que cai em
/// cima de outro empurra o de baixo para baixo, então nada fica escondido.
enum Grid {
    static let cols = 24
    static let minW = 4, minH = 5

    static func defaultSize(_ kind: DeviceKind) -> (Int, Int) {
        switch kind {
        case .celular: (7, 14)
        case .tablet: (12, 8)
        case .dobravel: (9, 9)
        case .relogio: (6, 6)
        }
    }

    /// Primeiro lugar livre, linha a linha, da esquerda para a direita.
    static func firstFree(w: Int, h: Int, among taken: [GridSpot]) -> GridSpot {
        for y in 0... {
            for x in 0...(cols - w) {
                let s = GridSpot(x: x, y: y, w: w, h: h)
                if !taken.contains(where: { $0.intersects(s) }) { return s }
            }
        }
        fatalError("inalcançável")
    }

    /// Põe `id` em `spot` e empurra para baixo, em cascata, quem colidir.
    static func place(_ id: String, at spot: GridSpot, in spots: [String: GridSpot]) -> [String: GridSpot] {
        var out = spots
        out[id] = spot
        var queue = [id]
        while let a = queue.popLast() {
            guard let A = out[a] else { continue }
            for (b, B) in out where b != a && B.intersects(A) && b != id {
                out[b]?.y = A.y + A.h
                queue.append(b)
            }
        }
        return out
    }
}

extension AppModel {
    /// Lugar de cada aparelho da lista; quem ainda não tem ganha o primeiro livre.
    func ensureSpots(for list: [Device]) {
        var changed = false
        for d in list where spots[d.id] == nil {
            let (w, h) = Grid.defaultSize(d.info.kind)
            spots[d.id] = Grid.firstFree(w: w, h: h, among: Array(spots.values))
            changed = true
        }
        if changed { save() }
    }

    func commitSpots(_ next: [String: GridSpot]) {
        spots = next
        save()
    }

    /// Volta todos ao tamanho padrão, um ao lado do outro, na ordem do painel.
    func resetSpots() {
        var next: [String: GridSpot] = [:]
        for d in ordered {
            let (w, h) = Grid.defaultSize(d.info.kind)
            next[d.id] = Grid.firstFree(w: w, h: h, among: Array(next.values))
        }
        commitSpots(next)
    }
}

struct GridBoard: View {
    @Environment(AppModel.self) private var model
    let list: [Device]

    /// Bordas que o gesto mexe: nenhuma = mover; uma = lado; duas = canto.
    struct Edges: OptionSet, Hashable {
        let rawValue: Int
        static let left = Edges(rawValue: 1), right = Edges(rawValue: 2), top = Edges(rawValue: 4), bottom = Edges(rawValue: 8)
    }

    private struct Drag { var id: String; var edges: Edges; var start: GridSpot; var translation: CGSize }
    @State private var drag: Drag?
    @State private var preview: [String: GridSpot]?

    private let pad: CGFloat = 16
    private let gap: CGFloat = 8

    var body: some View {
        GeometryReader { geo in
            let cell = max(20, (geo.size.width - 2 * pad) / CGFloat(Grid.cols))
            let spots = preview ?? model.spots
            let rows = list.compactMap { spots[$0.id] }.map { $0.y + $0.h }.max() ?? 0
            ScrollView(.vertical) {
                ZStack(alignment: .topLeading) {
                    Color.clear.frame(width: cell * CGFloat(Grid.cols), height: cell * CGFloat(rows + 6))
                    if let drag, let target = spots[drag.id] {
                        RoundedRectangle(cornerRadius: 10)
                            .strokeBorder(style: StrokeStyle(lineWidth: 2, dash: [6, 4]))
                            .foregroundStyle(Duo.azul)
                            .background(RoundedRectangle(cornerRadius: 10).fill(Duo.azul.opacity(0.12)))
                            .frame(width: rect(target, cell).width, height: rect(target, cell).height)
                            .offset(x: rect(target, cell).minX, y: rect(target, cell).minY)
                    }
                    ForEach(list) { d in
                        if let s = spots[d.id] {
                            let r = liveRect(d.id, s, cell)
                            DeviceTile(device: d)
                                .overlay(alignment: .top) { moveHandle(d, cell) }
                                .overlay { resizeHandles(d, cell) }
                                .frame(width: r.width, height: r.height)
                                .offset(x: r.minX, y: r.minY)
                                .zIndex(drag?.id == d.id ? 2 : 0)
                        }
                    }
                }
                .padding(pad)
                .animation(.snappy(duration: 0.22), value: preview)
            }
        }
        .onAppear { model.ensureSpots(for: list) }
        .onChange(of: list.map(\.id)) { model.ensureSpots(for: list) }
    }

    private func rect(_ s: GridSpot, _ cell: CGFloat) -> CGRect {
        CGRect(x: CGFloat(s.x) * cell, y: CGFloat(s.y) * cell,
               width: CGFloat(s.w) * cell - gap, height: CGFloat(s.h) * cell - gap)
    }

    /// Durante o gesto, o quadro segue o mouse; o tracejado mostra onde vai encaixar.
    private func liveRect(_ id: String, _ s: GridSpot, _ cell: CGFloat) -> CGRect {
        guard let drag, drag.id == id else { return rect(s, cell) }
        var r = rect(drag.start, cell)
        let t = drag.translation, e = drag.edges
        if e.isEmpty { return r.offsetBy(dx: t.width, dy: t.height) }
        let minW = CGFloat(Grid.minW) * cell - gap, minH = CGFloat(Grid.minH) * cell - gap
        if e.contains(.right) { r.size.width = max(minW, r.width + t.width) }
        if e.contains(.bottom) { r.size.height = max(minH, r.height + t.height) }
        if e.contains(.left) {
            let w = max(minW, r.width - t.width)
            r.origin.x = r.maxX - w; r.size.width = w
        }
        if e.contains(.top) {
            let h = max(minH, r.height - t.height)
            r.origin.y = r.maxY - h; r.size.height = h
        }
        return r
    }

    private func snapped(_ drag: Drag, _ cell: CGFloat) -> GridSpot {
        var s = drag.start
        let dx = Int((drag.translation.width / cell).rounded()), dy = Int((drag.translation.height / cell).rounded())
        let e = drag.edges
        if e.isEmpty {
            s.x = min(max(0, s.x + dx), Grid.cols - s.w)
            s.y = max(0, s.y + dy)
            return s
        }
        let right = s.x + s.w, bottom = s.y + s.h
        if e.contains(.right) { s.w = min(max(Grid.minW, s.w + dx), Grid.cols - s.x) }
        if e.contains(.bottom) { s.h = max(Grid.minH, s.h + dy) }
        if e.contains(.left) {
            s.x = min(max(0, s.x + dx), right - Grid.minW)
            s.w = right - s.x
        }
        if e.contains(.top) {
            s.y = min(max(0, s.y + dy), bottom - Grid.minH)
            s.h = bottom - s.y
        }
        return s
    }

    private func gesture(_ d: Device, edges: Edges, _ cell: CGFloat) -> some Gesture {
        DragGesture(minimumDistance: 2, coordinateSpace: .global)
            .onChanged { v in
                guard let start = model.spots[d.id] else { return }
                var g = drag ?? Drag(id: d.id, edges: edges, start: start, translation: .zero)
                g.translation = v.translation
                drag = g
                let next = Grid.place(d.id, at: snapped(g, cell), in: model.spots)
                if next != preview { preview = next }
            }
            .onEnded { _ in
                if let preview { model.commitSpots(preview) }
                drag = nil
                preview = nil
            }
    }

    /// Faixa do título: arrasta para mover; dois cliques amplia.
    private func moveHandle(_ d: Device, _ cell: CGFloat) -> some View {
        Color.clear
            .frame(height: MesaLayout.pad + 24 - edge)
            .padding(.top, edge)
            .padding(.horizontal, edge)
            .padding(.trailing, 90)  // deixa o fps e o estado fora da área de pegar
            .contentShape(Rectangle())
            .pointerStyle(.grabIdle)
            .gesture(gesture(d, edges: [], cell))
            .onTapGesture(count: 2) { withAnimation(.snappy) { model.focused = d.id } }
    }

    /// Espessura da faixa de redimensionar: cabe na margem do quadro, sem
    /// cobrir botão nem tela.
    private let edge: CGFloat = 7

    /// Quatro lados e quatro cantos, cada um com o cursor de redimensionar do macOS.
    private func resizeHandles(_ d: Device, _ cell: CGFloat) -> some View {
        let corner: CGFloat = 16
        return ZStack {
            strip(d, cell, [.top], .top).frame(height: edge).frame(maxHeight: .infinity, alignment: .top).padding(.horizontal, corner)
            strip(d, cell, [.bottom], .bottom).frame(height: edge).frame(maxHeight: .infinity, alignment: .bottom).padding(.horizontal, corner)
            strip(d, cell, [.left], .leading).frame(width: edge).frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, corner)
            strip(d, cell, [.right], .trailing).frame(width: edge).frame(maxWidth: .infinity, alignment: .trailing).padding(.vertical, corner)
            strip(d, cell, [.top, .left], .topLeading).frame(width: corner, height: corner).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            strip(d, cell, [.top, .right], .topTrailing).frame(width: corner, height: corner).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing)
            strip(d, cell, [.bottom, .left], .bottomLeading).frame(width: corner, height: corner).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
            strip(d, cell, [.bottom, .right], .bottomTrailing).frame(width: corner, height: corner).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
        }
    }

    private func strip(_ d: Device, _ cell: CGFloat, _ edges: Edges, _ position: FrameResizePosition) -> some View {
        Color.clear
            .contentShape(Rectangle())
            .pointerStyle(.frameResize(position: position))
            .gesture(gesture(d, edges: edges, cell))
    }
}
