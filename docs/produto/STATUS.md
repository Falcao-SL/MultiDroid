# Status — MultiDroid

> Atualizado em **2026-10-27**. Fonte única de "em que pé está".

| Campo | Valor |
|---|---|
| **Estágio** | **Publicado: 0.2 no GitHub Releases** (2026-10-18), código aberto. Fase 1 quase toda feita; fase 2 não começou |
| **Plataformas** | Publicado: macOS 26, Apple Silicon. Em pré-release: Mac, Windows e Linux pela versão Rust (Preview 0.3) |
| **Stack** | Swift Package em `mac/`: SwiftUI + gRPC sobre o Android Emulator oficial |
| **Versão** | 0.2 (build 2), assinada com Developer ID (`scripts/build-app.sh`); notarização e `.dmg` por `scripts/release.sh` |
| **Identificador** | `com.falcaosl.multidroid` · Apple Team `6EZ89E2F5S` |
| **Distribuição** | `.dmg` assinado e notarizado no GitHub Releases, fora da Mac App Store (decisão M-6); repositório público |
| **Site** | https://multidroid.falcaosl.com (em `site/`, inglês) |
| **Monetização** | **Nenhuma: código aberto, GPL-3.0** (2026-10-10; [MONETIZACAO.md](MONETIZACAO.md)) |
| **Versão Rust** | Preview 0.3 em `rust/` (Rust + Slint), inglês como língua principal e português como tradução. No Mac: 4 celulares + 1 relógio ligados com toque, teclas e texto, e memória igual à 0.2 (~115 MB com 5 aparelhos, renderizador por software). No Windows: emulador liga aparelhos via WHPX, baixa emulador automaticamente, captura via MMAP e fecha salvando estado (2026-10-22). No Linux: pacotes .deb, .rpm e .AppImage gerados no CI; motor leve (Redroid) com 8 aparelhos. Interface com painel recolhível em trilho, barra inferior de botões no quadro, redimensionamento proporcional e barra de progresso nos downloads com velocidade e ETA. Ver [`../spec/MULTIPLATAFORMA.md`](../spec/MULTIPLATAFORMA.md) |
| **Próximo marco** | Mac: várias mesas e importar do Android Studio pela interface, para a Rust substituir a 0.2. Linux e Windows: testar em máquinas nativas |
| **Data-alvo** | A definir |

Em 2026-10-09 o MultiDroid passou a ser só o app de Mac. O plano dos apps de
celular foi arquivado em [`../arquivo/celular/`](../arquivo/celular/README.md).

## Fases (spec/MAC.md §3)

| Fase | Escopo | Situação |
|------|--------|----------|
| **0 — Provar e medir** | dois emuladores sem janela, telas via `MMAP`, toque; medir memória, CPU, boot, latência | **Feita** (2026-10-07). Android 13 vira o padrão; ~4,5 GB por celular |
| **1 — O app** | perfis de aparelho, criar, duplicar (`clonefile`), grade organizável, estados (pausar/retomar/desligar), instalador próprio | **Quase toda feita**; falta medir o critério num Mac limpo |
| **2 — Recursos** | sincronizar entrada, arrastar arquivo do Finder (APK, foto), atalhos por aparelho | não começou |

## O que já existe (`mac/`)

| Área | Feito |
|------|-------|
| Aparelhos | celular (Pixel 9), tablet (Pixel Tablet), dobrável (Pixel 9 Pro Fold), relógio (Wear OS redondo) |
| Painel | lista, estado, memória/CPU ao vivo, memória do Mac, teto de memória, grupos, ações em lote, criar, duplicar, apagar (renomear ainda não) |
| Mesa | grade livre de 24 colunas, arranjo automático e por grupo, uma mesa por janela (⌘N), menu Mesas, painel recolhível (⌘0) |
| Quadro | voltar, início, recentes, volume, girar, digitar, captura de tela, pausar, hibernar, desligar |
| Hibernação | ao passar do teto de memória, hiberna quem está há mais tempo sem uso; quadro hibernado mostra a última tela e acorda com um clique (MAC.md §2.3) |
| Tela e entrada | `streamScreenshot` por `MMAP` escalado no emulador; toque e teclas por gRPC |
| Instalador | emulador e qualquer imagem arm64 estável do Android 11 em diante, baixados de dl.google.com; licença no app, SHA-1, atualizar com aparelhos ligados, importar do Android Studio, remover com confirmação; cada aparelho escolhe a sua imagem ao ser criado |
| Modo leve (fase 0) | protótipo `mac/tools/leve.sh`: Redroid em contêiner numa VM do Virtualization.framework; medido em 2026-10-09, ~0,85 GB por aparelho com 8 ligados contra ~4,5 GB do emulador (MAC.md §3) |

