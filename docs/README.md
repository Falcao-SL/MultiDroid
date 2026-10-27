# Documentação — MultiDroid

Tudo o que não é código mora aqui. O MultiDroid é um app só para Mac, em
[`mac/`](../mac): vários aparelhos Android lado a lado, sobre o Android
Emulator oficial sem janela.

## Especificação — [`spec/`](spec)

| Documento | Conteúdo |
|-----------|----------|
| [spec/MAC.md](spec/MAC.md) | Fonte única de comportamento: arquitetura, frames, estados, duplicar, perfis de aparelho, painel, instalador, mesa e grade, fases, resultados da fase 0, decisões (M-1 a M-6), riscos |
| [spec/MULTIPLATAFORMA.md](spec/MULTIPLATAFORMA.md) | Mac, Windows e Linux com um código em Rust e interface Slint: decisões X-1 a X-6, arquitetura, o que muda por sistema, etapas R-0 a R-8, como testar tudo a partir do Mac, critério para substituir a 0.2, release dos três pelo GitHub Actions |
| [spec/LINUX.md](spec/LINUX.md) | Versão Linux (proposta, sem código): Redroid em contêiner direto no kernel, com a GPU do PC; motores Leve e Completo, apps só-ARM, interface, fase 0, riscos; por que não Windows |

## Identidade visual — [`design/`](design)

| Documento | Conteúdo |
|-----------|----------|
| [design/IDENTIDADE_VISUAL.md](design/IDENTIDADE_VISUAL.md) | Falcao Neo-Brutalism: princípios, tokens, ícone do app, tom de voz |
| [design/tokens.json](design/tokens.json) | Tokens (cores, borda, raio, sombra, tipografia, movimento) |
| [design/icone/](design/icone) | Fonte do ícone (`multidroid-icon.svg`, `AppIcon.icon`) e exportador |

## Arquitetura — [`arquitetura/`](arquitetura)

| Documento | Conteúdo |
|-----------|----------|
| [arquitetura/ARQUITETURA.md](arquitetura/ARQUITETURA.md) | Visão curta do app e do repositório; detalhes em MAC.md §2 |
| [arquitetura/ADR-001-nucleo-nativo-e-android-studio.md](arquitetura/ADR-001-nucleo-nativo-e-android-studio.md) | Núcleo C no hospedeiro, ponte adb e como o Run do Android Studio acha o aparelho |

## Produto — [`produto/`](produto)

| Documento | Conteúdo |
|-----------|----------|
| [produto/PRODUTO.md](produto/PRODUTO.md) | O que é, para quem, o que não faz |
| [produto/STATUS.md](produto/STATUS.md) | Estágio, o que existe, o que falta, riscos |
| [produto/DISTRIBUICAO.md](produto/DISTRIBUICAO.md) | Developer ID + notarização, fora da Mac App Store |
| [produto/MONETIZACAO.md](produto/MONETIZACAO.md) | Sem monetização: código aberto, GPL-3.0 |
| [produto/MARKETING_ASO.md](produto/MARKETING_ASO.md) | Posicionamento, nome, público, canais |

## Publicação — [`publicacao/`](publicacao)

| Documento | Conteúdo |
|-----------|----------|
| [publicacao/POLITICA_DE_PRIVACIDADE.md](publicacao/POLITICA_DE_PRIVACIDADE.md) | Política de privacidade do app de Mac |

## Arquivo — [`arquivo/`](arquivo)

| Documento | Conteúdo |
|-----------|----------|
| [arquivo/celular/](arquivo/celular/README.md) | Plano dos apps de Android e iOS, descontinuado em 2026-10-09; só para consulta |
