# Vetores de teste

Casos de comportamento comum às duas plataformas. As suítes do Android e do
iOS leem **estes arquivos** e têm de passar em 100 % dos casos. Se um caso
falhar numa plataforma, o bug é da plataforma, não do vetor. Se um vetor
precisar mudar, a mudança vale para as duas ao mesmo tempo (mesmo PR).

Todos os arquivos são JSON UTF-8. Strings são comparadas **exatamente**
(acentos, espaços, maiúsculas). `null` é significativo: "sem valor", não
"qualquer valor".

Origem: os vetores foram escritos junto com a especificação (2026-09-21) e
conferidos com uma implementação de referência descartável, feita só para
isso e não versionada. Quando o núcleo Kotlin (`android/core`) existir, ele
passa a ser a referência.

## Como cada plataforma lê

| Plataforma | Caminho a partir da raiz do projeto | Como lê |
|------------|-------------------------------------|---------|
| Android (`android/`) | `../docs/spec/vetores/*.json` e `../docs/spec/dados/*.json` | `core/build.gradle.kts` passa o caminho absoluto de `../docs/spec` na propriedade de sistema `duo.spec` (declarada como entrada da task de teste); os testes JUnit de `core/src/test/.../paridade` leem com `kotlinx-serialization-json` |
| iOS (`ios/DuoCore/`, pacote Swift) | `../../docs/spec/vetores/*.json` | os testes resolvem a partir do próprio arquivo: `URL(fileURLWithPath: #filePath)` sobe até a raiz do repositório (SwiftPM não copia recursos de fora do pacote) |

O mesmo vale para `docs/spec/dados/`: cada plataforma embute uma cópia de
`servicos.json` e `aparencia.json` no app e tem um teste que compara a cópia
com o JSON (mesmos ids, na mesma ordem, mesmos campos).

## Arquivos

| Arquivo | Casos | O que testa | Função no núcleo |
|---------|------:|-------------|------------------|
| `url-personalizada.json` | 43 | validação e normalização da URL de site personalizado | `normalizarUrlPersonalizada(entrada)` |
| `navegacao.json` | 50 | classificação de navegação: dentro do contêiner, fora, app oficial, bloquear | `classificarNavegacao(url, conta)` |
| `contas.json` | 14 + 7 + 10 + 7 + 6 | apelido, apelido padrão, inicial do avatar, limite de contas, cor padrão | `validarApelido`, `apelidoPadrao`, `inicial`, `podeAdicionar`, `corPadrao` |
| `inicio.json` | 1 + 12 + 5 | ordenação, busca com filtro de serviço, normalização da busca | `ordenar`, `buscar`, `normalizarBusca` |
| `deeplink.json` | 14 | interpretação de `duo://` | `interpretarDeepLink(url)` |
| `clonaveis.json` | 10 + 2 | **só Android, fase 2** (motor nativo): classificação de app clonável e diretório da cópia | `classificarClonavel`, `diretorioDaCopia` |

Total: 129 casos comuns às duas plataformas + 12 casos só do Android (fase 2).

## Esquemas

### `url-personalizada.json`

```json
{ "casos": [ { "entrada": "…", "valida": true, "url": "https://…/", "dominioBase": "exemplo.com.br", "servicoSugerido": null, "erro": null } ] }
```

- Chame a função com `entrada` **exatamente como está** (o `trim` faz parte
  da função).
- Válida: `url`, `dominioBase` e `servicoSugerido` (id do catálogo quando o
  host pertence a um serviço — a tela sugere usar o serviço) preenchidos;
  `erro` null.
- Inválida: os três null e `erro` com o texto exato.
- Regras em [FUNCIONALIDADES.md §5.3](../FUNCIONALIDADES.md#53-site-personalizado).

### `navegacao.json`

```json
{ "contas": { "<nome>": { "servico": "instagram" } },
  "casos": [ { "conta": "<nome>", "url": "…", "acao": "conteiner", "destino": "https://…", "servicoApp": null } ] }
```

- `conta` aponta para `contas`; conta personalizada traz `dominioBase`.
- `acao`: `conteiner` | `fora` | `app` | `bloquear`.
- `destino`: a URL a carregar (`conteiner`, já com `http` → `https`) ou a
  entregar ao sistema (`fora`); `null` em `app` e `bloquear`.
- `servicoApp`: em `app`, o id do serviço dono do esquema (para o texto do
  aviso); `null` se o esquema não é do catálogo ou se é loja de apps.
- Regras em [CONTEINERES.md §6](../CONTEINERES.md#6-navegação-o-que-abre-dentro-e-o-que-sai).

### `contas.json`

| Chave | Entrada | Esperado |
|-------|---------|----------|
| `validarApelido` | `entrada`, `existentesNoServico` (apelidos das outras contas do mesmo serviço) | `valido`, `apelido` (normalizado; `null` quando vazio — usar o padrão), `erro` |
| `apelidoPadrao` | `nomeServico`, `existentesNoServico` | `apelido` |
| `inicial` | `apelido` | `inicial` (uma letra ou dígito em maiúscula, ou `?`) |
| `podeAdicionar` | `quantidade` de contas atual, `contasGratis` (`null` = sem limite de plano) | `pode`, `motivo` (texto, ou a chave `plano`) |
| `corPadrao` | `quantidade` de contas atual | `cor` (chave de `cor.acento`) |

`maxContas` vem de `dados/aparencia.json → limites.maxContas` (50).

### `inicio.json`

- `contas`: a lista usada por todos os casos.
- `ordenar`: ids na ordem de exibição.
- `buscar`: `consulta`, `servico` (id do chip de filtro ou `null`) → `ids`.
- `normalizarBusca`: `entrada` → `saida` (a função não apara; a busca apara antes).

### `deeplink.json`

- `url` → `resultado`: `null` (não é do Duo), `{ "rota": "inicio" }`,
  `{ "rota": "conteiner", "id": "<uuid em minúsculas>" }` ou
  `{ "rota": "adicionar", "servico": "<id>" | null }`.

### `clonaveis.json` (só Android, fase 2)

- `classificar`: `app` (metadados de `PackageManager`), `abisSuportadas`,
  `incompativeis` → `clonavel`, `motivo` (texto exato ou `null`). Regras em
  [VIRTUALIZACAO.md §5](../VIRTUALIZACAO.md#5-quais-apps-são-clonáveis).
- `diretorioDaCopia`: `contaId`, `pacote` → `diretorio`.
- **A suíte iOS ignora este arquivo** (o iOS não tem motor nativo); só a
  suíte Android o roda.

## Mudou uma regra?

1. Atualize o texto da regra em `FUNCIONALIDADES.md` ou `CONTEINERES.md`.
2. Ajuste ou acrescente os casos aqui e a contagem desta página.
3. Rode as duas suítes: `cd android && ./gradlew :core:test` e
   `cd ios/DuoCore && swift test`. As duas falham até cada núcleo acompanhar.
4. Suba spec, vetores e os dois núcleos no mesmo PR.

Mudou só uma plataforma e um vetor falhou? O bug é da plataforma: não mexa
no vetor.
