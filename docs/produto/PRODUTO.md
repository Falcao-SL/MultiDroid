# Produto — Duo

**Vários aparelhos Android lado a lado no seu Mac — cada um com as suas
contas, organizados numa mesa.**

| | |
|---|---|
| **Plataforma** | macOS 26, Apple Silicon — só Mac ([spec/MAC.md](../spec/MAC.md)) |
| **Idioma** | Português do Brasil (a interface hoje); outros idiomas a decidir |
| **Preço** | A decidir ([`MONETIZACAO.md`](MONETIZACAO.md)) |
| **Estado** | ver [`STATUS.md`](STATUS.md) |

## O problema

Quem precisa de várias contas do mesmo serviço, ou de vários aparelhos
Android ao mesmo tempo, hoje empilha celulares na mesa ou abre emuladores um
a um no Android Studio, cada um numa janela solta, sem visão do que está
ligado nem de quanta memória sobra. No Windows existem gerenciadores de
múltiplas instâncias; no Mac com Apple Silicon, não há um equivalente leve.

## Para quem é

- Quem gerencia várias contas do mesmo app (social media, criadores, lojas).
- Quem testa um app em vários formatos ao mesmo tempo (celular, tablet,
  dobrável, relógio).
- Quem quer o app oficial do serviço, com notificações, em vez da versão web.

## O que ele faz

- Roda **celular, tablet, dobrável e relógio (Wear OS)** sobre o Android
  Emulator oficial, sem janela, nativo em Apple Silicon.
- **Painel**: lista de aparelhos, estado, memória e CPU ao vivo, teto de
  memória, grupos, ações em lote.
- **Mesa**: grade livre (arrastar, redimensionar), arranjo automático ou por
  grupo, uma mesa por janela, aparelho ampliado.
- **Duplicar** um aparelho na hora, sem gastar disco (`clonefile`).
- **Pausar** um aparelho: CPU zero, memória mantida; desligar salva o estado
  e ele volta em segundos.
- **Instala sozinho** o emulador e as imagens, direto do Google, mostrando a
  licença; não precisa de Android Studio.
- Sem servidor, sem conta Duo, sem coleta de uso.

## Por que este e não outro

- **Nativo de Mac**: SwiftUI, um processo, feito para não competir por
  memória com os Androids.
- **Emulador oficial**: GPU do Mac, imagens arm64 sem tradução, Play Store e
  Wear OS.
- **Organização**: painel e mesa são o diferencial, não a virtualização
  (decisão M-1).

## O que ele deliberadamente não faz

- Não automatiza nada nas contas; sincronizar entrada (fase 2) será manual e
  visível.
- Não embute nem redistribui imagens do Google: cada usuário baixa e aceita
  a licença (decisão M-5).
- Não burla a verificação de integridade dos apps (bancos e alguns apps não
  abrem em emulador).
- Não roda em Mac com Intel nem em outros sistemas.

## Princípios de produto

1. **Leve acima de tudo.** O Duo gasta só com o que está à vista.
2. **Tudo num painel.** Nenhuma ação exige terminal ou Android Studio.
3. **Nada sai do Mac por causa do Duo.** Só baixa do Google o que o usuário
   pede.
4. **Honestidade sobre limites.** Dizer quanto de memória cada aparelho
   custa e o que não funciona em emulador.
