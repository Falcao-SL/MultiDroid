# Arquitetura — MultiDroid

O MultiDroid é **um app de Mac**: um processo SwiftUI que comanda vários Android
Emulators oficiais sem janela, por gRPC em localhost. O desenho completo
(frames, estados, duplicar, perfis, painel, instalador, mesa) está em
[spec/MAC.md §2](../spec/MAC.md#2-arquitetura); aqui fica só o mapa.

```
MultiDroid/
├── mac/                        o app
│   ├── Package.swift           Swift Package, macOS 26; grpc-swift 2 + swift-protobuf
│   ├── Sources/MultiDroid/            código (abaixo)
│   │   └── Generated/          cliente gRPC gerado de proto/ (não editar)
│   ├── proto/                  emulator_controller.proto, copiado do SDK
│   ├── scripts/                build-app.sh (monta MultiDroid.app), gen-grpc.sh
│   └── tools/                  measure-image.sh (medições da fase 0)
└── docs/                       spec, design, produto, publicação, arquivo
```

## Peças do código

| Arquivo | Papel |
|---------|-------|
| `MultiDroidApp.swift` | entrada do app, janelas (uma mesa por janela), menus |
| `Model.swift` | estado observável: aparelhos, grupos, mesas, teto de memória; liga/pausa/desliga; `state.json` |
| `Catalog.swift` | perfis de aparelho, criar AVD (`config.ini`), duplicar por `clonefile`, importar do Android Studio |
| `SDK.swift` | pastas do MultiDroid, ambiente do emulador, console do emulador (desligar salvando) |
| `Images.swift` / `ImagesView.swift` | instalador: repositório do Google, licença, download, SHA-1, troca da pasta |
| `Screen.swift` | conexão gRPC por aparelho: status, tela por `MMAP`, toque, teclas, pausa |
| `Metrics.swift` | memória (physical footprint) e CPU por processo |
| `Views.swift`, `Desk.swift`, `Mesa.swift`, `GridBoard.swift` | painel, mesa, grade livre, quadro de cada aparelho |
| `Theme.swift` | cores e componentes Brutal (de `docs/design/tokens.json`) |

## Onde ficam os dados

| O quê | Onde |
|-------|------|
| Emulador e imagens | `~/Library/Application Support/MultiDroid/sdk/` |
| Aparelhos (AVDs) | `~/Library/Application Support/MultiDroid/avd/` |
| Estado do app (ordem, grupos, mesas, posições, teto) | `~/Library/Application Support/MultiDroid/state.json` |
| Capturas de tela | `~/Pictures/MultiDroid/` |

## Princípios

1. **O app não compete por memória** com os Androids: um processo, sem
   trabalho com o que não está à vista (MAC.md §2.3).
2. **Emulador oficial, não motor próprio** (M-1); a API gRPC é fixada por
   versão do emulador (`proto/`, `gen-grpc.sh`).
3. **Independente**: nada de Android Studio, Java, `sdkmanager` ou `adb`
   (MAC.md §2.7).
4. **Nada sai do Mac por causa do MultiDroid**: só downloads do Google que o
   usuário pede; o emulador roda com `-no-metrics`.
