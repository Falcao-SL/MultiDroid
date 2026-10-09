# Status — Duo

> Atualizado em **2026-10-09**. Fonte única de "em que pé está".

| Campo | Valor |
|---|---|
| **Estágio** | **App de Mac em construção.** Fase 0 medida (2026-10-07); fase 1 quase toda feita; fase 2 não começou |
| **Plataforma** | só macOS: Duo para Mac em `mac/` (Swift Package, SwiftUI + gRPC), macOS 26, Apple Silicon |
| **Versão** | 0.1 (build 1), assinatura ad hoc (`scripts/build-app.sh`) |
| **Identificador** | `com.falcaosl.duo.mac` · Apple Team `6EZ89E2F5S` |
| **Distribuição** | Developer ID + notarização, fora da Mac App Store (decisão M-6); ainda não configurada |
| **Monetização** | **A decidir** ([MONETIZACAO.md](MONETIZACAO.md)) |
| **Próximo marco** | Fechar o critério da fase 1: criar, duplicar e organizar 4 celulares + 1 relógio em menos de 5 minutos num Mac limpo |

Em 2026-10-09 o Duo passou a ser só o app de Mac. O plano dos apps de
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
| Quadro | voltar, início, recentes, volume, girar, digitar, captura de tela, pausar, desligar |
| Tela e entrada | `streamScreenshot` por `MMAP` escalado no emulador; toque e teclas por gRPC |
| Instalador | emulador e imagens baixados de dl.google.com, licença no app, SHA-1, atualizar com aparelhos ligados, importar do Android Studio |

## O que falta

- Medir o critério da fase 1 num Mac limpo.
- Fase 2 inteira.
- Desenho por Metal (decisão M-2): hoje a tela é desenhada como imagem no
  SwiftUI.
- Assinatura Developer ID, notarização e canal de download (DISTRIBUICAO).
- Decidir monetização, nome público, idiomas e site.
- Revisar com advogado a licença das imagens do Google antes de vender
  (MAC.md §5).

## Riscos

Os riscos estão em [MAC.md §5](../spec/MAC.md): licença das imagens Play e
Wear, apps que bloqueiam emulador, termos dos serviços, memória em Macs de
8–16 GB e mudanças na API do emulador.
