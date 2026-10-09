# Duo

**Vários aparelhos Android lado a lado no seu Mac.** Celular, tablet,
dobrável e relógio (Wear OS), cada um com as suas contas, organizados numa
mesa que você arruma do seu jeito. O Duo é um app só para Mac.

| | |
|---|---|
| **Código** | [`mac/`](mac) — Swift Package (SwiftPM), SwiftUI + gRPC |
| **Requisitos** | macOS 26, Apple Silicon |
| **Versão** | 0.1, em construção |
| **Identificador** | `com.falcaosl.duo.mac` |
| **Spec** | [`docs/spec/MAC.md`](docs/spec/MAC.md) |

## Como funciona

- Cada aparelho é um **Android Emulator oficial** rodando sem janela
  (`-no-window`); o Duo fala com ele por **gRPC** em localhost, com token.
- A tela chega por memória compartilhada (`MMAP`), já escalada pelo emulador
  para o tamanho do quadro; toque, teclas e texto voltam pelo mesmo canal.
- **Painel** com a lista de aparelhos, estado, memória e CPU ao vivo, teto de
  memória, grupos e ações em lote (ligar, pausar, desligar, duplicar).
- **Mesa**: grade livre de 24 colunas (arrastar, redimensionar), arranjo
  automático ou por grupo; uma mesa por janela (⌘N).
- **Duplicar** um aparelho é clonar a pasta com `clonefile` do APFS:
  instantâneo e sem ocupar disco até divergir.
- **Instalador próprio**: o Duo baixa o emulador e as imagens do repositório
  do Google, mostra a licença, confere o SHA-1 e guarda tudo em
  `~/Library/Application Support/Duo/`. Não precisa de Android Studio, Java
  nem `adb`. Imagem padrão: Android 13 com Google Play (Wear OS 4 no relógio).

Não automatiza contas nem burla a verificação de integridade dos apps.

## Compilar

```sh
cd mac
scripts/build-app.sh          # gera build/Duo.app (assinatura ad hoc)
```

## Documentação

Índice em [docs/README.md](docs/README.md). Principais:

| Documento | Conteúdo |
|---|---|
| [docs/spec/MAC.md](docs/spec/MAC.md) | Especificação: arquitetura, painel, mesa, instalador, fases, medições, decisões, riscos |
| [docs/produto/STATUS.md](docs/produto/STATUS.md) | Estágio, o que existe, o que falta |
| [docs/produto/PRODUTO.md](docs/produto/PRODUTO.md) | O que é, para quem, o que não faz |
| [docs/design/IDENTIDADE_VISUAL.md](docs/design/IDENTIDADE_VISUAL.md) | Marca, tokens, ícone |
| [docs/arquivo/celular/](docs/arquivo/celular/README.md) | Plano antigo dos apps de celular (descontinuado) |

---

Um app da [Falcao SL](https://falcaosl.com).
