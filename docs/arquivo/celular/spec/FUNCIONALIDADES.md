# Duo — especificação funcional

Documento central de produto e comportamento, comum ao **Android** (Kotlin +
Compose) e ao **iOS** (Swift + SwiftUI). Cada plataforma tem seu próprio
núcleo nativo; a paridade é garantida por este documento, por
[`CONTEINERES.md`](CONTEINERES.md) (isolamento por conta), pelo catálogo em
[`dados/`](dados/) e pelos vetores em [`vetores/`](vetores/README.md).
A identidade visual está em [`../design/IDENTIDADE_VISUAL.md`](../../../design/IDENTIDADE_VISUAL.md).

Não há app de referência: os dois apps nascem desta especificação. Onde uma
escolha poderia ir para um lado ou outro, ela está em [Decisões](#decisões);
o que depende do usuário está em [A decidir pelo usuário](#a-decidir-pelo-usuário).

Convenção de texto: tudo entre `crases` nesta página é **texto exato** que
aparece na interface (acentos, maiúsculas, reticências `…` U+2026, travessão
`—` U+2014). As duas plataformas usam as mesmas strings. `<…>` é substituído.

---

## 1. Visão geral

O Duo deixa usar **várias contas do mesmo serviço** — TikTok, Instagram,
X, YouTube, Telegram Web e outros — num só aparelho, sem sair e entrar de
novo. Cada conta é um **contêiner web**: um perfil de navegador isolado que
abre a versão web do serviço, com cookies, armazenamento local, IndexedDB e
cache próprios. Duas contas do Instagram no Duo são, para o Instagram, dois
navegadores diferentes.

Cada conta roda num **motor** (§1.1). O motor **web** (Android e iOS, fase
1) é um navegador com gavetas separadas: não reempacota APK/IPA, não
virtualiza nada e não automatiza ações nas contas — mecânica em
[CONTEINERES.md](CONTEINERES.md). O motor **nativo** (só Android, fase 2,
condicionado às decisões de licença e de loja) roda uma cópia do app oficial
já instalado — está em [VIRTUALIZACAO.md](VIRTUALIZACAO.md).

### 1.1 Motores de conta

| Motor | Plataformas | Fase | O que é | Onde |
|-------|-------------|------|---------|------|
| `web` | Android e iOS | **1 (v1)** | contêiner web isolado: a versão web do serviço num perfil de navegador próprio | [CONTEINERES.md](CONTEINERES.md) |
| `nativo` | só Android, só no build `completo` | 2 | cópia do app oficial instalado, com dados separados por cópia e notificações | [VIRTUALIZACAO.md](VIRTUALIZACAO.md) |

- Toda conta tem um motor, escolhido ao criar e **imutável** (os dados de um
  não servem ao outro). No código, os dois implementam a mesma abstração
  `MotorDeConta` (abrir, descartar, limpar dados, excluir, faxina) —
  [ARQUITETURA.md](../../../arquitetura/ARQUITETURA.md).
- O iOS só tem o motor `web`: o sistema não permite executar outro app
  dentro de um app (App Store 2.5.2), então "motor nativo" é **não suportado
  pela plataforma**, não uma pendência.
- Tudo o que esta especificação diz sobre contêiner, navegação, downloads e
  User-Agent vale para o motor `web`. O motor `nativo` tem telas próprias
  descritas no §5.8 e regras em VIRTUALIZACAO.md.
- **v1 = fase 1**: motor `web` nas duas plataformas. Nada do motor `nativo`
  entra na v1.

Números do catálogo: **14 serviços** em [`dados/servicos.json`](dados/servicos.json)
+ **site personalizado** (qualquer endereço `https`).

### Princípios

| Princípio | Regra verificável |
|-----------|-------------------|
| Isolamento real | Cada conta tem um armazenamento web próprio e persistente (iOS `WKWebsiteDataStore(forIdentifier:)`, Android `Profile` do `androidx.webkit`). Nenhum cookie ou dado de site de uma conta é visível em outra. Conferido por teste instrumentado (CONTEINERES §10). |
| Nada sai do aparelho | Sem servidor do Duo, sem conta Duo, sem analytics, sem crash reporting remoto, sem SDK de terceiros. A única rede é a que os sites abertos pelo usuário fazem, direto do aparelho para o serviço. |
| Sem automação | O Duo não preenche formulários, não clica, não publica, não coleta nada das páginas e não injeta script de interação. O único JavaScript injetado é o mínimo técnico listado em CONTEINERES §8. |
| Honestidade | O app diz o que não funciona: login do Google bloqueado em WebView, recursos que exigem o app oficial, notificações que não chegam (§11). |
| Sem marca de terceiros | Nenhum logo, ícone ou cor de marca de serviço. O serviço é identificado pelo nome em texto e por uma cor do nosso tema (§4, IDENTIDADE_VISUAL). |
| Local-first | Tudo (contas, sessões, preferências) mora no aparelho. Sem backup em nuvem das sessões (Android `allowBackup=false`; iOS dados em Application Support excluídos do backup). |

### O que o app não faz

- Não roda o app oficial de nenhum serviço e não usa recursos exclusivos dele
  (câmera do Snapchat, edição de Reels, lives, publicação de vídeo no TikTok).
- Não recebe notificação push dos serviços (§11).
- Não baixa vídeo nem mídia que o próprio site não oferece para baixar (§10).
- Não contorna bloqueio de login (Google, Apple) nem esconde que é um WebView
  de quem pergunta (CONTEINERES §4).
- Não esconde o IP nem a identidade do aparelho: os serviços podem perceber
  que várias contas vêm do mesmo aparelho (CONTEINERES §3).
- Não sincroniza contas entre aparelhos e não faz backup de sessão.
- Não tem app web nem versão para computador.

---

## 2. Telas, rotas e navegação

Pilha simples: o Início é a raiz; toda outra tela abre por cima dele e tem
`Voltar` (e o gesto/tecla de voltar do sistema). Folhas (`sheet`) abrem por
cima da tela atual. Não há abas nem menu lateral.

| ID da tela | Rota | Tipo | Título | Abre a partir de |
|------------|------|------|--------|------------------|
| `inicio` | `/` | tela raiz | `Duo` | abertura do app |
| `adicionar` | `/adicionar` | folha | `Adicionar conta` | botão `Adicionar conta` do Início; deep link `duo://adicionar` |
| `adicionar-form` | `/adicionar/{servicoId}` | passo da folha | nome do serviço, ou `Site personalizado` | escolher um serviço em `adicionar` |
| `conteiner` | `/conta/{id}` | tela cheia | apelido da conta | card de conta; atalho; deep link `duo://conta/{id}` |
| `trocar` | `/conta/{id}/trocar` | folha | `Trocar de conta` | botão de trocar na barra do contêiner |
| `editar` | `/conta/{id}/editar` | folha | `Editar conta` | menu do card; menu do contêiner |
| `ajustes` | `/ajustes` | tela | `Ajustes` | ícone de ajustes na app bar do Início |
| `sobre` | `/sobre` | tela | `Sobre` | linha `Sobre o Duo` em `ajustes` |
| `bloqueio` | — (sobreposição) | tela cheia | — | abertura/retorno com bloqueio ligado (§8.1) |
| `sem-suporte` | — (sobreposição) | tela cheia | `Atualize o WebView` | Android sem `MULTI_PROFILE` (§16) |

- `{id}` é o UUID da conta (v4, minúsculas). `{servicoId}` é o id do
  catálogo ou `personalizado`. IDs nunca mudam.
- Android: Navigation Compose com rotas sem a barra inicial (`inicio`,
  `conta/{id}`…); folhas em `ModalBottomSheet`. iOS: `NavigationStack` com
  `enum Rota { case conteiner(UUID), ajustes, sobre }` e folhas por
  `.sheet(item:)`.
- Rota com `{id}` inexistente → volta para o Início com o aviso
  `Conta não encontrada` (toast, §4.7).
- `bloqueio` e `sem-suporte` não entram na pilha: cobrem tudo até serem
  resolvidas.

---

## 3. Modelo de dados

### Conta

| Campo | Tipo | Regra |
|-------|------|-------|
| `id` | UUID v4 em minúsculas | gerado na criação; nunca muda; é a chave do contêiner (CONTEINERES §2) |
| `motor` | `web` \| `nativo` | escolhido ao criar; imutável. iOS: sempre `web` |
| `servico` | string | motor `web`: id de `dados/servicos.json` ou `personalizado`. Motor `nativo`: `app:<pacote>` (ex.: `app:com.instagram.android`) |
| `url` | string ou null | só `personalizado`: URL normalizada (§5.3). Serviços do catálogo usam `urlInicial` do catálogo |
| `dominioBase` | string ou null | só `personalizado` (§5.3) |
| `apelido` | string | 1–30 caracteres (§5.4) |
| `cor` | string | chave de `dados/aparencia.json → cores` |
| `emoji` | string ou null | um item de `dados/aparencia.json → emojis`; `null` = inicial do apelido |
| `fixada` | booleano | padrão `false` |
| `ordem` | inteiro ≥ 0 | posição manual; ao criar = maior ordem existente + 1 (a conta nova vai para o fim) |
| `bloqueada` | booleano | pede biometria para abrir (§8.2); padrão `false` |
| `ultimaUrl` | string ou null | última URL de quadro principal carregada com sucesso (§7.2) |
| `geracao` | inteiro ≥ 1 | só Android: sufixo do perfil; sobe a cada `Limpar dados` (CONTEINERES §9) |
| `criadaEm` | data-hora ISO 8601 UTC | informativo |
| `usadaEm` | data-hora ou null | última abertura do contêiner (ordena a folha `trocar` e os atalhos dinâmicos) |

### Preferências

| Chave | Valores | Padrão |
|-------|---------|--------|
| `tema` | `sistema` \| `claro` \| `escuro` | `sistema` |
| `bloqueioApp` | `desligado` \| `imediato` \| `1min` \| `5min` | `desligado` |
| `ocultarNoSeletor` | booleano | `true` |
| `agruparPorServico` | booleano | `false` |
| `abrirLinksExternos` | `navegador` \| `perguntar` | `navegador` |

---

## 4. Início (`inicio`)

### 4.1 Estrutura (de cima para baixo)

1. **App bar** azul: título `Duo`; à direita, botão de ajustes (rótulo
   `Ajustes`).
2. **Campo de busca** (fixo sob a app bar, aparece só com 4 contas ou mais):
   placeholder `Buscar conta…`, botão de limpar (rótulo `Limpar busca`) só
   com texto.
3. **Chips de serviço** (rolagem horizontal, aparecem só quando há contas de
   2 serviços ou mais): `Todas` e um chip por serviço que tem conta, na ordem
   do catálogo, `Site personalizado` por último. Um chip selecionado por vez;
   não é persistido.
4. **Grade** rolável de cards de conta, com as seções abaixo, cada uma
   precedida de selo (BrutalBadge): `FIXADAS` e `CONTAS`. Seção vazia não
   aparece, nem o selo. Com `agruparPorServico` ligado, `CONTAS` é
   substituída por uma seção por serviço (selo = nome do serviço em
   maiúsculas, `SITE PERSONALIZADO` por último), cada uma na ordem manual.
5. **Botão flutuante** `Adicionar conta` (ícone `+`), no canto inferior
   direito.

### 4.2 Card de conta

- Avatar quadrado (emoji, ou a inicial do apelido) sobre a cor da conta;
  apelido (até 2 linhas, reticências); nome do serviço (1 linha; no
  personalizado, o `dominioBase`); ícone de alfinete quando fixada; ícone de
  cadeado quando `bloqueada`.
- Toque abre `conteiner`. Toque longo (Android) / menu de contexto (iOS)
  abre o menu da conta: `Abrir`, `Fixar` / `Desafixar`, `Editar`,
  `Adicionar à tela inicial` (§9), `Limpar dados`, `Excluir`.
- A inicial do avatar segue `inicial(apelido)`: primeira letra ou dígito
  (Unicode L/N), em maiúscula; nenhuma → `?`. Vetores em `vetores/contas.json`.

### 4.3 Ordem

- `FIXADAS` primeiro, depois as outras; dentro de cada grupo por `ordem`
  crescente; desempate por `id`. Vetores em `vetores/inicio.json`.
- **Organizar**: item `Organizar` no menu da app bar (ícone de ordenar,
  rótulo `Organizar contas`) liga o modo de organização. Cada card mostra
  alça de arrastar e os botões `Mover para cima` / `Mover para baixo`
  (acessíveis, sem depender de arrastar). Um card só se move dentro do
  próprio grupo (fixadas / não fixadas). `Concluir` sai do modo. Ao sair,
  `ordem` é renumerada `0…n−1` na ordem exibida e persistida.
- Fixar leva a conta para o fim de `FIXADAS`; desafixar, para o fim de
  `CONTAS` (ordem = maior + 1).

### 4.4 Busca

- `consulta = normalizarBusca(trim(texto))`; a conta aparece se `consulta`
  vazia ou se está contida em `normalizarBusca(apelido)` ou em
  `normalizarBusca(nome do serviço)` (no personalizado, o `dominioBase`).
- `normalizarBusca`: minúsculas; `á à â ã → a`, `é ê → e`, `í → i`,
  `ó ô õ → o`, `ú → u`, `ç → c` — a mesma do BR Generator.
- Combina com o chip de serviço (os dois filtros valem juntos).
- Nenhum resultado: `Nada encontrado para "<texto aparado>"`.
- A busca não é persistida.

### 4.5 Estado vazio

Sem nenhuma conta: no lugar da grade, BrutalBox com o título
`Várias contas, um aparelho.`, o texto
`Cada conta abre num contêiner separado, com login e dados próprios. Nada sai do seu aparelho.`
e o BrutalButton `Adicionar primeira conta`, que abre `adicionar`. Busca e
chips não aparecem.

### 4.6 Grade

2 colunas no celular. Em telas largas (tablet),
`colunas = max(2, floor((larguraÚtil + 12) / (160 + 12)))`, célula mínima
160, espaço 12 (tokens `tamanho.cardConta`).

### 4.7 Avisos rápidos (toast)

Faixa curta a 24 da base, 1800 ms, não bloqueia toque (componente Toast da
identidade). Textos usados no app:

| Quando | Texto |
|--------|-------|
| Conta criada | `Conta adicionada` |
| Conta excluída | `Conta excluída` |
| Dados limpos | `Dados da conta apagados` |
| URL copiada | `Link copiado` |
| Deep link para conta inexistente | `Conta não encontrada` |
| Atalho pedido ao sistema (Android) | `Atalho enviado para a tela inicial` |

---

## 5. Adicionar, editar e excluir conta

### 5.1 Escolher serviço (`adicionar`)

- Folha com título `Adicionar conta`.
- **Android, build `completo`, fase 2**: a folha começa por duas abas —
  `Versão web` (a lista abaixo) e `App nativo (clonar)` (§5.8). No build
  `play` e no iOS não há abas: só a lista abaixo.
- Lista na ordem de `dados/servicos.json`: cada linha = chip de serviço (cor
  do catálogo, inicial do nome) + nome + selo de compatibilidade quando não
  é `boa`: `Limitado` ou `Experimental`. Por último, a linha
  `Site personalizado` com o texto de apoio `Qualquer endereço https`.
- Sem logos (princípio "sem marca de terceiros").
- Antes de abrir a folha, `podeAdicionar(quantidade)` (§5.4). Se não puder,
  a folha não abre e aparece o diálogo `Limite atingido` com o `motivo`.

### 5.2 Formulário (`adicionar-form`)

Campos, de cima para baixo:

1. Só personalizado: **Endereço** (`Endereço do site`, placeholder
   `exemplo.com.br`, teclado de URL, sem autocorreção).
2. **Apelido** (`Apelido`, placeholder = `apelidoPadrao`, §5.4).
3. **Cor**: seletor com as 7 cores de `aparencia.json → cores`; padrão
   `corPadrao(quantidade)`.
4. **Ícone**: `Inicial` (padrão) ou um dos 32 emojis de
   `aparencia.json → emojis` em grade.
5. Pré-visualização do card com os valores atuais.
6. Aviso de compatibilidade do serviço quando `limitada`/`experimental`:
   o texto de `observacoes` do catálogo, em caixa sem sombra.
7. BrutalButton `Adicionar e entrar`.

Ao confirmar: valida (§5.3, §5.4), grava a conta, mostra `Conta adicionada`
e abre o `conteiner` da conta nova na `urlInicial` (ou na URL do
personalizado), onde o usuário faz login no próprio site.

### 5.3 Site personalizado

`normalizarUrlPersonalizada(entrada)` — vetores em
[`vetores/url-personalizada.json`](vetores/url-personalizada.json):

1. Apara espaço, `\t`, `\n`, `\r` e U+00A0 nas pontas. Vazio →
   `informe o endereço do site`.
2. Mais de 2048 caracteres → `endereço muito longo`.
3. Espaço em branco no meio → `endereço inválido`.
4. Com `esquema://`: só `http` e `https` (sem diferenciar maiúsculas); outro →
   `use um endereço que comece com https://`. Sem `://`: se o texto antes
   do primeiro `:` for `javascript`, `mailto`, `tel`, `sms`, `data`, `file`,
   `intent`, `about`, `blob`, `market` ou `itms-apps` → a mesma mensagem;
   senão assume `https://`.
5. `usuário@` ou `usuário:senha@` na autoridade →
   `endereço com usuário ou senha não é permitido`.
6. Porta: só a padrão (`443`, ou `80` quando veio `http://`), que é removida;
   outra → `porta não suportada: use o endereço sem porta`; porta não
   numérica → `endereço inválido`.
7. Host em minúsculas, sem ponto final; nome internacional convertido para
   punycode (`café.com.br` → `xn--caf-dma.com.br`).
8. IPv4 ou IPv6 → `use um endereço com domínio, não um IP`.
9. Pelo menos 2 rótulos; cada rótulo 1–63 de `[a-z0-9-]`, sem hífen nas
   pontas; último rótulo só letras (2–63) ou `xn--…`. Senão →
   `endereço inválido` (`localhost`, `exemplo`, `exe_mplo.com`).
10. Remove o fragmento (`#…`); caminho vazio vira `/`; caracteres fora do
    ASCII no caminho/consulta viram `%XX` (UTF-8, maiúsculas). O esquema final
    é sempre `https`.
11. `dominioBase` = host sem um `www.` inicial (se sobrarem 2 rótulos ou
    mais). Define o que abre dentro do contêiner (CONTEINERES §6).
12. `servicoSugerido`: se o host pertence a um serviço do catálogo, a tela
    mostra `Este endereço é do <nome>. Usar o <nome> do catálogo?` com
    `Usar <nome>` e `Continuar como site`. A escolha é do usuário.

O erro aparece sob o campo, em minúsculas e sem ponto final (padrão dos
motivos do BR Generator), e o botão fica desabilitado até o campo ser válido.

### 5.4 Regras de conta

Vetores em [`vetores/contas.json`](vetores/contas.json).

| Regra | Definição |
|-------|-----------|
| Normalizar apelido | apara as pontas (mesmos espaços do §5.3) e troca cada sequência interna desses espaços por um espaço |
| Validar apelido | caractere de controle (Unicode Cc) → `apelido não pode ter caracteres de controle`; vazio → válido, usa o padrão; mais de 30 pontos de código → `apelido deve ter no máximo 30 caracteres`; igual a outro apelido **do mesmo serviço** após `normalizarBusca` → `já existe uma conta com esse apelido neste serviço` |
| Mesmo serviço | mesmo `servico`; no personalizado, mesmo `servico` **e** mesmo `dominioBase` |
| Apelido padrão | `<nome do serviço> <n>` com o menor `n ≥ 1` livre no serviço (comparação por `normalizarBusca`); no personalizado, `<dominioBase> <n>` |
| Contagem | caracteres = pontos de código Unicode (Kotlin `codePointCount`, Swift `unicodeScalars.count`) |
| Cor padrão | `cores[quantidade % 7]` |
| Limite | `podeAdicionar(quantidade, contasGratis)`: `quantidade ≥ limites.maxContas` (50) → `limite de 50 contas atingido`; `contasGratis` não nulo e `quantidade ≥ contasGratis` → motivo `plano` (§13). Nesta fase `contasGratis = null` |

Diálogo de limite: título `Limite atingido`, texto `Você chegou a 50 contas. Exclua uma conta para adicionar outra.`, botão `Entendi`.

### 5.5 Editar (`editar`)

Mesma folha do formulário, título `Editar conta`, com apelido, cor e ícone.
**Serviço e endereço não mudam** depois de criada (mudariam o sentido dos
dados guardados no contêiner): para outro serviço, crie outra conta. Abaixo
dos campos, a área de risco com `Limpar dados` e `Excluir conta`. Botão
`Salvar`.

### 5.6 Excluir

Diálogo: título `Excluir "<apelido>"?`, texto
`A conta sai do Duo e todos os dados dela neste aparelho são apagados: login, cookies, arquivos em cache. A conta no <serviço> continua existindo.`
Botões `Cancelar` e `Excluir` (vermelho). Com `bloqueada` ligada, pede
biometria antes do diálogo.

Ao confirmar: a conta some da lista na hora, o contêiner vivo é destruído e
o armazenamento é apagado conforme CONTEINERES §9 (no Android, na próxima
abertura do app, antes de qualquer WebView). Atalhos fixados da conta são
desativados (§9). Toast `Conta excluída`.

### 5.7 Limpar dados

Diálogo: título `Limpar dados de "<apelido>"?`, texto
`Você sai da conta e o Duo apaga cookies, armazenamento e cache dela. Apelido, cor e posição continuam.`
Botões `Cancelar` e `Limpar` (vermelho). Efeito em CONTEINERES §9. Depois,
o contêiner reabre na URL inicial. Toast `Dados da conta apagados`.

### 5.8 App nativo (clonar) — Android, fase 2

- A aba `App nativo (clonar)` lista os apps instalados que são clonáveis
  (regra em VIRTUALIZACAO.md §5, vetores em `vetores/clonaveis.json`), em
  ordem alfabética do nome, com o ícone e o nome **do próprio sistema** (o
  Duo não embute ícones de terceiros; ele mostra o que o launcher já mostra).
- Apps não clonáveis instalados aparecem abaixo, desabilitados, com o motivo
  (`Não roda em ambiente clonado`, `App do sistema`, `Arquitetura não suportada`).
- O formulário é o mesmo do §5.2 sem o campo de endereço; o apelido padrão
  usa o nome do app. Ao confirmar, o Duo prepara a cópia (tela de progresso
  `Preparando <app>…`) e abre o app clonado.
- Conta `nativo` no Início: mesmo card, com o selo `APP` no canto. Tocar
  abre o app clonado em tela cheia (não há barra do contêiner web). Menu do
  card: `Abrir`, `Fixar`, `Editar`, `Adicionar à tela inicial`,
  `Limpar dados`, `Excluir`, `Forçar parada`.
- O app oficial atualizado no aparelho é usado na próxima abertura da cópia;
  app oficial desinstalado → a conta mostra `App original desinstalado` e só
  permite `Excluir` (os dados ficam até lá).

---

## 6. Contêiner (`conteiner`)

### 6.1 Estrutura

1. **Barra do contêiner** (topo, fundo = cor da conta, borda inferior 2,5):
   `Voltar` para o Início (seta), avatar pequeno + apelido (toque abre
   `trocar`), e à direita: `Recarregar` e o menu `Mais opções`.
2. **Barra de progresso** fina (cor `tinta`) sob a barra enquanto carrega.
3. **Área web** do contêiner (WebView / WKWebView do perfil da conta).
4. **Barra inferior de navegação**: `Voltar página`, `Avançar página`
   (desabilitados quando não há histórico), `Trocar de conta` (ícone de
   duas pessoas, abre `trocar`), `Compartilhar link`.

Menu `Mais opções`: `Copiar link`, `Abrir no navegador`, `Editar conta`,
`Limpar dados`, `Adicionar à tela inicial`, `Ir para o início do site`
(carrega a URL inicial).

- **Voltar do sistema** (gesto/botão Android, gesto de borda iOS): volta
  uma página no histórico do site; sem histórico, volta ao Início.
- **Puxar para atualizar**: no topo da página, puxar recarrega (Android
  `PullToRefreshBox` em volta do WebView, só quando `scrollY == 0`; iOS
  `UIRefreshControl` no `scrollView` do `WKWebView`).
- Rolagem para baixo esconde a barra inferior; para cima, mostra.
- Título da janela para acessibilidade: `<apelido> — <serviço>`.

### 6.2 Sessão

O login é feito **no site**, pelo usuário, como em qualquer navegador. O
contêiner persiste o armazenamento: fechar o app, reiniciar o aparelho ou
atualizar o Duo não desloga. O que desloga: `Limpar dados`, `Excluir`, o
próprio serviço encerrar a sessão, ou desinstalar o app.

### 6.3 Erros de carregamento

Erro de rede no **quadro principal** (sem conexão, DNS, tempo esgotado, TLS)
substitui a área web por uma tela nativa:

| Situação | Título | Texto | Ação |
|----------|--------|-------|------|
| Sem conexão | `Sem conexão` | `Confira sua internet e tente de novo.` | `Tentar de novo` |
| Servidor não encontrado / tempo esgotado | `Não foi possível abrir <host>` | `O site não respondeu. Tente de novo em instantes.` | `Tentar de novo` |
| Certificado inválido | `Conexão não segura` | `O certificado de <host> não é válido. O Duo não abre sites com certificado inválido.` | `Voltar` |
| Processo web encerrado (memória) | — | recarrega sozinho a última URL, sem tela de erro (CONTEINERES §5) | — |

- Erro de certificado **nunca** tem "continuar mesmo assim".
- Erro em subrecurso (imagem, script) não mostra tela: é problema do site.
- Voltar a ter rede com a tela de erro visível tenta de novo sozinho uma vez.

### 6.4 Avisos dentro do contêiner

Faixa de aviso (BrutalBox sem sombra, sob a barra, com botão de fechar), um
de cada vez:

| Gatilho (CONTEINERES §6) | Texto | Botões |
|--------------------------|-------|--------|
| Link para o app oficial de um serviço (`acao = app` com `servicoApp`) | `Este link tenta abrir o app do <serviço>. O Duo mantém você nesta conta.` | `Abrir fora`, `OK` |
| Link para loja de apps ou app desconhecido (`acao = app` sem `servicoApp`) | `Este link tenta abrir outro app.` | `Abrir fora`, `OK` |
| Página de login do Google/Apple (`dominiosIdentidade`) | `O Google e a Apple não permitem login dentro de outros apps. Se o site oferecer, entre com e-mail e senha.` | `OK` |

`Abrir fora` entrega a URL original ao sistema (o que pode abrir o app
oficial, com a conta dele — é escolha explícita do usuário).

### 6.5 Links que saem do serviço

Navegação classificada como `fora` (CONTEINERES §6):

- `abrirLinksExternos = navegador` (padrão): entrega ao navegador do sistema.
- `abrirLinksExternos = perguntar`: folha `Abrir link` com o host e as
  opções `Abrir no navegador`, `Copiar link` e — quando o host é de um
  serviço do catálogo em que o usuário tem contas — uma linha por conta
  desse serviço (`Abrir em <apelido>`).

---

## 7. Troca rápida entre contas e retomada

### 7.1 Folha `trocar`

- Abre pelo apelido na barra, pelo botão `Trocar de conta` ou pelo gesto de
  **deslizar horizontalmente na barra do contêiner** (esquerda = próxima
  conta do mesmo serviço, direita = anterior, na ordem do Início).
- Mostra primeiro as outras contas **do mesmo serviço** (selo
  `MESMO SERVIÇO`), depois as demais por `usadaEm` mais recente (selo
  `OUTRAS CONTAS`); a conta atual aparece marcada e desabilitada.
- Tocar troca o contêiner sem passar pelo Início (substitui a tela no topo
  da pilha). Conta com `bloqueada` pede biometria antes.

### 7.2 Retomar onde parou

- A cada carregamento completo do quadro principal, o Duo grava
  `ultimaUrl` (só se `acao = conteiner` para essa URL).
- Contêiner **vivo em memória**: a troca volta exatamente ao estado
  anterior (rolagem, vídeo pausado, formulário), porque a WebView não foi
  destruída.
- Contêiner descartado (limite de memória, app encerrado): reabre em
  `ultimaUrl`; se nula ou se não for mais `conteiner`, na URL inicial. O
  histórico de voltar/avançar não é restaurado (paridade: o Android não tem
  API pública para isso; o iOS não usa `interactionState` para não divergir).

### 7.3 Contêineres vivos

No máximo `limites.maxContextosVivos` = **3** WebViews vivas ao mesmo tempo
(a da tela e as duas usadas mais recentemente), descartando a menos recente
(LRU). Depois de um aviso de memória do sistema, o limite cai para 1 até o
app ir para segundo plano e voltar. Detalhes e motivos em CONTEINERES §5.

---

## 8. Privacidade e segurança

### 8.1 Bloqueio do app

- Ajuste `Bloquear o Duo` com as opções `Desligado` (padrão), `Ao sair do app`
  (`imediato`), `Depois de 1 minuto`, `Depois de 5 minutos`.
- Ligado: ao abrir o app, ou ao voltar do segundo plano depois do tempo
  escolhido, a sobreposição `bloqueio` cobre tudo: logo do Duo, texto
  `O Duo está bloqueado` e botão `Desbloquear`, que chama a autenticação do
  sistema. O pedido abre sozinho uma vez ao aparecer a tela.
- Autenticação aceita biometria **ou** a senha do aparelho:
  iOS `LAContext.evaluatePolicy(.deviceOwnerAuthentication)` com
  `NSFaceIDUsageDescription` = `O Duo usa o Face ID para proteger suas contas.`;
  Android `BiometricPrompt` (androidx.biometric) com
  `BIOMETRIC_WEAK or DEVICE_CREDENTIAL` (aceita em todas as versões a
  partir do `minSdk`; `BIOMETRIC_STRONG or DEVICE_CREDENTIAL` só existe do
  API 30 em diante), título `Desbloquear o Duo`.
- Sem biometria e sem senha no aparelho: o ajuste fica desabilitado com o
  texto `Defina uma senha de bloqueio no aparelho para usar esta opção.`
- Enquanto bloqueado, nenhum conteúdo de conta é desenhado (os contêineres
  vivos continuam na memória, mas cobertos).

### 8.2 Bloqueio por conta

Chave `Pedir desbloqueio para abrir` no `editar` de cada conta. Ligada, abrir
o contêiner (card, `trocar`, atalho, deep link) pede a mesma autenticação
do §8.1, com o título `Abrir <apelido>`. Um desbloqueio vale até o app ir
para segundo plano. O card mostra o cadeado.

### 8.3 Ocultar no seletor de apps

Ajuste `Ocultar conteúdo no seletor de apps` (padrão ligado):

- iOS: ao entrar em `.inactive`/`.background`, uma cobertura com a cor de
  fundo e o logo é posta sobre a janela antes do instantâneo do sistema.
- Android 13+ (API 33): `Activity.setRecentsScreenshotEnabled(false)`.
  Android 8–12: `FLAG_SECURE` na janela **só** enquanto o bloqueio do app
  estiver ligado (o `FLAG_SECURE` também impede capturas de tela, por isso
  não é o padrão).

### 8.4 Nada sai do aparelho

- O Duo não tem servidor, conta, analytics, crash reporting remoto, anúncio
  nem SDK de terceiros. Nenhuma requisição de rede parte do código do Duo;
  só as páginas abertas pelo usuário fazem rede.
- Senhas digitadas nos sites vão direto para o site. O Duo não lê campos,
  não guarda senha e não injeta script em formulário. O preenchimento
  automático do sistema (Google Autofill, Senhas do iCloud) continua
  funcionando porque é do sistema.
- Dados locais: a lista de contas e preferências (§14) e o armazenamento
  web de cada contêiner. Tudo fora do backup em nuvem (§14).
- `WebView.setWebContentsDebuggingEnabled(false)` / `isInspectable = false`
  no release.

---

## 9. Atalhos e deep links

### 9.1 Deep links

Esquema `duo://`. Vetores em [`vetores/deeplink.json`](vetores/deeplink.json).

| URL | Destino |
|-----|---------|
| `duo://conta/<uuid>` | `conteiner` da conta (bloqueios do §8 valem); conta inexistente → Início + `Conta não encontrada` |
| `duo://adicionar` | Início com a folha `adicionar` |
| `duo://adicionar?servico=<id>` | direto em `adicionar-form` do serviço; id desconhecido → `adicionar` |
| qualquer outra `duo://…` | Início |

- Esquema e rota sem diferenciar maiúsculas; UUID aceito em maiúsculas e
  normalizado para minúsculas; barra final ignorada.
- Android: `intent-filter` com `<data android:scheme="duo"/>` na Activity
  única (`launchMode="singleTask"`), tratado em `onNewIntent`. iOS:
  `CFBundleURLTypes` com `duo` e `.onOpenURL`.
- Sem App Links / Universal Links nesta fase (não há site do Duo).

### 9.2 Atalho na tela inicial (por conta)

- **Android**: `Adicionar à tela inicial` chama
  `ShortcutManagerCompat.requestPinShortcut` com id `conta-<uuid>`, rótulo
  curto = apelido (até 10 caracteres; o longo = `<apelido> — <serviço>`),
  ícone adaptativo gerado (fundo = cor da conta, emoji ou inicial no centro,
  sem logo de serviço) e intent `ACTION_VIEW duo://conta/<uuid>`. Se o
  launcher não suporta (`isRequestPinShortcutSupported == false`), a opção
  não aparece. Ao excluir a conta, `disableShortcuts([id], "Conta excluída")`.
  Ao editar apelido/cor/ícone, `updateShortcuts`.
- **iOS**: não existe API para o app fixar ícone na tela inicial. A opção
  `Adicionar à tela inicial` abre uma folha que explica o caminho:
  `No app Atalhos, crie um atalho com a ação "Abrir conta no Duo", escolha <apelido> e toque em Adicionar à Tela de Início.`
  com o botão `Abrir o app Atalhos`. Para isso o app expõe **App Intents**:
  `AppEntity` `ContaEntidade` (id, apelido, serviço), `AppIntent`
  `AbrirContaIntent` (`openAppWhenRun = true`, abre `duo://conta/<id>`) e um
  `AppShortcutsProvider` com a frase `Abrir conta no \(.applicationName)`,
  para Siri, Spotlight e o botão de Ação.

### 9.3 Atalhos do ícone do app

As 4 contas com `usadaEm` mais recente aparecem ao tocar e segurar o ícone
do Duo: Android `ShortcutManagerCompat.pushDynamicShortcut` (id
`recente-<uuid>`), iOS `UIApplication.shared.shortcutItems`
(`UIApplicationShortcutItem`, tipo `conta`, `userInfo["id"]`). Atualizados
ao abrir um contêiner e ao editar/excluir. Conta com `bloqueada` aparece
igual, mas pede desbloqueio ao abrir.

---

## 10. Downloads e mídia

| Recurso | Suportado? | Como |
|---------|:----------:|------|
| Baixar arquivo que o site oferece como download (`Content-Disposition: attachment`, link `download` com URL `https`) | sim | CONTEINERES §7; salva em Downloads (Android) ou abre `Salvar em Arquivos`/compartilhar (iOS) |
| Salvar imagem pelo toque longo | sim | menu do sistema da WebView (iOS `Salvar na Fotos` exige `NSPhotoLibraryAddUsageDescription`); Android: menu próprio `Salvar imagem` para `<img>` com URL `https` |
| Download de `blob:` ou `data:` gerado por script | não nesta fase | a maioria das redes sociais não oferece; exigiria ponte JS (CONTEINERES §7) |
| Baixar vídeo/foto que o serviço **não** oferece para baixar | **não, nunca** | fere os termos dos serviços e a guideline 5.2.3 da Apple |
| Enviar foto/vídeo/arquivo para o site (`<input type=file>`) | sim | seletor do sistema: galeria, arquivos, câmera (CONTEINERES §7) |
| Câmera e microfone pedidos pelo site (`getUserMedia`: chamadas, stories na web) | sim, sob pedido | permissão do sistema na primeira vez + confirmação por conta e site (CONTEINERES §8) |
| Localização pedida pelo site | **não** | recusada sempre nesta fase; o app não declara permissão de localização |
| Vídeo em tela cheia | sim | tela cheia nativa da WebView |
| Reprodução em segundo plano / picture-in-picture | não | o áudio para quando o app vai para segundo plano |

---

## 11. Notificações

**O Duo não mostra notificações dos serviços.** Web Push não funciona dentro
de `WKWebView` nem do `WebView` do Android: no iOS, o Web Push existe só
para sites adicionados à Tela de Início pelo Safari; no Android, a API
`Notification`/`PushManager` não está disponível para páginas em WebView.
Mesmo que estivesse, o Duo precisaria de um servidor para receber os envios
— e o Duo não tem servidor.

Decisão desta fase: **nenhuma notificação**, nem lembrete local. O app não
pede permissão de notificação (Android `POST_NOTIFICATIONS` ausente; iOS sem
`UNUserNotificationCenter`). Pedidos de permissão de notificação feitos pelas
páginas são recusados em silêncio.

Onde isso é dito ao usuário:

- Ajustes → linha fixa `Notificações` com o texto
  `O Duo não recebe notificações dos sites. Para ser avisado, mantenha o app oficial em uma das contas.`
- Descrição das lojas (MARKETING_ASO) e PRODUTO.md.

---

## 12. Ajustes (`ajustes`) e Sobre (`sobre`)

### 12.1 Ajustes

Coluna rolável, nesta ordem, com selos:

1. `APARÊNCIA`: `Tema` (`Sistema`, `Claro`, `Escuro`); `Agrupar contas por serviço` (chave).
2. `PRIVACIDADE`: `Bloquear o Duo` (§8.1); `Ocultar conteúdo no seletor de apps` (§8.3).
3. `NAVEGAÇÃO`: `Links de outros sites` (`Abrir no navegador` / `Perguntar sempre`, §6.5).
4. `NOTIFICAÇÕES`: o texto fixo do §11.
5. `DADOS`: `Apagar todas as contas` (vermelho) → diálogo
   `Apagar todas as contas?` / `Todas as contas e os dados delas neste aparelho serão apagados. Não dá para desfazer.` / `Cancelar` / `Apagar tudo`; pede autenticação do §8.1 se o bloqueio estiver ligado.
6. `SOBRE`: `Sobre o Duo` → `sobre`.

### 12.2 Tema

Três estados `sistema` (padrão), `claro`, `escuro`; aplica na hora e
persiste. O tema do Duo vale para a interface nativa. O conteúdo dos sites
recebe `prefers-color-scheme` coerente com o tema escolhido (iOS: o
`overrideUserInterfaceStyle` da janela vale também para o `WKWebView`;
Android, `targetSdk` ≥ 33: o WebView segue o atributo `isLightTheme` do
tema da Activity, e o escurecimento algorítmico fica desligado —
`WebSettingsCompat.setAlgorithmicDarkeningAllowed(false)` — para o site não
ser invertido à força).

### 12.3 Sobre

1. Caixa: título `DUO` e o texto
   `Várias contas do mesmo serviço num só aparelho. Cada conta roda num contêiner web isolado, com login e dados próprios — e nada sai do seu aparelho.`
2. Caixa sem sombra: `O Duo não é afiliado, patrocinado nem aprovado por nenhum dos serviços do catálogo. Os nomes pertencem aos seus donos e aparecem só para identificar o site que você abre.`
3. Selo `LINKS`; tile `Política de privacidade` → URL da política
   ([POLITICA_DE_PRIVACIDADE.md](../../../publicacao/POLITICA_DE_PRIVACIDADE.md)); tile `Falcao SL` → `https://falcaosl.com`.
4. Versão: `Versão <versionName> (<build>)`.

Os links abrem no navegador do sistema.

---

## 13. Limites e plano

- Parâmetros em [`dados/aparencia.json → limites`](dados/aparencia.json):
  `maxContas` = 50 (teto técnico, para todos) e `contasGratis` = `null`.
- **Nesta fase não há paywall, compra no app nem assinatura**. O modelo
  proposto está em [MONETIZACAO.md](../../../produto/MONETIZACAO.md) e é
  **a decidir pelo usuário**.
- Quando `contasGratis` virar número, `podeAdicionar` devolve o motivo
  `plano` e o app mostra a tela de plano (a especificar junto com a decisão).
  Contas já criadas acima do limite **nunca** são bloqueadas nem apagadas.

---

## 14. Persistência local

| Dado | Conteúdo | Android | iOS |
|------|----------|---------|-----|
| Contas | lista de `Conta` (§3) | Room (tabela `contas`) | JSON em Application Support (`contas.json`), gravação atômica |
| Preferências | §3 | DataStore Preferences | `UserDefaults` / `@AppStorage` |
| Exclusões pendentes | perfis/armazenamentos a apagar (CONTEINERES §9) | DataStore (`perfisPendentes`, conjunto de nomes) | `UserDefaults` (`armazenamentosPendentes`, UUIDs) |
| Armazenamento web | um por conta | perfis do WebView em `app_webview/` (gerido pelo WebView) | `WKWebsiteDataStore` persistente por UUID (gerido pelo WebKit) |

- Gravação a cada mudança.
- Backup: Android `android:allowBackup="false"` e `dataExtractionRules`
  excluindo tudo (sessão de conta restaurada em outro aparelho seria um
  risco); iOS: diretório com `isExcludedFromBackup = true`; os dados do
  `WKWebsiteDataStore` ficam em `Library/` gerida pelo WebKit — CONTEINERES §9
  registra o que se sabe sobre backup deles.
- Falha de armazenamento não pode quebrar o app: mostra o Início com o que
  conseguiu ler e um aviso `Não foi possível salvar. Verifique o espaço do aparelho.`

---

## 15. Acessibilidade e idioma

| Requisito | Regra |
|-----------|-------|
| Rótulos | Todo botão só-ícone tem rótulo: `Ajustes`, `Limpar busca`, `Adicionar conta`, `Voltar`, `Recarregar`, `Mais opções`, `Voltar página`, `Avançar página`, `Trocar de conta`, `Compartilhar link`, `Mover para cima`, `Mover para baixo`, `Organizar contas`. |
| Card de conta | Lido como um botão: `<apelido>, <serviço>` + `, fixada` / `, bloqueada` quando for o caso; ações do menu como ações de acessibilidade (Android `customActions`, iOS `accessibilityActions`). |
| Avisos | Toasts e faixas de aviso anunciados (live region / `AccessibilityNotification.Announcement`). |
| Toque | Mínimo 48 × 48 dp (Android) e 44 × 44 pt (iOS). |
| Contraste | Texto ≥ 4,5:1; texto sobre acento sempre `tintaSobreAcento`; bordas ≥ 3:1. |
| Fonte | Dynamic Type e escala de fonte do sistema até 200 % sem cortar texto: cards crescem em altura. Conteúdo web: Android `WebSettings.textZoom` = escala de fonte do sistema × 100; iOS segue o zoom de texto do WebKit (`pageZoom` = 1, sem bloquear pinça). |
| Movimento | Animação de pressão desligada com redução de movimento. |
| Idioma | Interface em pt-BR. Todas as strings em recursos (`strings.xml`; String Catalog `Localizable.xcstrings`), nenhuma no código, prontas para `en`. Datas e números pelo locale. |

---

## 16. Aparelho sem suporte (Android)

O isolamento no Android depende de `WebViewFeature.MULTI_PROFILE`
(CONTEINERES §2.2). Se `isFeatureSupported(MULTI_PROFILE)` for `false`:

- O app **não abre nenhum contêiner** — nunca cai para um WebView
  compartilhado, porque isso misturaria as contas.
- Início, adicionar, editar e ajustes funcionam; tocar numa conta mostra a
  sobreposição `sem-suporte`: título `Atualize o WebView`, texto
  `O Duo precisa de uma versão mais nova do Android System WebView para separar as contas. Atualize pela Play Store e abra o Duo de novo.`
  e os botões `Abrir na Play Store` (`market://details?id=com.google.android.webview`,
  com fallback `https://play.google.com/store/apps/details?id=com.google.android.webview`)
  e `Agora não`.
- A verificação é refeita a cada `onResume` (o WebView pode ter sido
  atualizado em segundo plano).

No iOS não existe caso equivalente: o `minimumOSVersion` 17.0 garante a API.

---

## 17. Matriz de paridade

Estado atual: nenhum app implementado. Ao mudar uma célula, atualize também
o README da plataforma.

| Funcionalidade | Obrigatória | Android (recurso nativo sugerido) | iOS (recurso nativo sugerido) |
|----------------|:-----------:|-----------------------------------|-------------------------------|
| Motor `nativo` (clonar app instalado, fase 2) | não (fase 2) | a implementar — só no build `completo` (VIRTUALIZACAO.md) | **não suportado pela plataforma** (App Store 2.5.2) |
| Abstração `MotorDeConta` | sim | a implementar — interface no `:app` | a implementar — protocolo no app (só `web`) |
| Núcleo puro: URL, navegação, apelido, ordem, busca, deep link | sim | a implementar — `android/core` Kotlin/JVM | a implementar — `ios/DuoCore` Swift Package |
| Vetores de `docs/spec/vetores/` passando | sim | a implementar — JUnit (`:core:test`) | a implementar — XCTest (`swift test`) |
| Catálogo de `docs/spec/dados/` idêntico | sim | a implementar — teste de dados | a implementar — teste de dados |
| Contêiner isolado por conta | sim | a implementar — `ProfileStore` + `WebViewCompat.setProfile` | a implementar — `WKWebsiteDataStore(forIdentifier:)` |
| Aparelho sem suporte (§16) | sim | a implementar — `WebViewFeature.isFeatureSupported` | — (garantido pelo iOS 17) |
| Início: grade, seções, chips, busca, vazio | sim | a implementar — `LazyVerticalGrid` | a implementar — `LazyVGrid` |
| Organizar (arrastar + mover) | sim | a implementar — `Modifier.draggable` + botões | a implementar — `onMove`/`draggable` + botões |
| Adicionar/editar/excluir conta | sim | a implementar — `ModalBottomSheet` | a implementar — `.sheet` |
| Site personalizado validado | sim | a implementar — núcleo + `java.net.IDN` | a implementar — núcleo + `URLComponents` / IDNA |
| Barra do contêiner, voltar/avançar/recarregar | sim | a implementar — `WebView.goBack/goForward/reload` | a implementar — `WKWebView` |
| Puxar para atualizar | sim | a implementar — `PullToRefreshBox` | a implementar — `UIRefreshControl` |
| Classificação de navegação e avisos | sim | a implementar — `shouldOverrideUrlLoading` | a implementar — `decidePolicyFor navigationAction` |
| Trocar conta (folha + gesto) | sim | a implementar | a implementar |
| Até 3 contêineres vivos (LRU) + memória baixa | sim | a implementar — `onTrimMemory`, `onRenderProcessGone` | a implementar — `didReceiveMemoryWarning`, `webViewWebContentProcessDidTerminate` |
| Retomar em `ultimaUrl` | sim | a implementar | a implementar |
| Bloqueio do app e por conta | sim | a implementar — `BiometricPrompt` | a implementar — `LAContext` |
| Ocultar no seletor de apps | sim | a implementar — `setRecentsScreenshotEnabled` / `FLAG_SECURE` | a implementar — cobertura em `scenePhase` |
| Limpar dados / excluir com apagamento | sim | a implementar — nova geração de perfil + `deleteProfile` na abertura | a implementar — `removeData` / `WKWebsiteDataStore.remove(forIdentifier:)` |
| Deep links `duo://` | sim | a implementar — `intent-filter` | a implementar — `onOpenURL` |
| Atalho por conta na tela inicial | sim | a implementar — `requestPinShortcut` | a implementar — App Intents + instrução do app Atalhos |
| Atalhos do ícone (4 recentes) | não | a implementar — dynamic shortcuts | a implementar — `UIApplicationShortcutItem` |
| Downloads de arquivo `https` | sim | a implementar — `DownloadListener` + `DownloadManager` com cookies do perfil | a implementar — `WKDownload` |
| Upload de arquivo/câmera | sim | a implementar — `onShowFileChooser` + `ActivityResultContracts` | a implementar — nativo do `WKWebView` |
| Câmera/microfone para o site | sim | a implementar — `onPermissionRequest` | a implementar — `requestMediaCapturePermissionFor` |
| Tema sistema/claro/escuro | sim | a implementar — DataStore | a implementar — `@AppStorage` |
| Ajustes e Sobre | sim | a implementar | a implementar |
| Acessibilidade mínima (§15) | sim | a implementar | a implementar |
| Strings em recurso, pronto para `en` | sim | a implementar — `strings.xml` | a implementar — `Localizable.xcstrings` |
| Ícone do app e splash | sim | a implementar — ícone adaptativo + SplashScreen API | a implementar — `AppIcon.icon` + `UILaunchScreen` |
| iPad / tablet | não | a implementar — grade larga | a implementar — grade larga, multitarefa |

---

## Decisões

Escolhas tomadas ao escrever esta especificação (2026-09-21). Mudar uma delas
é mudar a spec primeiro.

| # | Tema | Decisão | Motivo |
|---|------|---------|--------|
| D-1 | Tecnologia de isolamento | Motor `web` nas duas plataformas (v1); motor `nativo` só no Android, fase 2, fora do build da Play (decisões do usuário + VIRTUALIZACAO.md) | O motor web é o único aceito nas duas lojas; a virtualização é impossível no iOS e de alto risco na Play |
| D-21 | Motor da conta | Escolhido ao criar, imutável | Dados de um motor não servem ao outro |
| D-22 | Builds Android | `play` (só `web`) e `completo` (`web` + `nativo`, distribuído fora da Play) | Isola o risco de política e de licença do motor nativo (CONFORMIDADE_LOJAS.md) |
| D-2 | Android sem `MULTI_PROFILE` | Bloqueia contêineres e pede atualização do WebView (§16); nunca usa WebView compartilhado | Misturar sessões quebraria a promessa central |
| D-3 | `minSdk` Android | **26** (Android 8.0) | `MULTI_PROFILE` exige WebView multiprocesso, que começa no Android 8.0 (CONTEINERES §2.2); abaixo disso o app nunca funcionaria |
| D-4 | iOS mínimo | **17.0** | `WKWebsiteDataStore(forIdentifier:)` é iOS 17+ |
| D-5 | Início | Tudo junto, com seção `FIXADAS`, chips de filtro por serviço e opção de agrupar por serviço | Quem tem várias contas de um só serviço não ganha nada com agrupamento; quem tem de vários ganha o filtro |
| D-6 | Serviço e endereço editáveis? | Não | Os dados do contêiner pertencem ao site original |
| D-7 | Emoji | Lista fechada de 32 | Validação igual nas duas plataformas; sem teclado de emoji livre |
| D-8 | Contêineres vivos | 3 (1 com memória baixa), LRU | Cada WebView de rede social pesa 100–300 MB; 3 dá troca instantânea sem ser morto pelo sistema |
| D-9 | Retomada | Só `ultimaUrl`, sem histórico | Paridade: o Android não restaura histórico por API pública |
| D-10 | Links de outros sites | Navegador do sistema por padrão; `Perguntar sempre` opcional | Mantém o contêiner no serviço e evita misturar domínios |
| D-11 | Links para o app oficial | Bloqueados com aviso e opção `Abrir fora` | Abrir o app oficial trocaria de conta sem o usuário perceber |
| D-12 | User-Agent | Padrão do motor; `desktop` só onde a web do serviço é só para computador; nunca disfarçar em domínios de identidade | Evitar contornar bloqueio de segurança (Play: Abuso de dispositivo e rede) |
| D-13 | Notificações | Nenhuma nesta fase | Web Push não funciona em WebView; lembrete local sem conteúdo não ajuda |
| D-14 | Localização pedida por site | Sempre recusada | Evita declarar permissão de localização e a pergunta das lojas |
| D-15 | Downloads | Só `https` direto; sem `blob:` nesta fase; nunca "baixar vídeo" | 5.2.3 da Apple e termos dos serviços |
| D-16 | Limite de contas | 50 técnico; `contasGratis = null` | Monetização a decidir |
| D-17 | Atalho na tela inicial no iOS | App Intents + instrução do app Atalhos | Não há API pública para fixar ícone |
| D-18 | Erro de certificado | Sem "continuar mesmo assim" | Contêiner guarda sessões; aceitar certificado inválido é risco de roubo de sessão |
| D-19 | Backup | Fora do backup em nuvem | Sessão restaurada em outro aparelho é risco; consistente com o homelab/local-first |
| D-20 | Site personalizado | Só `https` (http vira https), sem IP, sem porta | Evita sites locais/inseguros num app que guarda sessões |

## A decidir pelo usuário

| # | Tema | Opções | Onde |
|---|------|--------|------|
| U-1 | Monetização | grátis total / 3 contas grátis + assinatura / compra única (proposta em MONETIZACAO.md) | [MONETIZACAO.md](../../../produto/MONETIZACAO.md) |
| U-2 | Nome na loja | `Duo: Múltiplas Contas` (recomendado) ou alternativas | [MARKETING_ASO.md](../../../produto/MARKETING_ASO.md) |
| U-3 | Serviços de maior risco no catálogo de lançamento | manter ou tirar Snapchat, WhatsApp Web e Discord (experimentais) | [CONFORMIDADE_LOJAS.md](../publicacao/CONFORMIDADE_LOJAS.md) |
| U-4 | Domínio da política de privacidade | `duo.falcaosl.com/privacidade.html` ou página em `falcaosl.com` | [POLITICA_DE_PRIVACIDADE.md](../../../publicacao/POLITICA_DE_PRIVACIDADE.md) |
| U-6 | Motor nativo: base e licença | abrir o código do Android sob GPL-3.0 / comprar licença comercial / implementação própria / não fazer | [VIRTUALIZACAO.md](VIRTUALIZACAO.md) §3 |
| U-7 | Distribuição do build `completo` | site próprio (APK) / outras lojas / não distribuir | [DISTRIBUICAO.md](../../../produto/DISTRIBUICAO.md) |
| U-5 | Idioma de lançamento | só pt-BR ou pt-BR + en | [DISTRIBUICAO.md](../../../produto/DISTRIBUICAO.md) |
