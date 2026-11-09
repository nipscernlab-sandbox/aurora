# AURORA para VS Code

A extensão do VS Code para projetos do [AURORA](../../README.md): as fontes
do projeto, o topo do design, o fluxo SAPHO (C± e assembly com realce, o
build dos processadores, criar e configurar), a verificação, a simulação, a
onda, a síntese e os arquivos do projeto (pôr, tirar, ordenar, renomear).
Realce e lint de Verilog ficam com as extensões que já existem.

A extensão tem versão própria, separada da da AURORA, e ainda não está no
Marketplace: o `aurora-vscode-<versão>.vsix` vai anexado à release da AURORA, e
se instala com `code --install-extension aurora-vscode-<versão>.vsix` ou, no
VS Code, em Extensões > `...` > **Install from VSIX...** (o passo a passo
está no [README da AURORA](../../README.md#extensão-do-vs-code)).

## O que faz

- Ativa num workspace com um `.spf`. Com mais de um, o projeto ativo se
  escolhe pela barra de status ou por **AURORA: Escolher o projeto ativo**.
- Numa pasta sem `.spf` (ou sem pasta aberta), a vista da AURORA mostra o
  botão **Novo projeto** (`aurora new`): na própria pasta aberta, numa pasta
  nova dentro dela ou em outro lugar. Dentro do workspace, o projeto criado
  vira o ativo; fora, a extensão oferece abri-lo.
- A vista **Fontes**, no ícone da AURORA da barra de atividades, mostra o
  `aurora status --json` do projeto ativo: os avisos do `.spf`, o RTL (na
  ordem do `.spf`, que é a de compilação), os Testbenches, os Processadores
  e os Não registrados, cada grupo com a contagem. O topo, o testbench
  simulado e cada processador aparecem com o nome colorido, um selo (T, S,
  C) e a descrição em maiúsculas (TOPO, SIMULADO, COMPILADO, NÃO
  COMPILADO); um arquivo com erro no último check, ou um processador com
  erro no último build, ganha um selo vermelho com o número de erros. A vista se atualiza depois de cada comando, quando o
  `.spf` muda no disco e quando um `.v` ou `.sv` é criado ou apagado.
- **Definir como topo** (`aurora top`), no botão direito do explorador, do
  editor, da aba e da vista. Some no topo atual, num testbench registrado e
  num arquivo com nome de testbench (`tb.v`, `tb_*.v`, `*_tb.v`).
- **Build, Check, Sim, Wave e Synth** ficam na barra de status, ao lado do
  projeto ativo, e valem para o projeto inteiro (o Build só aparece num
  projeto com processador). Cada botão gira enquanto roda (o
  clique cancela, menos no Wave: com o `aurora serve`, pelo `$/cancel`; com
  `aurora.transport` em `cli`, com SIGTERM ao aurora) e, depois de uma falha, fica com o
  fundo de erro e o número de erros. A saída das ferramentas sai ao vivo no
  canal **AURORA: ferramentas**. As variantes estão na paleta de comandos e no
  botão direito da vista e do explorador.
- **Build** (`aurora build`): compila os processadores SAPHO com o YANC, todos
  ou um (botão direito no processador). Os erros do compilador vão para o
  painel Problemas, na linha do fonte `.cmm`; os do build que o sim e o
  synth fazem antes, também. O processador passa a COMPILADO na vista, e o
  AURORA põe no projeto o `.v` e o testbench que ele gerou (no RTL e nos
  Testbenches); a saída diz o que entrou.
- **Novo processador** (`aurora proc add`), no + do grupo Processadores e no
  menu da vista: o nome e a linguagem (C± ou C); o fonte-modelo abre no
  editor. **Configurar o processador** (`aurora proc set`): a frequência, os
  clocks simulados e a exportação de arrays, que valem no próximo build.
- **C± e o assembly do SAPHO** têm realce, comentários, pares de
  delimitadores e snippets das diretivas e das funções (`.cmm`, e o `.asm`
  que o YANC 7.1 gera em `<processador>/<processador>/`). As listas são as do Studio, conferidas contra o YANC; o
  `i` só é a unidade imaginária dentro de um literal complexo (`2+3i`).
- **Check** (`aurora check`): o projeto, um arquivo, um processador, ou com o
  lint do Verilator. Os erros e avisos vão para o painel Problemas, só os
  dos arquivos do projeto (os da biblioteca SAPHO do bundle ficam na
  contagem da saída).
- **Sim** (`aurora sim`): o testbench do projeto; pelo botão direito, outro
  testbench (que passa a ser o do projeto) ou um processador; pela paleta, a
  simulação rápida (Verilator, sem onda). O que o testbench escreve aparece
  depois de "|", e no fim as saídas das portas do processador e os testes
  cocotb. A notificação do fim tem o botão "Abrir a onda".
- **Wave** (`aurora wave`): a onda do testbench simulado, a de um processador
  (botão direito), ou um `.vcd`, `.fst` ou `.ghw` pelo explorador. O
  surfer-aurora continua aberto depois.
- **Synth** (`aurora synth`): o topo ou um processador no Yosys, com as
  estatísticas na saída, e o **RTL viewer** numa aba: o esquemático
  interativo da AURORA (o mesmo do Studio, de `packages/rtl-viewer`), com as
  cores do tema do editor. Clicar duas vezes numa célula abre o fonte ao
  lado. **AURORA: Abrir o RTL viewer** mostra a última síntese sem sintetizar
  de novo. Um `aurora synth --open` no terminal integrado abre a mesma aba, e
  não o navegador: a extensão define `AURORA_OPENER=vscode` nos terminais
  dela. Com `aurora.synth.schematic`, também o SVG do Graphviz.
- **Pôr no projeto** (`aurora add`): o + nos não registrados (e no grupo, para
  todos), o botão direito no explorador, **Adicionar como testbench**, e
  **Novo arquivo Verilog**, que pergunta o caminho e o tipo e deixa a AURORA
  criar o arquivo do modelo. Quando aparece um `.v` ou `.sv` novo na pasta,
  fora do projeto, um aviso oferece adicioná-lo; num arquivo vazio, com o
  modelo. Arrastar arquivos do explorador para a vista também os põe no
  projeto (nos Testbenches, como testbench).
- **Tirar do projeto** (`aurora remove`), na vista e no explorador: o arquivo
  continua no disco.
- **A ordem de compilação** (`aurora order`): **Subir**, **Descer**,
  **Primeiro** e **Último** no botão direito do RTL e dos Testbenches, ou
  arrastando um arquivo sobre outro da mesma lista (sobre o grupo, vai para
  o começo). Os compiladores leem nessa ordem: um `` `define `` só vale para
  os arquivos de baixo.
- **Renomear ou mover** (`aurora move`), na vista: o caminho a partir da pasta
  do projeto. O arquivo continua no projeto, com o topo e o testbench
  simulado, e as abas abertas acompanham.
- **O explorador**: um arquivo do projeto (ou uma pasta com arquivos dele)
  renomeado ou movido no explorador continua no projeto, no lugar novo
  (`aurora move --after`, de uma AURORA mais nova que a 0.6.0; com um mais
  antigo, a extensão avisa que o `.spf` ficou com o nome velho). Um apagado
  sai do projeto (`aurora remove`). Renomear a pasta ou o fonte de um
  processador quebra o projeto, e a extensão avisa para desfazer.
  Desligado com `aurora.files.followExplorer`.

## Requisitos

A AURORA 0.2.0 ou mais novo; check, sim e synth precisam da 0.3.0, e a
simulação rápida, da 0.4.0. Ao ativar, a extensão confere a versão
(`aurora --version`) e desliga o que a instalada não suporta
(`src/aurora/version.ts`). Ela não roda em workspace sem confiança.

| Configuração | Padrão | Efeito |
|---|---|---|
| `aurora.path` | vazio | o executável `aurora`; vazio, o primeiro do `PATH`. Vale só nas configurações do usuário: um workspace não escolhe o que a extensão executa |
| `aurora.transport` | `auto` | como a extensão fala com o aurora: `auto` abre um `aurora serve` em segundo plano quando a AURORA instalada o tem (0.9.0 ou mais nova) e, senão, roda um processo por comando; `cli` sempre um processo por comando; `serve` só o `aurora serve` (com uma AURORA mais antiga a extensão fica indisponível). Também só nas configurações do usuário |
| `aurora.check.lint` | `false` | toda verificação roda também o `verilator --lint-only` |
| `aurora.check.onSave` | `false` | verifica o projeto ao gravar um `.v` ou `.sv` dele, sem notificação |
| `aurora.sim.verilator` | `false` | simula no Verilator, com onda, em vez do Icarus |
| `aurora.sim.timeout` | `0` | encerra a simulação que passar disto, em segundos; `0`, sem limite |
| `aurora.synth.openViewer` | `true` | abre o RTL viewer depois de cada síntese |
| `aurora.synth.schematic` | `false` | também desenha o esquemático do Graphviz (`--svg`) e o abre |
| `aurora.add.askForNewFiles` | `true` | o aviso de arquivo novo fora do projeto |
| `aurora.files.followExplorer` | `true` | o que o explorador renomeia, move ou apaga acompanha no `.spf` |

As cores da vista são do tema (`aurora.topForeground`,
`aurora.simulatedForeground`, `aurora.builtForeground`,
`aurora.notBuiltForeground`, `aurora.unregisteredForeground`) e se trocam em
`workbench.colorCustomizations`.

## Como fala com a AURORA

Pelo executável `aurora`, sem shell, de um de dois jeitos, conforme
`aurora.transport`: um `aurora serve` em segundo plano, com os pedidos em
JSON-RPC 2.0 por stdin e stdout (`src/aurora/serveClient.ts`), ou um processo
por comando, com `--json`, `--events` e `-C <.spf>`
(`src/aurora/cliClient.ts`) ([docs/CLI.md](../../docs/CLI.md)). Vistas e
comandos dependem só do `AuroraClient` (`src/aurora/client.ts`), e
`src/connection.ts` escolhe a implementação. O `cliClient.ts` é o único
módulo que cria processos, o do `aurora serve` também, e o lint cobra isso. Os tipos de
`src/aurora/aurora-types.ts` são gerados de `docs/schema/` por
`npm run gen:types`, como os do Studio.

## Compilar e testar

Com Node 22. O build copia o RTL viewer de `packages/rtl-viewer`, que
precisa das dependências dele:

```sh
(cd packages/rtl-viewer && npm ci)
cd editors/vscode
npm ci
npm run build              # dist/extension.js, pelo esbuild
npm run typecheck          # tsc
npm run lint               # eslint
npm run test:unit          # unitários e de contrato, sem VS Code e sem AURORA
npm run test:integration   # num VS Code de teste, baixado em .vscode-test/
```

Para experimentar: `npm run watch` e
`code --extensionDevelopmentPath="$PWD" <pasta de um projeto>`.

Os testes não usam a AURORA instalada. Um aurora falso
(`test/fixtures/fake-aurora/aurora.mjs`) devolve as respostas de
`test/fixtures/responses/`. As de `cases.json` foram gravadas por
`npm run record:fixtures` com o aurora compilado deste repositório (a 0.7.0
da HEAD, que já escreve o `saved_layout` do `wave`) e o bundle instalado; o
cabeçalho do script diz como. As de `synthetic.json` foram escritas à mão,
para o que o aurora não faz de propósito (pânico, saída que não é JSON). O
teste de contrato valida cada resposta gravada contra `docs/schema/`, sem
aceitar campo fora do schema, e confere que `aurora-types.ts` é o que os
schemas geram. Quando um schema muda, ele quebra: grave de novo, gere os
tipos e veja o que a extensão usa do que mudou.

O teste de integração roda o aurora falso pelo shebang, então vale para Linux
e macOS. Sem tela, `xvfb-run -a npm run test:integration`.
`VSCODE_TEST_VERSION=1.90.0` testa no VS Code mais antigo que `engines`
aceita.

## Empacotar

```sh
npm run package                          # aurora-0.2.1.vsix nesta pasta
code --install-extension aurora-0.2.1.vsix
```
