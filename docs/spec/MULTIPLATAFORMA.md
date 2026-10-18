# MultiDroid em Mac, Windows e Linux — Rust e Slint

> Especificação aberta em 2026-10-18. Decisão do dono: um código só em Rust,
> com interface em Slint, nos três sistemas, desenvolvido e testado a partir do
> Mac. Substitui a proposta Kotlin/Compose de [`LINUX.md`](LINUX.md) §2.5; o
> resto do LINUX.md (motor leve, GPU, fase 0) continua valendo.
> O comportamento do produto continua definido em [`MAC.md`](MAC.md): esta
> spec diz **como** ele chega aos três sistemas, não **o que** ele faz.

## 1. Decisões

| # | Decisão | Por quê |
|---|---|---|
| X-1 | **Um código em Rust para os três sistemas**, núcleo e interface | cada recurso é feito uma vez; sem runtime (JVM, .NET), memória baixa num app que divide a RAM com vários Androids |
| X-2 | **Interface em Slint** nos três, no visual neo-brutalista de [`design/tokens.json`](../design/tokens.json) | compila para código nativo, renderiza na GPU e roda igual no Mac, no Windows e no Linux; o app já usa visual próprio, não os controles do sistema |
| X-3 | **O app SwiftUI (`mac/`) continua sendo a versão publicada** até a versão Rust alcançar o mesmo ponto no Mac | a 0.2 funciona e está nas mãos de quem baixou; a troca só acontece quando a nova fizer tudo o que a 0.2 faz (§5, critério) |
| X-4 | **Tudo desenvolvido no Mac**; o que o Mac não roda é testado em VM local ou no GitHub Actions | o dono não tem PC Windows nem Linux (§4) |
| X-5 | **Os três sistemas são compilados no GitHub Actions** a cada tag `v*` e publicados na mesma release | um `git push` da tag gera `.dmg`, instalador do Windows e pacotes do Linux (§6) |
| X-6 | Licença das imagens: **nada muda** | o MultiDroid continua sem embutir nem redistribuir imagem do Google; cada pessoa baixa e aceita a licença (MAC.md §5) |

## 2. Arquitetura

```
rust/                         workspace Cargo
├── core/      (multidroid-core)   biblioteca, sem interface
│   ├── catalog    catálogo de imagens: lê os XML do repositório oficial, canal estável,
│   │              SHA-1, licença (porte de Images.swift)
│   ├── install    baixar, conferir, extrair (zip direto, sem depender de ditto/unzip)
│   ├── devices    perfis de aparelho e arquivos do AVD (porte de Catalog.swift)
│   ├── emulator   ligar, pausar, desligar; gRPC com tonic a partir de
│   │              mac/proto/emulator_controller.proto
│   ├── screen     frames: MMAP no Mac e no Linux, memória compartilhada no Windows
│   ├── state      mesas, grupos, ordem, teto de memória, hibernação (state.json)
│   └── platform   o que muda por sistema (pastas, clonefile/reflink/cópia, métricas)
└── app/       (multidroid)        binário: interface Slint sobre o core
```

### 2.1 O que muda por sistema

| | Mac | Windows | Linux |
|---|---|---|---|
| Motor | emulador oficial (arm64) | emulador oficial (x86_64, WHPX) | **leve**: Redroid em contêiner (LINUX.md); **completo**: emulador oficial (x86_64, KVM) |
| Pasta de dados | `~/Library/Application Support/MultiDroid` | `%LOCALAPPDATA%\MultiDroid` | `$XDG_DATA_HOME/multidroid` |
| Duplicar | `clonefile` (APFS) | cópia com bloco (ReFS) ou cópia simples | `FICLONE` (Btrfs/XFS) ou cópia simples |
| Frames | `MMAP` num arquivo | `MMAP` num arquivo (a API aceita `file:///`; medir na fase 0) | `MMAP`; no motor leve, o protocolo do scrcpy |
| Memória e CPU do processo | `proc_pid_rusage` | `GetProcessMemoryInfo` | `/proc/<pid>/status` |
| Pacote | `.dmg` assinado e notarizado | `.msi` (WiX) ou `.exe` (NSIS) | `.AppImage` e `.deb` |

Tudo o que está na coluna é atrás de um `trait` em `core::platform`; o resto do
core não sabe em que sistema está.

### 2.2 Interface

- Mesmas telas do Mac (MAC.md §2.6 e §2.8): painel, mesa em grade de 24
  colunas, quadro de cada aparelho com os botões, novo aparelho, imagens.
- Tokens de cor, borda (2,5), raio (10), sombra dura (4, 4, sem desfoque) e
  movimento (pressão em 70 ms) lidos de `design/tokens.json` na compilação, para
  a interface Rust e a SwiftUI não divergirem.
- Textos em português, como no Mac, separados em um arquivo por idioma desde o
  começo (o Slint tem tradução com gettext).
- Tela do Android desenhada como textura na GPU (renderizador Skia ou FemtoVG do
  Slint), sem passar por imagem a cada frame: é o que a decisão M-2 previa para o
  Mac e ainda não foi feito no SwiftUI.

## 3. Ordem de construção

