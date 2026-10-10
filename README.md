# MultiDroid

**Um ambiente para rodar várias contas e aparelhos Android ao mesmo tempo,
no seu Mac.** Celular, tablet, dobrável e relógio (Wear OS), cada um com as
suas contas, organizados numa mesa que você arruma do seu jeito. Só para Mac;
até 2026-10-10 se chamava Duo.

| | |
|---|---|
| **Código** | [`mac/`](mac) — Swift Package (SwiftPM), SwiftUI + gRPC |
| **Requisitos** | macOS 26, Apple Silicon |
| **Versão** | 0.1, em construção |
| **Identificador** | `com.falcaosl.multidroid` |
| **Spec** | [`docs/spec/MAC.md`](docs/spec/MAC.md) |
| **Licença** | código aberto, [GPL-3.0](LICENSE); grátis, sem monetização |

## Como funciona

- Cada aparelho é um **Android Emulator oficial** rodando sem janela
  (`-no-window`); o MultiDroid fala com ele por **gRPC** em localhost, com token.
- A tela chega por memória compartilhada (`MMAP`), já escalada pelo emulador
  para o tamanho do quadro; toque, teclas e texto voltam pelo mesmo canal.
- **Painel** com a lista de aparelhos, estado, memória e CPU ao vivo, teto de
  memória, grupos e ações em lote (ligar, pausar, desligar, duplicar).
- **Mesa**: grade livre de 24 colunas (arrastar, redimensionar), arranjo
  automático ou por grupo; uma mesa por janela (⌘N).
- **Duplicar** um aparelho é clonar a pasta com `clonefile` do APFS:
  instantâneo e sem ocupar disco até divergir.
- **Instalador próprio**: o MultiDroid baixa o emulador e as imagens do repositório
  do Google, mostra a licença, confere o SHA-1 e guarda tudo em
  `~/Library/Application Support/MultiDroid/`. Não precisa de Android Studio, Java
  nem `adb`. Imagem padrão: Android 13 com Google Play (Wear OS 4 no relógio).

Não automatiza contas nem burla a verificação de integridade dos apps.

## Compilar

```sh
cd mac
scripts/build-app.sh          # gera build/MultiDroid.app (assinatura ad hoc)
```

## Documentação

Índice em [docs/README.md](docs/README.md). Principais:

| Documento | Conteúdo |
|---|---|
| [docs/spec/MAC.md](docs/spec/MAC.md) | Especificação: arquitetura, painel, mesa, instalador, fases, medições, decisões, riscos |
| [docs/spec/LINUX.md](docs/spec/LINUX.md) | Proposta da versão Linux: Androids em contêiner com a GPU do PC; fase 0 a medir |
| [docs/produto/STATUS.md](docs/produto/STATUS.md) | Estágio, o que existe, o que falta |
| [docs/produto/PRODUTO.md](docs/produto/PRODUTO.md) | O que é, para quem, o que não faz |
| [docs/design/IDENTIDADE_VISUAL.md](docs/design/IDENTIDADE_VISUAL.md) | Marca, tokens, ícone |
| [docs/arquivo/celular/](docs/arquivo/celular/README.md) | Plano antigo dos apps de celular (descontinuado) |

---

Um app da [Falcao SL](https://falcaosl.com).
