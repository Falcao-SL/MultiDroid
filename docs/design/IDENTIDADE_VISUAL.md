# Identidade visual — Falcao Neo-Brutalism

Identidade do MultiDroid (SwiftUI): neo-brutalismo (bordas grossas, sombra
dura, cor chapada, tipografia pesada), com o azul como acento. Os **valores** estão em [`tokens.json`](tokens.json) — este
documento explica o uso; nunca repita um número aqui sem que ele esteja lá.
O comportamento das telas está em [`../spec/MAC.md`](../spec/MAC.md).

Unidade: `1` = 1 pt.

## Princípios

1. **Bordas fortes.** Todo bloco tem borda de `forma.larguraBorda` (2,5) na
   cor `borda` (quase-preta no claro, off-white no escuro).
2. **Sombra dura.** Deslocamento `(4, 4)`, desfoque 0, mesmo raio da forma,
   cor `sombra`. Nenhum componente usa sombra com desfoque.
3. **Cor chapada.** Sem gradiente, sem transparência em superfícies. O texto
   sobre acento é sempre `tintaSobreAcento` (#141414).
4. **Tipografia pesada.** Pesos 600–900, fonte do sistema.
5. **Movimento físico.** Botão pressionado desliza sobre a própria sombra
   (a sombra some e o bloco anda `(4, 4)` em 70 ms).
6. **Cantos discretos.** Raio 10 nos blocos.
7. **A tela do Android é intocável.** O MultiDroid desenha a moldura, os botões e o
   painel; nunca pinta por cima da tela de um aparelho.

## Acento e cores

- O acento do MultiDroid é o **azul** (`cor.acento.azul`): botão primário (`Novo
  aparelho`, ações), seleção.
- Cores de estado nos botões do aparelho: `verde` ligar/retomar, `azul`
  pausar, `vermelho` desligar e apagar, `amarelo` avisos (ex.: falta
  imagem).
- Tema escuro: só `cor.escuro` muda; acentos, forma e tipografia são iguais
  nos dois temas.

## Tokens

| Grupo | Onde | Resumo |
|-------|------|--------|
| Cores base por tema | `cor.claro`, `cor.escuro` | `fundo`, `superficie`, `borda`, `sombra`, `tinta`, `tintaSobreAcento` |
| Acentos | `cor.acento` | azul, rosa, amarelo, verde, ciano, roxo, laranja, vermelho |
| Ciclo | `cor.cicloAcentos` | 7 acentos em ordem, para colorir itens em série |
| Forma | `forma` | borda 2,5; raio 10; sombra (4,4,0) |
| Tipografia | `tipografia` | família do sistema, pesos, escala |
| Movimento | `movimento` | pressão 70 ms |

`tokens.json` ainda traz grupos que eram dos apps de celular (papéis,
espaços e tamanhos de componentes que não existem no Mac); não valem para o
MultiDroid até alguém os usar.

## Componentes (Mac)

| Componente | Em `mac/Sources/MultiDroid/Theme.swift` | Uso |
|------------|----------------------------------|-----|
| BrutalBox | `.brutal(fill:shadow:)` | painel, cartões, folhas; variante sem sombra para avisos |
| BrutalButton | `BrutalButtonStyle(fill:)` | ações com texto; fundo `azul` por padrão, `verde` para confirmar, branco para secundárias |
| Botão de ícone | `BrutalIconStyle(fill:)` | coluna de botões ao lado de cada aparelho; afunda 2 ao pressionar |

## Iconografia

SF Symbols, cheios quando existir, na cor do texto ao redor. Nenhum emoji
como ícone de interface. Nenhum logo de serviço nem do Google/Android.

| Uso | SF Symbol |
|-----|-----------|
| Novo aparelho | `plus` |
| Ligar / retomar | `play.fill` |
| Pausar | `pause.fill` |
| Desligar | `stop.fill` |
| Voltar / Início / Recentes (Android) | `chevron.backward` / `circle` / `square.on.square` |
| Volume + / − | `speaker.plus` / `speaker.minus` |
| Girar | `rotate.right` |
| Digitar | `keyboard` |
| Captura de tela | `camera` |
| Voltar à mesa | `square.grid.2x2` |
| Celular / tablet / dobrável / relógio | `iphone` / `ipad.landscape` / `rectangle.split.2x1` / `applewatch` |

## Ícone do app

- Arte (2026-10-07): **dois aparelhos iguais deslocados** (a ideia de
  "duplicar"), em releitura neo-brutalista: aparelho de trás `roxo`
  (`#A78BFA`), da frente `ciano` (`#5CE1E6`), borda grossa off-white
  (`#EFEDE6`) e a sombra dura na cor do aparelho de trás, sobre grafite
  (`#14161B`; `#0B0C10` no modo escuro). Sem qualquer referência a marcas.
- Fonte única: `icone/multidroid-icon.svg` (camadas `bg`, `back`, `shadow`, `front`).
  Tudo é gerado por `icone/export_icons.py`; nunca edite as saídas à mão.
- **Mac**: `icone/AppIcon.icon` (Icon Composer, Xcode 26+), com fundo
  próprio no escuro e uma camada por aparelho; `mac/scripts/build-app.sh`
  compila com `actool` para macOS.
- O exportador ainda gera saídas para iOS, visionOS e Android/Wear
  (`icone/png/`, `icone/android/`), feitas para os apps de celular
  descontinuados; não são usadas pelo Mac.

## Tom de voz

- **Direto, em português do Brasil.** Frases curtas; nada de "Ops!", emoji ou
  exclamação.
- **Honesto sobre limites**: dizer quanto de memória um aparelho custa, o
  que não funciona em emulador e por que uma ação está desabilitada.
- **Nomes de serviço e do Google são só rótulos**; nunca sugerir parceria.
- **Motivos de erro** começam em minúscula, sem ponto final, dizem o que
  falhou (`não duplicou: …`).

## Implementação

`mac/Sources/MultiDroid/Theme.swift` repete os valores de `tokens.json` (cores,
borda, raio, sombra). Regras:

1. Cores, borda, raio e sombra só em `Theme.swift` (tamanhos de fonte e
   espaços ainda estão nas telas).
2. Mudança de identidade = alterar `tokens.json`, este documento e
   `Theme.swift` juntos.
3. Ainda não há teste que confira `Theme.swift` contra `tokens.json`.
