# Distribuição — Duo

O Duo é um app de Mac distribuído **fora da Mac App Store**, assinado com
**Developer ID** e **notarizado** pela Apple (decisão M-6, [spec/MAC.md §4](../spec/MAC.md)).
É **grátis e de código aberto**, sob a GPL-3.0 ([MONETIZACAO.md](MONETIZACAO.md)).

## Por que fora da Mac App Store

O Duo baixa e executa imagens de sistema Android e roda o emulador, que usa
o Hypervisor.framework. A Mac App Store não aceita esse tipo de app (MAC.md
M-6). Developer ID + notarização deixa o Gatekeeper abrir o app sem aviso de
desenvolvedor desconhecido.

## O que precisa

| Item | Situação |
|------|----------|
| Conta Apple Developer (Team `6EZ89E2F5S`) | existe |
| Certificado **Developer ID Application** | a configurar |
| Assinatura com hardened runtime e entitlements necessários | a definir (hoje `build-app.sh` assina ad hoc) |
| Notarização (`notarytool`) e grampo (`stapler`) | a configurar |
| Embalagem (`.dmg` ou `.zip`) | a decidir |
| Página de download | GitHub Releases do repositório (público quando o dono decidir); página no `falcaosl.com` apontando para lá |
| Homebrew cask | depois da primeira release notarizada |
| Atualização automática | a decidir |
| Pipeline de build e release | a criar (GitHub Actions: compilar, assinar, notarizar, anexar à release) |

## O que nunca vai no pacote

- **Imagens de sistema do Google** (Play e Wear): a licença proíbe
  redistribuir. O usuário baixa e aceita dentro do app (decisão M-5).
- **O próprio emulador**: também vem do repositório do Google pelo
  instalador do app (MAC.md §2.7).

## Requisitos para o usuário

- macOS 26 ou mais novo, Mac com Apple Silicon.
- Memória: cada celular custa ~4,5 GB (medido na fase 0); um Mac de 16 GB
  comporta ~2 aparelhos sem swap, um de 24 GB ~3–4 (MAC.md §3).
- Disco para o emulador e as imagens baixadas.

## Legal

- **Licença do Duo**: GPL-3.0 ([LICENSE](../../LICENSE)); cada release publica o
  código-fonte junto.
- **Licença das imagens do Google**: o Duo não vende nada nem redistribui as
  imagens, mas um app de uso geral que as baixa continua em área cinzenta;
  revisar com advogado antes de tornar o repositório público (MAC.md §5).
- **LGPD**: o Duo não coleta dado pessoal; mesmo assim, publicar a
  [política de privacidade](../publicacao/POLITICA_DE_PRIVACIDADE.md) na
  página de download.
- **Termos dos serviços**: o Duo não automatiza contas; dizer na página que
  alguns apps (bancos, por exemplo) não abrem em emulador.

## Cronograma resumido

| Etapa | Situação |
|-------|----------|
| Fase 0 (medir) | **Feito** |
| Fase 1 (o app) | quase feito |
| Developer ID + notarização | a fazer |
| Página de download e política publicada | a fazer |
| Lançamento | a decidir |
