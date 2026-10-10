# Duo para Mac — vários Androids lado a lado

> Especificação escrita em 2026-10-07. É o produto Duo (desde 2026-10-09 o Duo
> é só para Mac), em `mac/`. Os
> números marcados **(hipótese)** ainda não foram medidos; a fase 0 existe para
> medi-los.

## 1. O que é

Um app de Mac que roda **vários aparelhos Android ao mesmo tempo**, cada um com
a sua conta, e os mostra organizados numa grade, como os gerenciadores de
múltiplas instâncias do Windows. Resolve o problema de várias contas do mesmo
serviço, cada uma isolada, com aparelhos inteiros em vez de perfis de
navegador: o app nativo do serviço roda de verdade, com notificações, câmera
virtual e tudo o que a versão web não tem. (O plano antigo de apps de celular
está em [`../arquivo/celular/`](../arquivo/celular/README.md).)

Tipos de aparelho desde o começo: **celular, tablet, dobrável e relógio (Wear
OS)**.

Requisitos acima de todos: **o mais rápido e leve possível**, e **um painel
para gerenciar tudo** (seção 2.6).

## 2. Arquitetura

```
Duo.app (SwiftUI + Metal, um processo)
├── Painel ────────── lista de aparelhos, estado, recursos, ações em lote (2.6)
├── Grade ─────────── compõe as telas numa só MTKView; arrastar, agrupar, ampliar
├── Instâncias ────── uma por aparelho: inicia, pausa, retoma, encerra o emulador
├── Frames ────────── streamScreenshot via memória compartilhada (MMAP), escalado
├── Entrada ───────── toque, teclado, roda, gestos → sendTouch / sendKey / injectWheel
├── Sincronizar ───── (opcional) repete a entrada de um aparelho nos marcados
└── Catálogo ──────── perfis de aparelho, imagens instaladas, clones
        │ gRPC (localhost, com token)
        ▼
emulator -avd <clone> -no-window -grpc <porta> -grpc-use-token -no-audio …   × N
(Android Emulator oficial, Apache 2.0, nativo em Apple Silicon via Hypervisor.framework,
 GPU do Mac pelo gfxstream)
```

### 2.1 Por que o emulador oficial

- Roda imagens **arm64** direto no Apple Silicon, sem tradução de instruções.
- Usa a GPU do Mac para o Android (renderização acelerada), o que importa para
  vídeo e rolagem.
- Tem todas as imagens que o Duo quer: celular, tablet e dobrável (mesma
  imagem, perfil de hardware diferente) e Wear OS.
- Roda **sem janela** (`-no-window`) e expõe uma API gRPC
  (`emulator/lib/emulator_controller.proto` no SDK) com o que o Duo precisa:
  `streamScreenshot`, `sendTouch`, `sendKey`, `injectWheel`, `setVmState`,
  `setPosture` (dobrável), `getStatus`. É o mesmo caminho que o Android Studio
  usa para mostrar o emulador dentro da própria janela.

Alternativa mais leve, estudada e adiada: vários Androids em contêiner (Redroid)
dentro de uma única VM Linux do Virtualization.framework. Gasta menos memória
por instância, mas o Linux convidado não tem GPU acelerada nesse framework
(vídeo pesa na CPU), e não há Wear OS nem Play Store. A fase 0 do emulador
mostrou ~4,5 GB por celular; por isso o modo leve ganhou a sua própria fase 0
(seção 3, "Fase 0 do modo leve").

### 2.2 Frames: o caminho mais barato possível

1. Cada instância recebe um buffer de memória compartilhada do tamanho do seu
   quadro na grade.
2. `streamScreenshot` com `format = RGBA8888`, **`width` e `height` juntos** =
   tamanho do quadro (o **emulador** escala, não o Duo; com só a largura ele
   devolve o quadro cheio, medido na fase 0) e `transport = MMAP` com um handle
   `file:///…`: os pixels caem direto no buffer, e a resposta gRPC vem vazia.
