# MultiDroid para Linux — Androids em contêiner, com a GPU do PC

> **2026-10-18:** a interface e a linguagem mudaram para Rust e Slint nos três sistemas
> ([`MULTIPLATAFORMA.md`](MULTIPLATAFORMA.md)); a proposta Kotlin/Compose do §2.5
> fica só como histórico. O motor leve, a GPU e a fase 0 abaixo continuam valendo.

> Especificação escrita em 2026-10-10. Ainda não há código: o MultiDroid é só
> Mac ([`MAC.md`](MAC.md)). Os números marcados **(hipótese)** não foram
> medidos; a fase 0 existe para medi-los. Windows fica fora desta spec (seção 6).

## 1. O que é

O mesmo produto do Mac: **vários aparelhos Android ao mesmo tempo**, cada um
com a sua conta, numa grade, com um painel para gerenciar tudo. Muda o motor.

No Mac, cada aparelho é um emulador inteiro (~4,6 GB) porque o macOS não roda
Android direto. O modo leve, Redroid numa VM Linux, caiu para ~0,85 GB por
aparelho, mas a VM não tem GPU e **o vídeo não toca** (MAC.md, "Resultado da
fase 0 do modo leve"). No Linux, esses dois obstáculos somem:

- **Não há VM:** o kernel do PC já é Linux. Os Androids rodam como contêineres
  direto nele, com o binder do próprio kernel. A memória volta ao sistema
  quando um aparelho desliga, o que no Mac exigiria um balão de memória.
- **Há GPU:** o contêiner usa a placa de vídeo do PC pelo `/dev/dri`, com o
  Mesa. É isso que deve destravar o vídeo (hipótese a medir: L-5).

Se a fase 0 confirmar as duas coisas, o Linux é onde o MultiDroid cabe mais
aparelhos por GB: **15–20 em 16 GB** (hipótese), contra 3–4 no Mac.

Público: quem já usa Linux no desktop e quem tem um PC ou servidor sobrando
para deixar contas Android ligadas. Quase não há concorrência: os
gerenciadores de múltiplas instâncias populares (BlueStacks, LDPlayer, MuMu)
são só Windows.

## 2. Arquitetura

```
MultiDroid (app de desktop Linux, um processo)
├── Painel ─────── a mesma lista, estados, teto de memória e lotes do Mac (MAC.md 2.6)
├── Grade ──────── telas lado a lado; vídeo H.264 decodificado na GPU (VA-API)
└── Motores
    ├── Leve ───── Redroid em contêiner, sem VM (padrão)
    │              PC Linux ── kernel com binder ── Podman/Docker
    │                           ├── Android nº 1 ┐ mesmo kernel e mesma imagem em cache;
    │                           ├── Android nº 2 ├ GPU do PC por /dev/dri (Mesa);
    │                           └── …            ┘ KSM funde as páginas iguais
    └── Completo ── emulador oficial do Google com KVM, imagens com Play (como no Mac)
```

### 2.1 Dois motores, como no Mac

| | **Leve** (Redroid) | **Completo** (emulador) |
|---|---|---|
| Memória por aparelho | ~0,85 GB (medido no Mac; hipótese no Linux) | ~4,5 GB (hipótese: como no Mac) |
| Boot | 4–7 s (medido no Mac) | ~20 s |
| Play Store | **não** (AOSP; apps por APK ou loja alternativa) | sim |
| Licença da imagem | Apache 2.0: o MultiDroid pode embutir e distribuir | do Google: baixada pelo usuário, como no Mac |
| GPU | a do PC, por `/dev/dri` | a do PC, pelo emulador |

O Leve é o motivo de existir a versão Linux. O Completo fica para quem
precisa da Play Store e de uma conta Google certificada, e reaproveita quase
tudo do Mac (catálogo, licença, perfis), trocando as imagens arm64 pelas
x86_64 num PC Intel ou AMD.

### 2.2 Arquitetura do processador e apps só-ARM

Num PC Intel ou AMD, o Redroid roda Android x86_64. Muitos apps de conta
(redes sociais, bancos, mensageiros) trazem bibliotecas **só para ARM**. Para
eles abrirem num Android x86_64 é preciso um tradutor de instruções ARM
(`libndk_translation` do Google ou `libhoudini` da Intel), e nenhum dos dois
tem licença que permita ao MultiDroid distribuí-lo. É o maior risco de produto
desta spec (seção 5).

Saídas, em ordem de preferência:

1. **Linux em ARM64** (Ampere, Raspberry Pi 5, Snapdragon X com Linux): o
   Android arm64 roda os apps sem tradução. Público menor, mas sem o problema.
2. **PC x86_64 com tradução baixada pelo usuário**, do mesmo jeito que as
   imagens do Google no Mac: o MultiDroid busca da fonte original e o usuário
   aceita os termos. A fonte e a licença precisam de revisão antes.
3. **PC x86_64 sem tradução:** só apps com biblioteca x86_64 ou sem código
   nativo. A fase 0 mede quantos dos apps-alvo entram aqui (L-7).

### 2.3 Tela e toque

O Redroid não tem a memória compartilhada do emulador (MAC.md 2.2). O caminho
é o protocolo do `scrcpy`: o Android codifica a tela em H.264 na própria GPU
e o app decodifica com VA-API, também na GPU. Toque e teclado vão pelo mesmo
canal. O hibernar do Mac (2.3) vira **congelar** o contêiner (`podman pause`,
cgroup freezer): instantâneo, e a memória continua dele até desligar.

### 2.4 O que vem do Mac

| Do Mac | No Linux |
|---|---|
| Lógica de aparelhos: ligar, hibernar, teto de memória, grupos, mesa | mesma regra, reescrita (ver 2.5) |
| Catálogo de imagens do Google (`Images.swift`) | só no motor Completo, com `x86_64` no lugar de `arm64-v8a` |
| Perfis de aparelho (celular, tablet, dobrável, relógio) | mesmos perfis; no Leve viram resolução e DPI do contêiner |
| Interface SwiftUI | **não vem**: SwiftUI só existe no Mac |

### 2.5 Interface: Compose Multiplatform (proposta)

Um app Kotlin com Compose Multiplatform para desktop. Motivos: um código só
serve Linux e, se um dia houver, Windows; Kotlin é a língua do ecossistema
Android; o gRPC do emulador e o adb têm bibliotecas prontas na JVM. O custo é
a JVM (~150–250 MB de memória, hipótese), pequeno perto de um aparelho.

Alternativas descartadas por enquanto: GTK/Rust (mais leve, mas sem nada
reaproveitável para Windows) e Swift no Linux (sem interface gráfica madura).
Fica em aberto até o fim da fase 0 (seção 4).

### 2.6 Instalação e permissões

O Leve precisa do módulo `binder_linux` (ou binderfs) no kernel e de
contêineres com privilégios. Isso não cabe num Flatpak. Proposta: pacote
`.deb` e `.rpm` mais um AppImage, com um ajudante que, na primeira vez e com
`pkexec`, carrega o módulo do binder, liga o KSM e instala o Podman se faltar.
Distribuição pelo GitHub Releases, como o Mac.

## 3. Fases

### Fase 0 — Provar e medir (1 semana)

**Pergunta:** o Redroid direto no Linux, com a GPU do PC, mantém os
~0,85 GB por aparelho do Mac **e** toca vídeo com rolagem fluida? Se sim,
o MultiDroid para Linux nasce com o motor Leve como padrão.

**Máquina:** um PC com Linux instalado no disco (não pendrive: as medidas de
memória e disco precisam do sistema de verdade), Ubuntu 24.04 ou Fedora 41,
**16 GB de RAM ou mais**, GPU Intel ou AMD (o Mesa é o caminho do Redroid;
NVIDIA com driver proprietário provavelmente não serve, hipótese). Um Mac
com Asahi Linux seria ideal (ARM64 com GPU), mas o Asahi não suporta o M4.

**O homelab não serve:** o i3-2100 tem 2 núcleos e 4 threads, o vídeo é um
HD 2000 de 2011 (fraco demais para várias telas H.264) e o node roda os
serviços de produção sem redundância. Experimentos de carga ali arriscam
tudo o que roda nele.

**Ferramenta:** `linux/tools/leve.sh`, adaptado de `mac/tools/leve.sh` sem a
VM do Lima (`setup`, `up <N> [ksm] [gpu]`, `medir`, `rolar`, `video`, `down`).

**O que medir** (mesmos critérios das medições do Mac, para comparar):

| # | Medida | Como |
|---|---|---|
| L-1 | Custo de 1, 4, 8 e 16 Androids | memória do sistema (`free`, PSS dos contêineres com `smem`) antes e depois; custo por aparelho = diferença / N |
| L-2 | Ganho do KSM | repetir L-1 com o KSM ligado (`ksm-exec`, como no Mac) |
| L-3 | Boot | segundos até `sys.boot_completed` |
| L-4 | Rolagem | 20 rolagens em Configurações, quadros com atraso no `gfxinfo`, com GPU do PC (`redroid_gpu_mode=host`) e em software (`guest`) |
| L-5 | **Vídeo** | o mesmo feed H.264 720×1280 do Mac: toca? CPU e GPU durante a reprodução |
| L-6 | Memória devolvida | desligar todos e ver se o sistema volta ao número de antes (sem VM, deve voltar) |
| L-7 | Apps só-ARM | em x86_64: abrir 5 apps de conta comuns, sem tradução e, se a licença permitir testar, com `libndk_translation` |
| L-8 | Tela no app | 8 telas pelo protocolo do `scrcpy` ao mesmo tempo: CPU do PC e atraso do toque |
| L-9 | Motor Completo | um emulador x86_64 com KVM e Play, para comparar com o Mac (4,6 GB) |

**Passa se:** L-1 ≤ 1 GB por aparelho, L-5 toca, L-4 com a GPU do PC abaixo de
5% de quadros com atraso e L-8 com 8 telas sem travar. Se L-5 falhar também
no Linux, o Leve volta a ser só para contas sem vídeo, e a versão Linux
perde o motivo principal.

### Fase 1 — O app (4–6 semanas)

Painel, grade, criar, ligar, congelar e apagar no motor Leve; imagens Redroid
embutidas ou baixadas do Docker Hub; teto de memória. Pastas `linux/` no
repositório, ao lado de `mac/`.

### Fase 2 — Motor Completo e recursos

Emulador com KVM e o catálogo do Google (x86_64), duplicar, grupos, mesas,
sincronizar toques entre aparelhos (o mesmo plano da fase 2 do Mac).

## 4. Decisões em aberto

| Decisão | Proposta | Decide quando |
|---|---|---|
| Interface | Compose Multiplatform | fim da fase 0 |
| Motor padrão | Leve, se a fase 0 passar | fim da fase 0 |
| x86_64 ou só ARM64 | x86_64 com apps só-ARM conforme L-7 e a revisão da licença da tradução | fim da fase 0 |
| Contêineres | Podman (sem serviço rodando como root); Docker como alternativa | fase 1 |
| Pacotes | `.deb`, `.rpm` e AppImage no GitHub Releases | fase 1 |

## 5. Riscos

| Risco | Efeito | O que fazer |
|---|---|---|
| **Tradução ARM sem licença distribuível** | em PC Intel/AMD, apps de conta com código só-ARM não abrem | medir em L-7 quantos apps-alvo dependem disso; usuário baixa da fonte original; ARM64 como plataforma de primeira |
| **Sem Play Store no Leve** | contas que exigem Play Integrity (bancos, alguns jogos) não funcionam | deixar claro no app; motor Completo para esses casos |
| Vídeo continuar falhando mesmo com GPU | o Leve não serve para redes de vídeo | L-5 decide; sem vídeo, rever se vale a versão Linux |
| GPU NVIDIA com driver proprietário | Redroid sem aceleração nessas máquinas | detectar e cair para GPU em software, com aviso |
| Diversidade de distribuições e kernels | binder ausente ou desativado em alguns kernels | ajudante de instalação testado em Ubuntu, Fedora e Arch; mensagem clara quando faltar |
| Duas interfaces (SwiftUI e Compose) | dobro de trabalho em cada recurso de tela | manter regras e textos na spec, não no código; o Mac continua sendo a referência |

## 6. E o Windows

Fora desta spec. No Windows não há kernel Linux para os contêineres (o WSL2
é uma VM, como no Mac), então o MultiDroid só teria o motor Completo, sem
vantagem de memória sobre BlueStacks, LDPlayer e MuMu, que já são gratuitos.
Se a interface Compose for adotada no Linux, o Windows vira principalmente
empacotamento, e a decisão fica para depois da fase 1.
