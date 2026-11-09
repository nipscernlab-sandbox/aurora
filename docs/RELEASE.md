# Releases da AURORA

Este documento define o número de cada release da AURORA, quando lançar, o
passo a passo e o que fazer quando uma release sai com defeito. Vale para
quem lança e para quem revisa uma mudança que vai entrar numa release.

O que conta como mudança compatível no protocolo do `aurora serve` está em
[METODOLOGIA.md](METODOLOGIA.md), seção 5. A seção 2 daqui usa aquela
tabela e a estende ao resto da AURORA: a linha de comando, os projetos, a
instalação e o bundle.

## 1. Os números de versão

| Número | Formato | Onde mora | Quando muda |
|---|---|---|---|
| AURORA API | `X.Y.Z`, em [SemVer](https://semver.org/lang/pt-BR/) | o `Cargo.toml` e os pacotes `aurora-learn` e `rtl-viewer` de `bundle/versions.json` (seção 6.1) | a cada release, pela seção 2 |
| AURORA IDE | `X.Y.Z`, em SemVer, própria | os cinco lugares da IDE na seção 6.1, com o pacote `studio` de `bundle/versions.json` | quando a IDE muda; a primeira com o nome novo é a 8.0.0 |
| Extensão do VS Code | `X.Y.Z`, em SemVer, própria | `editors/vscode/package.json` | quando o `.vsix` muda (seção 2.7) |
| `PROTOCOL_VERSION` | inteiro | `crates/aurora-protocol/src/lib.rs` | quando um cliente antigo do `aurora serve` deixaria de funcionar ([METODOLOGIA.md](METODOLOGIA.md), seção 5) |
| Bundle | data, `AAAA.MM.DD` | o campo `bundle` de `bundle/versions.json` | quando qualquer ferramenta do bundle muda ([CONTRIBUTING.md](../CONTRIBUTING.md), "Atualizar a versão de uma ferramenta do bundle") |
| surfer-aurora pré-compilado | a versão do fork, `vA.B.C-nips.N` | a tag `surfer-aurora-<versão>`, uma pré-release deste repositório | quando o commit do surfer-aurora em `bundle/versions.json` muda (`.github/workflows/surfer-aurora.yml`) |
| Pacote msys do Windows | `ucrt64-vN` | uma release do repositório `lace-toolchain`, apontada em `bundle/versions.json` | quando o Icarus, o Verilator ou o resto do bloco MSYS2 mudam no Windows ([BUNDLE.md](BUNDLE.md), seção 6) |

A AURORA tem um número só. A CLI, o core, o instalador, o `aurora learn` com as
trilhas, o assistente, o Studio e o RTL viewer saem juntos, na mesma
versão; não há release só do Studio nem só da CLI. A extensão do VS Code é
a única peça com versão própria, e mesmo ela é publicada dentro da release
da AURORA.

As tags que começam com `v` são só das releases da AURORA: o `release.yml`
roda em toda tag `v*`. Os outros artefatos usam prefixo próprio, como o
`surfer-aurora-`.

## 2. Que número a release recebe

A versão tem três números, `MAIOR.MENOR.CORREÇÃO`, como em `0.9.1`. Cada
release sobe exatamente um deles e zera os que estão à direita dele. Qual
sobe depende só do que a release leva.

### 2.1 As três posições

**`X.0.0`, a versão maior.** Sobe quando a release leva pelo menos uma
quebra de compatibilidade: uma mudança depois da qual algo que funcionava
na versão anterior para de funcionar sem que a pessoa tenha mudado nada do
lado dela. Os contratos que contam estão na seção 2.3. Exemplos: renomear
um comando, como o `aurora sim`; tirar ou renomear um campo do `--json`;
mudar a forma de um método do `aurora serve`, o que sobe o
`PROTOCOL_VERSION`; um projeto que só abre depois de convertido, como na
troca da estrutura do YANC 6.x pela do 7.1; deixar de atender o Ubuntu
22.04. Zera as outras duas: `1.4.2` vira `2.0.0`. Enquanto a versão
começar com 0, este número não sobe (seção 2.4).

**`0.X.0`, a versão menor.** Sobe quando a release não quebra nada e leva
pelo menos uma novidade: algo que a pessoa pode fazer e antes não podia,
ou algo que passa a funcionar de outro jeito por padrão. Exemplos: um
comando, uma flag ou um método novo, como o `aurora serve` da 0.9.0; um
campo novo no `--json`; uma vista, uma preferência ou um atalho novo no
Studio; um exercício novo no `aurora learn`; uma placa FPGA nova; um bundle
novo; uma opção que passa a vir ligada. Zera a correção: `0.9.3` vira
`0.10.0`.

**`0.0.X`, a correção.** Sobe quando tudo o que a release leva é conserto:
a versão publicada fazia algo errado, e a nova faz o que já devia fazer,
sem nada novo para aprender e sem nada a mudar do lado de quem usa.
Exemplos: o texto digitado durante a gravação, que ficava marcado como
gravado sem ter sido; o fonte que podia ficar pela metade se a gravação
fosse interrompida; o terminal do Studio que, reiniciado, misturava o
processo antigo com o novo; o Ctrl+Z do campo de busca, que desfazia a
digitação do editor. `0.10.0` vira `0.10.1`.

### 2.2 Como decidir

Leia a seção "Não publicada" do CHANGELOG (seção 5) item por item e faça a
cada um estas perguntas, na ordem. A primeira resposta "sim" diz a posição
do item.

1. Depois da mudança, algo que funcionava na versão anterior para de
   funcionar sem que a pessoa mude nada do lado dela? Quebra: `X.0.0`
   (antes da 1.0, `0.X.0`; seção 2.4).
2. A pessoa ganha algo que não existia, ou algo passa a funcionar de outro
   jeito por padrão? Novidade: `0.X.0`.
3. A mudança só faz a versão publicada fazer o que já devia? Correção:
   `0.0.X`.

Uma mudança que não responde "sim" a nenhuma das três, como documentação,
teste, CI ou código interno sem efeito para quem usa, não conta para o
número. A release recebe a posição mais alta entre os itens: uma correção
e uma novidade juntas fazem uma versão menor. Na dúvida entre duas
posições, vale a mais alta.

| Caso | Posição |
|---|---|
| Um conserto que obriga quem usa a mudar algo do lado dele, como um campo do `--json` com nome errado que passa a ter o certo | `X.0.0`: é quebra, mesmo sendo conserto |
| Um bundle novo, com outra versão de alguma ferramenta | `0.X.0`, mesmo sem quebra: outra versão do Icarus ou do Yosys pode dar outro resultado na mesma simulação ou síntese |
| Um pacote do bundle refeito para consertar um defeito, sem mudar o que a ferramenta produz | `0.0.X` |
| Uma dependência atualizada (um crate, um pacote do npm, o Tauri) | `0.0.X` se conserta um defeito ou uma falha de segurança; senão, espera a próxima `0.X.0` |
| Ficar mais rápido, com o mesmo resultado | `0.X.0`; `0.0.X` se a lentidão era um defeito, como um prazo que estourava |
| Uma frase errada numa mensagem, na ajuda ou na tela | `0.0.X`; uma língua nova é `0.X.0` |
| Um recurso experimental (`aurora experimental`; hoje `fpga`, `assistant` e `git`) novo ou mudado | `0.X.0`; mudar ou tirar um recurso experimental nunca conta como quebra |
| Um recurso que deixa de ser experimental | `0.X.0` |
| Um comando ou método marcado como obsoleto, com o substituto | `0.X.0` |
| Tirar o que estava obsoleto | `X.0.0` |

Antes da 1.0, onde a tabela diz `X.0.0`, sobe o do meio (seção 2.4).

### 2.3 O que conta como quebra

Os contratos da AURORA, e o que quebra cada um:

| Contrato | Onde está | Quebra |
|---|---|---|
| Linha de comando | `crates/aurora-cli/src/main.rs`, [CLI.md](CLI.md) | tirar ou renomear um comando ou uma flag; mudar o significado de um código de saída (0, 1, 2 e 130) |
| `--json` e `--events` | `docs/schema/` | tirar ou renomear um campo, mudar o tipo ou o significado dele ([CONTRIBUTING.md](../CONTRIBUTING.md), "Contrato do `--json`") |
| Protocolo do `aurora serve` | `crates/aurora-protocol/src/lib.rs` | a tabela de [METODOLOGIA.md](METODOLOGIA.md), seção 5 |
| Códigos de erro | `AuroraError::code()`, em `crates/aurora-core/src/error.rs` | mudar um código publicado, o que nunca se faz |
| Projeto | o `.spf` (o mesmo da AURORA), o `fpga.json`, a pasta `wave/`, a pasta `.aurora8/` | um projeto que abria deixa de abrir, ou passa a precisar de conversão, como a da estrutura do YANC 6.x para a do 7.1 |
| Instalação e atualização | os nomes dos arquivos da release, o `index.json` e o `files.json` de cada plataforma, o recibo `install.json`, a pasta da instalação | o `aurora update` de uma versão publicada deixa de conseguir atualizar |
| Bundle | `bundle/versions.json`; a versão mínima do YANC, em `crates/aurora-core/src/yanc.rs` | tirar uma ferramenta; subir a versão mínima do YANC |
| Sistemas | Ubuntu 22.04 (glibc 2.35), macOS 13 em Apple Silicon e Windows 10, no `installers.yml` e no `installer/windows/aurora.iss` | deixar de atender um deles |
| Preferências | o `settings.json` do Studio, o `experimental.json` | a versão nova perder o que a anterior gravou |

O texto que a AURORA mostra para pessoas, como mensagens, ajuda e a cara do
Studio, fica fora dos contratos: mudar uma frase não quebra nada.

O contrato mais frágil é o da atualização. Cada `aurora update` já publicado
lê a release nova com o código da versão dele. Um arquivo renomeado ou um
campo a menos no `index.json` quebra a atualização de todo mundo que tem
aquela versão, e essas pessoas só saem disso reinstalando. Os nomes do
instalador (`aurora-<versão>-<plataforma>.tar.gz` e
`aurora-<versão>-windows-x64-setup.exe`) são a reserva de todas as versões e
nunca mudam. Uma mudança no resto mantém o caminho antigo funcionando e é
conferida atualizando a partir da versão anterior (seção 6.4).

Quem tem o Lace (0.4 a 0.9) chega à AURORA pelo mesmo contrato, com os
nomes de antes. O `lace update` procura o índice
`lace-<versão>-<plataforma>-index.json`, não acha, e cai para o instalador
inteiro, `lace-<versão>-<plataforma>.tar.gz` ou
`lace-<versão>-windows-x64-setup.exe`. O `release.yml` publica os
instaladores da AURORA também com esses nomes, em toda release, e o
`install` e o assistente da AURORA reconhecem o Lace e o trocam pela AURORA
([crates/aurora-installer/src/lace.rs](../crates/aurora-installer/src/lace.rs)).
Esses nomes também não mudam, e a release nunca publica um índice com o
nome antigo, que levaria o `lace update` à atualização por componentes,
dentro da pasta do Lace.

### 2.4 Antes da 1.0

Enquanto a versão começar com 0, o primeiro número não sobe. O que pediria
`X.0.0` sobe o do meio, com a quebra no alto do CHANGELOG (seção 5): de
`0.9.3` para `0.10.0`. Novidade continua subindo o do meio, e correção, o
último. O Cargo e o npm leem uma versão 0 do mesmo jeito: para eles,
`0.9.x` e `0.10.x` são incompatíveis, e `0.9.1` e `0.9.2` não.

Os números não têm teto: depois da `0.9.x` vem a `0.10.0`, depois a
`0.11.0`. A 1.0 só sai pelos critérios da seção 2.6.

Depois da 1.0, uma quebra sobe o primeiro número, e só depois do período
de obsoleto de [METODOLOGIA.md](METODOLOGIA.md), seção 4: o que vai sair
continua funcionando por pelo menos uma versão menor inteira, com o
substituto no CHANGELOG.

### 2.5 Quando o número é escolhido

Na hora de lançar, pela seção "Não publicada" do CHANGELOG. Até lá, os
lugares da seção 6.1 continuam com a versão da última release. A
renomeação para AURORA já tem os números: API 1.0.0 e IDE 8.0.0. Escolher o número
no começo do trabalho obriga a adivinhar o que a release vai levar, e uma
correção que precise sair antes fica com um número de novidade.

Quem escreve uma novidade já sabe que ela sai na próxima versão menor e usa
esse número onde precisar dele, como na tabela `FEATURES` da extensão
(`editors/vscode/src/aurora/version.ts`).

### 2.6 A 1.0

A 1.0 promete que os contratos da seção 2.3 só quebram numa versão maior.
A data dela, e de toda versão maior depois, é decisão da coordenação do
projeto. Os critérios mínimos:

1. Os testes do Rust e o typecheck do TypeScript rodam na CI a cada push,
   nos três sistemas ([METODOLOGIA.md](METODOLOGIA.md), seção 8, fase 1).
2. Os métodos estáveis do protocolo estão marcados
   ([METODOLOGIA.md](METODOLOGIA.md), seção 4).
3. As duas últimas versões menores saíram sem quebra fora dos recursos
   experimentais.
4. Na última release, a atualização a partir da versão anterior foi
   conferida nos três sistemas (seção 6.4).

Um recurso experimental não segura a 1.0: ele continua experimental depois
dela.

### 2.7 A extensão do VS Code

A extensão tem versão própria porque o VS Code só reconhece uma
atualização quando o número do `.vsix` muda. As posições se decidem como
nas seções 2.1 e 2.2, pelo que a extensão faz. O número sobe sempre que
muda algo que vai dentro do `.vsix`: o que o `.vscodeignore` não tira de
`editors/vscode/` (o `src/` entra compilado, em `dist/`, com os tipos
gerados) e o RTL viewer de `packages/rtl-viewer/`:

```bash
git diff --stat v0.9.0 -- editors/vscode packages/rtl-viewer
```

A release da AURORA leva o `.vsix` mesmo quando a extensão não mudou: o
README aponta para ele dentro da release de cada versão
(`releases/download/v<versão>/aurora-vscode-<versão da extensão>.vsix`).

## 3. Quando lançar

Uma release fica para sempre. Os arquivos dela não se apagam (seção 7); o
`aurora` instalado avisa da versão nova uma vez por dia até a pessoa
atualizar (`crates/aurora-cli/src/update_check.rs`); e cada versão publicada
é mais uma que o `aurora update` das seguintes precisa atender. Por isso cada
release junta o trabalho de vários assuntos.

**Versão menor.** Sai quando a `main` tem pelo menos um recurso terminado,
com teste, documentação e linha no CHANGELOG. Recurso pela metade não
segura nem apressa a release: ele fica na branch dele ou atrás de um
recurso experimental. Entre duas versões menores passa pelo menos uma
semana; o ritmo normal é uma a cada duas a quatro semanas.

**Correção.** Sai quando a correção de um defeito da última release chegou
na `main` e o defeito atrapalha quem usa:

- no mesmo dia ou no seguinte, quando o defeito impede instalar, atualizar
  ou abrir a AURORA ou o Studio; apaga ou sobrescreve arquivo do usuário; dá
  resultado errado sem avisar (build, simulação, síntese, a correção do
  `aurora learn`); ou é uma falha de segurança;
- nos outros casos, junto com outras correções, em até duas semanas. Se a
  próxima versão menor está perto, a correção vai nela.

**Sem release.** Mudança só de documentação, de teste, de CI ou de código
interno sem efeito para quem usa não pede release: ela sai na próxima. O
`install.sh` e o `install.ps1` também não pedem: eles são servidos da
`main` e valem assim que chegam nela, para a release publicada no momento.
Por isso uma mudança neles é testada contra essa release.

**Quem lança.** Cada release tem um responsável, que escolhe o número, faz
a seção 6 inteira e acompanha os dias seguintes. Não lance quando ninguém
vai poder responder a um defeito nos dois dias seguintes.

## 4. Branches e tags

```
main             recebe as branches feat/, fix/ e docs/ (METODOLOGIA.md, seção 7)
release/0.10     criada da main para a 0.10.0; só o commit da release e correções
  v0.10.0        tag no commit da release
  v0.10.1        tag numa correção levada para a release/0.10
```

- **`release/X.Y`.** Uma por versão menor, criada da `main` quando a
  `X.Y.0` vai sair. Recebe o commit da release (seção 6.1) e, depois, as
  correções das versões `X.Y.Z`. Depois de cada tag, ela volta para a
  `main` (seção 6.3), levando a versão nova, o CHANGELOG e os comandos do
  README.
- **Por que fora da `main`.** Enquanto a release não está publicada, a
  `main` continua com a versão que está no ar, e os comandos do
  `README.md` e do `docs/INSTALL.md` apontam para arquivos que existem. E
  uma versão de correção sai sem levar o que a `main` já recebeu para a
  próxima versão menor.
- **Uma correção.** Vai primeiro para a `main`, pela branch `fix/` de
  sempre, e depois para a `release/X.Y` com `git cherry-pick -x <commit>`.
  Se a `main` mudou tanto que o commit não se aplica, a correção é feita na
  `release/X.Y` e chega na `main` pelo merge depois da tag.
- **Só a linha mais nova recebe correção.** Depois da `0.10.0`, a
  `release/0.9` não recebe mais nada, e quem está na 0.9 atualiza para a
  0.10. O `release.yml` deixa o GitHub escolher sozinho qual é a última
  release, e uma `0.9.x` publicada depois da `0.10.0` não pode correr o
  risco de virar a última que o `install.sh`, o `install.ps1` e o
  `aurora update` procuram. Corrigir uma linha antiga exige antes o
  `--latest=false` no `gh release create` do `release.yml`.
- **Tags.** `vX.Y.Z`, anotada, no commit da release:
  `git tag -a v0.10.0 -m "AURORA 0.10.0"`. Depois que a release foi
  publicada, a tag nunca se move nem se apaga.
- **As branches antigas.** Até a 0.9.0, as releases saíram de branches
  `lace-X.Y.Z`, da `main` e da `feat/core-api`. Elas ficam como estão; da
  0.10.0 em diante vale a `release/X.Y`.

## 5. CHANGELOG

- **Dois arquivos.** O `CHANGELOG.md` é da AURORA inteira, e é dele que o
  `release.yml` tira as notas da release. O `studio/CHANGELOG.md` tem os
  detalhes do Studio, no formato do Keep a Changelog. A extensão do VS Code
  não tem CHANGELOG próprio: as mudanças dela vão no `CHANGELOG.md`, com a
  versão nova da extensão.
- **Durante o desenvolvimento**, o alto de cada arquivo tem a seção sem
  número: `## Não publicada` no `CHANGELOG.md` e `## [Não publicada]` no
  `studio/CHANGELOG.md`. Toda mudança que o usuário percebe ganha uma linha
  ali, no mesmo pull request.
- **Na release**, a seção ganha o número e a data: `## 0.10.0 (2026-11-03)`
  e `## [0.10.0] - 2026-11-03`. O `release.yml` procura no `CHANGELOG.md`
  a linha que começa com `## <versão> ` e recusa a tag sem ela.
- **Uma quebra** abre a seção da versão, numa subseção
  `### Quebras de compatibilidade`, uma por item, com o que quebra e o que
  a pessoa faz. É a primeira coisa das notas da release.
- **Um recurso experimental** é dito experimental na linha dele.
- **Numa correção**, cada linha diz o que estava errado e desde que versão.

## 6. Passo a passo

O exemplo é a `0.10.0`, depois da `0.9.0`.

### 6.1 Preparar

1. **O número.** Leia as seções "Não publicada" dos dois CHANGELOGs e
   confira com o que entrou desde a última tag:

   ```bash
   git log --oneline v0.9.0..main
   git diff --stat v0.9.0..main -- docs/schema
   ```

   O diff de `docs/schema/` é o portão do contrato
   ([METODOLOGIA.md](METODOLOGIA.md), seção 5). Com a seção 2, ele decide o
   número.
2. **A branch.** Versão menor: `git switch -c release/0.10 main`.
   Correção: `git switch release/0.10` e um `git cherry-pick -x` por
   correção.
3. **O bundle.** O identificador do bundle não muda na release: ele muda
   junto com a ferramenta, na mudança que a troca. Confira só os
   pré-requisitos. O pacote `msys` de `bundle/versions.json` aponta para
   uma release publicada do lace-toolchain, com os dois SHA-256 preenchidos
   (do `SHA256SUMS` dela). Se o commit do surfer-aurora mudou, rode antes o
   `gh workflow run surfer-aurora.yml` e ponha as URLs e os SHA-256 do
   resumo no `prebuilt` dele, e o cliente web da mesma versão no `web`. O
   `bundle.py` usa o `prebuilt` que estiver lá, sem conferir o commit: com o
   antigo, a release leva o surfer-aurora antigo. Só sem `prebuilt` ela
   compila o surfer-aurora, uns 14 minutos a mais por plataforma.
4. **O commit da release**, com a mensagem `AURORA 1.1.0` e nenhuma outra
   mudança dentro:
   - a versão da AURORA API (a da tag) no `Cargo.toml`
     (`[workspace.package] version`) e nos pacotes `aurora-learn` e
     `rtl-viewer` de `bundle/versions.json`;
   - a versão da AURORA IDE, própria (8.0.0 na primeira release com o nome
     novo, em que a API é a 1.0.0), em cinco lugares iguais: o
     `studio/package.json` e o `studio/package-lock.json` (o
     `npm version 8.0.0 --no-git-tag-version` em `studio/` troca os dois), o
     `studio/src-tauri/Cargo.toml`, o `studio/src-tauri/tauri.conf.json` e o
     pacote `studio` de `bundle/versions.json`. O `bundle.py` recusa a IDE
     com versões diferentes, e a IDE só sobe de versão quando muda;
   - os dois `Cargo.lock`, com `cargo update --workspace` na raiz e em
     `studio/src-tauri/`;
   - as seções "Não publicada" dos dois CHANGELOGs, com o número e a data
     (seção 5);
   - a versão nos comandos do `README.md` e do `docs/INSTALL.md` que
     baixam um arquivo da release direto, o `.vsix` incluído. O
     `install.sh` e o `install.ps1` não mudam: eles acham a última release
     sozinhos;
   - se a extensão mudou (seção 2.7), a versão nova dela, com
     `npm version <versão> --no-git-tag-version` em `editors/vscode/`, e o
     nome do `.vsix` no `README.md` e no `editors/vscode/README.md`.

### 6.2 Conferir

Na `release/0.10`, com o commit da release. Fora a extensão e o RTL
viewer, que o `vscode.yml` confere a cada push que os muda, nada disto roda
sozinho no GitHub hoje: o `release.yml` só monta os instaladores, sem teste
(seção 11).

- As verificações de "Antes de mandar a mudança", do
  [CONTRIBUTING.md](../CONTRIBUTING.md), com o `cargo test --workspace`
  rodando com `AURORA_TEST_BUNDLE` definido, num bundle com o identificador
  de `bundle/versions.json`.
- O Studio: `npm run build` em `studio/`; `cargo clippy` e `cargo test` em
  `studio/src-tauri/`, com o bundle; e o roteiro de fumaça
  ([studio/docs/DEVELOPMENT.md](../studio/docs/DEVELOPMENT.md), seção 3).
  No Windows, o binário de testes do backend pode não abrir
  (STATUS_ENTRYPOINT_NOT_FOUND), porque o `tauri-build` só liga o manifesto
  do Common Controls 6 nos executáveis do aplicativo (seção 11).
- A extensão: `npm run typecheck`, `npm run lint` e `npm run test:unit` em
  `editors/vscode/`, e o `vscode.yml` verde no último push que a mudou.
- O RTL viewer: `npm run typecheck` em `packages/rtl-viewer/`.
- A passagem do Lace (seção 2.3): o `lace-ponte.yml` verde no push da
  `release/X.Y`, que roda o `lace update` de um Lace 0.9.0 contra um espelho
  com o `install` do commit, no Linux e no macOS. No Windows, o assistente
  compilado com `/DAppGuid=` e `/DLaceGuid=` de teste, rodado sobre um Lace
  de teste com esse AppId: o Lace sai, e os aplicativos dele vêm marcados.

Cada sistema que o responsável tem roda a lista inteira. O que ficou sem
rodar, como um sistema que ninguém tinha, vai escrito nas notas da release
(seção 6.4).

### 6.3 Publicar

```bash
git push -u origin release/0.10
git tag -a v0.10.0 -m "AURORA 0.10.0"
git push origin v0.10.0
```

1. O `release.yml` confere que a tag é a versão do `Cargo.toml` e do
   Studio e que o CHANGELOG tem a seção. Depois chama o `installers.yml`
   nas três plataformas, que monta o bundle e os instaladores, e publica a
   release com os instaladores (o `.tar.gz` do Linux e o do macOS; o
   completo e o web do Windows), os pedaços do bundle, o `SHA256SUMS` e as
   notas do CHANGELOG. Acompanhe em Actions > release, ou com
   `gh run watch`. Os instaladores de cada execução também ficam como
   artefatos do workflow por 14 dias.
2. A extensão vai anexada à mão, a partir do commit da tag. O
   `npm run package` monta antes o RTL viewer, por isso o `npm ci` dele vem
   primeiro:

   ```bash
   (cd packages/rtl-viewer && npm ci)
   (cd editors/vscode && npm ci && npm run package)
   cp editors/vscode/aurora-<versão da extensão>.vsix aurora-vscode-<versão da extensão>.vsix
   gh release upload v0.10.0 aurora-vscode-<versão da extensão>.vsix
   ```

3. A `release/0.10` volta para a `main` por pull request, com merge, sem
   squash nem rebase: o commit da tag precisa ficar na história da `main`,
   para o `git log v0.10.0..main` da próxima release funcionar.

### 6.4 Depois de publicar

No mesmo dia, o responsável confere:

- **Os arquivos.** `gh release view v0.10.0` lista, para cada plataforma,
  o instalador (o `.tar.gz` no Linux e no macOS, o `-setup.exe` e o
  `-web-setup.exe` no Windows), o `-index.json`, o `-files.json`, o
  `-aurora.tar.zst` e os pedaços `-cNN.tar.zst`, e mais o `.vsix`, o
  `SHA256SUMS` e os instaladores com o nome do Lace
  (`lace-<versão>-linux-x64.tar.gz`, `lace-<versão>-darwin-arm64.tar.gz` e
  `lace-<versão>-windows-x64-setup.exe`, seção 2.3).
- **A instalação nova**, pelo comando de um passo do README:
  `aurora --version` diz a versão nova, `aurora tools --verify` passa, o Studio
  abre, e um exemplo de `examples/` compila e simula.
- **A atualização**, numa instalação da versão anterior: `aurora update` acha
  a versão nova e atualiza; depois, a mesma conferência da instalação nova.
  É ela que protege o contrato da atualização (seção 2.3).
- **A passagem do Lace**, numa instalação do Lace 0.9.0: `lace update`
  instala a AURORA com os mesmos aplicativos e tira o Lace; depois, a mesma
  conferência da instalação nova.
- **A extensão**: o `.vsix` da release instala com
  `code --install-extension`, e o VS Code mostra a versão nova.

O que foi conferido, e em que sistema, vai no fim das notas da release:

```bash
gh release view v0.10.0 --json body --jq .body > notas.md
gh release edit v0.10.0 --notes-file notas.md
```

Entre os dois comandos, acrescente ao fim de `notas.md` o que foi
conferido. Nos dias seguintes, um defeito sério segue a seção 7.

## 7. Quando uma release sai com defeito

**Antes de publicar.** Se o `release.yml` falhou no `check` ou nos
instaladores, nada chegou a ninguém. Apague a tag, corrija na
`release/X.Y` e crie a tag de novo:

```bash
git push origin :refs/tags/v0.10.0
git tag -d v0.10.0
```

Se sobrou uma release em rascunho, apague-a também
(`gh release delete v0.10.0`).

**Depois de publicar.** A release fica como está. Os arquivos dela não se
apagam nem se renomeiam: o `aurora install` de cada versão baixa os pedaços
da própria versão, conferidos pelo `SHA256SUMS` dela, e o `aurora update`
baixa da versão nova o índice, o manifesto e os pedaços que mudaram e,
como reserva, o instalador. Sem esses arquivos, quem tem aquela versão
instalada não consegue mais instalar aplicativos nem atualizar.

O conserto é uma versão de correção, no prazo da seção 3. Enquanto ela não
sai:

- o defeito e o jeito de contorná-lo vão no alto das notas da release
  (`gh release edit`);
- se o defeito impede instalar ou atualizar, a release anterior volta a ser
  a última, com `gh release edit v0.9.0 --latest`. O `install.sh`, o
  `install.ps1` e o `aurora update` passam a achar a anterior, e quem já
  atualizou fica onde está, porque o `aurora update` só troca por versão mais
  nova. A versão de correção, quando sai, vira a última;
- se o defeito é no próprio `aurora update` da versão nova, quem a instalou
  só sai dela pelo instalador ou pelo comando de um passo, e as notas da
  correção dizem isso.

## 8. Montar localmente

Linux ou macOS (o Studio pede o Node.js e, no Linux, o
`libwebkit2gtk-4.1-dev`; `--only` sem `studio` monta sem ele):

```bash
python3 scripts/bundle.py --out dist/toolchain
cargo build --release -p aurora-cli -p aurora-installer
target/release/aurora-pack tui --toolchain dist/toolchain \
  --aurora target/release/aurora --installer target/release/aurora-installer \
  --out dist/aurora-<versão>-linux-x64 --tar-gz
```

Windows: o bundle é montado no shell MINGW64 do MSYS2, que compila o YANC,
e o resto em PowerShell. O pacote msys de `bundle/versions.json` precisa
apontar para uma release publicada do lace-toolchain, com os dois hashes
([BUNDLE.md](BUNDLE.md), seção 6). `<bundle>` é o campo `bundle` de
`bundle/versions.json`.

```bash
python scripts/bundle.py --out dist/toolchain
```

```powershell
cargo build --release -p aurora-cli -p aurora-installer
target\release\aurora-pack.exe tui --toolchain dist\toolchain --aurora target\release\aurora.exe --installer target\release\aurora-installer.exe --out dist\tui --assets dist\apps
target\release\aurora-pack.exe inno --toolchain dist\toolchain --aurora target\release\aurora.exe --out dist\inno --web dist\tui
& "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe" /DStage=$PWD\dist\inno /DAuroraVersion=<versão> /DBundle=<bundle> /O$PWD\dist installer\windows\aurora.iss
& "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe" /DWeb /DStage=$PWD\dist\inno /DAuroraVersion=<versão> /DBundle=<bundle> /O$PWD\dist installer\windows\aurora.iss
```

O segundo `ISCC` (`/DWeb`) monta o assistente web,
`aurora-<versão>-windows-x64-web-setup.exe`: ele leva só o `aurora.exe` e baixa
da release os pedaços que o `aurora-pack tui --assets` gravou, conferidos
pelo SHA-256 que o `aurora-pack inno --web` pôs no `web.iss`. Os pedaços
publicados precisam ser exatamente os dessa montagem. Para testar sem
publicar, sirva a pasta `dist\apps` como `<espelho>/v<versão>/` e rode o
assistente com `/MIRROR=<espelho>`; com `/DAppGuid=<outro GUID>` no `ISCC`,
a instalação de teste não mexe na entrada da AURORA de verdade na lista de
programas.

O `aurora-pack inno` roda em qualquer sistema; só o `ISCC` precisa do Windows
(ou do Wine).

## 9. Como os instaladores são feitos

- `scripts/bundle.py` monta o bundle e grava, ao lado da pasta dele, o
  `<pasta>.contents.json` (`dist/toolchain.contents.json`): que
  arquivo é de que componente (`bundle/components.json` diz o que cada
  componente leva).
- `aurora-pack` (crate `aurora-installer`) agrupa os arquivos pelo conjunto
  exato de componentes que os usa. Cada grupo vira um pedaço `.tar.zst` do
  payload da TUI, ou uma entrada `[Files]` do Inno Setup com
  `Components: a or b`.
- A TUI (`install`) e o assistente só extraem os grupos que a seleção usa.
- Os perfis de instalação vêm de `profiles` em `bundle/components.json`: o
  `bundle.py` confere cada um (componentes que existem, com o que cada um
  exige) e os grava no `contents.json`; o `aurora-pack` os põe no
  `index.json` da TUI e gera, no `components.iss`, um `[Types]` por perfil,
  o Custom (`avancada`) e a função `ProfileDetail`, com o texto de cada um.
- A cara do assistente: o ícone da AURORA IDE e as imagens de
  `installer/windows/art/`, geradas da marca (`studio/public/brand`) por
  `python scripts/installer_art.py`, só com a biblioteca padrão. Mudou a
  marca, ou o `WizardSizePercent` do `aurora.iss`, gere de novo (a proporção
  da faixa da esquerda segue o `SIZE_PERCENT` do script).

Além dos instaladores, a release publica o bundle em pedaços, para o
`aurora install` baixar só os aplicativos que faltam e o `aurora update` só o
que mudou: para cada plataforma, o índice
(`aurora-<versão>-<plataforma>-index.json`), o manifesto de arquivos
(`aurora-<versão>-<plataforma>-files.json`, o SHA-256 de cada arquivo por
pedaço) e cada pedaço (`aurora-<versão>-<plataforma>-c03.tar.zst`), que o
`aurora-pack tui --assets` grava e o `installers.yml` sobe com os
instaladores. O mesmo manifesto vai dentro do pedaço sempre instalado e no
estágio do Inno Setup: cada instalação fica com o da versão dela, em
`toolchain/files.json`.

| Ferramenta | Versão fixada | Onde |
|---|---|---|
| Inno Setup | 6.7.3 | `INNO_VERSION` no `installers.yml`; instalador em `github.com/jrsoftware/issrc/releases`, SHA-256 `9c73c3bae7ed48d44112a0f48e66742c00090bdb5bef71d9d3c056c66e97b732` |
| zstd (pedaços da TUI) | crate `zstd` 0.14 | `Cargo.toml` |

## 10. Testar sem Windows

O instalador de Windows compila e instala no Wine. No Wine 11, o `dot.exe`
do Graphviz 16.1.0 trava quando o ambiente não tem `LANG`, e a AURORA roda as
ferramentas com ambiente limpo: o esquemático não sai no Wine. Num Windows
de verdade o `LANG` normalmente nem existe. O `installers.yml` não roda
teste: quem confirma que o esquemático funciona lá é o
`synthesis_and_schematic_of_processor_and_top_level`
(`crates/aurora-core/tests/tools.rs`), rodado à mão num Windows com o bundle.

## 11. O que falta

- **Assinar os instaladores.** O de Windows (Authenticode) e o de macOS,
  assinado e notarizado. Sem isso, o SmartScreen e o Gatekeeper avisam na
  primeira execução (ver [INSTALL.md](INSTALL.md)).
- **Os testes na release.** O `release.yml` monta e publica sem rodar
  teste, e a seção 6.2 é feita à mão. Com a fase 1 de
  [METODOLOGIA.md](METODOLOGIA.md), seção 8, os testes passam a rodar na
  CI, e o `release.yml` deve exigi-los antes dos instaladores.
- **Os testes do backend do Studio no Windows.** O binário de testes pode
  não abrir (STATUS_ENTRYPOINT_NOT_FOUND, visto com o toolchain GNU do
  Rust) porque não tem o manifesto do Common Controls 6. O contorno usado
  até aqui: copiar o `.exe` de testes para outro nome, pôr ao lado um
  `<nome>.exe.manifest` que pede o `Microsoft.Windows.Common-Controls`
  6.0.0.0, pôr a pasta `target/debug` (a do `WebView2Loader.dll`) no
  `PATH` e rodar o `.exe` direto.
- **Um script de versão**, que troque a versão em todos os lugares do passo
  4 da seção 6.1 e confira que estão iguais, no lugar da lista à mão.
- **Instaladores antes da tag.** O `installers.yml` só roda chamado pelo
  `release.yml`. Com um `workflow_dispatch`, a `release/X.Y` montaria os
  instaladores antes da tag, e a conferência da seção 6.4 aconteceria antes
  de publicar.
- **O `.vsix` montado pelo workflow**, junto com os outros arquivos.
  Anexado à mão, ele depende de a release aceitar arquivo depois de
  publicada, o que deixa de valer se a imutabilidade de releases do GitHub
  for ligada no repositório.
- **A data do CHANGELOG conferida.** O `release.yml` aceita qualquer linha
  `## <versão> `, até uma `## 0.10.0 (não publicada)`.
- **Pré-releases.** Uma tag `v0.10.0-rc.1` não deve ser criada antes de
  três mudanças: o `is_newer` de `crates/aurora-installer/src/release.rs`
  compara os grupos de dígitos e põe a `0.10.0-rc.1` depois da `0.10.0`; o
  `VersionInfoVersion` de `installer/windows/aurora.iss` só aceita números; e
  o `release.yml` publica toda tag como release comum, sem `--prerelease`.