3. O emulador só manda frame quando a tela muda: tela parada não custa nada.
4. O buffer vira textura Metal sem cópia (`MTLBuffer` sobre a mesma memória) e a
   grade inteira é desenhada numa passada.

### 2.3 Gastar só com o que está à vista

| Estado do aparelho | O que acontece |
|---|---|
| Em foco (ampliado) | frames no tamanho real, até 60 fps, entrada direta |
| Visível na grade | frames no tamanho do quadro, até 30 fps (hipótese) |
| Fora da tela (rolado) | stream cancelado; o Android segue rodando |
| Minimizado / pausado | `setVmState(PAUSED)`: CPU zero, memória mantida |
| Encerrado | snapshot salvo; volta em segundos com Quick Boot |

### 2.4 Duplicar

Duplicar um aparelho = **clonar a pasta do AVD com `clonefile` do APFS**
(cópia-sob-escrita do sistema de arquivos). É instantâneo e não ocupa espaço até
o clone divergir. Cada clone ganha nome, porta gRPC e token próprios e roda
independente. A imagem de sistema é só leitura e compartilhada por todos.

### 2.5 Perfis de aparelho

| Perfil | Imagem | Tela | RAM (hipótese) |
|---|---|---|---|
| Celular | `android-33;google_apis_playstore;arm64-v8a` | 1080×2400 | 2 GB |
| Tablet | a mesma | 1600×2560 | 3 GB |
| Dobrável | a mesma, hardware dobrável (`setPosture`) | dobrado/aberto | 3 GB |
| Relógio | `android-33;android-wear;arm64-v8a` | 454×454 redondo | 2 GB (mínimo imposto) |

Ajustes de leveza em todos: `-no-audio` quando não precisa, `-no-boot-anim`,
`-gpu host`, núcleos limitados (2 por celular, 1 por relógio), Quick Boot.

### 2.6 Painel

A tela principal do app, ao lado da grade (barra lateral ou aba). Tudo o que se
faz com os aparelhos sai daqui:

| Área | O que mostra / faz |
|---|---|
| Lista de aparelhos | nome, tipo (celular, tablet, dobrável, relógio), conta/grupo, estado (ligado, pausado, desligado, ligando), versão do Android |
| Recursos | memória e CPU de cada aparelho e do total, ao vivo; memória livre do Mac e alerta antes do swap |
| Ações por aparelho | ligar, pausar, retomar, desligar, reiniciar, duplicar, renomear, apagar, abrir em tela cheia |
| Ações em lote | selecionar vários e ligar/pausar/desligar/duplicar de uma vez; ligar um grupo inteiro |
| Grupos | agrupar aparelhos por conta ou projeto; a grade segue os grupos |
| Criar aparelho | escolher perfil (2.5), imagem, nome, grupo; mostra quanto de memória vai custar antes de criar |
| Imagens | imagens instaladas, espaço em disco, baixar/remover (com a licença do Google mostrada) |
| Sincronizar | marcar quais aparelhos repetem a entrada do aparelho em foco |
| Limite de memória | o usuário define um teto; o painel não liga um aparelho que passaria dele |

### 2.7 Independência: o Duo instala o que usa

O Duo não depende de Android Studio, Java, `sdkmanager`, `avdmanager` nem `adb`.
Tudo mora em `~/Library/Application Support/Duo/` (`sdk/`, `avd/`, `state.json`).

| Antes | Agora | Como |
|---|---|---|
| `sdkmanager` | **Imagens** no app | lê `repository2-3.xml` e `sys-img/<tag>/sys-img2-3.xml` em dl.google.com, canal estável, arquivo `macosx`/`aarch64`; baixa, confere o SHA-1 publicado, extrai com o `ditto` do macOS e troca a pasta de uma vez |
| licença no terminal | folha de licença no app | texto vindo do próprio XML; o aceite fica em `sdk/licenses/<id>` com o SHA-1 do texto (formato do sdkmanager): se o Google mudar o texto, o Duo pede de novo |
| `avdmanager create` | `Catalog.create` | escreve `<nome>.ini` e `config.ini` com os campos que o avdmanager grava para cada hardware (dobradiça do Pixel 9 Pro Fold incluída); o emulador cria o disco do usuário no primeiro boot |
| `adb emu kill` | console do emulador | TCP local na porta do console, `auth` com `~/.emulator_console_auth_token`, depois `kill`: desliga salvando o estado. Sem resposta, SIGTERM (desliga limpo, sem salvar) |
| Android Studio instalado | opcional | se existir, **Copiar do Studio** clona a imagem com `clonefile` (instantâneo, sem disco extra) e **Trazer** move aparelhos de `~/.android/avd` |