| Etapa | O que | Testa onde | Pronto quando |
|---|---|---|---|
| **R-0** | Workspace, CI nos três sistemas, uma janela Slint com os tokens | Mac + GitHub Actions | `cargo test` e `cargo build` verdes no Mac, no Windows e no Linux |
| **R-1** | `catalog` e `install`: listar, baixar, conferir e extrair imagens e o emulador | Mac (testes com os XML reais guardados como fixtures) | instala no Mac as mesmas imagens que a 0.2 instala, na mesma pasta |
| **R-2** | `devices` e `state`: criar, duplicar, apagar, ler o `state.json` da 0.2 | Mac | abre os aparelhos e as mesas criados pela 0.2 sem mudar nada |
| **R-3** | `emulator`: ligar sem janela, gRPC, pausar, hibernar, desligar | Mac; Linux x86_64 no GitHub Actions (KVM) | liga e desliga um aparelho nos dois |
| **R-4** | `screen` + mesa: frames na grade, toque, teclas, texto | Mac | 4 celulares e 1 relógio na mesa, como a 0.2 |
| **R-5** | Painel completo e lotes | Mac | paridade com a 0.2 (critério §5) |
| **R-6** | Linux: motor leve (Redroid) | VM Linux ARM no Mac | 8 aparelhos leves ligados com vídeo (LINUX.md, fase 0) |
| **R-7** | Windows: motor completo, pacote | VM Windows 11 ARM no Mac (tela); VM x86_64 alugada na nuvem (Androids) | instala, liga 2 celulares e mostra na mesa |
| **R-8** | Release dos três pelo GitHub Actions | GitHub | uma tag gera `.dmg`, instalador do Windows e pacotes do Linux na mesma release |

### Andamento

| Etapa | Situação |
|---|---|
| R-0 | **Feita** (2026-10-18): workspace, tokens, janela Slint; CI verde no macOS, no Windows e no Linux |
| R-1 | **Feita** (2026-10-18): `catalog`, `install` e `ini`. 12 testes com listas de exemplo (os XML do repositório oficial não entram no repositório do MultiDroid) e 2 testes contra o repositório real (`cargo test -- --ignored`): leem o emulador para os três hosts e instalam a imagem AOSP 33. A extração é feita pela biblioteca de zip, sem `ditto`. `cargo run -p multidroid-core --example instaladas` lista no Mac o que a 0.2 instalou |

## 4. Como testar cada sistema a partir do Mac

O Mac do dono é um M4 com macOS 27, com virtualização aninhada.

| O quê | Onde | Observação |
|---|---|---|
| Núcleo e interface | no Mac | `cargo test`, `cargo run` |
| Linux, motor leve | **VM Linux ARM no Mac** (UTM ou Virtualization.framework) | o protótipo `mac/tools/leve.sh` já roda Redroid assim |
| Linux, emulador oficial | **GitHub Actions** (`ubuntu-latest`, x86_64, com KVM) | o Google só publica o emulador de Linux para x86_64 (conferido em 2026-10-18 no `repository2-3.xml`); roda sem tela nos testes |
| Windows, interface e pacote | **VM Windows 11 ARM no Mac** | o Slint roda em ARM; o instalador é testado ali |
| Windows, Androids rodando | **VM Windows x86_64 alugada por hora** (Azure ou AWS, com virtualização aninhada) | o emulador oficial não existe para Windows ARM; só nos marcos de R-7 |
| Compilar para Windows e Linux sem CI | no Mac, `cargo xwin` e `cargo zigbuild` | opcional; o caminho normal é o CI |

## 5. Critério para a versão Rust substituir a 0.2 no Mac

A versão Rust vira a publicada no Mac quando, num Mac limpo:

1. instala o emulador e as imagens pelo próprio app;
2. cria, duplica, pausa, hiberna, desliga e apaga aparelhos;
3. abre os aparelhos, as mesas e as preferências da 0.2 sem perda;
4. mostra 4 celulares e 1 relógio na mesa com toque, teclas e texto;
5. não gasta mais memória que a 0.2 com os mesmos aparelhos (medir).

Até lá, `mac/` recebe só correções.

## 6. Release dos três sistemas

Workflow `.github/workflows/release.yml`, disparado por tag `v*`:

| Máquina | Gera | Assinatura |
|---|---|---|
| `macos-latest` | `.dmg` | Developer ID + notarização, com o certificado e a senha de app como segredos do repositório |
| `windows-latest` | `.msi` ou `.exe` | sem assinatura no começo: o SmartScreen avisa na primeira vez; certificado de código do Windows é compra à parte |
| `ubuntu-latest` | `.AppImage` e `.deb` (x86_64; arm64 depois) | não se aplica |

Os artefatos vão para a release da tag, e o site aponta um botão por sistema.
Um workflow `ci.yml` roda `cargo fmt --check`, `cargo clippy` e `cargo test` nos
três a cada push.

## 7. Riscos

| Risco | Efeito | O que fazer |
|---|---|---|
| `MMAP` no Windows pode não funcionar como no Mac | frames por gRPC, com mais cópia e CPU | medir em R-7; plano B: frames por gRPC em PNG/RGBA escalados no emulador |
| Duas interfaces durante a transição (SwiftUI e Slint) | trabalho em dobro em recursos novos | recursos novos entram só na versão Rust; `mac/` só recebe correção (§5) |
| Integrações do macOS (menu, atalhos, várias janelas) mais trabalhosas no Slint | detalhes faltando no Mac | listar na paridade do §5 e cobrir antes da troca |
| SmartScreen no Windows sem certificado | desconfiança na primeira abertura | explicar no site; avaliar o certificado quando houver usuários no Windows |
| Teste de Windows com Androids só em VM alugada | bugs de Windows descobertos tarde | fazer R-7 cedo o bastante para medir o `MMAP` (risco 1) |