## O que mudou na 0.2 (2026-10-18)

- O instalador extrai o Wear OS 33 (o `ditto` não lia o zip do Google; agora cai no `unzip`).
- A linha de cada aparelho no painel não quebra mais "ligado" em duas linhas: estado numa linha, memória e CPU na de baixo.
- Site, histórico do git conferido (sem segredos) e repositório aberto.

## Mac, Windows e Linux (2026-10-18)

Decidido: um código em Rust com interface Slint nos três sistemas ([`spec/MULTIPLATAFORMA.md`](../spec/MULTIPLATAFORMA.md)), em `rust/`. A 0.2 SwiftUI continua sendo a versão publicada no Mac até a Rust alcançá-la. Etapas R-0 a R-5 feitas em código (2026-10-18): catálogo e instalador, aparelhos e o `state.json` da 0.2, emulador por gRPC, mesa com a tela por MMAP, Imagens, Novo aparelho, teto, lote e grade. O emulador real liga, pausa, mostra a tela e desliga no CI do Linux. A R-6 está feita: no Linux, o motor leve (Redroid) liga 8 Androids com vídeo ao mesmo tempo, ~1,1 GB cada, com tela e toque pelo protocolo do scrcpy; pacotes `.deb`, `.rpm` e `.AppImage` gerados pelo CI. A R-7 avançou em 2026-10-22 com testes no Windows 11 x86_64 (WHPX): boot em 46 s, emulador baixado ao ligar com platform-tools, captura MMAP e desligamento salvando estado. Interface atualizada com painel lateral recolhível em trilho, botões na barra inferior do quadro, redimensionamento proporcional e barra de progresso nos downloads com velocidade e tempo restante. Os pacotes dos três sistemas saem por tag como "MultiDroid Preview" em pré-release (R-8). Pendente no Mac: ligar 4 celulares e 1 relógio e medir a memória, o que pede espaço em disco (o emulador quer 7,4 GB livres por aparelho novo).

## Aparelho no Android Studio e núcleo nativo (2026-10-27)

Decisão em [ADR-001](../arquitetura/ADR-001-nucleo-nativo-e-android-studio.md). O Run do Studio usa o aparelho do MultiDroid pelo adb da porta 5037. O caminho quente da tela é C no processo hospedeiro (NEON ou SSE2), não código dentro do Android.

| Peça | Neste corte | Depois |
|------|-------------|--------|
| Núcleo C (`rust/native`, CMake) | Carimbo do quadro RGBA no binário Rust; bomba de uma thread; a mesa e o scrcpy passam por ele | Medir o blit com aparelhos ligados; dois buffers; desenho da mesa por GPU (M-2) |
| Ponte adb | Servidor na 5037 antes do emulador; serial `emulator-<console>`; motor leve segue `127.0.0.1:<porta>` | Não deixar um adb de outra versão reiniciar o servidor do Studio |
| Studio | `rust/scripts/studio-adb.sh` e `.ps1`. Não se cria AVD. A 0.2 não troca a porta do adb | Botão na interface com o serial |

Como apontar o Studio: o passo a passo está no ADR-001. Resumo: rodar o script, ligar o aparelho em seguida, escolher `emulator-5600` (ou o par seguinte) no Run.

## O que falta

- Medir o critério da fase 1 num Mac limpo.
- Fase 2 inteira.
- Modo leve: **vídeo não toca** no Redroid 13 (decodificador falha ao mapear o buffer; MAC.md §3, L-5) — Redroid 12, 13 e 14 falham igual; próximo passo é mexer na imagem; depois balão de memória na VM, apps reais por APK sem serviços do Google, tela e toque pelo protocolo do scrcpy.
- Versão Linux: spec escrita ([LINUX.md](../spec/LINUX.md)), sem código; a fase 0 precisa de um PC Linux com GPU Intel ou AMD e 16 GB (o homelab não serve).
- Desenho por Metal (decisão M-2): hoje a tela é desenhada como imagem no
  SwiftUI.
- Pipeline de build e release no GitHub Actions; Homebrew cask depois da primeira release notarizada.
- Interface só em português; decidir os outros idiomas.
- Revisar com advogado a licença das imagens do Google antes de tornar o
  repositório público (MAC.md §5).

## Riscos

Os riscos estão em [MAC.md §5](../spec/MAC.md): licença das imagens Play e
Wear, apps que bloqueiam emulador, termos dos serviços, memória em Macs de
8–16 GB e mudanças na API do emulador.
