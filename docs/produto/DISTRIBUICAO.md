# Distribuição — MultiDroid

O MultiDroid é um app de Mac distribuído **fora da Mac App Store**, assinado com
**Developer ID** e **notarizado** pela Apple (decisão M-6, [spec/MAC.md §4](../spec/MAC.md)).
É **grátis e de código aberto**, sob a GPL-3.0 ([MONETIZACAO.md](MONETIZACAO.md)).

## Por que fora da Mac App Store

O MultiDroid baixa e executa imagens de sistema Android e roda o emulador, que usa
o Hypervisor.framework. A Mac App Store não aceita esse tipo de app (MAC.md
M-6). Developer ID + notarização deixa o Gatekeeper abrir o app sem aviso de
desenvolvedor desconhecido.

## O que precisa

| Item | Situação |
|------|----------|
| Conta Apple Developer (Team `6EZ89E2F5S`) | existe |
| Certificado **Developer ID Application** | no Keychain; `mac/scripts/build-app.sh` usa sozinho quando encontra |
| Assinatura com hardened runtime | **feito**: Developer ID + `--options runtime --timestamp`, sem entitlements (o app não usa o Hypervisor nem sandbox; quem usa é o emulador, assinado pelo Google) |
| Notarização (`notarytool`) e grampo (`stapler`) | `mac/scripts/release.sh` no Mac; no CI via `APPLE_API_KEY` + ID + ISSUER (`scripts/ci/notarize.sh`) |
| Embalagem | **`.dmg`** assinado (`mac/build/MultiDroid-<versão>.dmg`), com atalho para Aplicativos |
| Página de download | GitHub Releases; site `multidroid.falcaosl.com` |
| Homebrew cask | **no repo**: [`packaging/homebrew/Casks/multidroid.rb`](../../packaging/homebrew/Casks/multidroid.rb); publicar no tap é passo manual do dono |
| Atualização automática | a decidir |
| Pipeline de build e release | **feito** — ver tabela abaixo |

## Pipelines no GitHub Actions

| Workflow | Quando | O que faz |
|----------|--------|-----------|
| [`swift-mac.yml`](../../.github/workflows/swift-mac.yml) | push/PR em `mac/` | Compila `MultiDroid.app` (ad hoc no PR) |
| [`release-swift.yml`](../../.github/workflows/release-swift.yml) | tag `v*` = versão Swift (`0.2`) | `.dmg` da 0.2; assina/notariza se houver segredos; anexa à release |
| [`rust.yml`](../../.github/workflows/rust.yml) | push/PR em `rust/` | fmt, clippy, test, build nos 3 SO; `emulador-linux` + `leve-linux`; macOS/Windows emulador só sob pedido |
| [`release.yml`](../../.github/workflows/release.yml) | tag `v*` = versão Cargo (`0.3.0`) | Preview: dmg/exe/AppImage/deb/rpm; dmg assinado/notarizado se houver segredos |
| [`deploy-site.yml`](../../.github/workflows/deploy-site.yml) | site | Publica multidroid.falcaosl.com |

### Segredos Apple (opcionais)

Sem eles o CI **não falha**: sobe artefato unsigned/ad hoc e imprime `::notice::`.
Com eles, assina e notariza no runner.

| Segredo | Uso |
|---------|-----|
| `APPLE_CERTIFICATE_P12` | `.p12` do Developer ID em **base64** |
| `APPLE_CERTIFICATE_PASSWORD` | senha do `.p12` |
| `APPLE_SIGN_IDENTITY` | opcional — `Developer ID Application: …` |
| `APPLE_API_KEY` | chave `.p8` (PEM cru ou base64) |
| `APPLE_API_KEY_ID` | Key ID |
| `APPLE_API_ISSUER` | Issuer UUID |

Scripts: `scripts/ci/import-apple-cert.sh`, `resign-dmg.sh`, `notarize.sh`.

### Fluxo híbrido (sem segredos no GitHub)

1. CI publica o `.dmg` unsigned na release.
2. No Mac do dono: `mac/scripts/release.sh` (0.2) ou `rust/scripts/release-mac.sh <tag>` (Preview; `DO_CI=1` reassina o da release).
3. `gh release upload <tag> ….dmg --clobber`.

## Homebrew

Cask da **0.2** no repositório:

```sh
brew install --cask ./packaging/homebrew/Casks/multidroid.rb
# ou, depois de publicar no tap falcaosl / iramarfalcao:
# brew tap <dono>/falcaosl && brew install --cask multidroid
```

Atualizar `version`/`sha256` após release notarizada:

```sh
packaging/homebrew/update-cask.sh v0.2
```

Publicar no tap Homebrew (core ou próprio) continua sendo ação manual do dono.

## O que nunca vai no pacote

- **Imagens de sistema do Google** (Play e Wear): a licença proíbe
  redistribuir. O usuário baixa e aceita dentro do app (decisão M-5).
- **O próprio emulador**: também vem do repositório do Google pelo
  instalador do app (MAC.md §2.7).

## Como gerar uma release local (0.2)

```sh
mac/scripts/release.sh            # compila, assina, monta o .dmg, notariza e grampeia
NOTARIZE=0 mac/scripts/release.sh # só monta e assina (para testar)
```

A versão vem de `CFBundleShortVersionString` em `build-app.sh`.
Credencial local (uma vez): `xcrun notarytool store-credentials notary --apple-id <e-mail> --team-id 6EZ89E2F5S`.

## Requisitos para o usuário

- macOS 26 (Tahoe) ou mais novo, Mac com Apple Silicon.
- Memória: cada celular custa ~4,5 GB (medido na fase 0); um Mac de 16 GB
  comporta ~2 aparelhos sem swap, um de 24 GB ~3–4 (MAC.md §3).
- Disco para o emulador e as imagens baixadas.

## Legal

- **Licença do MultiDroid**: GPL-3.0 ([LICENSE](../../LICENSE)); cada release publica o
  código-fonte junto.
- **Licença das imagens do Google**: o MultiDroid não vende nada nem redistribui as
  imagens, mas um app de uso geral que as baixa continua em área cinzenta;
  revisar com advogado antes de tornar o repositório público (MAC.md §5).
- **LGPD**: o MultiDroid não coleta dado pessoal; mesmo assim, publicar a
  [política de privacidade](../publicacao/POLITICA_DE_PRIVACIDADE.md) na
  página de download.
- **Termos dos serviços**: o MultiDroid não automatiza contas; dizer na página que
  alguns apps (bancos, por exemplo) não abrem em emulador.

## Cronograma resumido

| Etapa | Situação |
|-------|----------|
| Fase 0 (medir) | **Feito** |
| Fase 1 (o app) | quase feito |
| Developer ID + notarização | assinatura feita; CI secrets-ready; local com perfil `notary` |
| Pipeline GitHub Actions | **Feito** (`swift-mac`, `release-swift`, `rust`, `release`) |
| Homebrew cask no repo | **Feito**; tap remoto a cargo do dono |
| Página de download e política publicada | site no ar; política em docs |
| Lançamento | 0.2 publicada; Preview Rust em pré-release |
