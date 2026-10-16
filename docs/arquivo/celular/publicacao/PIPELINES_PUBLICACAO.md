# CI/CD e publicação — Android e iOS

> **Esqueleto.** Os workflows ainda não existem; este documento define a
> forma que eles devem ter. A fonte da verdade, quando existirem, serão
> `.github/workflows/android.yml` e `ios.yml`.

Dois workflows, um por plataforma:

| Plataforma | Workflow | Destino |
|------------|----------|---------|
| Android | `.github/workflows/android.yml` (`name: Android`) | Google Play (build `play`); o build `completo` é artefato para distribuição fora da Play |
| iOS | `.github/workflows/ios.yml` (`name: iOS`) | App Store Connect / TestFlight |

## Visão geral

| | Android | iOS |
|---|---------|-----|
| **Projeto** | `android/` | `ios/` |
| **Runner** | `ubuntu-latest` (emulador x86_64 com KVM para os testes de isolamento) | `macos-26` (Xcode 26+) |
| **CI (testes + build)** | `pull_request` e `push` na `main` que toquem `android/**`, `docs/spec/**`, `docs/design/**` ou o workflow | idem, com `ios/**` |
| **Publica quando** | tag `android-vX.Y.Z` ou `workflow_dispatch` com `publicar = true` | tag `ios-vX.Y.Z` ou `workflow_dispatch` com `publicar = true` |
| **Environment** | `google-play` | `app-store` |

## Cadeia de jobs

Mesma cadeia nos dois; cada job só começa se o anterior passar:

```
testes-unitarios ──► testes-completos ──► build ──► publicar
```

| Job | Android | iOS |
|-----|---------|-----|
| `testes-unitarios` | `:core:test` — núcleo + paridade com `docs/spec/vetores` e `docs/spec/dados`; `:app:testDebugUnitTest` — tokens de `docs/design` | `swift test` no `DuoCore` — núcleo + vetores + tokens |
| `testes-completos` | `:app:lintPlayDebug` (+ regra que falha se o manifesto `play` tiver `QUERY_ALL_PACKAGES`); `connectedDebugAndroidTest` no emulador — **testes de isolamento** do motor web (CONTEINERES §10) | `xcodebuild test` no simulador — testes do app + `DuoTests` (isolamento) + UI |
| `build` | `:app:bundlePlayRelease` (AAB assinado) + `:app:assembleCompletoRelease` (APK, artefato `android-apk-completo`); confere zero permissões de risco no manifesto `play` | `archive` Release; com os secrets, exporta `.ipa`; sem eles, arquiva com `CODE_SIGNING_ALLOWED=NO` só para validar |
| `publicar` | envia o AAB `play` à trilha (o `completo` **não** é enviado à Play; é publicado fora — DISTRIBUICAO) | envia o `.ipa` ao App Store Connect |

- O build `completo` (motor nativo) **nunca** é enviado à Google Play.
- Só a fase 2 gera o `completo`; na fase 1 o Android produz só o `play`.

## Versionamento

| Plataforma | Número interno | Versão visível |
|------------|----------------|----------------|
| Android | `versionCode = github.run_number + <offset>` | `versionName = X.Y.Z` da tag `android-vX.Y.Z` |
| iOS | `CURRENT_PROJECT_VERSION = github.run_number` | `MARKETING_VERSION = X.Y.Z` da tag `ios-vX.Y.Z` |

Tag fora do formato falha o job `build`. `android-v*` só dispara o Android;
`ios-v*` só o iOS.

## Secrets (a criar)

| Workflow | Secret | Conteúdo |
|----------|--------|----------|
| Android | `ANDROID_KEYSTORE_BASE64` | keystore de upload em base64 |
| Android | `ANDROID_KEYSTORE_PASSWORD` | senha do keystore |
| Android | `ANDROID_KEY_ALIAS` | alias da chave |
| Android | `ANDROID_KEY_PASSWORD` | senha da chave |
| Android | `PLAY_SERVICE_ACCOUNT_JSON` | conta de serviço da API do Play |
| iOS | `IOS_CERT_P12_BASE64` | certificado Apple Distribution (.p12) base64 |
| iOS | `IOS_CERT_PASSWORD` | senha do .p12 |
| iOS | `IOS_PROFILE_BASE64` | perfil App Store `Duo App Store` (.mobileprovision) base64 |
| iOS | `ASC_API_KEY_ID` | Key ID da API do App Store Connect |
| iOS | `ASC_API_ISSUER_ID` | Issuer ID |
| iOS | `ASC_API_KEY_P8` | conteúdo do `AuthKey_<id>.p8` |

## O que falta configurar

- [ ] Criar os dois workflows.
- [ ] Environments `google-play` e `app-store` (com revisores e restrição de
      tags/branches).
- [ ] Secrets acima.
- [ ] Conta Google Play (US$ 25) e primeira release manual do build `play`
      (a API do Play só aceita envio depois do primeiro AAB pela interface).
- [ ] App Store Connect: App ID `com.falcaosl.duo` (Team `6EZ89E2F5S`) e
      perfil `Duo App Store` com o nome usado em `PROVISIONING_PROFILE_SPECIFIER`
      e no `ExportOptions.plist`.
- [ ] Publicar a política de privacidade e colar a URL nas fichas.

## Como publicar (quando pronto)

```bash
# Android → Play (build play), versionName 1.0.0
git tag android-v1.0.0 && git push origin android-v1.0.0

# iOS → App Store Connect / TestFlight
git tag ios-v1.0.0 && git push origin ios-v1.0.0
```

O build `completo` (motor nativo, fase 2) é distribuído fora da Play; o
processo dele (assinatura, hospedagem do APK, updater) será detalhado quando
a fase 2 for aprovada (U-6/U-7).
