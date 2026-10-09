# Monetização — Duo

**Modelo: a decidir pelo dono.** Este documento lista opções; nenhuma está
implementada. O Duo para Mac hoje não tem compra, assinatura, licença nem
anúncio.

> Mudar isso é reescrever este arquivo primeiro, com data, e só depois mexer
> no código.

## O que vale em qualquer modelo

- Sem anúncios e sem SDK de anúncio ou analytics (o app não coleta nada).
- Sem servidor de operação: o custo recorrente é ~zero.
- Fora da Mac App Store (decisão M-6): não há compra pela loja da Apple;
  qualquer cobrança precisa de um meio próprio.
- **Antes de vender**: revisar com advogado a licença das imagens do Google
  (MAC.md §5). Cobrar pelo Duo não pode significar cobrar pelas imagens, que
  o usuário sempre baixa do Google.

## Opções

| Modelo | Como | Prós | Contras |
|--------|------|------|---------|
| **A. Grátis** | tudo liberado | máxima adoção; nenhum meio de pagamento a manter | nenhuma receita |
| **B. Grátis + versão paga** | limite de aparelhos ou recursos no grátis (ex.: sincronizar entrada, fase 2) liberados por licença | converte quem usa muitos aparelhos, o público principal | precisa de licença e meio de pagamento fora da loja |
| **C. Compra única** | uma licença vitalícia por versão maior | simples; comum em apps de Mac fora da loja | receita não recorre |
| **D. Assinatura** | recursos pagos enquanto assina | receita recorrente | atrito; precisa de verificação de licença |

Valores, limites e meio de pagamento: **a decidir**.

## Como implementar quando decidir (esqueleto)

- Ponto único no código que decide o que está liberado (hoje não existe).
- Aparelhos já criados nunca são apagados por mudança de plano.
- Verificação de licença sem enviar dados de uso.

## Recomendação

Nenhuma ainda. A decisão depende da revisão jurídica da licença das imagens
e do público que a fase 1 atrair. **Decisão é do dono.**
