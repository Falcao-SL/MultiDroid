# Arquitetura — Duo

O Duo é **dois aplicativos nativos independentes** que implementam o mesmo
produto: usar várias contas do mesmo serviço num aparelho. Nenhum código é
compartilhado entre eles; cada plataforma tem seu núcleo escrito na sua
linguagem. O que é compartilhado é o **contrato**: uma especificação única,
um catálogo de dados, vetores de teste e tokens de design, que as duas
suítes de teste leem e conferem.

```
Duo/
├── android/                 Kotlin + Jetpack Compose        → Google Play (build play) + APK (build completo)
├── ios/                     Swift + SwiftUI                 → App Store
├── docs/
│   ├── spec/                CONTRATO FUNCIONAL (fonte única de comportamento)
│   │   ├── FUNCIONALIDADES.md   telas, rotas, fluxos, motores, paridade, decisões
│   │   ├── CONTEINERES.md       motor web: isolamento por conta (APIs, ciclo de vida, navegação…)
│   │   ├── VIRTUALIZACAO.md     motor nativo (só Android, fase 2)
│   │   ├── dados/*.json         catálogo de serviços, aparência, apps nativos
│   │   └── vetores/*.json       casos de teste lidos pelas duas plataformas
│   ├── design/              CONTRATO VISUAL
│   │   ├── IDENTIDADE_VISUAL.md princípios, componentes, ícones, tom de voz
│   │   └── tokens.json          cores, borda, raio, sombra, tipografia, movimento
│   ├── arquitetura/         esta visão geral
│   ├── produto/             produto, status, monetização, marketing, distribuição
│   └── publicacao/          conformidade, privacidade, pipelines
└── .github/workflows/       android.yml, ios.yml
```

## Dois apps, um contrato

Os dois projetos são independentes: cada um abre sozinho no Android Studio ou
no Xcode e não depende do build do outro. O que os mantém iguais:

| Contrato | Pasta | Conferido por |
|----------|-------|---------------|
| Funcionalidades, telas, textos, fluxos | `docs/spec/FUNCIONALIDADES.md` | matriz de paridade (§17) |
| Isolamento por conta (motor web) | `docs/spec/CONTEINERES.md` | testes instrumentados de isolamento |
| Catálogo e parâmetros | `docs/spec/dados/*.json` | teste de dados em cada plataforma |
| Vetores de teste | `docs/spec/vetores/*.json` | suíte de cada plataforma |
| Identidade e tokens | `docs/design/` | teste de tokens em cada plataforma |

## Regra de mudança

1. Atualize `docs/spec/` (texto e/ou dados/vetores em JSON). Mudança visual:
   `docs/design/`.
2. Os testes das duas plataformas passam a falhar.
3. Implemente em `android/` e `ios/` até os dois ficarem verdes.
4. Atualize a matriz de paridade.

Mudanças só de interface nativa (um gesto do iOS, um menu do Android) não
precisam da spec, desde que não mudem o que o usuário vê como funcionalidade.

## Camadas dentro de cada plataforma

| Camada | Android | iOS |
|--------|---------|-----|
| Núcleo puro (sem UI) | `android/core` (Kotlin/JVM): URL, navegação, apelido, ordem, busca, deep link, catálogo | `ios/DuoCore` (Swift Package): o mesmo |
| Design system | `app/.../ui/theme` (Brutal + tokens) | `Duo/DesignSystem` |
| Telas | `app/.../ui/screens` | `Duo/Features` |
| Motores de conta | `app/.../motor` (`MotorDeConta` + `MotorWeb`, e `MotorNativo` no flavor `completo`) | `Duo/Motor` (só `MotorWeb`) |
| Persistência local | Room / DataStore | arquivos + UserDefaults |
| Testes de paridade | `core/src/test` | `DuoCore/Tests` |
| Testes de isolamento | `app/src/androidTest` | `DuoTests` (simulador) |

### Núcleo puro × motores

O **núcleo** (`core` / `DuoCore`) não importa nada de UI nem de WebView: é o
que os vetores testam sem emulador — validação de URL, classificação de
navegação, apelido, ordem, busca, deep link, leitura do catálogo, e (no
Android) `classificarClonavel`/`diretorioDaCopia`. É onde vive a lógica
comum às duas plataformas.

Os **motores** ficam no app (precisam de `WebView`/`WKWebView` e, no motor
nativo, do sistema), atrás de uma abstração:

```
MotorDeConta            abrir(conta) → View/UIView ; descartar ; limparDados ; excluir ; faxina
├── MotorWeb            Android e iOS — CONTEINERES.md
└── MotorNativo         só Android, flavor "completo", fase 2 — VIRTUALIZACAO.md
```

- Cada conta guarda seu `motor` (FUNCIONALIDADES §3); o app escolhe a
  implementação por esse campo.
- O iOS só tem `MotorWeb`; não há `MotorNativo` (limite da plataforma).
- A classificação de navegação, apelido etc. usam só o núcleo, então valem
  para os dois motores.

## Flavors do Android

| Flavor | Motores | `QUERY_ALL_PACKAGES` | Distribuição |
|--------|---------|:--------------------:|--------------|
| `play` | só `web` | não | Google Play |
| `completo` | `web` + `nativo` | sim | APK no site / outras lojas (fase 2) |

O código do `MotorNativo` e as permissões dele existem **só** no source set
`completo`; o CI confere que o manifesto do `play` não tem
`QUERY_ALL_PACKAGES` nem o processo `:virtual` (PIPELINES_PUBLICACAO). O iOS
tem um alvo só.

## Princípios comuns

1. **Núcleo sem UI** — testável sem emulador nem simulador.
2. **UI fina** — nenhuma regra (validação de URL, classificação, limite) nas
   telas.
3. **Isolamento é sagrado** — nunca cair para armazenamento compartilhado
   (Android sem `MULTI_PROFILE` → tela `sem-suporte`, não WebView comum).
4. **Nada sai do aparelho** — sem servidor, sem analytics, sem SDK de
   terceiros, sem rede partindo do código do Duo.
5. **Sem automação** — o Duo cria o ambiente isolado, não age nas contas.

## Identificadores

| Item | Valor |
|------|-------|
| `applicationId` (Android) | `com.falcaosl.duo` |
| bundle id (iOS) | `com.falcaosl.duo` |
| Apple Team | `6EZ89E2F5S` |
| Esquema de deep link | `duo://` |
| `minSdk` / iOS mínimo | 26 / 17.0 |
