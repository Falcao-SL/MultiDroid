# Política de Privacidade — Duo

**Vigente a partir de:** [data da primeira publicação]

O Duo permite usar várias contas do mesmo serviço num só aparelho, cada uma
isolada. Ele foi construído para não precisar dos seus dados — e não precisa.

## Resumo

- **Não coletamos nenhum dado pessoal.**
- **Não temos cadastro, login nem conta de usuário do Duo.**
- **Nada que você faz nas suas contas sai do seu aparelho por causa do Duo.**
- **Não usamos rastreadores, analytics nem SDK de terceiros.**
- **O Duo não tem servidor.**

## Como o Duo funciona

Cada conta que você adiciona é um **contêiner isolado** dentro do próprio
aparelho:

- **Motor web**: um perfil de navegador separado (no iOS, `WKWebsiteDataStore`
  por conta; no Android, um perfil do WebView por conta) que abre a versão web
  do serviço. Cookies, login, armazenamento e cache de cada conta ficam
  guardados só no seu aparelho e separados dos das outras contas.
- **Motor nativo** (somente Android, quando disponível): uma cópia do app que
  você já instalou, rodando isolada, com dados guardados só no seu aparelho,
  numa pasta separada por cópia.

## Dados que o app processa (tudo local)

- **Sua lista de contas e preferências** (apelido, cor, ordem, tema, opções
  de bloqueio) ficam num armazenamento local no aparelho.
- **Os dados das suas sessões** (cookies, login, cache) ficam no contêiner de
  cada conta, no aparelho. Quando você entra numa conta, você entra **no
  site/app do serviço**, direto do seu aparelho para o serviço — o Duo não
  fica no meio, não lê o que você digita e não guarda suas senhas.
- Você pode apagar os dados de uma conta (`Limpar dados`), excluir a conta ou
  desinstalar o app a qualquer momento.

## Dados que NÃO coletamos

Não coletamos, não armazenamos em servidores e não compartilhamos: nome,
e-mail, telefone, localização, identificadores de dispositivo, contatos,
conteúdo das suas contas, dados de uso ou qualquer informação pessoal. O Duo
não faz nenhuma requisição de rede por conta própria; a rede que acontece é a
dos sites/apps que **você** abre.

## Permissões

- **Câmera / microfone**: pedidas só quando um site que você abriu quer usá-las
  (ex.: enviar uma foto, uma chamada), e só com a sua autorização. O Duo não
  usa a câmera nem o microfone por conta própria.
- **Localização**: o Duo recusa pedidos de localização dos sites e não declara
  permissão de localização.
- **Notificações**: no motor web, o Duo não recebe notificações dos serviços
  (limitação técnica dos navegadores incorporados) e não pede permissão de
  notificação. No motor nativo (Android), as notificações são as do próprio
  app clonado.
- **Ver apps instalados** (Android, só no build completo com motor nativo):
  usada apenas para listar os apps que você pode clonar; essa lista não sai
  do aparelho.

## Sobre os serviços de terceiros

Os serviços que você abre no Duo (redes sociais, mensageiros etc.) têm as
**próprias políticas de privacidade e termos de uso**, que continuam valendo
na sua relação com eles. O Duo **não é afiliado, patrocinado nem aprovado**
por nenhum deles; os nomes citados pertencem aos seus donos e servem só para
identificar o serviço que você abre. O Duo não automatiza ações nas suas
contas.

## Segurança

Os dados ficam na área do app protegida pelo sistema operacional. Você pode
ativar o bloqueio por biometria (do app e/ou por conta). Nada é enviado para
fora do aparelho pelo Duo, e não há backup em nuvem das suas sessões.

## Seus direitos (LGPD)

Como o Duo **não coleta nem trata dado pessoal algum**, não há nada em nosso
poder para acessar, corrigir, excluir ou portar: tudo o que o app guarda está
no seu aparelho, sob seu controle. Dúvidas: o contato abaixo.

## Crianças

O app não é direcionado a crianças e não coleta dados de ninguém.

## Alterações

Podemos atualizar esta política; mudanças relevantes aparecem na data no topo
e na versão publicada nas lojas.

## Contato

Falcao SL (Francisco Iramar Falcão Junior), São Paulo — Brasil.
Dúvidas sobre privacidade: **dev@falcaosl.com**

---

*A publicar em `https://duo.falcaosl.com/privacidade.html` (ou página em
`falcaosl.com` — U-4); é essa URL que vai nas fichas da Play e da App Store.*
