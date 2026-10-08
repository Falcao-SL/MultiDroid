import SwiftUI

/// Tamanho-base da tela de um quadro, em pontos, na escala 1 (`Device.frameSize`
/// pela metade). A mesa multiplica todos pela mesma escala, então um tablet
/// continua maior que um relógio.
struct TileBase: LayoutValueKey {
    static let defaultValue = CGSize(width: 300, height: 660)
}

/// Se o quadro tem a coluna de botões (só aparelho ligado ou pausado).
struct TileToolbar: LayoutValueKey {
    static let defaultValue = false
}

extension View {
    func tile(base: CGSize, toolbar: Bool) -> some View {
        layoutValue(key: TileBase.self, value: base).layoutValue(key: TileToolbar.self, value: toolbar)
    }
}

/// A mesa dos aparelhos (MAC.md §2.6). Procura a maior escala em que todos os
/// quadros cabem lado a lado na área visível, em linhas centralizadas. Se nem
/// na escala mínima cabem, fica na mínima e sobrepõe os quadros de cada linha
/// como cartas na mão; o quadro sob o mouse vem para a frente.
struct MesaLayout: Layout {
    var gap: CGFloat = 18
    var minScale: CGFloat = 0.4
    var maxScale: CGFloat = 2.4

    /// Moldura do quadro: margem, cabeçalho e a coluna de botões.
    static let pad: CGFloat = 10
    static let header: CGFloat = 28
    static let toolbarWidth: CGFloat = 36
    /// Altura mínima da área de tela quando há botões (a coluna compacta).
    static let toolbarMinHeight: CGFloat = 200

    static func tileSize(base: CGSize, toolbar: Bool, scale: CGFloat) -> CGSize {
        let sw = base.width * scale, sh = base.height * scale
        let w = sw + 2 * pad + (toolbar ? toolbarWidth + 8 : 0)
        let h = max(sh, toolbar ? toolbarMinHeight : 0) + 2 * pad + header
        return CGSize(width: w, height: h)
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        proposal.replacingUnspecifiedDimensions(by: CGSize(width: 900, height: 700))
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        guard !subviews.isEmpty else { return }
        let area = bounds.insetBy(dx: gap, dy: gap)
        let sizes = { (s: CGFloat) in
            subviews.map { Self.tileSize(base: $0[TileBase.self], toolbar: $0[TileToolbar.self], scale: s) }
        }

        // Maior escala que cabe sem sobrepor (busca binária).
        var lo = minScale, hi = maxScale
        if rows(sizes(lo), width: area.width).height <= area.height {
            for _ in 0..<24 {
                let mid = (lo + hi) / 2
                if rows(sizes(mid), width: area.width).height <= area.height { lo = mid } else { hi = mid }
            }
            place(rows(sizes(lo), width: area.width), sizes: sizes(lo), in: area, overlap: false, subviews: subviews)
            return
        }

        // Muitos aparelhos: escala mínima, linhas que cabem na altura, sobreposição dentro da linha.
        let all = sizes(minScale)
        let rowH = all.map(\.height).max() ?? 0
        let nRows = max(1, min(subviews.count, Int((area.height + gap) / (rowH + gap))))
        let perRow = Int((Double(subviews.count) / Double(nRows)).rounded(.up))
        var layout = Rows()
        for start in stride(from: 0, to: subviews.count, by: perRow) {
            let idx = Array(start..<min(start + perRow, subviews.count))
            layout.items.append(idx)
            layout.heights.append(idx.map { all[$0].height }.max() ?? 0)
        }
        layout.height = layout.heights.reduce(0, +) + gap * CGFloat(layout.items.count - 1)
        place(layout, sizes: all, in: area, overlap: true, subviews: subviews)
    }

    private struct Rows {
        var items: [[Int]] = []
        var heights: [CGFloat] = []
        var height: CGFloat = 0
    }

    /// Quebra em linhas pela largura, na ordem da grade.
    private func rows(_ sizes: [CGSize], width: CGFloat) -> Rows {
        var r = Rows(), line: [Int] = [], used: CGFloat = 0
        for (i, s) in sizes.enumerated() {
            let need = line.isEmpty ? s.width : used + gap + s.width
            if !line.isEmpty && need > width {
                r.items.append(line); line = [i]; used = s.width
            } else {
                line.append(i); used = need
            }
        }
        if !line.isEmpty { r.items.append(line) }
        r.heights = r.items.map { $0.map { sizes[$0].height }.max() ?? 0 }
        r.height = r.heights.reduce(0, +) + gap * CGFloat(max(0, r.items.count - 1))
        // Um quadro mais largo que a área não cabe em escala nenhuma menor.
        if sizes.contains(where: { $0.width > width }) { r.height = .infinity }
        return r
    }

    private func place(_ r: Rows, sizes: [CGSize], in area: CGRect, overlap: Bool, subviews: Subviews) {
        var y = area.minY + max(0, (area.height - r.height) / 2)
        for (line, rowH) in zip(r.items, r.heights) {
            let widths = line.map { sizes[$0].width }
            var total = widths.reduce(0, +) + gap * CGFloat(line.count - 1)
            var step = { (k: Int) in widths[k] + gap }
            if overlap && total > area.width && line.count > 1 {
                // Encolhe o passo para a linha caber; o último quadro aparece inteiro.
                let squeeze = (area.width - widths.last!) / widths.dropLast().reduce(0, +)
                step = { k in widths[k] * squeeze }
                total = area.width
            }
            var x = area.minX + max(0, (area.width - total) / 2)
            for (k, i) in line.enumerated() {
                let s = sizes[i]
                subviews[i].place(at: CGPoint(x: x, y: y + (rowH - s.height) / 2), anchor: .topLeading,
                                  proposal: ProposedViewSize(s))
                x += step(k)
            }
            y += rowH + gap
        }
    }
}