O emulador exige uma pasta `platform-tools` dentro do SDK, mas aceita vazia. O
Duo roda com `-no-metrics` (sem coleta de uso) e aponta `ANDROID_SDK_ROOT`,
`ANDROID_AVD_HOME` e `ANDROID_EMULATOR_HOME` para a própria pasta.

Medido em 2026-10-07: aparelho escrito à mão liga em 18 s (relógio) e 22,5 s
(dobrável) a frio; depois do `kill` pelo console, volta em 2 s pelo estado salvo,
inclusive depois de mover a pasta do aparelho. A licença das imagens Play e
Wear proíbe redistribuir: o Duo nunca embute imagem; cada usuário aceita e
baixa do Google (decisão M-5).

### 2.8 Mesa e grade

Três arranjos, no topo da área dos aparelhos:

- **Grade** (padrão): 24 colunas de células quadradas que acompanham a largura
  da janela. Cada aparelho tem lugar e tamanho em células, salvos em
  `state.json`. Move pelo título; redimensiona pelos quatro lados e cantos, com
  o cursor de redimensionar do macOS. Quadro que cai sobre outro empurra o de
  baixo; um tracejado mostra onde vai encaixar.
- **Automática**: procura a maior escala em que todos cabem lado a lado na
  área visível. Só sobrepõe os quadros, como cartas, quando nem na escala
  mínima cabem; o quadro sob o mouse vem para a frente.
- **Por grupo**: uma mesa automática por grupo.

Cada janela é uma **mesa**. A primeira é a principal; ⌘N abre uma mesa vazia,
e os aparelhos criados nela ficam nela (`workspaceOf` em `state.json`).
Arranjo, aparelho ampliado e painel recolhido são de cada janela. Fechar a
janela não desliga os aparelhos; o menu **Mesas** reabre a mesa fechada, e
mesa vazia fechada some.

Cada quadro tem uma coluna de botões à direita: voltar, início, recentes,
volume, girar, digitar, captura, pausar e desligar. Em quadro baixo, os extras
vão para um menu. O painel lateral recolhe com ⌘0.

## 3. Fases

### Fase 0 — Provar e medir (1 semana)

- Dois emuladores sem janela, telas na mesma janela SwiftUI via `MMAP`, toque
  funcionando.
- Medir no M4 de 24 GB: memória e CPU por instância parada e rolando um feed,
  tempo de partida a frio e com Quick Boot, latência do toque à tela.
- **Critério:** quantas instâncias de celular cabem sem swap (`vm_stat`), e se a
  grade de 4 fica fluida.

### Fase 1 — O app (3–4 semanas)

- Catálogo de perfis, criação de aparelhos, duplicar por `clonefile`.
- Grade organizável (arrastar, agrupar por conta, ampliar com dois cliques),
  estados da tabela 2.3.
- Instalador próprio (2.7): baixa emulador e imagens do repositório do
  Google, mostrando a licença ao usuário. Feito.
- **Critério:** criar, duplicar e organizar 4 celulares + 1 relógio em menos de
  5 minutos num Mac limpo.

### Fase 2 — Recursos (3–4 semanas)

- Sincronizar entrada entre aparelhos marcados.
- Arrastar arquivo do Finder para instalar APK ou enviar foto.
- Atalhos de teclado por aparelho; perfis salvos por conta do Duo.

### Resultado da fase 0 (2026-10-07, M4 de 24 GB)

