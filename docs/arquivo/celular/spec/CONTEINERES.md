# Contêineres — isolamento por conta

Especificação técnica de como cada conta do Duo vira um navegador isolado,
no Android e no iOS. O comportamento visível está em
[FUNCIONALIDADES.md](FUNCIONALIDADES.md); aqui estão as APIs, o ciclo de
vida e as regras que as duas implementações precisam seguir igual.

Pesquisa feita em 2026-09-21. As fontes estão no fim ([§12](#12-fontes)).

---

## 1. Modelo

```
Conta (id = UUID)  ──1:1──►  armazenamento web persistente  ──1:N──►  WebView viva (no máximo 1 por conta)
                             iOS:     WKWebsiteDataStore(forIdentifier: id)
                             Android: Profile "conta-<id>-g<geracao>" (ProfileStore)
```

- Uma conta tem **um** armazenamento. O armazenamento guarda cookies,
  `localStorage`, `sessionStorage`, IndexedDB, Cache Storage, service
  workers e cache HTTP.
- Em um dado momento, uma conta tem **no máximo uma** WebView viva. Popups
  da mesma conta (§6.5) usam o mesmo armazenamento e vivem só enquanto
  estão abertos.
- Nenhuma WebView do Duo usa o armazenamento padrão do sistema: o perfil
  `Default` (Android) e `WKWebsiteDataStore.default()` (iOS) ficam vazios.
  Testado em §10.

---

## 2. APIs por plataforma

### 2.1 iOS (17.0+)

| Uso | API |
|-----|-----|
| Criar/obter o armazenamento da conta | `WKWebsiteDataStore(forIdentifier: conta.id)` — cria na primeira chamada, persiste em disco |
| Ligar à WebView | `let cfg = WKWebViewConfiguration(); cfg.websiteDataStore = store` antes de `WKWebView(frame:configuration:)` |
| Listar armazenamentos existentes | `WKWebsiteDataStore.fetchAllDataStoreIdentifiers` |
| Apagar os dados, mantendo o armazenamento | `store.removeData(ofTypes: WKWebsiteDataStore.allWebsiteDataTypes(), modifiedSince: .distantPast)` |
| Apagar o armazenamento | `WKWebsiteDataStore.remove(forIdentifier:)` — só depois de liberar todas as `WKWebView` que o usam; falha com erro se ainda estiver em uso |

- `minimumOSVersion` = **17.0**: as quatro APIs acima são iOS 17.0+ (e
  iPadOS 17.0+). Sem alternativa para iOS 16: antes do 17 só existia
  `.default()` (compartilhado) e `.nonPersistent()` (some ao fechar), e
  nenhum dos dois atende "isolado **e** persistente".
- Um `WKWebsiteDataStore` por conta, criado sob demanda e guardado num
  dicionário `[UUID: WKWebsiteDataStore]` do gerenciador de contêineres
  (`@MainActor`).

### 2.2 Android (`androidx.webkit` multiperfil)

| Uso | API |
|-----|-----|
| Verificar suporte | `WebViewFeature.isFeatureSupported(WebViewFeature.MULTI_PROFILE)` |
| Criar/obter o perfil | `ProfileStore.getInstance().getOrCreateProfile(nome)` |
| Ligar à WebView | `WebViewCompat.setProfile(webView, nome)` — **antes de qualquer outra chamada** na WebView além de anexá-la à hierarquia de views |
| Cookies do perfil | `profile.cookieManager` |
| Armazenamento do perfil | `profile.webStorage` |
| Service workers do perfil | `profile.serviceWorkerController` |
| Listar perfis | `ProfileStore.getInstance().allProfileNames` (inclui o `Default`) |
| Apagar o perfil | `ProfileStore.getInstance().deleteProfile(nome)` |

**Requisitos encontrados:**

| Requisito | Valor | Fonte |
|-----------|-------|-------|
| Biblioteca | `androidx.webkit:webkit` **≥ 1.9.0** (a API multiperfil entrou no 1.9.0, estável em 29/11/2023). Usar a estável atual: **1.17.0** (12/08/2026) | notas de versão do `androidx.webkit` |
| `minSdk` da biblioteca | **24** a partir do 1.16.0 (antes 23, antes 21) | idem |
| Versão do WebView do aparelho | o recurso é anunciado pelo WebView instalado; foi liberado (sem sufixo de dev) no Chromium **119** (commit `7fcb89b`, 29/09/2023, posição `#1203119`, dentro do ramo M119). WebView ≥ 119 é o piso prático; o app **não** compara número de versão — confia só em `isFeatureSupported` | Chromium, `[AW][multi-profile] Remove DEV_SUFFIX from MULTI_PROFILE feature` |
| WebView multiprocesso | `MULTI_PROFILE` só é reportado se `WebViewCompat.isMultiProcessEnabled()` for verdadeiro (código do `WebViewFeatureInternal`). O renderizador fora do processo existe do Android 8.0 (API 26) em diante: ligado em todos os aparelhos 64 bits e 32 bits com bastante memória, e em todos a partir do Android 11. Aparelhos 32 bits com pouca memória no Android 8–10 rodam em processo único e **não** terão o recurso | `androidx.webkit` (fonte) e documentação do renderizador do WebView |
| Thread | `ProfileStore`, `setProfile` e `getProfile` são `@UiThread` | Javadoc |

**Decisões que decorrem disso** (FUNCIONALIDADES D-2, D-3):

- `minSdk` = **26**. Abaixo do 26 não há WebView multiprocesso, logo nunca
  haveria `MULTI_PROFILE`; publicar para 24–25 só geraria instalações que
  não funcionam.
- `targetSdk` = o exigido pela Play no momento do envio (hoje 36; conferir
  em PIPELINES_PUBLICACAO).
- Sem `MULTI_PROFILE` → tela `sem-suporte` (FUNCIONALIDADES §16). Nunca cair
  para uma WebView sem perfil.

**Regras de `setProfile`** (do Javadoc — cada uma lança
`IllegalStateException`): não chamar depois de a WebView navegar, depois de
`evaluateJavascript`, depois de `getProfile`, nem duas vezes. Por isso a
WebView de conta é criada por uma fábrica única:

```kotlin
@UiThread
fun criarWebView(contexto: Context, conta: Conta): WebView {
    check(WebViewFeature.isFeatureSupported(WebViewFeature.MULTI_PROFILE))
    val nome = nomePerfil(conta)            // "conta-<uuid>-g<geracao>"
    ProfileStore.getInstance().getOrCreateProfile(nome)
    return WebView(contexto).also { wv ->
        WebViewCompat.setProfile(wv, nome)   // primeira chamada, sempre
        configurar(wv)                       // §11, só depois
    }
}
```

- `CookieManager.getInstance()` e `WebStorage.getInstance()` são do perfil
  **padrão**: proibidos no código de contêiner (regra de lint no CI; §10).

### 2.3 Nomes

| | Android | iOS |
|---|---------|-----|
| Chave do armazenamento | `conta-<uuid>-g<geracao>` (ex.: `conta-3f2b…9a1b-g1`) | o próprio UUID da conta |
| Por que a geração | `deleteProfile` não apaga perfil já carregado no processo (§9); `Limpar dados` troca para um perfil novo | `removeData` apaga no lugar |

---

## 3. O que fica isolado e o que não fica

| Item | Isolado por conta? | Observação |
|------|:------------------:|------------|
| Cookies (inclusive de terceiros) | sim | |
| `localStorage`, `sessionStorage`, IndexedDB | sim | |
| Cache Storage, service workers | sim | Android: `profile.serviceWorkerController` |
| Cache HTTP | sim | |
| Permissões de site concedidas na sessão | sim | Android: `profile.geolocationPermissions` (sempre negado, §8) |
| Histórico de navegação (voltar/avançar) | sim | vive na WebView; perdido quando ela é descartada (§5) |
| Endereço IP e rede | **não** | todas as contas saem pelo mesmo IP |
| Impressão digital do aparelho (User-Agent, tela, fontes, fuso, idioma, GPU/canvas) | **não** | os serviços podem associar contas ao mesmo aparelho; o Duo não tenta esconder (FUNCIONALIDADES §1) |
| Preenchimento automático do sistema (Google Autofill, Senhas do iCloud) | **não** | é do sistema; pode sugerir a senha de outra conta do mesmo site |
| Permissão de câmera/microfone no sistema | **não** | é do app Duo; a autorização por site é por conta (§8) |
| Área de transferência | **não** | do sistema |
| Pasta de downloads | **não** | arquivos baixados vão para o mesmo lugar (§7) |
| Navegação segura (Safe Browsing) | **não** | serviço do motor (Google no Android; WebKit no iOS), igual para todas as contas |

---

## 4. User-Agent

Chave `userAgent` de cada serviço em [`dados/servicos.json`](dados/servicos.json):

| Chave | Android | iOS | Uso |
|-------|---------|-----|-----|
| `padrao` | `WebSettings.getDefaultUserAgent` intacto (contém `; wv`) | UA padrão do `WKWebView` | todos os serviços, salvo os abaixo |
| `mobile` | UA padrão sem `; wv` e sem `Version/4.0 ` | `applicationNameForUserAgent = "Version/<iOS>.0 Mobile/15E148 Safari/604.1"` | reservado para serviço que entrega página quebrada ao WebView; nenhum usa hoje |
| `desktop` | UA de Chrome para computador + `useWideViewPort` + `loadWithOverviewMode` | `WKWebpagePreferences.preferredContentMode = .desktop` | Snapchat, Discord, WhatsApp Web (web oficial só para computador) |

Regras:

1. **Nunca** aplicar `mobile` ou `desktop` a hosts de `dominiosIdentidade`
   (`accounts.google.com`, `appleid.apple.com`, `login.microsoftonline.com`,
   `login.live.com`): nesses, a WebView usa sempre o UA padrão. O Google
   bloqueia login em WebView incorporada desde 2021/2023 (erro
   `disallowed_useragent`) e o Duo **não contorna** — contornar seria burlar
   uma proteção de segurança (Play: Abuso de dispositivo e rede). A faixa
   do FUNCIONALIDADES §6.4 explica ao usuário.
2. O UA é definido na criação da WebView e não muda durante a vida dela. No
   Android, quando um contêiner `desktop` navega para um domínio de
   identidade, a navegação é tratada como `fora`.
3. Android com `USER_AGENT_METADATA`: no modo `desktop`, definir
   `WebSettingsCompat.setUserAgentMetadata` com `mobile = false` e a
   plataforma coerente, para os client hints não contradizerem o UA.
4. Placeholders `{chromeMaior}` / `{iosMaior}` vêm do aparelho em tempo de
   execução (versão do WebView instalado / `UIDevice.systemVersion`).

---

## 5. Ciclo de vida e memória

### 5.1 Pool de contêineres

- Gerenciador único por processo (Android: objeto no `Application`, com as
  WebViews criadas com um `MutableContextWrapper` trocado para a Activity
  atual; iOS: `ContainerManager` `@MainActor`).
- Capacidade: `limites.maxContextosVivos` = **3**. Ordem LRU por último uso.
- Abrir uma conta: se já viva, reanexa a mesma WebView à tela (estado
  intacto); senão cria (§2) e carrega `ultimaUrl` ou a URL inicial
  (FUNCIONALIDADES §7.2). Se o pool passar da capacidade, descarta a menos
  recente que não está na tela.
- Descartar: grava `ultimaUrl`, para a carga, remove da hierarquia,
  `webView.destroy()` (Android) / solta todas as referências e
  `navigationDelegate = nil` (iOS).
- WebViews fora da tela: Android `webView.onPause()` (nunca `pauseTimers()`,
  que é global); iOS `pauseAllMediaPlayback` e `setAllMediaPlaybackSuspended(true)`.
  Ao voltar: `onResume()` / `setAllMediaPlaybackSuspended(false)`.

### 5.2 Memória baixa

| Evento | Android | iOS | Ação |
|--------|---------|-----|------|
| Aviso de memória | `ComponentCallbacks2.onTrimMemory(level ≥ TRIM_MEMORY_RUNNING_LOW)` com o app em primeiro plano | `UIApplication.didReceiveMemoryWarningNotification` | descarta todas menos a da tela; capacidade = `maxContextosVivosMemoriaBaixa` (1) até o próximo retorno do segundo plano |
| App em segundo plano | `onTrimMemory(TRIM_MEMORY_UI_HIDDEN)` | `scenePhase == .background` | mantém o pool; o sistema pode matar o processo |
| Processo de renderização morreu | `WebViewClient.onRenderProcessGone` → **retornar `true`**, destruir a(s) WebView(s) afetadas e recriar a da tela na `ultimaUrl` | `webViewWebContentProcessDidTerminate` → `reload()` se a WebView está na tela, senão marcar para recarregar ao reanexar | sem tela de erro (FUNCIONALIDADES §6.3) |

No Android todas as WebViews do app compartilham **um** processo de
renderização: a morte dele atinge todo o pool, e `onRenderProcessGone` chega
a cada WebView. Retornar `false` derruba o app — proibido.

### 5.3 Por que 3

Uma rede social em WebView ocupa algumas centenas de MB entre o processo do
app e o de renderização. Três contêineres cobrem o uso típico (alternar
entre duas ou três contas) sem o sistema matar o app em aparelhos de 4 GB.
Valor revisável após medir em aparelho real (STATUS.md).

---

## 6. Navegação: o que abre dentro e o que sai

### 6.1 Onde classificar

| Plataforma | Gancho | Retorno |
|------------|--------|---------|
| Android | `WebViewClient.shouldOverrideUrlLoading(view, request)` — chamado também para redirecionamentos de servidor (`request.isRedirect`) | `false` = segue dentro; `true` = o Duo tratou (fora/app/bloquear) |
| iOS | `WKNavigationDelegate.webView(_:decidePolicyFor:decisionHandler:)` | `.allow` / `.cancel` |

Só navegações de **quadro principal** são classificadas (Android
`request.isForMainFrame`, iOS `navigationAction.targetFrame?.isMainFrame`).
Quadros internos: `http`/`https`/`about`/`blob`/`data` seguem; qualquer
outro esquema é cancelado em silêncio.

### 6.2 Algoritmo `classificarNavegacao(url, conta)`

Vetores em [`vetores/navegacao.json`](vetores/navegacao.json). Resultado:
`acao` ∈ `conteiner` | `fora` | `app` | `bloquear`, `destino`, `servicoApp`.

1. Sem esquema válido (`^[A-Za-z][A-Za-z0-9+.-]*:`) → `bloquear`.
2. `about:blank`, `about:srcdoc` → `conteiner`; outro `about:` → `bloquear`.
3. `blob:` → `conteiner`.
4. `javascript:`, `data:`, `file:`, `content:`, `chrome:`, `filesystem:`,
   `view-source:` → `bloquear`.
5. `http:`/`https:`:
   - host (minúsculas, sem porta, sem `usuário@`, sem ponto final);
   - host é loja de apps (`apps.apple.com`, `itunes.apple.com`,
     `play.google.com`) → `app` sem `servicoApp`;
   - host casa com `dominios` + `dominiosAuxiliares` do serviço da conta
     (igual ao domínio ou termina com `.` + domínio) — no personalizado,
     com `[dominioBase]` → `conteiner`, com `destino` em `https` (um `http`
     é reescrito para `https` antes de carregar: não há exceção de ATS no
     iOS nem `usesCleartextTraffic` no Android);
   - senão → `fora`.
6. `intent:` (Android; também tratado no iOS se aparecer):
   - com `S.browser_fallback_url` → classificar o fallback (decodificado);
   - senão, com `scheme=http|https` → classificar `scheme://` + o trecho
     entre `intent://` e `#Intent`;
   - se o resultado for `conteiner`, é esse; senão `app`, com `servicoApp` =
     serviço dono do `scheme` do intent.
7. `mailto:`, `tel:`, `sms:` → `fora`.
8. Qualquer outro esquema (`instagram:`, `snssdk1233:`, `market:`,
   `itms-apps:`, desconhecidos) → `app`, com `servicoApp` = serviço cujo
   `esquemasApp` contém o esquema, ou `null`.

Casamento de domínio: `instagram.com.golpe.io` **não** casa com
`instagram.com`; `notinstagram.com` também não (vetores).

### 6.3 O que cada ação faz

| Ação | Android | iOS |
|------|---------|-----|
| `conteiner` | se `destino == url`, retorna `false`; se reescrito (http→https, intent), retorna `true` e `view.loadUrl(destino)` | `.allow`; se reescrito, `.cancel` + `load(URLRequest(url: destino))` |
| `fora` | `true` + entrega ao sistema conforme `abrirLinksExternos` (FUNCIONALIDADES §6.5) | `.cancel` + idem |
| `app` | `true` + faixa de aviso (FUNCIONALIDADES §6.4); `Abrir fora` = `Intent.parseUri(url, URI_INTENT_SCHEME)` com `CATEGORY_BROWSABLE`, `component = null`, `selector = null` (evita abrir componente interno de outro app) | `.cancel` + faixa; `Abrir fora` = `UIApplication.shared.open(url)` |
| `bloquear` | `true`, nada | `.cancel`, nada |

- Host de `dominiosIdentidade` com `acao = conteiner`: carrega (a página do
  Google mostrará o bloqueio dele) e mostra a faixa de login do §6.4 do
  FUNCIONALIDADES uma vez por sessão do contêiner.

### 6.4 Abrir no navegador e links universais

- **Entregar ao navegador (Android)**: `ACTION_VIEW` com
  `CATEGORY_BROWSABLE`. Para `Abrir no navegador` do menu (a URL é do próprio
  serviço), o Duo resolve o navegador padrão
  (`resolveActivity(Intent(ACTION_VIEW, Uri.parse("https://")), MATCH_DEFAULT_ONLY)`)
  e fixa o `package`, para o App Link não abrir o app oficial. Sem navegador
  → toast `Nenhum navegador instalado`.
- **Entregar ao navegador (iOS)**: `UIApplication.shared.open(url)`. Não há
  como impedir que um link universal abra o app oficial quando instalado;
  para a URL do próprio serviço, o menu diz `Abrir no Safari` e usa
  `open(url, options: [.universalLinksOnly: false])` — limitação registrada.
- **Links universais dentro do contêiner (iOS)**: o WebKit pode entregar ao
  app dono do domínio uma navegação `.linkActivated` para outro host.
  Regra: navegação de quadro principal com `navigationType == .linkActivated`
  e host diferente do host atual, classificada `conteiner`, é **cancelada e
  recarregada** com `webView.load(URLRequest(url:))` (carga programática não
  dispara link universal). A confirmar no teste em aparelho (STATUS.md).
- **App Links dentro do contêiner (Android)**: a WebView nunca abre outro
  app sozinha; tudo passa por `shouldOverrideUrlLoading`. Nada a fazer.

### 6.5 Novas janelas (`target=_blank`, `window.open`)

- Android `settings.setSupportMultipleWindows(true)` +
  `WebChromeClient.onCreateWindow`; iOS
  `WKUIDelegate.webView(_:createWebViewWith:for:windowFeatures:)`.
- A primeira URL da janela passa pelo §6.2:
  - `conteiner` e **com** `window.opener` usado (popup de login, pagamento):
    abre uma **folha de popup** sobre o contêiner com uma WebView temporária
    **da mesma conta** (Android: `setProfile` com o mesmo nome antes de
    entregar pelo `WebViewTransport`; iOS: devolver `WKWebView` criada com a
    `configuration` recebida, que já traz o mesmo `websiteDataStore`). Botão
    `Fechar`; `window.close()` fecha (Android `onCloseWindow`, iOS
    `webViewDidClose`).
  - `conteiner` sem opener (link comum com `target=_blank`): carrega na
    própria WebView do contêiner.
  - `fora`, `app`, `bloquear`: como no §6.3; a janela não é criada.
- Como distinguir: janela aberta por `window.open` com `windowFeatures`
  (iOS) / `isUserGesture == false` ou popup com dimensões (Android) → folha;
  `target=_blank` por toque em link → própria WebView. Popups nunca abrem
  sem gesto do usuário (`javaScriptCanOpenWindowsAutomatically = false`;
  `preferences.javaScriptCanOpenWindowsAutomatically = false`).

---

## 7. Downloads e envio de arquivos

### 7.1 Download

| Etapa | Android | iOS |
|-------|---------|-----|
| Detectar | `WebView.setDownloadListener` (url, userAgent, contentDisposition, mimeType) | `decidePolicyFor navigationResponse`: `!canShowMIMEType` ou `Content-Disposition: attachment` → `.download`; `navigationAction.shouldPerformDownload` → `.download` |
| Só `https` | outro esquema (`blob:`, `data:`) → toast `Este download não é suportado` | idem |
| Baixar | `DownloadManager.Request(url)` com cabeçalhos `Cookie` = `profile.cookieManager.getCookie(url)` (do **perfil da conta**, nunca o padrão), `User-Agent` e `Referer`; destino `Environment.DIRECTORY_DOWNLOADS/Duo/` com `URLUtil.guessFileName`; notificação do próprio `DownloadManager` | `WKDownload` (sessão da própria WebView, com os cookies do armazenamento da conta); destino temporário em `tmp/` |
| Entregar | fica em Downloads; toast `Baixando <arquivo>` | ao terminar, folha de compartilhar (`UIActivityViewController`: `Salvar em Arquivos`, `Salvar Imagem`…) |
| Permissão | API 26–28: `WRITE_EXTERNAL_STORAGE` com `maxSdkVersion="28"`, pedida só no primeiro download; API 29+: nenhuma | nenhuma (salvar na Fotos pede `NSPhotoLibraryAddUsageDescription` na hora) |

`blob:`/`data:` ficam fora desta fase (FUNCIONALIDADES D-15): exigiriam ler o
conteúdo por JavaScript injetado, o que contraria o princípio de não injetar
script.

### 7.2 Salvar imagem (toque longo)

- iOS: menu de contexto nativo do `WKWebView` (inclui `Salvar na Fotos`).
- Android: `WebView.hitTestResult` do tipo `IMAGE_TYPE` /
  `SRC_IMAGE_ANCHOR_TYPE` com URL `https` → menu `Salvar imagem`,
  `Copiar link da imagem`, `Abrir imagem no navegador`; salvar usa o mesmo
  caminho do §7.1.

### 7.3 Envio de arquivos e câmera (`<input type="file">`)

- iOS: nativo do `WKWebView` (galeria, arquivos, câmera). `Info.plist`:
  `NSCameraUsageDescription` = `Para enviar fotos e vídeos aos sites que você abre no Duo.`
  e `NSMicrophoneUsageDescription` = `Para gravar vídeos com som e fazer chamadas nos sites que você abre no Duo.`
- Android: `WebChromeClient.onShowFileChooser` → `fileChooserParams.createIntent()`
  por um `ActivityResultLauncher`; quando `acceptTypes` inclui imagem/vídeo e
  `isCaptureEnabled`, oferecer câmera com `MediaStore.ACTION_IMAGE_CAPTURE` /
  `ACTION_VIDEO_CAPTURE` gravando num `FileProvider` (`cache/captura/`).
  Cancelar devolve `null` ao callback (obrigatório, senão o site trava).
- Arquivos temporários de captura apagados ao fechar o contêiner.

---

## 8. Permissões pedidas pelos sites e script injetado

| Pedido do site | Android | iOS | Comportamento |
|----------------|---------|-----|---------------|
| Câmera / microfone (`getUserMedia`) | `WebChromeClient.onPermissionRequest`: para `RESOURCE_VIDEO_CAPTURE` / `RESOURCE_AUDIO_CAPTURE`, diálogo do Duo `<host> quer usar a câmera` / `o microfone` / `a câmera e o microfone` com `Permitir` / `Não permitir`; se permitido e o app não tem `CAMERA`/`RECORD_AUDIO`, pede a permissão do sistema; então `request.grant(...)`, senão `deny()` | `WKUIDelegate.webView(_:requestMediaCapturePermissionFor:initiatedByFrame:type:decisionHandler:)` → `.prompt` (diálogo do WebKit por origem) | pedido a cada carregamento de página; nada fica guardado entre sessões |
| Localização | `setGeolocationEnabled(false)` e `onGeolocationPermissionsShowPrompt` → `callback.invoke(origin, false, false)` | sem `NSLocationWhenInUseUsageDescription`, o WebKit nega | sempre negado (FUNCIONALIDADES D-14) |
| Notificações | a API não existe em WebView | a API não existe em `WKWebView` | nada (FUNCIONALIDADES §11) |
| Recurso protegido (`RESOURCE_PROTECTED_MEDIA_ID`, DRM) | `deny()` | — | vídeos com DRM podem não tocar |
| MIDI, sensores | `deny()` | — | |
| Área de transferência | escrita com gesto do usuário funciona; leitura depende do motor | idem | sem configuração |

**Script injetado: nenhum nesta fase.** O Duo não usa `addJavascriptInterface`,
`WKUserScript` nem `evaluateJavascript` em páginas de serviço. Exceção única:
os testes de isolamento (§10), em páginas de teste. Mudar isso exige
atualizar este parágrafo, o princípio "Sem automação" do FUNCIONALIDADES §1
e a política de privacidade.

---

## 9. Limpeza de dados

### 9.1 Limpar dados (conta continua)

| Android | iOS |
|---------|-----|
| 1. Destroi a WebView viva da conta. 2. Limpeza imediata do perfil atual: `profile.cookieManager.removeAllCookies(null)`, `flush()`, `profile.webStorage.deleteAllData()`. 3. `geracao += 1` e grava a conta. 4. Põe o nome antigo em `perfisPendentes`. 5. Cria a WebView com o perfil novo e carrega a URL inicial. | 1. `store.removeData(ofTypes: allWebsiteDataTypes(), modifiedSince: .distantPast)` e aguarda. 2. Recarrega a WebView viva (se houver) na URL inicial. |

Por que a geração no Android: `deleteProfile` lança `IllegalStateException`
se houver WebView viva com o perfil **ou** se o perfil já foi carregado no
processo por `getOrCreateProfile`/`getProfile` — ou seja, um perfil usado
nesta execução só pode ser apagado na próxima. Trocar de perfil garante que
nada do anterior é visto de novo, e o apagamento definitivo vem no §9.3.

### 9.2 Excluir conta

| Android | iOS |
|---------|-----|
| 1. Destroi a WebView viva. 2. Limpeza imediata como no §9.1 (passo 2). 3. Põe `conta-<uuid>-g<geracao>` em `perfisPendentes`. 4. Remove a conta. 5. Desativa atalhos (FUNCIONALIDADES §9). | 1. Remove a WebView do pool e garante que foi liberada. 2. `try await WKWebsiteDataStore.remove(forIdentifier: id)`; se falhar, põe o UUID em `armazenamentosPendentes`. 3. Remove a conta. 4. Atualiza atalhos. |

### 9.3 Faxina na abertura

Na abertura do app, **antes de criar qualquer WebView**, na thread principal:

- Android: para cada nome de `ProfileStore.getInstance().allProfileNames`
  que não é `Default` e não é o perfil atual (`conta-<uuid>-g<geracao>`) de
  uma conta existente → `deleteProfile(nome)`; tira de `perfisPendentes` os
  que deram certo. Cobre pendências e perfis órfãos de uma falha no meio.
- iOS: `fetchAllDataStoreIdentifiers`; para cada UUID sem conta →
  `remove(forIdentifier:)`; limpa `armazenamentosPendentes`.
- `Apagar todas as contas` (Ajustes) = excluir cada conta + faxina.

A exclusão de dados pelo WebView/WebKit pode terminar de forma assíncrona
(o Javadoc de `deleteProfile` avisa: "some data may be deleted async").

### 9.4 Backup

- Android: `allowBackup="false"` e `dataExtractionRules` excluindo todos os
  domínios (`root`, `file`, `database`, `sharedpref`, `external`) para
  `cloud-backup` e `device-transfer`. Perfis de WebView ficam em
  `app_webview/` dentro dos dados do app, cobertos pela regra.
- iOS: `contas.json` com `isExcludedFromBackup`. Os dados de
  `WKWebsiteDataStore` vivem em `Library/WebKit/`, gerida pelo sistema; o
  comportamento de backup deles não é documentado pela Apple. Ação:
  verificar no teste em aparelho (restaurar backup e conferir se as sessões
  voltam) e registrar em STATUS.md.

---

## 10. Testes de isolamento

Cada plataforma tem testes instrumentados (Android `androidTest`, iOS
`DuoTests` no simulador) que falham se o isolamento quebrar:

| Teste | Como |
|-------|------|
| Cookie não vaza | conta A grava cookie `sessao=a` para `https://exemplo.test` (Android `profileA.cookieManager.setCookie`; iOS `storeA.httpCookieStore.setCookie`); ler em B devolve vazio |
| `localStorage` não vaza | carregar HTML com `baseURL`/`loadDataWithBaseURL("https://exemplo.test/")` em A e B; em A `localStorage.setItem('k','a')`; em B `localStorage.getItem('k') === null` |
| Perfil padrão intacto | depois de usar A e B, o perfil `Default` / `WKWebsiteDataStore.default()` não tem cookie de `exemplo.test` |
| Persistência | grava em A, destroi a WebView, cria outra para A: o cookie continua |
| Limpar dados | depois do §9.1, A está vazia e B intacta |
| Excluir + faxina | depois do §9.2 e de simular nova abertura, o nome/UUID de A não aparece em `allProfileNames` / `fetchAllDataStoreIdentifiers` |
| Sem API proibida (Android) | lint/grep no CI: `CookieManager.getInstance()` e `WebStorage.getInstance()` não aparecem fora de testes |

---

## 11. Configuração da WebView

| Ajuste | Android (`WebSettings`) | iOS |
|--------|-------------------------|-----|
| JavaScript | `javaScriptEnabled = true` | padrão |
| Armazenamento DOM | `domStorageEnabled = true` | padrão |
| Acesso a arquivos locais | `allowFileAccess = false`, `allowContentAccess = false` | não se aplica |
| Conteúdo misto | `mixedContentMode = MIXED_CONTENT_NEVER_ALLOW` | padrão do WebKit (bloqueia ativo) |
| Cookies de terceiros | `profile.cookieManager.setAcceptThirdPartyCookies(webView, true)` (logins entre domínios do mesmo dono, ex. Instagram ↔ Facebook) | padrão do WebKit (ITP) |
| Mídia | `mediaPlaybackRequiresUserGesture = true` | `mediaTypesRequiringUserActionForPlayback = .all`, `allowsInlineMediaPlayback = true` |
| Janelas | `setSupportMultipleWindows(true)`, `javaScriptCanOpenWindowsAutomatically = false` | `javaScriptCanOpenWindowsAutomatically = false` |
| Navegação segura | `safeBrowsingEnabled = true` (padrão) | `isFraudulentWebsiteWarningEnabled = true` (padrão) |
| Zoom | `setSupportZoom(true)`, `builtInZoomControls = true`, `displayZoomControls = false` | padrão |
| Depuração | `WebView.setWebContentsDebuggingEnabled(BuildConfig.DEBUG)` | `isInspectable = false` no release |
| Tema | escurecimento algorítmico desligado | segue `overrideUserInterfaceStyle` |
| Gestos | — | `allowsBackForwardNavigationGestures = true` |

Não usar: `addJavascriptInterface`, `setAllowUniversalAccessFromFileURLs`,
`setAllowFileAccessFromFileURLs`, `WKUserScript`, `setWebContentsDebuggingEnabled(true)` em release.

---

## 12. Fontes

- `androidx.webkit` — notas de versão (1.9.0 introduz o multiperfil; minSdk 23 → 24; 1.17.0 estável): https://developer.android.com/jetpack/androidx/releases/webkit
- `ProfileStore`, `Profile`, `WebViewCompat.setProfile`, `WebViewFeature.MULTI_PROFILE` — código-fonte e Javadoc do AndroidX: https://github.com/androidx/androidx/tree/androidx-main/webkit/webkit/src/main/java/androidx/webkit e https://developer.android.com/reference/androidx/webkit/ProfileStore
- Requisito de multiprocesso para `MULTI_PROFILE` — `WebViewFeatureInternal.MULTI_PROFILE`: https://github.com/androidx/androidx/blob/androidx-main/webkit/webkit/src/main/java/androidx/webkit/internal/WebViewFeatureInternal.java
- Chromium, liberação do recurso no M119 — commit `7fcb89be1f9276f474617531743ae5f19e6e063d` ("[AW][multi-profile] Remove DEV_SUFFIX from MULTI_PROFILE feature"): https://github.com/chromium/chromium/commit/7fcb89be1f9276f474617531743ae5f19e6e063d
- Renderizador do WebView fora do processo (Android 8.0+, por arquitetura/memória; todos a partir do Android 11): https://chromium.googlesource.com/chromium/src/+/HEAD/android_webview/renderer/README.md
- `WKWebsiteDataStore(forIdentifier:)`, `remove(forIdentifier:)`, `fetchAllDataStoreIdentifiers` (iOS 17.0+): https://developer.apple.com/documentation/webkit/wkwebsitedatastore/init(foridentifier:)
- Bloqueio de login do Google em WebView incorporada (`disallowed_useragent`): https://developers.googleblog.com/upcoming-security-changes-to-googles-oauth-20-authorization-endpoint-in-embedded-webviews/
