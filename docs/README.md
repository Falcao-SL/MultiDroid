# Documentação — Duo

Tudo o que não é código de um dos dois apps (`android/`, `ios/`) mora aqui: o
contrato comum que os dois implementam, a identidade visual, a arquitetura, o
produto e a publicação.

## Contrato funcional — [`spec/`](spec)

Fonte única de comportamento. Os testes das duas plataformas leem estes
arquivos direto do disco; mudou aqui, as suítes falham até cada app acompanhar.

| Documento | Conteúdo |
|-----------|----------|
| [spec/FUNCIONALIDADES.md](spec/FUNCIONALIDADES.md) | Telas, rotas, fluxos, motores de conta, matriz de paridade, decisões e o que ficou a decidir pelo usuário |
| [spec/CONTEINERES.md](spec/CONTEINERES.md) | Motor web: isolamento por conta nas duas plataformas — APIs, ciclo de vida, User-Agent, navegação, downloads, permissões, limpeza de dados, limites de memória |
| [spec/VIRTUALIZACAO.md](spec/VIRTUALIZACAO.md) | Motor nativo (só Android, fase 2): virtualização, motores de código aberto e licença, riscos |
| [spec/MAC.md](spec/MAC.md) | Duo para Mac: vários aparelhos Android (celular, tablet, dobrável, Wear) lado a lado, sobre o emulador oficial sem janela |
| [spec/dados/](spec/dados) | Catálogo de serviços (`servicos.json`), aparência/limites (`aparencia.json`), apps nativos (`apps-nativos.json`) |
| [spec/vetores/](spec/vetores/README.md) | Vetores de teste lidos pelas duas plataformas |

## Identidade visual — [`design/`](design)

| Documento | Conteúdo |
|-----------|----------|
| [design/IDENTIDADE_VISUAL.md](design/IDENTIDADE_VISUAL.md) | Falcao Neo-Brutalism: princípios, componentes do Duo, ícones, tom de voz |
| [design/tokens.json](design/tokens.json) | Tokens (cores, borda, raio, sombra, tipografia, movimento), conferidos pelos testes das duas plataformas |

## Arquitetura — [`arquitetura/`](arquitetura)

| Documento | Conteúdo |
|-----------|----------|
| [arquitetura/ARQUITETURA.md](arquitetura/ARQUITETURA.md) | Dois apps nativos, o contrato comum, a abstração `MotorDeConta`, os flavors do Android e a regra de mudança |

## Produto — [`produto/`](produto)

| Documento | Conteúdo |
|-----------|----------|
| [produto/PRODUTO.md](produto/PRODUTO.md) | O que é, para quem, o que não faz |
| [produto/STATUS.md](produto/STATUS.md) | Estágio, fases, o que falta, riscos |
| [produto/MONETIZACAO.md](produto/MONETIZACAO.md) | Modelos propostos — a decidir |
| [produto/MARKETING_ASO.md](produto/MARKETING_ASO.md) | Posicionamento, nome de loja, ASO |
| [produto/DISTRIBUICAO.md](produto/DISTRIBUICAO.md) | Plano de distribuição (Play, App Store, build completo fora da Play) |

## Publicação — [`publicacao/`](publicacao)

| Documento | Conteúdo |
|-----------|----------|
| [publicacao/CONFORMIDADE_LOJAS.md](publicacao/CONFORMIDADE_LOJAS.md) | Auditoria de guidelines Apple/Google, com mitigações |
| [publicacao/POLITICA_DE_PRIVACIDADE.md](publicacao/POLITICA_DE_PRIVACIDADE.md) | Política de privacidade |
| [publicacao/PIPELINES_PUBLICACAO.md](publicacao/PIPELINES_PUBLICACAO.md) | Esqueleto dos pipelines de publicação |
