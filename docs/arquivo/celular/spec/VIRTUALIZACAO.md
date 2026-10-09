# Motor nativo — virtualização de apps (só Android, fase 2)

Especificação do segundo motor de conta do Duo: rodar uma **cópia do app
oficial já instalado** (TikTok, Instagram…) dentro do próprio processo do
Duo, com dados separados por cópia e notificações — no estilo Parallel
Space / Dual Space (o Island não virtualiza: usa o perfil de trabalho do
Android). É **exclusivo do Android** e da **fase 2**; o
comportamento comum de conta (Início, apelido, ordem, atalhos) está em
[FUNCIONALIDADES.md](FUNCIONALIDADES.md), e a abstração `MotorDeConta` em
[ARQUITETURA.md](../../../arquitetura/ARQUITETURA.md).

> **Status: proposta técnica, não implementada.** A fase 2 só começa depois
> de o usuário decidir U-6 (motor base e licença) e U-7 (distribuição), e
> depende de risco de loja (CONFORMIDADE_LOJAS.md). A v1 do Duo **não**
> inclui este motor. Pesquisa feita em 2026-09-21 ([§12](#12-fontes)).

---

## 1. Por que é só Android e só fora da Play (resumo)

| Ponto | Consequência |
|-------|--------------|
| iOS não deixa um app executar código de outro app (App Store 2.5.2, sandbox do iOS) | motor nativo **impossível** no iOS — não é pendência, é limite da plataforma |
| A virtualização mexe fundo no funcionamento de outros apps e do sistema | alto risco na Google Play (§8) → build separado `completo`, distribuído fora da Play (§9, DISTRIBUICAO.md) |
| Os motores de código aberto viáveis são GPL-3.0 ou de licença hostil | ou o app Android do Duo vira GPL-3.0 (código aberto), ou se compra licença comercial, ou não se faz (§3, U-6) |

---

## 2. Como funciona a virtualização (visão geral)

Um app só roda no Android depois de instalado, porque o sistema registra o
pacote, cria um UID e um diretório de dados. Um motor de virtualização "engana"
o sistema para rodar um APP que **não** está instalado no sistema, dentro do
diretório do app hospedeiro (o Duo). Três camadas:

1. **Carregar o código do alvo.** O Duo lê o APK do app oficial **já
   instalado** (caminho de `PackageManager`, sem baixar nada) e o executa com
   um `ClassLoader` próprio, num processo do Duo (`:virtual`).
2. **Proxy dos serviços do sistema.** As chamadas do app clonado a
   `ActivityManager`, `PackageManager`, `ContentResolver`, `NotificationManager`,
   etc. passam por proxies de `Binder` instalados por reflexão: o proxy troca
   o nome do pacote e o UID do clone pelos do hospedeiro na ida (para o
   sistema aceitar) e desfaz na volta. Atividades do clone rodam em
   `Activity`s "stub" pré-declaradas no manifesto do Duo.
3. **Redirecionamento de E/S.** Os caminhos absolutos do clone
   (`/data/data/<pacote>`) são redirecionados para o diretório da cópia
   dentro do Duo (`diretorioDaCopia`, §6) — em parte por API, em parte por
   hook nativo (`syscall`/libc) para caminhos gravados no código do alvo.

O Duo **não modifica** o APK do alvo, não injeta código no app oficial fora
do ambiente virtual e não automatiza ações dentro do clone (mesmo princípio
"sem automação" do FUNCIONALIDADES §1). Ele só cria o ambiente onde a cópia
roda isolada.

### O que NÃO é

- Não é `adb`/usuários múltiplos do Android (aquilo é do sistema; o Duo é um
  app comum, sem root).
- Não é modificação de APK (o `applicationId` não é reescrito; a separação é
  em tempo de execução, não no arquivo).
- Não é acesso aos dados do app oficial: o clone começa **vazio** (login do
  zero); nada do app oficial original é lido.

---

## 3. Motores de código aberto e licença (recomendação)

Levantamento em 2026-09-21:

| Motor | Licença | Android suportado | Manutenção | Observação |
|-------|---------|-------------------|-----------|------------|
| **VirtualApp** (asLody) | **Sem licença livre válida.** Começou LGPL-3.0 (jul/2016) → GPL-3.0 (set/2016); o autor depois tentou fechar. Tribunais na China (Shenzhen 2021, Guangzhou 2021) decidiram que **as versões publicadas sob GPL-3.0 continuam GPL-3.0**, mas a empresa dona (Luohe/罗盒) **processa quem usa em produto comercial** e move ações-crime por violação de PI | "14.0 na versão comercial" | repo público parado em dez/2017; versão comercial paga ativa | Fonte de quase todos os forks. Usar a base GPL obriga a abrir o código; usar a comercial exige compra e contato direto. Risco jurídico real |
| **VirtualXposed** (SandHook/VirtualApp) | GPL-3.0 | até ~Android 10 | parado (2024) | herda o VirtualApp e o mesmo risco |
| **BlackBox** (FBlackBox) | sem licença (all rights reserved) | até 12/13 | **descontinuado**: o autor apagou o projeto após pressão jurídica do dono do VirtualApp | derivado do VirtualApp; não usar |
| **NewBlackbox** (ALEX5402) | Apache-2.0 (declarada) | diz "5.0 a 14+"; correções para 14/15 | ativo (jul/2026) | fork do BlackBox, que é derivado do VirtualApp: a **procedência real** é VirtualApp/GPL, então a etiqueta Apache-2.0 é duvidosa. Créditos citam VirtualApp, Dobby, xDL, FreeReflection |

**Conclusão de licença:** todo motor pronto e capaz descende do VirtualApp.
As opções reais são:

1. **Abrir o app Android do Duo sob GPL-3.0** e usar uma base GPL-3.0
   (VirtualApp de época GPL, ou fork honesto). Impacto no negócio: o
   **código-fonte do app Android** teria de ser oferecido a quem recebe o
   APK; ainda é possível **vender/cobrar** (a GPL permite cobrar pela
   distribuição) e o app pode ficar fora da Play, mas qualquer um poderia
   recompilar e redistribuir. Não obriga a abrir o app **iOS** (projeto
   separado, sem código GPL). O contrato comum em `docs/` e os vetores podem
   seguir com a licença do repositório.
2. **Licença comercial do VirtualApp** (contato direto com a Luohe): custo e
   dependência de terceiro; libera app fechado.
3. **Implementação própria mínima** só para os poucos apps-alvo (redes
   sociais que já têm versão web): esforço muito alto (proxies de Binder e
   hooks nativos por versão de Android), mas sem amarra de licença.
4. **Não fazer o motor nativo** e ficar só no motor web (o web já entrega a
   proposta "várias contas num aparelho" para todos os serviços do catálogo).

**Recomendação:** para a v1, **opção 4** — entregar só o motor web, que é
aceito nas duas lojas, sem risco jurídico e cobre o caso de uso. Se a fase 2
for adiante, **opção 1** (GPL-3.0 + base aberta), porque a opção 2 cria
dependência paga de um fornecedor litigioso e a 3 é cara demais para um app
de portfólio. A opção 1 exige **decisão consciente de abrir o código do app
Android** — é a troca central de U-6, registrada aqui para o usuário decidir.
Enquanto U-6 não for decidido, nenhum motor de terceiros entra no repositório.

---

## 4. Requisitos de sistema e ABI

| Item | Decisão | Motivo |
|------|---------|--------|
| `minSdk` do build `completo` | 26 (mesmo do app) | abaixo disso nem o motor web funciona |
| Teto testado | definir na fase 2 por versão do Android; a cada nova versão maior do Android o motor pode quebrar (proxies dependem de API interna) | virtualização depende de detalhe interno do sistema |
| `targetSdk`/`compileSdk` | o mesmo do app (compileSdk 37 disponível nesta máquina); quanto mais novo o `targetSdk`, mais restrições de API oculta (§7) | exigência da Play e do próprio sistema |
| ABI | **arm64-v8a** (`dados/apps-nativos.json → abisSuportadas`) | aparelhos atuais são 64 bits; suportar app 32 bits exigiria embutir um segundo processo/host 32 bits (como o VirtualApp faz com pacote principal + plugin), fora de escopo na fase 2 |
| App 32 bits (só `armeabi-v7a`) | **não clonável** nesta fase (motivo `Arquitetura não suportada`, §5) | sem host 32 bits embutido |

---

## 5. Quais apps são clonáveis

`classificarClonavel(app, abisSuportadas, incompativeis)` — vetores em
[`vetores/clonaveis.json`](vetores/clonaveis.json). Ordem das regras (a
primeira que falha define o `motivo` exato):

1. pacote == `pacoteProprio` (`com.falcaosl.duo`) → `O próprio Duo`.
2. app do sistema (`ApplicationInfo.FLAG_SYSTEM`) → `App do sistema`.
3. sem atividade de launcher (`queryIntentActivities` com
   `CATEGORY_LAUNCHER` vazio) → `Sem tela inicial`.
4. tem bibliotecas nativas e nenhuma casa com `abisSuportadas` → `Arquitetura
   não suportada` (app sem `.so` — `abis` vazio — passa).
5. pacote em `incompativeis` (`dados/apps-nativos.json`) → `Não roda em
   ambiente clonado`.
6. senão → clonável.

`classificarClonavel` roda sobre **metadados** (`PackageManager`), então é
testável fora do aparelho (§10). O que exige o app real é a lista
`incompativeis`, que nasce dos testes em emulador/aparelho.

Apps que tipicamente **não** funcionam clonados (candidatos a
`incompativeis`, a confirmar em teste):

- **Bancos e carteiras** e apps com SafetyNet/Play Integrity: detectam
  ambiente virtual e recusam (§8.3). O Duo não tenta enganar a atestação.
- **Jogos com anticheat** (detecção de processo/ambiente).
- Apps que dependem de recurso do sistema que o motor não proxia (alguns
  DRM, Health Connect, algumas integrações do sistema).

---

## 6. Dados por cópia e isolamento

- `diretorioDaCopia(contaId, pacote)` = `virtual/<uuid minúsculo>/<pacote>`,
  relativo aos dados do app do Duo. Vetores em `clonaveis.json`.
- Cada cópia tem seu próprio diretório de dados, cache, `SharedPreferences`,
  bancos e arquivos. Duas cópias do mesmo pacote (duas contas) não se veem.
- O clone é redirecionado a esse diretório; o diretório do app oficial no
  sistema **não** é lido nem tocado.
- `Limpar dados` (FUNCIONALIDADES §5.7): apaga o conteúdo de
  `virtual/<uuid>/<pacote>` (o clone volta a pedir login). `Excluir`
  (§5.6): mata o processo do clone e apaga a pasta `virtual/<uuid>/` inteira.
  `Forçar parada`: mata o processo `:virtual` do clone sem apagar dados.
- **Faxina na abertura**: pastas em `virtual/` sem conta correspondente são
  apagadas (paralelo ao CONTEINERES §9.3).

---

## 7. API oculta (hidden API) e permissões

### 7.1 Restrições de API não-SDK

Desde o Android 9 o sistema restringe reflexão sobre API interna (listas
"greylist"/"blocklist"), e o motor **depende** dessa reflexão (Binder,
`ActivityThread`, `PackageParser`…). Contorno, na ordem de preferência:

- `dalvik.system.VMRuntime.setHiddenApiExemptions("L")` via reflexão de
  bootstrap, ou bibliotecas conhecidas (`FreeReflection`/`BlackReflection`,
  citadas pelos forks) — sem root, no início do processo.
- Isso é frágil por versão do Android; cada `targetSdk` novo pode fechar uma
  brecha. É o principal motivo de o motor exigir manutenção contínua e de o
  teto de versão ser conservador (§4).

### 7.2 Permissões que o Duo declara (build `completo`)

| Permissão | Para quê | Nota de loja |
|-----------|----------|--------------|
| `QUERY_ALL_PACKAGES` | listar os apps instalados na aba "App nativo (clonar)" | permissão de alto risco na Play: exige o formulário de declaração e um caso de uso permitido; **não** listado entre os permitidos para "clonador". É um dos motivos de o build `completo` ficar fora da Play (§8, CONFORMIDADE_LOJAS.md). No build `play` a permissão **não** é declarada |
| `FOREGROUND_SERVICE` (+ tipo) | manter o processo do clone e as notificações vivos | comum; declarar o tipo correto |
| `POST_NOTIFICATIONS` | repassar as notificações dos clones (Android 13+) | pedida em runtime |
| `REQUEST_DELETE_PACKAGES` | não é necessária (o Duo não instala/desinstala apps) | **não declarar** |
| `INSTALL_PACKAGES` / `REQUEST_INSTALL_PACKAGES` | idem | **não declarar** |

O manifesto do build `play` não tem nenhuma dessas do motor nativo, exceto o
que o motor web precise. A diferença é conferida pelo CI (PIPELINES).

### 7.3 Google Play Services dentro do clone

- Muitos apps (login com Google, mapas, FCM) chamam o **Google Play
  Services**, que se comunica por Binder e verifica a assinatura do pacote
  chamador. Em ambiente clonado isso costuma falhar (login com Google não
  conclui; FCM não registra), como acontece nos apps de "espaço duplo".
- O Duo **não** embute um Play Services falso (microG) nesta fase: aumenta
  muito a superfície de licença e de política. Consequência honesta ao
  usuário: numa cópia nativa, **login com Google pode não funcionar** e
  **notificações via FCM podem não chegar**. Isso é dito na tela de escolha
  do motor e na descrição da loja/site.
- Para o serviço que só entra com Google, a recomendação ao usuário é usar o
  **motor web** dessa conta.

---

## 8. Detecção, atestação e apps que não funcionam

### 8.1 Detecção de ambiente virtual

Apps podem detectar que rodam clonados (caminho de dados diferente do
esperado, ausência do pacote no `PackageManager` real, nome de processo,
múltiplos `ClassLoader`). O Duo **não** promete derrotar essas detecções:
quando um app recusa rodar clonado, ele entra em `incompativeis` (§5) e some
da lista de clonáveis — o Duo não fica numa corrida de gato e rato para
enganar apps que não querem ser clonados.

### 8.2 Play Integrity / SafetyNet

Bancos e apps sensíveis usam Play Integrity: a atestação **falha** em
ambiente virtual (é o objetivo dela). O Duo não tenta forjar atestação
(seria burlar segurança — Play, Abuso de dispositivo e rede). Esses apps
ficam marcados não clonáveis.

### 8.3 Lista viva

`dados/apps-nativos.json → incompativeis` cresce com os testes. A tela de
escolha mostra o motivo e, quando faz sentido, sugere o motor web.

---

## 9. Builds, memória e bateria

- **Dois flavors Android**: `play` (só motor web, sem `QUERY_ALL_PACKAGES`,
  sem processo `:virtual`, sem código do motor nativo) e `completo` (web +
  nativo). Só o `completo` é distribuído fora da Play (DISTRIBUICAO.md). Se a
  fase 2 não acontecer, existe só o equivalente ao `play`.
- **Memória/bateria**: cada clone é praticamente um app inteiro rodando; o
  custo é bem maior que o de uma WebView. Aplicar limite de processos vivos
  parecido com o do motor web (poucos clones ativos), matar clones em
  segundo plano sob pressão de memória, e deixar claro que muitos clones
  ativos pesam no aparelho.
- Cada clone roda em processo `:virtual` isolado do processo de UI do Duo.

---

## 10. Segurança e privacidade

- **O hospedeiro (Duo) tecnicamente enxerga os dados dos clones**, porque
  eles vivem dentro do sandbox do Duo. Isso é inerente à virtualização e
  precisa ser dito com honestidade.
- Promessa do Duo, igual ao motor web: **nada sai do aparelho**. O Duo não
  lê, não coleta e não envia nada dos clones; sem servidor, sem analytics,
  sem SDK de terceiros. Não injeta código no app clonado além do necessário
  para o ambiente rodar (sem captura de tela, teclas, mensagens).
- Os dados dos clones ficam no sandbox do app do Duo (só o Duo e o root os
  acessam), fora do backup em nuvem (`allowBackup=false`, herdado do app).
- **Criptografia em repouso**: os dados do app já ficam na área criptografada
  pelo sistema (File-Based Encryption do Android). O Duo **não** adiciona
  uma camada própria de criptografia dos diretórios de clone na fase 2
  (chave teria de morar no próprio app, dando pouca proteção real); se um dia
  for pedido, entra por trás do bloqueio biométrico (FUNCIONALIDADES §8).
- Isto é declarado na política de privacidade e na tela de escolha do motor.

---

## 11. O que dá para testar (e o que não dá)

| Testável sem aparelho (unitário, lê `vetores/`) | Exige emulador/aparelho (instrumentado, fase 2) |
|-------------------------------------------------|--------------------------------------------------|
| `classificarClonavel` (regras de §5) a partir de metadados de app | criar uma cópia e ela abrir |
| `diretorioDaCopia` (mapeamento contaId+pacote → caminho) | isolamento real de dados entre duas cópias |
| Diferença de manifesto entre `play` e `completo` (grep no CI: `QUERY_ALL_PACKAGES` só no `completo`) | notificações do clone chegarem |
| Presença/ausência do código do motor no flavor `play` | exemption de hidden API funcionando por versão de Android |
| Faxina: quais pastas `virtual/` seriam apagadas dada uma lista de contas | apps reais que entram em `incompativeis` |

Os vetores de `clonaveis.json` rodam nas duas plataformas? **Não**: são só do
Android (o iOS não tem motor nativo). A suíte iOS ignora `clonaveis.json`; a
Android roda. Isso está dito no `vetores/README.md`.

---

## 12. Fontes

- VirtualApp (asLody) — repositório e nota de licença comercial: https://github.com/asLody/VirtualApp
- Análise das decisões judiciais sobre a licença GPL-3.0 do VirtualApp (Shenzhen 2021; Guangzhou 2021): https://segmentfault.com/a/1190000041444355/en
- BlackBox (FBlackBox) — projeto descontinuado após pressão jurídica (issues 121/122): https://github.com/FBlackBox/BlackBox
- NewBlackbox (ALEX5402), Apache-2.0 declarada, créditos a VirtualApp/Dobby/xDL/FreeReflection: https://github.com/ALEX5402/NewBlackbox
- Google Play — `QUERY_ALL_PACKAGES` / visibilidade de pacotes (casos permitidos e formulário): https://support.google.com/googleplay/android-developer/answer/10158779
- Google Play — Abuso de dispositivo e rede: https://support.google.com/googleplay/android-developer/answer/9888379
- Restrições de interfaces não-SDK (hidden API): https://developer.android.com/guide/app-compatibility/restrictions-non-sdk-interfaces
- Apple App Store Review Guidelines 2.5.2 (um app não pode baixar/executar código que muda funcionalidades): https://developer.apple.com/app-store/review/guidelines/
