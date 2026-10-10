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
| Certificado **Developer ID Application** | no Keychain; `build-app.sh` usa sozinho quando encontra |
| Assinatura com hardened runtime | **feito**: Developer ID + `--options runtime --timestamp`, sem entitlements (o app não usa o Hypervisor nem sandbox; quem usa é o emulador, assinado pelo Google) |
| Notarização (`notarytool`) e grampo (`stapler`) | `scripts/release.sh` faz; falta guardar a credencial uma vez: `xcrun notarytool store-credentials notary --apple-id <e-mail> --team-id 6EZ89E2F5S` |
| Embalagem | **`.dmg`** assinado (`mac/build/MultiDroid-<versão>.dmg`, ~11 MB), com atalho para Aplicativos |
| Página de download | GitHub Releases do repositório (público quando o dono decidir); página no `falcaosl.com` apontando para lá |
| Homebrew cask | depois da primeira release notarizada |
| Atualização automática | a decidir |
| Pipeline de build e release | a criar (GitHub Actions: compilar, assinar, notarizar, anexar à release) |

## O que nunca vai no pacote

- **Imagens de sistema do Google** (Play e Wear): a licença proíbe
  redistribuir. O usuário baixa e aceita dentro do app (decisão M-5).
- **O próprio emulador**: também vem do repositório do Google pelo
  instalador do app (MAC.md §2.7).

## Como gerar uma release

```sh
mac/scripts/release.sh            # compila, assina, monta o .dmg, notariza e grampeia
NOTARIZE=0 mac/scripts/release.sh # só monta e assina (para testar)
```

A versão vem de `CFBundleShortVersionString` em `build-app.sh`.

## Requisitos para o usuário

- macOS 26 ou mais novo, Mac com Apple Silicon.
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
| Developer ID + notarização | assinatura feita; notarização aguarda a credencial |
| Página de download e política publicada | a fazer |
| Lançamento | a decidir |
