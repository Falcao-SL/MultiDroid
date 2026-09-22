# Duo

Várias contas do mesmo serviço — TikTok, Instagram, Snapchat, YouTube e
outros — num só aparelho. Cada conta abre num contêiner isolado, com
cookies e sessão próprios, e nada sai do seu aparelho.

| Plataforma | Pasta | Stack |
|---|---|---|
| Android | [`android/`](android) | Kotlin + Jetpack Compose |
| iOS / iPadOS | [`ios/`](ios) | Swift + SwiftUI |

Sem app web. Os dois projetos são independentes; o que os mantém iguais é o
contrato em [`docs/`](docs) (especificação, catálogo, vetores e tokens), lido
pelos testes das duas plataformas.

## Como funciona

Cada conta é um **motor de conta**:

- **Motor web** (Android e iOS): um perfil de navegador isolado que abre a
  versão web do serviço — iOS `WKWebsiteDataStore(forIdentifier:)`, Android
  `androidx.webkit` multiperfil (`ProfileStore` + `WebViewCompat.setProfile`).
  É o motor da versão 1. Detalhes em [`docs/spec/CONTEINERES.md`](docs/spec/CONTEINERES.md).
- **Motor nativo** (só Android, fase 2): clona por virtualização um app já
  instalado. Fica num build separado, fora da Play.
  Detalhes em [`docs/spec/VIRTUALIZACAO.md`](docs/spec/VIRTUALIZACAO.md).

Não é clonagem de APK/IPA no iOS nem automação de contas: o Duo isola
sessões, o usuário faz o login.

## Documentação

Índice em [docs/README.md](docs/README.md). Principais:

| Documento | Conteúdo |
|---|---|
| [docs/spec/FUNCIONALIDADES.md](docs/spec/FUNCIONALIDADES.md) | Telas, fluxos, motores, paridade e decisões |
| [docs/spec/CONTEINERES.md](docs/spec/CONTEINERES.md) | Isolamento por conta (motor web) |
| [docs/spec/VIRTUALIZACAO.md](docs/spec/VIRTUALIZACAO.md) | Motor nativo (Android, fase 2) |
| [docs/design/IDENTIDADE_VISUAL.md](docs/design/IDENTIDADE_VISUAL.md) | Identidade visual e componentes |
| [docs/produto/STATUS.md](docs/produto/STATUS.md) | Estágio, fases, riscos |
| [docs/publicacao/CONFORMIDADE_LOJAS.md](docs/publicacao/CONFORMIDADE_LOJAS.md) | Guidelines Apple/Google |

Identificador `com.falcaosl.duo` · deep link `duo://` · Android `minSdk` 26 ·
iOS 17.0.

---

Um app da [Falcao SL](https://falcaosl.com).
