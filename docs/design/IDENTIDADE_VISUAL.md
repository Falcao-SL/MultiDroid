# Identidade visual — Falcao Neo-Brutalism

Identidade do Duo no Android (Compose) e no iOS (SwiftUI). É o **mesmo
sistema** do BR Generator (bordas grossas, sombra dura, cor chapada,
tipografia pesada), com acento próprio: o azul. Os **valores** estão em
[`tokens.json`](tokens.json) — este documento explica o uso e especifica os
componentes; nunca repita um número aqui sem que ele esteja lá. O
comportamento das telas está em
[`../spec/FUNCIONALIDADES.md`](../spec/FUNCIONALIDADES.md).

Unidade: `1` = 1 dp (Android) = 1 pt (iOS).

## Princípios

1. **Bordas fortes.** Todo bloco tem borda de `forma.larguraBorda` (2,5) na
   cor `borda` (quase-preta no claro, off-white no escuro).
2. **Sombra dura.** Deslocamento `(4, 4)`, desfoque 0, mesmo raio da forma,
   cor `sombra`. Nenhum componente usa sombra com desfoque.
3. **Cor chapada.** Sem gradiente, sem transparência em superfícies. O texto
   sobre acento é sempre `tintaSobreAcento` (#141414).
4. **Tipografia pesada.** Pesos 600–900, fonte do sistema. Selos em caixa
   alta com espaçamento entre letras.
5. **Movimento físico.** Botão pressionado desliza sobre a própria sombra
   (a sombra some e o bloco anda `(4, 4)` em 70 ms). Sem ripple.
6. **Cantos discretos.** Raio 10 nos blocos.

## Acento e cor por serviço

- O acento do Duo é o **azul** (`cor.acento.azul`): app bar, botão primário,
  botão `Adicionar conta`, chip selecionado, switch ligado (`cor.papel`).
- Cada conta tem uma cor do ciclo `cor.cicloAcentos` (7 cores). É a cor do
  avatar do card, da barra do contêiner e do atalho da conta.
- Cada serviço do catálogo tem uma cor em `dados/servicos.json → cor`, que é
  **uma cor do nosso tema**, nunca a cor da marca real. Não usamos logo, cor
  de marca nem ícone de terceiros (FUNCIONALIDADES §1; CONFORMIDADE_LOJAS).
- Tema escuro: só `cor.escuro` muda; acentos, forma, espaço e tipografia são
  iguais nos dois temas.

## Tokens

| Grupo | Onde | Resumo |
|-------|------|--------|
| Cores base por tema | `cor.claro`, `cor.escuro` | `fundo`, `superficie`, `borda`, `sombra`, `tinta`, `tintaSobreAcento` |
| Acentos | `cor.acento` | azul, rosa, amarelo, verde, ciano, roxo, laranja, vermelho |
| Ciclo de contas | `cor.cicloAcentos` | 7 cores, índice `quantidade % 7` (FUNCIONALIDADES §5.4) |
| Papéis | `cor.papel` | que acento vai em cada elemento |
| Forma | `forma` | borda 2,5; raio 10; sombra (4,4,0); selo (raio 6, −2°); avatar (raio 8); campo (raio 4) |
| Espaço | `espaco` | escala e paddings de cada componente |
| Tamanho | `tamanho` | card de conta, ícones, alvo de toque, switch |
| Tipografia | `tipografia` | família do sistema, pesos, escala `11 12 14 16 18 22`, um estilo por uso |
| Movimento | `movimento` | pressão 70 ms; toast 1800 ms |

## Componentes

Notação: `estilo X` = `tipografia.estilos.X`; cores por nome de token.

### BrutalBox, BrutalButton, BrutalBadge

Iguais aos do BR Generator: caixa base (fundo `superficie` ou acento, borda
2,5, raio 10, sombra dura, padding 16, variante sem sombra para avisos);
botão de ação (fundo `azul`, animação de pressão de 70 ms, texto
`estilo botao` em `tintaSobreAcento`); selo de seção (fundo `borda`, texto
`fundo` em `estilo selo`, sombra dura `azul`, girado −2°, tratado como
cabeçalho por leitores de tela).

### Card de conta (Início)

BrutalBox com fundo `superficie`, padding `espaco.cardConta.padding` (12),
altura mínima = largura ÷ `tamanho.cardConta.proporcao` (1,2).

- **Avatar**: quadrado `tamanho.cardConta.avatar` (44), raio `forma.avatar`
  (8), fundo = cor da conta; dentro, o emoji ou a inicial (`estilo avatar`,
  `tintaSobreAcento`).
- **Apelido**: `estilo apelidoCard`, até 2 linhas, reticências.
- **Serviço**: `estilo servicoCard` em `tinta` 60 % (no personalizado, o
  `dominioBase`).
- Ícone de alfinete (fixada) e de cadeado (bloqueada) no canto superior
  direito, 16, `tinta`.
- Selo `APP` (`cor.papel.seloApp`, roxo) no canto para conta de motor nativo.
- Toda a caixa abre o contêiner; toque longo / menu de contexto abre o menu.

### Chip de serviço

BrutalBox pequena, raio `forma.chip` (8), padding `espaco.chip`; texto
`estilo chip`. Selecionado: fundo `azul`, texto `tintaSobreAcento`. Não
selecionado: fundo `superficie`, texto `tinta`. Rolagem horizontal, gap 8.

### Barra do contêiner (topo)

Faixa com fundo = cor da conta, borda inferior 2,5 `borda`, sem sombra.
Altura mínima = alvo de toque. Da esquerda para a direita: `Voltar` (seta),
avatar pequeno (`tamanho.avatarLista`, 40) + apelido (`estilo tituloBarra`,
`tintaSobreAcento`, toque abre `trocar`), espaço flexível, `Recarregar`,
`Mais opções`. Ícones 22 em `tintaSobreAcento`. Sob a barra, a barra de
progresso fina (`tamanho.barraProgresso`, 3, cor `tinta`) enquanto carrega.

### Barra inferior de navegação

Faixa fundo `superficie`, borda superior 2,5, padding `espaco.barraInferior`.
Quatro botões distribuídos: `Voltar página`, `Avançar página` (desabilitados
= `tinta` 40 %), `Trocar de conta`, `Compartilhar link`. Esconde ao rolar
para baixo, aparece ao rolar para cima (`movimento.barraInferiorMs`, sem
animação com redução de movimento).

### Folha de adicionar conta

`ModalBottomSheet` (Android) / `.sheet` (iOS), fundo `superficie`.

- **Escolher serviço**: título `estilo tituloTela`; no Android build
  `completo`, duas abas (`Versão web` / `App nativo (clonar)`). Lista de
  linhas: chip do serviço (inicial + cor do catálogo) + nome
  (`estilo tituloTile`) + selo de compatibilidade quando não é `boa`
  (`estilo seloCompat`: `LIMITADO` amarelo, `EXPERIMENTAL` laranja). Última
  linha: `Site personalizado`.
- **Formulário**: campos (endereço só no personalizado, apelido), seletor de
  cor (as 7 do ciclo, cada uma um quadrado com borda; a escolhida ganha
  contorno `tinta` de 2), seletor de ícone (`Inicial` + 32 emojis em grade),
  pré-visualização do card, aviso de compatibilidade (BrutalBox sem sombra,
  `estilo aviso`), BrutalButton `Adicionar e entrar`.

### Campos, switch, diálogo, toast

Iguais ao BR Generator: campo contornado (raio 4, borda 1 → 2 `tinta` no
foco, rótulo flutuante `estilo rotuloCampo`); switch de `tamanho.switch`
(trilho `azul` quando ligado); diálogo próprio (BrutalBox sobre véu, título
`estilo tituloDialogo`, botões de texto `estilo botao`, destrutivo em
`vermelho`); toast (fundo `borda`, texto `fundo`, `estilo toast`, 1800 ms).

### Faixa de aviso do contêiner

BrutalBox sem sombra, fundo `cor.papel.avisoContainer` (amarelo), sob a
barra do contêiner: ícone de informação + texto `estilo aviso` em
`tintaSobreAcento` + botão de fechar; até dois botões de ação à direita
(FUNCIONALIDADES §6.4).

### Tela de bloqueio

Cobre tudo: fundo `fundo`, logo do Duo centralizado, texto
`O Duo está bloqueado` (`estilo subtitulo`), BrutalButton `Desbloquear`.
Nenhum conteúdo de conta desenhado atrás.

### Estado vazio

`estilo textoVazio` em `tinta` 50 %, centralizado, com o título, o texto e o
botão do FUNCIONALIDADES §4.5, `espaco.vazio.topo` (48) abaixo da app bar.

## Iconografia

Ícones cheios (filled), na cor do texto ao redor. Nenhum emoji como ícone de
interface (o emoji só existe como avatar escolhido pelo usuário). Nenhum logo
de serviço.

| Uso | Android (Material Icons) | iOS (SF Symbols) |
|-----|--------------------------|------------------|
| Ajustes | `Icons.Filled.Settings` | `gearshape.fill` |
| Adicionar conta | `Icons.Filled.Add` | `plus` |
| Buscar | `Icons.Filled.Search` | `magnifyingglass` |
| Limpar busca | `Icons.Filled.Clear` | `xmark.circle.fill` |
| Fixar / desafixar | `Icons.Filled.PushPin` / `Icons.Outlined.PushPin` | `pin.fill` / `pin.slash` |
| Bloqueada | `Icons.Filled.Lock` | `lock.fill` |
| Voltar (tela) | `Icons.AutoMirrored.Filled.ArrowBack` | `chevron.backward` |
| Voltar/avançar página | `Icons.AutoMirrored.Filled.ArrowBack` / `ArrowForward` | `chevron.backward` / `chevron.forward` |
| Recarregar | `Icons.Filled.Refresh` | `arrow.clockwise` |
| Mais opções | `Icons.Filled.MoreVert` | `ellipsis` |
| Trocar de conta | `Icons.Filled.SwapHoriz` | `arrow.left.arrow.right` |
| Compartilhar link | `Icons.Filled.Share` | `square.and.arrow.up` |
| Copiar link | `Icons.Filled.ContentCopy` | `doc.on.doc` |
| Abrir no navegador | `Icons.Filled.OpenInBrowser` | `safari` |
| Editar | `Icons.Filled.Edit` | `pencil` |
| Limpar dados | `Icons.Filled.CleaningServices` | `eraser` |
| Excluir | `Icons.Filled.Delete` | `trash` |
| Adicionar à tela inicial | `Icons.Filled.AddToHomeScreen` | `plus.app` |
| Organizar | `Icons.Filled.Reorder` | `arrow.up.arrow.down` |
| Tema sistema/claro/escuro | `BrightnessAuto` / `LightMode` / `DarkMode` | `circle.lefthalf.filled` / `sun.max.fill` / `moon.fill` |
| Aviso | `Icons.Filled.Info` | `info.circle.fill` |
| Sem conexão | `Icons.Filled.WifiOff` | `wifi.slash` |

## Ícone do app e splash

- Arte: releitura neo-brutalista de **duas formas iguais deslocadas** (a
  ideia de "duplicar"): dois quadrados de cantos arredondados, um azul e um
  do fundo, com borda grossa e a sombra dura característica, sobre creme.
  Sem qualquer referência a marcas de serviços.
- **Android**: ícone adaptativo (fundo `#E7E3D9` + `foreground`) com camada
  monocromática (ícones temáticos, Android 13+) e ícone legado; splash pela
  SplashScreen API (`core-splashscreen`), com a arte sobre `cor.splash`.
- **iOS**: `AppIcon.icon` (Icon Composer, Xcode 26+) com variantes
  Default/Dark/Clear/Tinted; launch screen por `UILaunchScreen`.
- Arquivos: `docs/design/icone/` quando a arte for produzida.

## Tom de voz

- **Direto, em português do Brasil.** Frases curtas; nada de "Ops!", emoji ou
  exclamação — exceção: os toasts curtos (`Link copiado`).
- **Honesto sobre limites**: dizer o que não funciona (login do Google,
  notificações, apps que não clonam) sem rodeio.
- **Nomes de serviço são só rótulos**: "o site do <serviço>", "o app do
  <serviço>"; nunca sugerir parceria ou selo oficial.
- **Motivos de erro** começam em minúscula, sem ponto final, dizem a regra
  que falhou (`endereço inválido`) — como no BR Generator.

## Como cada plataforma implementa

| Plataforma | Tokens + tema | Componentes | Teste de tokens |
|------------|---------------|-------------|-----------------|
| Android | `android/app/.../ui/theme/DuoTokens.kt` (cópia de `tokens.json`, Kotlin puro) e `DuoTheme.kt` | `.../ui/theme/Brutal.kt` | `TokensTest.kt` lê `docs/design/tokens.json` pelo caminho passado em `duo.design` |
| iOS | `ios/Duo/DesignSystem/DuoTokens.swift` + `Brutal.swift` | idem | `TokensTests.swift` lê `docs/design/tokens.json` a partir de `#filePath` |

Regras:

1. Cada plataforma tem um teste que lê `docs/design/tokens.json` e confere que
   o tema usa exatamente os mesmos valores.
2. Nenhum valor visual literal fora do arquivo de tema de cada plataforma.
3. Mudança de identidade = alterar `tokens.json`, este documento e os dois
   temas no mesmo PR.
