# Status — MultiDroid

> Atualizado em **2026-10-09**. Fonte única de "em que pé está".

| Campo | Valor |
|---|---|
| **Estágio** | **App de Mac em construção.** Fase 0 medida (2026-10-07); fase 1 quase toda feita; fase 2 não começou |
| **Plataforma** | só macOS: MultiDroid em `mac/` (Swift Package, SwiftUI + gRPC), macOS 26, Apple Silicon |
| **Versão** | 0.1 (build 1), assinatura ad hoc (`scripts/build-app.sh`) |
| **Identificador** | `com.falcaosl.multidroid` · Apple Team `6EZ89E2F5S` |
| **Distribuição** | Developer ID + notarização, fora da Mac App Store (decisão M-6); ainda não configurada |
| **Monetização** | **Nenhuma: código aberto, GPL-3.0** (2026-10-10; [MONETIZACAO.md](MONETIZACAO.md)) |
| **Próximo marco** | Fechar o critério da fase 1: criar, duplicar e organizar 4 celulares + 1 relógio em menos de 5 minutos num Mac limpo |

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
| Instalador | emulador e imagens baixados de dl.google.com, licença no app, SHA-1, atualizar com aparelhos ligados, importar do Android Studio |
| Modo leve (fase 0) | protótipo `mac/tools/leve.sh`: Redroid em contêiner numa VM do Virtualization.framework; medido em 2026-10-09, ~0,85 GB por aparelho com 8 ligados contra ~4,5 GB do emulador (MAC.md §3) |

## O que falta

- Medir o critério da fase 1 num Mac limpo.
- Fase 2 inteira.
- Modo leve: **vídeo não toca** no Redroid 13 (decodificador falha ao mapear o buffer; MAC.md §3, L-5) — Redroid 12, 13 e 14 falham igual; próximo passo é mexer na imagem; depois balão de memória na VM, apps reais por APK sem serviços do Google, tela e toque pelo protocolo do scrcpy.
- Desenho por Metal (decisão M-2): hoje a tela é desenhada como imagem no
  SwiftUI.
- Assinatura Developer ID, notarização e canal de download (DISTRIBUICAO).
- Decidir nome público, idiomas e site; revisar o histórico do git antes de tornar o repositório público.
- Revisar com advogado a licença das imagens do Google antes de tornar o
  repositório público (MAC.md §5).

## Riscos

Os riscos estão em [MAC.md §5](../spec/MAC.md): licença das imagens Play e
Wear, apps que bloqueiam emulador, termos dos serviços, memória em Macs de
8–16 GB e mudanças na API do emulador.