| Medida | Resultado |
|---|---|
| Boot a frio do celular (Android 37 com Play) | 18 s |
| Volta do snapshot (Quick Boot) | 7–13 s |
| Duplicar por `clonefile` do APFS | 28 ms |
| Frames via `MMAP` a 360×808, rolando a tela | 17,5 fps, 0 MB/s pelo gRPC |
| Frames pelo gRPC comum a 360×808 | 19 fps, 22 MB/s |
| Frames sem escala (só largura) | quadro cheio 1080×2424, 185 MB/s |
| Envio de um toque | 2,4 ms |
| **Memória de um celular parado** | **6,4–6,7 GB** de footprint: o emulador **força 4 GB de RAM** nas imagens Play do Android 37 ("Increasing RAM size to 4096MB"), ignorando `hw.ramSize` e `-memory` |
| 2 celulares + 1 relógio | 3,6 GB para o swap |
| Relógio (`android-37.0;android-wear-signed`) | **não liga**: `offline` com erros `Failed to find ColorBuffer`, com e sem janela, com GPU do Mac e por software |

Comparação de imagens (`mac/tools/measure-image.sh`, uma por vez, boot a frio,
pedindo 2 GB ao celular e 1 GB ao relógio, medida após 45 s parado):

| Imagem | Boot | RAM que o emulador impõe | Footprint do processo | CPU parado |
|---|---|---|---|---|
| Android 37 Play | 18 s | 4 GB | 7,4 GB | 0,2% |
| Android 36 Play | 25 s | 2,5 GB | 5,3 GB | 17% |
| Android 36 `google_apis` | 24 s | 2,5 GB | 5,3 GB | 7% |
| **Android 13 (API 33) Play** | 22 s | aceita 2 GB | **4,5 GB** | 5,7% |
| Android 13 ATD (sem Play) | 21 s | aceita 2 GB | 3,8 GB | 0,9% |
| Wear OS 37 / 36 | — | — | **não liga** | — |
| **Wear OS 13 (API 33)** | **16 s** | 2 GB | 4,1 GB | 5,2% |

Consequências:

1. **Imagem padrão: Android 13 (API 33) com Play** para celular, tablet e
   dobrável, e **Wear OS 13** para o relógio, a única versão de Wear que liga
   neste emulador.
2. Tirar a Play Store não muda a memória; quem pesa é o próprio emulador: o
   processo custa ~2–3 GB **além** da RAM do Android, em todas as imagens
   (provavelmente o `gfxstream` via Vulkan/Metal). Próximo teste: modos de
   GPU mais econômicos.
3. Com 4,5 GB por celular, um Mac de 24 GB comporta ~3–4 aparelhos sem swap,
   e um de 16 GB ~2. O painel precisa mostrar esse custo antes de ligar
   (seção 2.6).
4. Frames e entrada estão provados: `MMAP` + escala no emulador é o caminho.

### Fase 0 do modo leve — Androids em contêiner (aberta em 2026-10-09)

**Pergunta:** quanto custa cada aparelho quando vários Androids dividem uma
só VM Linux, em vez de um emulador inteiro por aparelho? Se o custo cair
para perto de 1 GB, o Duo ganha um segundo tipo de aparelho, **Leve**, ao lado
do **Completo** (o emulador de hoje).

**Arquitetura medida**

```
Mac ── Virtualization.framework ── 1 VM Linux arm64 (Ubuntu 24.04, kernel 6.8, páginas de 4 KB)
                                     ├── binder_linux (módulo do kernel; binderfs montado pelo Redroid)
                                     ├── Docker
                                     ├── Redroid Android 13 (64 bits) nº 1 ┐ mesmo kernel, mesma
                                     ├── Redroid Android 13 (64 bits) nº 2 ├ imagem do sistema em
                                     └── …                                 ┘ cache, uma vez só
```

- **Por que uma VM com vários contêineres**, e não o framework
  Containerization da Apple: o da Apple roda uma VM leve **por contêiner**, o
  que desfaz justamente o que se quer medir — kernel, cache da imagem e
  páginas iguais compartilhados entre aparelhos.
