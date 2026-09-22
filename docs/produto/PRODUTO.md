# Produto — Duo

**Várias contas do mesmo serviço num só aparelho — cada uma isolada, e nada sai do seu aparelho.**

| | |
|---|---|
| **Plataformas** | Android (Kotlin) e iOS (Swift) — dois apps nativos com as mesmas funcionalidades e a mesma identidade |
| **Idioma** | Português do Brasil (preparado para inglês) |
| **Preço** | A decidir ([`MONETIZACAO.md`](MONETIZACAO.md)) |
| **Estado** | ver [`STATUS.md`](STATUS.md) |

## O problema

Muita gente tem mais de uma conta no mesmo serviço: a pessoal e a da loja no
Instagram, dois TikToks, o WhatsApp do trabalho. Os apps oficiais deixam
alternar mal e porcamente (ou nem deixam), e ficar saindo e entrando perde a
sessão e o contexto. As soluções existentes ou são sites cheios de anúncio,
ou "clonadores" que pedem permissões demais e mandam dados para fora.

## Para quem é

- Quem tem conta pessoal e profissional no mesmo serviço.
- Social media e criadores que gerenciam várias contas.
- Quem quer separar trabalho e vida pessoal no mesmo aparelho.

## O que ele faz

- Cada conta abre num **contêiner web isolado**: a versão web do serviço com
  cookies, login e dados próprios. Duas contas do mesmo serviço não se veem.
- Catálogo com 14 serviços (TikTok, Instagram, X, YouTube, Telegram Web,
  WhatsApp Web e mais) + qualquer site por endereço.
- Troca rápida entre contas, atalho por conta na tela inicial, bloqueio por
  biometria, tema claro/escuro.
- **No Android (fase 2)**, além do contêiner web, dá para **clonar o app
  nativo** já instalado, para quem quer o app oficial com notificações.
- Funciona sem servidor, sem conta Duo, sem coletar nada.

## Por que este e não outro

- **Privacidade real**: nada sai do aparelho, sem analytics, sem SDK de
  terceiros. Diferente dos clonadores que vivem de dados.
- **Honesto**: diz o que não funciona (login do Google em WebView,
  notificações web) em vez de prometer o impossível.
- **Duas plataformas** com a mesma cara e o mesmo comportamento.

## O que ele deliberadamente não faz

- Não automatiza nada nas contas (não posta, não curte, não segue por você).
- Não recebe notificação push dos serviços no motor web (limite técnico, §11
  da spec).
- Não esconde o IP nem finge ser outro aparelho.
- Não contorna bloqueio de login nem atestação de segurança de apps.
- Não tem app web nem versão para computador.

## Princípios de produto

1. **Isolamento é a promessa.** Se duas contas se misturam, o produto falhou.
2. **Nada sai do aparelho.** Sem servidor a manter, sem dado a vazar.
3. **Honestidade sobre limites.** É melhor avisar do que decepcionar na
   revisão da loja e na avaliação do usuário.
