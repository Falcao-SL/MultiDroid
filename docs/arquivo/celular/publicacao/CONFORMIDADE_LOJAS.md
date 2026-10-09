# Conformidade com as guidelines — Apple App Store e Google Play

Auditoria do Duo contra as diretrizes das duas lojas. Políticas conferidas em
2026-09-21 ([fontes](#fontes)). Onde uma política é séria, listo a mitigação
concreta no produto, na ficha e nas notas ao revisor.

## Resumo dos riscos

| # | Risco | Loja | Gravidade | Mitigação |
|---|-------|------|:---------:|-----------|
| R1 | "App cuja função principal é exibir site de terceiros" | Play (Spam/WebView) e Apple 4.2 | **alta** | ver §1 |
| R2 | Marca/logo/nome de serviços de terceiros | Play (PI/Personificação) e Apple 4.1/5.2.1 | **alta** | ver §2 |
| R3 | Acesso a serviços de terceiros / termos de uso | Play (Abuso de dispositivo e rede) e Apple 5.2.2 | média | ver §3 |
| R4 | Motor nativo: virtualização, `QUERY_ALL_PACKAGES`, interferir em apps | Play (Abuso de dispositivo e rede) | **muito alta** | ver §4 — fora da Play |
| R5 | Conflito de marca no nome "Duo" | ambas | média | ver §5 |
| R6 | WebKit obrigatório para navegar | Apple 2.5.6 | ok | ver §6 |
| R7 | Privacidade / login | ambas, Apple 5.1.1 | baixa | ver §7 |

---

## 1. R1 — "só um site de terceiros" (Play Spam/WebView; Apple 4.2)

- **Play** não permite "apps cuja função principal é fornecer um webview de
  um site sem permissão do dono". **Apple 4.2** barra o que não passa de um
  site reempacotado; **4.2.2** barra web clipping e coleção de links.
- **Por que o Duo não é isso**: a função principal não é exibir **um** site,
  e sim **gerenciar várias contas isoladas** — cada uma um perfil de
  navegador persistente e separado. O valor está no isolamento por conta, na
  troca rápida, no bloqueio por biometria, nos atalhos por conta, no catálogo
  e no site personalizado. É um gerenciador de sessões, não um navegador de
  um site só.
- **Mitigações**:
  - a ficha descreve o benefício (várias contas isoladas), não "abrir o
    TikTok";
  - funcionalidades nativas de peso: contêiner isolado, troca rápida,
    biometria por app e por conta, atalhos na tela inicial, tema, downloads;
  - o site personalizado mostra que não é atado a um serviço;
  - notas ao revisor com um roteiro (adicionar duas contas do mesmo serviço e
    alternar sem deslogar) mostrando o que um navegador comum não faz.
- Sobre "sem permissão do dono do site" (Play): o Duo abre a **versão web
  pública** que qualquer navegador abre, com o login do próprio usuário; não
  reempacota o site, não injeta conteúdo, não remove anúncios. Ainda assim, o
  revisor pode discordar — plano B: reforçar o site personalizado e o
  enquadramento de "navegador multi-perfil" (categoria de navegadores).

## 2. R2 — marcas de terceiros (Play PI/Personificação; Apple 4.1/5.2.1)

- **Play** proíbe usar marca/logo de terceiro de forma que confunda sobre a
  origem, e apps que se passam por outra entidade. **Apple 4.1** proíbe usar
  ícone/marca/nome de outro desenvolvedor; **5.2.1** proíbe material de
  terceiro sem permissão e metadados copycat.
- **Regras do Duo (já na spec e no design)**:
  - **nenhum logo, ícone ou cor de marca** de serviço no app, no ícone, no
    nome ou nas screenshots (`dados/servicos.json` usa cores do **nosso**
    tema; IDENTIDADE_VISUAL);
  - serviços identificados só pelo **nome em texto** e por uma cor nossa; na
    aba de clonar (Android), usa-se o ícone/nome que o **próprio sistema** já
    exibe para o app instalado, não um recurso embutido pelo Duo;
  - tela Sobre e ficha declaram: **"O Duo não é afiliado, patrocinado nem
    aprovado por nenhum serviço; os nomes pertencem aos seus donos e servem
    só para identificar o site que você abre."**;
  - nome/subtítulo da loja sem marca de serviço (MARKETING_ASO).
- Nomes de marca em texto nominativo ("funciona com a versão web de serviços
  como…") são uso nominativo, sem logo e sem sugerir parceria.

## 3. R3 — serviços de terceiros e termos de uso (Play; Apple 5.2.2)

- **Apple 5.2.2**: se o app usa/exibe conteúdo de um serviço de terceiro,
  garanta que os termos permitem; autorização mediante pedido. **Play (Abuso
  de dispositivo e rede)**: não usar um serviço/API de forma que viole os
  termos dele.
- **O que o Duo faz e não faz**: não automatiza, não raspa, não usa API
  privada dos serviços, não contorna proteção. Ele abre a versão web pública
  com o navegador do sistema (WebKit/WebView), com o login do usuário — o
  mesmo que a pessoa faria no Safari/Chrome. Não há "acesso automatizado".
- **Termos dos serviços sobre múltiplas contas / clientes** (2026-09-21):
  - **Meta (Facebook/Instagram)**: pede "criar somente uma conta (a sua
    própria)" e proíbe **coletar dados por meios automatizados**. O Duo não
    coleta nem automatiza; múltiplas contas são de responsabilidade do
    usuário (muita gente tem conta pessoal e comercial legítimas). Isso é
    entre o usuário e o serviço, não uma violação praticada pelo app.
    **Threads** usa login do Instagram.
    → **item de atenção**; manter Instagram/Facebook, mas ciente.
  - **Snapchat**: proíbe **"usar ou desenvolver apps de terceiros que
    interajam com os Serviços sem consentimento escrito"** e acesso
    automatizado. Um contêiner web é limítrofe. **Maior risco do catálogo.**
    → candidato a **sair do catálogo de lançamento** (U-3); marcado
    `experimental`.
  - **TikTok**: proíbe scripts automatizados e usar conta/sistema de outro
    sem autorização; **não** fixa número de contas. O Duo (sem automação)
    fica do lado de fora dessas proibições.
  - **X, LinkedIn, Pinterest, Reddit, Twitch, Discord, Telegram, YouTube**:
    o uso é navegar na versão web logado; sem automação, sem raspagem.
- **WhatsApp Web** não cria conta nova: vincula um aparelho a um WhatsApp
  existente em outro celular. Está no catálogo como `experimental` e a folha
  de adicionar explica isso (FUNCIONALIDADES / catálogo).
- **Documentação**: a tela Sobre e a política deixam claro que o Duo só isola
  sessões, não automatiza nada, e que o uso de cada serviço segue os termos
  do serviço, sob responsabilidade do usuário.

## 4. R4 — motor nativo (só Android, fora da Play)

O motor nativo (VIRTUALIZACAO.md) executa cópias de outros apps por
virtualização. Isso colide com a política de **Abuso de dispositivo e rede**
da Play ("apps que interferem no funcionamento de outros apps ou do sistema")
e exige `QUERY_ALL_PACKAGES` (permissão de alto risco, com formulário de
declaração e **casos de uso permitidos que não incluem "clonador"**).

- **Decisão**: o motor nativo **não vai para a Google Play**. Fica no build
  **`completo`**, distribuído fora da Play (DISTRIBUICAO.md). O build
  **`play`** só tem o motor web e **não** declara `QUERY_ALL_PACKAGES` nem o
  processo `:virtual` — conferido pelo CI.
- **Concorrentes existem na Play** (apps de "espaço paralelo/dual"), mas
  vários já foram removidos e o item é instável; não é base segura para
  apostar a conta.
- **Recomendação registrada**: manter o build da Play só com o motor web;
  levar o nativo à fase 2 fora da Play, e só se U-6 (licença) for resolvido.

## 5. R5 — conflito de marca no nome "Duo"

- Conflitos óbvios: **Google Duo** (marca da Google) e **Duolingo** ("Duo").
  Também há vários apps "Duo".
- **Mitigação**: nome de loja com qualificador — **`Duo: Múltiplas Contas`**
  (recomendado) — e ícone e identidade próprios, sem nada que lembre Google
  Duo ou Duolingo. O `applicationId`/bundle `com.falcaosl.duo` e a marca
  Falcao SL reduzem confusão de origem. Avaliar antes do envio se algum nome
  alternativo (MARKETING_ASO) tem menos atrito de busca.

## 6. R6 — WebKit (Apple 2.5.6)

- **2.5.6**: apps que navegam na web devem usar o WebKit. O motor web do iOS
  usa `WKWebView` (WebKit) — **conforme**, sem entitlement de motor
  alternativo.

## 7. R7 — privacidade e login

- **Apple 5.1.1(i)**: política de privacidade publicada e no app (tela
  Sobre). **5.1.1(v)**: o Duo não tem conta própria, então "excluir conta"
  não se aplica; e não força login para funcionar (o login é no site do
  serviço, escolha do usuário).
- **App Privacy / Data safety**: **nenhum dado coletado** — o Duo não tem
  servidor, analytics nem SDK de terceiros.
- **Login do Google em WebView**: o Google bloqueia (`disallowed_useragent`)
  e o Duo **não contorna** — o que é o comportamento correto de segurança e
  evita a acusação de burlar proteção (Play Abuso de dispositivo e rede). O
  app avisa o usuário (FUNCIONALIDADES §6.4).

---

## Checklist Apple App Store

| Guideline | Item | Status |
|-----------|------|--------|
| 2.5.6 | Navegação usa WebKit (`WKWebView`) | planejado ✅ |
| 2.5.2 | Não baixa/executa código que altera o app (sem motor nativo no iOS) | ✅ por design |
| 4.1 | Sem ícone/marca/nome de terceiros | ✅ regra na spec/design |
| 4.2 / 4.2.2 | Funcionalidade além de "um site" | ⚠️ mitigar com notas ao revisor (§1) |
| 4.3 | Não é cópia de app existente | ✅ isolamento + gerenciamento próprios |
| 5.1.1 | Política publicada + Data Not Collected | ⚠️ publicar URL e preencher no ASC |
| 5.2.1 | Sem material/metadado de terceiro | ✅ (§2) |
| 5.2.2 | Serviços de terceiros conforme termos; sem automação | ⚠️ documentado (§3); Snapchat em avaliação |
| 5.2.3 | Não baixar mídia de terceiros sem autorização | ✅ só download que o site oferece (FUNCIONALIDADES §10) |

## Checklist Google Play

| Política | Item | Status |
|----------|------|--------|
| Spam / WebView | Função é gerenciar contas, não exibir um site | ⚠️ ficha + notas (§1) |
| Propriedade intelectual | Sem logo/nome de marca no app/ícone/nome/prints | ✅ (§2) |
| Personificação | Não se passa por serviço; declara não afiliação | ✅ |
| Abuso de dispositivo e rede | Sem automação/raspagem; não contorna proteção; login do Google não é burlado | ✅ |
| Abuso de dispositivo e rede | Motor nativo/virtualização | ⛔ **fora da Play** (build `completo`); ✅ no build `play` |
| Permissões | Build `play` sem `QUERY_ALL_PACKAGES` (conferido no CI) | ✅ |
| Segurança dos dados | Nenhum dado coletado/compartilhado | ⚠️ preencher no Console |
| Classificação | Livre | ✅ |
| Política de privacidade | URL na ficha | ⚠️ publicar e colar |

## Ações humanas pendentes

1. Publicar a política de privacidade e colar a URL nas duas fichas.
2. Preencher Data safety (Play) e App Privacy (Apple) como "nenhum dado
   coletado"; anúncios "Não".
3. Escrever as **notas ao revisor** (roteiro de duas contas do mesmo serviço)
   para as duas lojas.
4. Decidir U-3 (Snapchat/WhatsApp/Discord no catálogo de lançamento) e U-6/U-7
   (motor nativo) antes da fase 2.
5. Escolher e registrar o nome de loja (U-2) evitando conflito com Google Duo
   e Duolingo.

## Fontes

- Play — Spam e funcionalidade mínima (webviews): https://support.google.com/googleplay/android-developer/answer/9899034
- Play — Propriedade intelectual (marca/logo): https://support.google.com/googleplay/android-developer/answer/9888072
- Play — Abuso de dispositivo e rede (termos de terceiros, interferência): https://support.google.com/googleplay/android-developer/answer/9888379
- Play — `QUERY_ALL_PACKAGES` / visibilidade de pacotes: https://support.google.com/googleplay/android-developer/answer/10158779
- Apple — App Store Review Guidelines (2.5.6, 4.1, 4.2, 4.3, 5.1.1, 5.2.1, 5.2.2, 5.2.3): https://developer.apple.com/app-store/review/guidelines/
- Termos: Meta https://www.facebook.com/legal/terms · Snap https://snap.com/en-US/terms · TikTok https://www.tiktok.com/legal/page/row/terms-of-service/en
- Google bloqueia login em WebView incorporada: https://developers.googleblog.com/upcoming-security-changes-to-googles-oauth-20-authorization-endpoint-in-embedded-webviews/