- **Por que Lima no protótipo:** ele cria a VM com o mesmo
  Virtualization.framework (`--vm-type=vz`) que o Duo embutiria; o protótipo
  troca só quem liga a VM, não o hipervisor. Embutir a VM no app fica para
  depois, se o modo leve passar.
- **Imagem:** `redroid/redroid:13.0.0_64only-latest` (AOSP + Redroid,
  Apache 2.0). Ao contrário das imagens do Google, **pode ser embutida e
  redistribuída** pelo Duo — o modo leve não tem o risco de licença da seção 5.
- **GPU:** só software (`androidboot.redroid_gpu_mode=guest`). O Linux
  convidado do Virtualization.framework não tem aceleração 3D.
- **Tela e toque no protótipo:** `scrcpy` pelo adb do contêiner, só para
  olhar. No app, o caminho provável é o mesmo protocolo do scrcpy (vídeo
  H.264 decodificado pelo VideoToolbox), a decidir depois da medição.

**Ferramenta:** `mac/tools/leve.sh` (`setup`, `up <N> [ksm]`, `medir`,
`rolar <i>`, `tela <i>`, `down`, `destruir`). VM com 6 núcleos e 12 GB; cada
Android a 720×1600, 320 dpi, 30 fps.

**O que medir** (no mesmo M4 de 24 GB da fase 0 do emulador, 45 s depois do
boot, como no `measure-image.sh`):

| # | Medida | Como |
|---|---|---|
| L-1 | Custo da VM vazia | `medir` sem nenhum Android ligado |
| L-2 | Custo de 1, 4 e 8 Androids | footprint do processo da VM no Mac e memória usada dentro da VM; o custo por aparelho é a diferença dividida por N |
| L-3 | Ganho da fusão de páginas (KSM) | repetir L-2 com `up <N> ksm`: o `ksm-exec` liga `PR_SET_MEMORY_MERGE` antes do `/init` do Android |
| L-4 | Boot | segundos até `sys.boot_completed` |
| L-5 | Fluidez com GPU em software | `rolar`: 20 rolagens em Configurações, quadros com atraso e percentis do `gfxinfo`; um feed de vídeo olhado no `scrcpy` |
| L-6 | Memória devolvida ao Mac | desligar os Androids e ver se o footprint da VM cai (a VM do Lima não tem balão de memória; o app teria de ter) |

**Critério para seguir:** com 8 Androids ligados, **≤ 1,5 GB por aparelho**
no footprint do Mac e rolagem sem atraso perceptível (menos de 10% de
quadros com atraso). Passando, o modo leve entra no plano como tipo de
aparelho **Leve**: só celular e tablet, sem Play Store (apps por APK), sem Wear,
GPU em software. Não passando, fica registrado aqui e o Duo segue só com o
emulador e com a hibernação de quem está fora de foco.

### Resultado da fase 0 do modo leve (2026-10-09, M4 de 24 GB)

O kernel genérico do Ubuntu 24.04 arm64 traz o binder como módulo
(`linux-modules-extra`), e ele carrega dentro da VM do Virtualization.framework.
O Redroid Android 13 sobe com a tela inicial, e o adb do Mac chega nele pela
porta encaminhada. Cada rodada começou com a VM reiniciada (`leve.sh reiniciar`).

