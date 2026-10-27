# ADR-001 — Núcleo nativo, ponte adb e Android Studio

- **Estado:** aceito. Primeiro corte em 2026-10-27.
- **Substitui:** o princípio "nada de adb" só no que diz respeito a esconder o aparelho do Android Studio. O instalador continua sem exigir Studio, Java ou `sdkmanager`.

## Contexto

O MultiDroid liga vários Androids e os mostra numa mesa. Quem desenvolve quer apertar Run no Android Studio e cair num desses aparelhos, sem criar um AVD no Studio e sem abrir a janela do emulador.

O outro pedido é desempenho: a mesa copia um quadro RGBA inteiro, muitas vezes por segundo, para cada aparelho. Esse trabalho é do processo hospedeiro (o app no Mac, no Windows ou no Linux), não de código dentro do Android.

## Decisão

Três peças, e só a primeira e a segunda têm código neste corte.

```
Android Studio  --adb :5037-->  aparelho do MultiDroid
                                 emulator-5600 | 127.0.0.1:5700
                                        |
                                        | gRPC / scrcpy (como já era)
                                        v
                                 processo hospedeiro (Rust)
                                        |
                                        | FFI
                                        v
                                 rust/native  (C, CMake)
                                 NEON ou SSE2 no quadro
```

### 1. Núcleo nativo no hospedeiro

`rust/native/` é uma biblioteca C11 montada pelo CMake (`multidroid_frame`). O `build.rs` de `rust/core` liga a biblioteca **estática** no binário Rust, então o pacote continua sendo um executável só. A compartilhada (`.dylib` / `.so` / `.dll`) é o mesmo código, gerada para o alvo existir; o binário não a carrega em separado.

O caminho quente é `md_frame_opaque`: copia o RGBA e força o alfa em 255. No Apple Silicon e no Linux arm64 isso é NEON; no x86_64, SSE2. A mesa (`screen`) e o decodificador do motor leve (`scrcpy`) passam por aí.

`md_pump` é uma thread só para essa cópia. Quem chama espera o fim. O mmap do emulador é um buffer só: a cópia tem de terminar antes do quadro seguinte. A thread tira o laço SIMD da tarefa que entrega o quadro à interface. Não há dois buffers ainda, então não há sobreposição com o quadro seguinte.

Sem `cmake` no PATH, o `build.rs` compila os mesmos `.c` com o crate `cc`. O CI confere que o CMake está presente e que a estática e a compartilhada foram instaladas.

**Não é o NDK do Android.** O NDK cruza código C para um processo Android (Bionic, JNI, APK). Isso não roda no processo do Mac, do Windows ou do Linux, e não coloca o aparelho na lista do Run. O hardware que este corte usa é o do hospedeiro: o vetor da CPU na cópia do quadro. O guest continua no emulador oficial (Hypervisor no Mac, WHPX no Windows, KVM no Linux) ou no Redroid. Código NDK dentro do Android, se um dia existir, é outro binário e outro ADR.

### 2. Ponte adb

O Android Studio não recebe um AVD nosso. Ele lista o que o servidor adb da porta **5037** enxerga. É a porta padrão do Studio. `ANDROID_ADB_SERVER_PORT` e `ADB_SERVER_SOCKET` são removidos do emulador e de todo comando `adb` do MultiDroid: outra porta esconde o aparelho.

| Motor | Serial no Studio | Quem anuncia |
|---|---|---|
| Emulador oficial | `emulator-<console>` (5600, 5602, …). O adb do aparelho é a porta seguinte (5601, 5603, …). | O próprio emulador, ao subir, se o servidor já escuta. `studio::prepare_adb_server` sobe o servidor antes do processo. Sem um `adb` no disco, o aparelho liga do mesmo jeito e o Studio só o vê depois. |
| Motor leve (Linux, Redroid) | `127.0.0.1:<porta>` a partir de 5700 | `adb connect`, no boot que o motor leve já fazia. Os comandos agora também cravam a 5037. |

A porta de console do emulador termina em 5678. Não cruza o 5700 do motor leve.

O `adb` procurado, nesta ordem: platform-tools do SDK do MultiDroid, `ANDROID_HOME` / `ANDROID_SDK_ROOT`, o SDK padrão do Studio, o `PATH`.

### 3. Android Studio

Não se cria um AVD, não se importa o projeto do MultiDroid e não se aponta o Studio para a pasta `avd/` do MultiDroid. O Run (e o Debug, que usa a mesma lista) escolhe o serial. O Studio instala o APK por adb, como em qualquer emulador.

A 0.2 (SwiftUI) não chama `adb`. Ela só deixa de herdar uma porta de adb diferente de 5037, e já sobe o emulador com `-port`. Para o Studio ver um aparelho da 0.2, o servidor precisa estar no ar antes — o script abaixo faz isso.

## Como apontar o Studio

1. Tenha um `adb`. No MultiDroid: Imagens → ferramentas da plataforma. Ou o que veio com o Android Studio.
2. Suba o servidor e leia a lista:
   - Mac ou Linux: `rust/scripts/studio-adb.sh`
   - Windows: `rust/scripts/studio-adb.ps1`
3. Ligue o aparelho no MultiDroid **depois** do script. Se ele já estava ligado, desligue e ligue de novo: o emulador só se anuncia na subida.
4. No Android Studio, no menu de aparelhos do Run, escolha `emulator-5600` (ou o próximo par). Não crie um AVD para isso.
5. Run. O log do aparelho no Mac começa com a linha `Android Studio: adb -s emulator-…`, em `~/Library/Logs/MultiDroid/<nome>.log`.

Motor leve no Linux, com o aparelho já de pé:

```sh
adb connect 127.0.0.1:5700
```

Não defina `ANDROID_ADB_SERVER_PORT`.

## Feito neste corte

- CMake, os dois `.c`, o cabeçalho e o FFI em `rust/core/src/native.rs`.
- A mesa e o scrcpy usam o carimbo nativo. Testes cobrem o alfa, o hospedeiro (Apple / Windows / Linux) e que a bomba não é a thread de quem chama.
- `launch` sobe o adb na 5037 e escreve o serial no log. O ambiente do emulador na 0.2 também não troca a porta.
- Scripts `studio-adb.sh` e `studio-adb.ps1`.

## Fora deste corte

- Medir o tempo do blit com aparelhos ligados (o relógio interno existe; falta a medição de mesa cheia).
- Dois buffers, para a cópia do quadro seguinte começar sem segurar a tarefa que fala com a interface.
- Desenho da mesa por GPU (Metal no Mac). A decisão M-2 continua em aberto; este corte é a CPU vetorial.
- Botão na interface que mostra o serial. Hoje o caminho é o script e o log.
- Travar o binário do adb no do Studio quando as versões diferem. Hoje o primeiro `adb` achado dá `start-server`; um adb de outra versão pode reiniciar o servidor que o Studio já tinha.
- NDK dentro do Android. Não é o caminho deste produto para a mesa nem para o Run.
