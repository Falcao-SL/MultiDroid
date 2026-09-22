# Status — Duo

> Atualizado em **2026-09-21**. Fonte única de "em que pé está".

| Campo | Valor |
|---|---|
| **Estágio** | **Especificação pronta; apps em construção.** Contrato (`docs/`) escrito; nenhum código de app ainda |
| **Plataformas** | Android (Kotlin + Compose) e iOS (Swift + SwiftUI) |
| **Identificador** | `com.falcaosl.duo` (Android e iOS) · Apple Team `6EZ89E2F5S` |
| **SO mínimo** | Android `minSdk` 26 · iOS 17.0 |
| **Deep link** | `duo://` |
| **Monetização** | **A decidir** ([MONETIZACAO.md](MONETIZACAO.md)) |
| **Publicação** | não iniciada nas lojas |
| **Próximo marco** | Fase 1: núcleos + motor web nas duas plataformas, passando os vetores |

## Fases

| Fase | Escopo | Plataformas |
|------|--------|-------------|
| **Fase 1 (v1)** | Motor **web**: contêiner isolado por conta, catálogo + site personalizado, Início, adicionar/editar/excluir, troca rápida, bloqueio, atalhos, downloads/upload, tema, acessibilidade | Android e iOS |
| **Fase 2** | Motor **nativo** (clonar app instalado por virtualização), build `completo` fora da Play | só Android |

**v1 é só a fase 1.** A fase 2 depende de duas decisões do usuário (motor
base e licença, U-6; distribuição, U-7) e do resultado da análise de loja
(CONFORMIDADE_LOJAS).

## O que já existe

- Contrato completo em `docs/spec/` (FUNCIONALIDADES, CONTEINERES,
  VIRTUALIZACAO), catálogo em `docs/spec/dados/`, 129 vetores comuns + 12 do
  Android em `docs/spec/vetores/`, identidade em `docs/design/`, arquitetura,
  produto e publicação.

## O que falta

- **Todo o código dos dois apps** (núcleo, motor web, telas, testes).
- Confirmar em **aparelho real** vários pontos deixados em aberto na
  pesquisa:
  - iOS: se links universais dentro do `WKWebView` respeitam a regra de
    recarga programática (CONTEINERES §6.4);
  - iOS: comportamento de backup dos dados de `WKWebsiteDataStore`
    (CONTEINERES §9.4);
  - Android: aparelhos onde `MULTI_PROFILE` não existe (tela `sem-suporte`);
  - fase 2: lista `incompativeis` de apps que não clonam.
- Decisões do usuário: U-1 a U-7 (ver FUNCIONALIDADES → A decidir).
- Arte do ícone e capturas.
- Configurar pipelines e contas de loja (PIPELINES_PUBLICACAO).

## Riscos e bloqueios

- **Compatibilidade do motor web por serviço**: alguns serviços empurram o
  app oficial (TikTok, Reddit) ou só têm web de computador (Snapchat,
  Discord, WhatsApp). Mitigado pelo selo de compatibilidade no catálogo e
  pela honestidade na ficha. Serviços experimentais podem sair do catálogo
  de lançamento (U-3).
- **Google bloqueia login em WebView**: afeta YouTube e qualquer "entrar com
  Google". Mitigado por aviso claro; sem contorno (seria burlar segurança).
- **Motor nativo × Google Play**: alto risco de rejeição; por isso fica em
  build separado fora da Play (CONFORMIDADE_LOJAS).
- **Licença do motor nativo**: as bases abertas descendem do VirtualApp
  (GPL-3.0 / litigioso). Usar a base GPL obrigaria abrir o código do app
  Android (VIRTUALIZACAO §3). Decisão em aberto.
- **App Store 4.2 (funcionalidade mínima)**: um app que "só mostra sites"
  pode ser barrado. Mitigado por ser um gerenciador de várias contas com
  isolamento, atalhos, bloqueio, troca rápida — não um web clipping
  (CONFORMIDADE_LOJAS).

## Definição de "pronto para publicar"

Cada versão passa por `docs/publicacao/CONFORMIDADE_LOJAS.md` com as duas
suítes verdes (vetores + isolamento) antes de subir.