| # | Medida | Resultado |
|---|---|---|
| L-1 | VM vazia | **1,2 GB** de footprint no Mac (0,4 GB usados dentro da VM) |
| L-2 | 1 Android | 3,0 GB no Mac; +0,7 GB usados na VM, +1,1 GB de cache da imagem |
| L-2 | 4 Androids | 5,2 GB no Mac → **~1,0 GB por aparelho**; 3,2 GB usados na VM, cache 1,7 GB (a imagem fica uma vez só) |
| L-2 | **8 Androids** | 8,0 GB no Mac → **~0,85 GB por aparelho**; ~0,6 GB por contêiner dentro da VM |
| L-3 | 8 Androids com KSM | o KSM funde **~1,1 GB** de páginas iguais dentro da VM (o `ksm-exec` funciona), mas o footprint no Mac não cai (8,1 GB): ver L-6 |
| L-4 | Boot | **4–7 s** por Android, contra 18–22 s do emulador (não há kernel para ligar) |
| L-5 | Rolagem com GPU em software | 0,74% de quadros com atraso; mediana de 34–36 ms por quadro (~29 fps, no limite de 30 fps configurado), igual com 4 e com 8 ligados. Feed de vídeo ainda não olhado |
| — | CPU com 8 parados | 31% de um núcleo no total (~4% por aparelho; o emulador fica em 5,7%) |
| L-6 | Desligar os 8 | o footprint **continua em 8,1 GB**: a VM não devolve ao macOS a memória que já tocou |

Consequências:

1. **O critério passou.** ~0,85 GB por aparelho contra ~4,5 GB do emulador:
   **cerca de 5 vezes menos**. Um Mac de 24 GB comporta uns 15–18 aparelhos
   leves ligados em vez de 3–4; um de 16 GB, uns 8–10 em vez de 2.
2. **A VM do app precisa de balão de memória**
   (`VZVirtioTraditionalMemoryBalloonDevice`). Sem ele, desligar aparelhos
   não devolve nada ao Mac, e o ganho do KSM fica preso dentro da VM. O Duo
   encolhe a VM pelo balão quando aparelhos desligam; com isso, o KSM deve
   levar o custo para perto de 0,7 GB (a medir depois do balão).
3. **A rolagem simples está boa, mas sem folga**: a GPU em software segura
   30 fps e não 60. Falta olhar um feed de vídeo (`leve.sh tela`) antes de
   prometer redes sociais no modo leve.
4. **O que falta provar:** apps reais por APK sem os serviços do Google
   (testar Instagram, TikTok, WhatsApp; microG como opção), vídeo, e o caminho
   de tela e toque no app (protocolo do scrcpy com VideoToolbox) — próxima
   rodada antes de entrar no plano como tipo **Leve**.

## 4. Decisões

| # | Decisão | Por quê |
|---|---|---|
| M-1 | Emulador oficial, não motor próprio | GPU acelerada, todas as imagens, API pronta; o diferencial do Duo é a organização e a leveza, não a virtualização |
| M-2 | SwiftUI + Metal, um processo | o app em si não pode competir por memória com os Androids |
| M-3 | Frames por `MMAP`, escalados no emulador | é o caminho com menos cópia e menos CPU que a API oferece |
| M-4 | Duplicar com `clonefile` do APFS | instantâneo e sem custo de disco |
| M-5 | Todos os tipos de aparelho desde o começo, com imagens Google Play e Wear (2026-10-07) | decisão do dono, ciente do risco de licença abaixo |
| M-6 | Distribuição fora da Mac App Store (Developer ID, notarizado) | o app baixa e executa imagens de sistema e usa o hypervisor; a MAS não aceita |

## 5. Riscos

| Risco | Efeito | O que fazer |
|---|---|---|
| **Licença das imagens do Google** (Play e Wear): o SDK é licenciado "somente para desenvolver aplicativos" e não pode ser redistribuído | produto de consumo em área cinzenta | o Duo **nunca** embute nem redistribui imagens: o usuário as baixa do Google pelo instalador, aceitando a licença; oferecer imagens AOSP (sem Google) como opção; revisar com advogado antes de vender |
| Apps que bloqueiam emulador (integridade do Play) | bancos e alguns apps não abrem | dizer na ficha; não tentar burlar a verificação |
| Termos dos serviços (várias contas, automação) | conta do usuário suspensa | o Duo isola contas; a sincronização de entrada é manual e visível, sem roteiro automático |
| Memória: ~4,5 GB por celular no emulador | 2 aparelhos num Mac de 16 GB | hibernar quem está fora de foco; modo leve em contêiner (fase 0 do modo leve) |
| Mudanças no emulador | quebra da API gRPC | fixar a versão do emulador por release do Duo; testes contra o proto |
