# Política de Privacidade — Duo

**Vigente a partir de:** [data da primeira publicação]

O Duo é um app de Mac que roda vários aparelhos Android lado a lado, sobre o
Android Emulator oficial. Ele foi construído para não precisar dos seus
dados — e não precisa.

## Resumo

- **Não coletamos nenhum dado pessoal.**
- **Não temos cadastro, login nem conta de usuário do Duo.**
- **Não usamos rastreadores, analytics nem SDK de terceiros.**
- **O Duo não tem servidor.**
- A única rede que o Duo usa por conta própria são os **downloads do
  Google** que você pede (emulador e imagens de sistema).

## O que o Duo baixa

Quando você instala ou atualiza o emulador ou uma imagem, o Duo lê o
catálogo público do Google e baixa os arquivos de
`https://dl.google.com/android/repository/`. É uma requisição comum ao
servidor do Google, sem dado seu além do que qualquer download envia (como o
endereço IP). O Duo confere o SHA-1 publicado pelo Google antes de instalar.
O uso do repositório e das imagens segue os termos do Google, que o Duo
mostra para você aceitar antes do download.

## Dados que ficam no seu Mac

| O quê | Onde |
|-------|------|
| Emulador, imagens e aceites de licença | `~/Library/Application Support/Duo/sdk/` |
| Seus aparelhos Android (contas, apps, fotos dentro deles) | `~/Library/Application Support/Duo/avd/` |
| Preferências do Duo (ordem, grupos, mesas, posições, teto de memória) | `~/Library/Application Support/Duo/state.json` |
| Capturas de tela que você tira | `~/Pictures/Duo/` |

Nada disso é enviado a ninguém pelo Duo. Você apaga um aparelho no próprio
app; apagar a pasta `~/Library/Application Support/Duo/` remove tudo, menos as
capturas em `~/Pictures/Duo/`.

## Os aparelhos Android

Cada aparelho é um Android de verdade rodando no seu Mac. Dentro dele, você
entra nas suas contas e usa os apps que quiser, como num celular: a rede que
esses apps usam é deles, direto do aparelho para o serviço. O Duo não guarda
o que você digita nem senhas e não lê o conteúdo das suas contas; ele só
mostra a tela e repassa toques, teclas e texto ao emulador, por uma conexão local
(localhost, com token) que não sai do Mac.

O Duo liga o emulador com a opção `-no-metrics`, que desliga o envio de
estatísticas de uso do emulador ao Google. O Android dentro de cada aparelho
(e os serviços do Google Play, se a imagem os tiver) tem as próprias regras
de privacidade, que valem entre você e o Google.

## Dados que NÃO coletamos

Não coletamos, não armazenamos em servidores e não compartilhamos: nome,
e-mail, telefone, localização, identificadores de dispositivo, contatos,
conteúdo dos aparelhos, dados de uso ou qualquer informação pessoal.

## Sobre os serviços de terceiros

Os apps e serviços que você usa dentro dos aparelhos têm as **próprias
políticas de privacidade e termos de uso**. O Duo **não é afiliado,
patrocinado nem aprovado** pelo Google nem por nenhum desses serviços. O
Duo não automatiza ações nas suas contas.

## Seus direitos (LGPD)

Como o Duo **não coleta nem trata dado pessoal algum**, não há nada em nosso
poder para acessar, corrigir, excluir ou portar: tudo o que o app guarda
está no seu Mac, sob seu controle. Dúvidas: o contato abaixo.

## Crianças

O app não é direcionado a crianças e não coleta dados de ninguém.

## Alterações

Podemos atualizar esta política; mudanças relevantes aparecem na data no
topo e na versão publicada.

## Contato

Falcao SL (Francisco Iramar Falcão Junior), São Paulo — Brasil.
Dúvidas sobre privacidade: **dev@falcaosl.com**

---

*Endereço de publicação: a decidir (página do Duo em `falcaosl.com` ou
domínio próprio), junto com a página de download.*
