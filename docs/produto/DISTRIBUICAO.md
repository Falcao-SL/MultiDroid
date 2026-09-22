# Distribuição — Duo

Público-alvo: usuário brasileiro de várias contas. No Brasil o Android é
~85% do mercado, então a Play é o canal principal. O app é gratuito nesta
fase (monetização a decidir, [MONETIZACAO.md](MONETIZACAO.md)).

## 1. Google Play — build `play` (prioridade)

- **Conta**: US$ 25 única. Ficha em pt-BR; distribuição inicial só Brasil.
- **Build `play`**: só o motor **web**, sem `QUERY_ALL_PACKAGES`, sem o
  código do motor nativo (ARQUITETURA → flavors). É o build de menor risco.
- **Teste fechado obrigatório**: contas novas precisam de 14 dias com
  testadores antes da produção. Recrutar em comunidades de social media/QA.
- **Data safety**: nenhum dado coletado nem compartilhado. Anúncios: Não.
  Compras no app: conforme a decisão de monetização.
- **Classificação**: Livre. Categoria: Ferramentas.
- **AAB** assinado com Play App Signing; `minSdk` 26; `targetSdk` o exigido
  pela Play no envio.

## 2. App Store — iOS

- **Conta**: US$ 99/ano; Team `6EZ89E2F5S`. Criar o App ID
  `com.falcaosl.duo` sem capabilities extras.
- Só o motor **web** (o nativo é impossível no iOS).
- **Risco de revisão**: 4.2 (funcionalidade mínima) e 4.3 (spam) — mitigado
  por ser um gerenciador de contas com isolamento, troca rápida, bloqueio e
  atalhos, não um web clipping. Notas ao revisor explicam.
- Categoria: **Utilities**.

## 3. Build `completo` (motor nativo, fase 2) — fora da Play (U-7)

O build `completo` (web + nativo) **não** vai para a Google Play (risco de
política e de licença, CONFORMIDADE_LOJAS / VIRTUALIZACAO). Opções de
distribuição, a decidir:

| Canal | Prós | Contras |
|-------|------|---------|
| **APK no site próprio** (`duo.falcaosl.com`) | controle total; sem revisão | usuário precisa permitir "fontes desconhecidas"; sem atualização automática (ou usar um updater próprio) |
| **Outras lojas** (Aptoide, APKPure, Samsung, F-Droid*) | alcance; atualização | políticas próprias; *F-Droid exige build reproduzível e **licença livre** — se o motor for GPL, combina; se for comercial, não entra |
| **Não distribuir o `completo`** | zero risco | perde o diferencial do motor nativo |

Recomendação: se a fase 2 acontecer, **APK no site** como canal primário,
mais uma loja alternativa; F-Droid só se o motor for aberto (GPL). A decisão
de fazer a fase 2 depende de U-6 (licença) e do apetite de risco.

## Legal (Brasil)

- **LGPD**: o Duo não coleta dado pessoal; ainda assim as lojas exigem
  política de privacidade publicada ([POLITICA_DE_PRIVACIDADE.md](../publicacao/POLITICA_DE_PRIVACIDADE.md)).
- **Termos dos serviços**: o Duo não automatiza nem raspa; isola sessões que
  o usuário abre. Ainda assim, alguns serviços restringem múltiplas contas ou
  clientes não oficiais — documentado em CONFORMIDADE_LOJAS. O disclaimer na
  tela Sobre deixa claro que o Duo não é afiliado a nenhum serviço.

## Idioma de lançamento (U-5)

pt-BR garantido; en preparado (strings em recurso). Lançar en junto amplia o
alcance mas dobra o trabalho de ficha e suporte — decisão do usuário.

## Cronograma resumido

| Fase | Situação | Entrega |
|------|----------|---------|
| Especificação | **Feito** | `docs/` completo |
| Fase 1 código | Próximo | motor web nas duas plataformas |
| Play (build `play`) | Depois | beta fechado → produção BR |
| App Store | Depois | revisão (Utilities) |
| Fase 2 (nativo) | Condicional | build `completo` fora da Play, se U-6/U-7 |
