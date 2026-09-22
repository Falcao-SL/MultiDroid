# Monetização — Duo

**Modelo: a decidir pelo usuário.** Este documento propõe opções; nenhuma
está implementada. Na spec, o limite de contas é um parâmetro
(`dados/aparencia.json → limites`) **sem paywall** nesta fase: `contasGratis`
= `null` (sem limite de plano), `maxContas` = 50 (teto técnico para todos).

> Enquanto esta decisão não for tomada, o app não tem compra, assinatura nem
> anúncio, e a ficha das lojas declara "sem compras no app". Mudar isso é
> reescrever este arquivo primeiro, com data, e só depois mexer no código.

## O que o Duo já garante, em qualquer modelo

- Sem anúncios e sem SDK de anúncio (o app não coleta nada; anúncio traria
  rastreador e quebraria "nenhum dado coletado").
- Sem servidor: o custo de operação é ~zero, então não há conta recorrente a
  cobrir.

## Opções propostas

| Modelo | Como | Prós | Contras |
|--------|------|------|---------|
| **A. Grátis total** | tudo liberado; `contasGratis = null` | máxima adoção; ficha "sem compras"; reforça a marca Falcao | nenhuma receita |
| **B. Grátis + assinatura** (recomendada para avaliar) | N contas grátis (ex.: 3); ilimitado (até o teto técnico) e extras (bloqueio por conta, temas) na assinatura | receita recorrente; converte quem usa muitas contas — justamente o público | precisa de billing das lojas; a Apple/Google ficam com a comissão |
| **C. Compra única** | N contas grátis; desbloqueio vitalício por compra única | simples; sem assinatura a manter | receita não recorre; billing mesmo assim |
| **D. Fase 2 (motor nativo) como diferencial pago** | motor web grátis; motor nativo (Android, build `completo`) pago fora da Play | monetiza o recurso caro de manter | complexidade; cobrança fora da loja tem atrito |

## Como implementar quando decidir (esqueleto)

- `contasGratis` deixa de ser `null` e vira o número do plano grátis.
- `podeAdicionar` (FUNCIONALIDADES §5.4, vetores `contas.json`) já devolve o
  motivo `plano` nesse caso; a tela de plano é especificada junto com a
  decisão.
- Contas já criadas acima do limite **nunca** são bloqueadas nem apagadas
  (regra fixa da spec).
- Billing: Google Play Billing / StoreKit 2; nenhum SDK de terceiros.

## Consequência para as lojas (por modelo)

| Loja | Campo | Modelo A | Modelos B/C/D |
|------|-------|----------|----------------|
| Google Play | Contém compras no app | Não | Sim |
| Google Play | Contém anúncios | Não | Não |
| Google Play | Segurança dos dados | Nenhum dado coletado | Nenhum dado coletado |
| App Store | In-App Purchases | Nenhum | Sim (StoreKit) |
| App Store | App Privacy | Data Not Collected | Data Not Collected |

## Recomendação

Lançar a **fase 1 no modelo A ou B**. O modelo B alinha com o objetivo de
portfólio que gera renda, e o público de "muitas contas" é o que converte;
mas exige billing. Se a prioridade for adoção e reputação primeiro, começar
no A e reavaliar. **Decisão é do usuário (U-1).**
