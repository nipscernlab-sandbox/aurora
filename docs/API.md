# AURORA-Core: guia da API

Este guia explica como a API funciona como um todo: conceitos, o que cada
operação executa, o que lê e escreve no disco, o formato dos resultados e dos
erros. A referência item a item está na documentação do código:

```
cargo doc -p aurora-core --no-deps --open
```

Todo item público do `aurora-core` tem documentação (o crate compila com
`#![deny(missing_docs)]`), e os exemplos da documentação rodam como teste em
`cargo test`.

Sumário:

1. [Conceitos](#1-conceitos)
2. [O bundle (`Toolchain`)](#2-o-bundle-toolchain)
3. [Projeto](#3-projeto)
4. [Arquivos do projeto](#4-arquivos-do-projeto)
5. [Operações](#5-operações)
6. [Resultados](#6-resultados)
7. [Diagnósticos](#7-diagnósticos)
8. [Erros](#8-erros)
9. [Garantias e limites](#9-garantias-e-limites)
10. [Diferenças em relação à AURORA](#10-diferenças-em-relação-à-aurora)

---

## 1. Conceitos

A API tem quatro peças:

| Peça | Tipo | O que é |
|---|---|---|
| Toolchain | `Toolchain` | O bundle de ferramentas instalado com a AURORA, aberto e conferido. É de onde a AURORA executa as ferramentas; o que ele roda de fora do bundle está em 2.4, 2.5, 5.6.1 e 5.8. |
| Projeto | `Project` | Um `.spf` da AURORA aberto, com seus arquivos Verilog e processadores. |
| Processador | `Processor` | Um processador SAPHO do projeto: nome, linguagem, caminhos, parâmetros de simulação. |
| Operação | funções livres | `build`, `simulate`, `simulate_project`, `check`, `hierarchy`, `synthesize`, `render_schematic`, `wave_signals`, `open_waveform`. Cada uma recebe a toolchain e o que operar, e devolve um resultado. As que executam ferramentas, todas menos `open_waveform`, recebem por último um `&Control`, que cancela a operação e mostra o que as ferramentas escrevem enquanto rodam (5.8). |

Um projeto é Verilog; processadores SAPHO são opcionais. Quando existem, o
cliente os compila com `build` antes de verificar, simular ou sintetizar,
como a CLI faz. O módulo `aurora_core::verilog` (4.2) trabalha sobre o texto
Verilog: classifica arquivos, acha módulos, lê portas e gera os
arquivos-modelo.

As operações não guardam estado entre si. O estado está no disco: `build`
grava o Verilog, `simulate` lê esse Verilog. Chamar `simulate` sem `build`
antes é erro (`NotBuilt`).

O ciclo de uso é sempre o mesmo:

```rust
use aurora_core::*;

let exe = camino::Utf8PathBuf::try_from(std::env::current_exe()?)?;
let toolchain = Toolchain::locate(&exe)?;      // <instalação>/toolchain
let project = Project::open("/home/eu/projetos/soma")?;
let soma = project.require_processor("soma")?;

let built = build(&toolchain, soma, &BuildOptions::default(), &Control::default())?;
if !built.succeeded() {
    for d in &built.diagnostics {
        println!("{:?}:{:?}: {}", d.file, d.line, d.message);
    }
}
```

---

## 2. O bundle (`Toolchain`)

A AURORA só executa ferramentas de um bundle versionado instalado com ele:
YANC, surfer-aurora e uma release fixa do OSS CAD Suite (Icarus, Verilator,
cocotb, Yosys, `dot`, openFPGALoader); no Windows, o Icarus, o Verilator e o
cocotb vêm do bloco MSYS2 do lace-toolchain (`msys/`), e o `dot`, do
Graphviz. Nunca do `PATH`, nunca de caminho
configurado. O que vem no bundle, as versões, como ele é montado e o que vem
do sistema estão em [BUNDLE.md](BUNDLE.md); os instaladores, em
[INSTALL.md](INSTALL.md).

O manifesto tem duas partes: o cabeçalho `bundle.json` (formato, versão do
bundle, plataforma) e um `components/<nome>.json` por componente instalado
(versão, origem, SHA-256 dos executáveis). O instalador deixa o usuário
escolher os componentes; um componente que não foi escolhido não tem arquivo
em `components/`.

### 2.1 Abrir

| Função | Faz |
|---|---|
| `Toolchain::locate(&executável)` | acha `toolchain/` ao lado de `bin/` (ou ao lado do executável) e abre |
| `Toolchain::open(dir)` | abre um bundle num diretório (desenvolvimento e testes) |

Abrir lê o `bundle.json` e os manifestos em `components/`
(`COMPONENTS_DIR`), confere o formato (`BUNDLE_SCHEMA`, hoje 2), que o bundle
é da plataforma atual, que cada `components/<nome>.json` descreve o
componente `<nome>` e que o diretório de cada componente existe e, resolvidos
os symlinks, fica dentro do bundle. Um bundle pode ser parcial (os
componentes que o usuário escolheu, ou só o YANC para testes de build): a
ferramenta de um componente não instalado dá `ComponentMissing` quando for
usada. `locate` resolve os symlinks do caminho do executável antes de
procurar, para o atalho `~/.local/bin/aurora` do instalador funcionar.

### 2.2 Consultar

| Método | Devolve |
|---|---|
| `manifest()` | o `BundleManifest`: identificador, plataforma e os componentes instalados, na ordem de `component::ALL` (`BundleComponent`: nome, versão, diretório, origem, SHA-256 do pacote e o hash de cada executável) |
| `component(nome)` | um componente instalado (nomes em `aurora_core::component`), ou `None` |
| `verified_files()` | quantos executáveis os manifestos listam |
| `tool(Tool)` | o caminho do executável, conferido no disco agora; recusa um symlink que sai do bundle (`InvalidBundle`) e, para o Verilator, a falta do `bin/verilator_bin` que o script Perl chama (sem ele, o script o procuraria no `PATH`) |
| `available()` | as ferramentas presentes |
| `tools()` | as ferramentas que a AURORA roda: todas, menos o `appcomp` e o `cpppp`, que o YANC 7.1 não tem mais (ficam em `Tool` para os relatórios antigos); é a lista do `aurora tools` e da aba Toolchain do Studio |
| `hdl_dir()`, `macros_dir()`, `headers_dir()` | os diretórios do YANC (`SAPHO/`, `Macros/`, `Header/`) |
| `surfer_web_dir()` | o cliente web do Surfer (`surfer-aurora/web/`: `index.html`, `surfer.js`, `surfer_bg.wasm`); `ToolchainIncomplete` num bundle que não o traz |
| `system_compiler()` | o `SystemCompiler` do Verilator, se encontrado |
| `verify()` | os executáveis cujo SHA-256 não confere com o manifesto (`FileMismatch`, com o componente que o lista) |
| `platform()` | a `Platform` |

### 2.3 Ferramentas

| `Tool` | Componente (`Tool::component()`) | Diretório |
|---|---|---|
| `Cmmcomp`, `Appcomp`, `Asmcomp`, `Cpppp`, `Cppcomp` | `yanc` | `yanc/` |
| `Iverilog`, `Vvp` | `icarus` | `oss-cad-suite/` (Linux, macOS) ou `msys/` (Windows) |
| `Verilator` | `verilator` | `oss-cad-suite/` (Linux, macOS) ou `msys/` (Windows) |
| `Yosys` | `yosys` | `oss-cad-suite/` |
| `Dot` | `graphviz` | `oss-cad-suite/` (Linux, macOS) ou `graphviz/` (Windows) |
| `Surfer` | `surfer-aurora` (o nome do programa é `surfer-aurora`) | `surfer-aurora/` |
| `OpenFpgaLoader` | `openfpgaloader` (o programa é `openFPGALoader`; no JSON, `openfpgaloader`) | `oss-cad-suite/` |
| `Quartus` | nenhum: o Quartus Prime do sistema (2.5). Cada passo roda um programa dele (`quartus_map`, `quartus_fit`, ...); `tool(Quartus)` dá o `quartus_sh` | a pasta dos programas do Quartus |
| `Perl` | nenhum: o do `SystemCompiler` (sistema no Linux e no macOS, bundle no Windows) | |

Icarus, Verilator, cocotb, Yosys e o `dot` do Linux e do macOS dividem o
diretório `oss-cad-suite/`: cada componente traz a parte do pacote da sua
ferramenta, e as bibliotecas em comum vêm com qualquer um deles. No
Windows, Icarus,
Verilator e cocotb dividem do mesmo jeito o `msys/`, o bloco MSYS2 do lace-toolchain,
com os programas em `ucrt64/bin`. O caminho de cada ferramenta dentro do
diretório é fixo por plataforma, no código; o diretório vem do manifesto do
componente. Nada é configurável.

### 2.4 O compilador do Verilator

O Verilator compila o modelo em C++, chamado pelo `make`, e o script dele
roda pelo Perl. Os três ficam num `SystemCompiler`. No Windows vêm do bundle,
no componente verilator (`msys/ucrt64/bin` e `msys/usr/bin`), e `bundled` é
verdadeiro. No Linux e no macOS, onde o OSS CAD Suite não traz compilador,
vêm do sistema: a exceção à regra do bundle.

| Função | Faz |
|---|---|
| `SystemCompiler::detect()` | procura nos locais padrão do sistema (chamado por `open` no Linux e no macOS; no Windows devolve `None`) |
| `SystemCompiler::in_dir(dir)` | os três num diretório |
| `SystemCompiler::in_msys2(raiz)` | uma instalação do MSYS2, no lugar do compilador do bundle no Windows |
| `toolchain.with_system_compiler(Some(c))` | troca o achado por um declarado |

Sem ele, só o que roda o Verilator falha: a simulação com Verilator e o
`check` com `lint`. No Linux e no macOS, com `SystemCompilerMissing`; no
Windows, com o erro do componente verilator (`ComponentMissing` ou
`ToolchainIncomplete`).

### 2.5 O Quartus Prime

O Quartus Prime compila para as placas Intel ([FPGA.md](FPGA.md)) e é a
segunda exceção à regra do bundle. O `open` não o procura: o Core não lê
variáveis de ambiente para achá-lo, e quem abre o bundle passa o que achou.

| Função | Faz |
|---|---|
| `fpga::Quartus::detect(home)` | procura nas pastas padrão do instalador da Intel (no Linux, também dentro de `home`) e fica com a versão mais nova; `None` no macOS |
| `fpga::Quartus::in_dir(dir)` | a instalação numa pasta declarada: a da versão, a `quartus` dentro dela ou a dos programas |
| `toolchain.with_quartus(Some(q))` / `toolchain.quartus()` | prende ao `Toolchain` e consulta |

A CLI e o Studio usam a pasta declarada (`--quartus`, `AURORA_QUARTUS`, a
preferência do Studio), senão `QUARTUS_ROOTDIR`, senão `detect`. Sem
Quartus, `tool(Tool::Quartus)` e `fpga::build` (5.10) dão
`QuartusMissing`.

Os programas do Quartus rodam com `QUARTUS_ROOTDIR` na instalação achada,
o `PATH` na pasta dos programas e no sistema base, e, do ambiente da AURORA,
a pasta e o nome do usuário (no Linux, também `LANG` e `TMPDIR`). A lista
é dedução, não conferida com o Quartus rodando.

## 3. Projeto

### 3.1 No disco

```
<raiz>/
  <projeto>.spf                  arquivo de projeto (JSON, formato AURORA)
  *.v                            Verilog do usuário, em qualquer subpasta, registrado no .spf
  <tb>.fst ou <tb>.vcd           onda da simulação do projeto (o $dumpfile do testbench)
  wave/<testbench>.json          a escolha de sinais da onda do testbench (5.7.3)
  wave/<testbench>.surf.ron      o layout salvo do Surfer (5.7.4)
  fpga.json                      a placa e as ligações (5.10)
  <processador>/                 o que a pessoa escreve:
    <processador>.cmm            o fonte (ou .cpp), que dá o nome ao processador
    input_<n>.txt, cade_<n>.txt  as entradas da simulação
    ...                          os arquivos que inicializam arrays, os cabeçalhos do C
    <processador>/               a pasta de saída: tudo o que o YANC gera
      <processador>.asm, .v, _data.mif, _inst.mif, _tb.v
      prog_log.txt, src_lines.txt, trad_opcode.txt, app_log.txt, pc_<processador>_mem.txt
      output_<n>.txt, toma_<n>.txt, <processador>_tb.fst   o que a simulação grava
  .aurora8/Temp/<processador>/      o que só a AURORA usa: .vvp, obj_dir, testbench instrumentado
  .aurora8/Temp/                    simulação do projeto (.vvp, obj_dir, testbench instrumentado)
  .aurora8/Temp/synth/<topo>/       síntese: script, log, netlist, esquemáticos
  .aurora8/Temp/hierarchy/          hierarquia: um .vvp por elaboração, refeita a cada chamada
  .aurora8/reports/                 o histórico de relatórios (5.9)
  .aurora8/fpga/<placa>/            o projeto do Quartus e os arquivos de gravação (5.10)
```

É a estrutura do YANC 7.1, a de quem o usa no terminal (`cmmcomp soma.cmm`
grava em `./soma/`), e a da AURORA desde a 7.1. O Verilog e o testbench que
o YANC gera citam os arquivos pelo caminho relativo à pasta de saída: o
projeto pode mudar de pasta sem compilar de novo.

Até o YANC 6.x cada processador tinha `Software/` (o fonte e o `.asm`),
`Hardware/` (o Verilog e as memórias) e `Simulation/` (o testbench e as
entradas e saídas), que não compilam mais. `processor.legacy_dirs()` diz
quais dessas pastas um processador ainda tem com algo dentro (pelo nome
exato, com as maiúsculas, e sem contar `.DS_Store`, `Thumbs.db`,
`desktop.ini`, `.gitkeep` e `._*`), `project.legacy_processors()` quais
processadores, `issues()` avisa (`legacy_layout`), e
`project.convert_layout()` converte (3.6). O `build` recusa
(`LegacyLayout`) enquanto o fonte não está no lugar novo
(`processor.needs_conversion()`); o que a conversão deixou numa pasta
antiga, porque o destino já existia, não impede o build.

A raiz é sempre o diretório do `.spf`. O `basePath` gravado dentro do arquivo
é ignorado na leitura, para que um projeto copiado de outra máquina funcione.

`Processor` tem `name`, `language`, `dir` (a pasta do processador),
`source` (`<dir>/<nome>.<ext>`), `output_dir` (`<dir>/<nome>`, a pasta de
saída), `temp_dir` (`.aurora8/Temp/<nome>`), `frequency_mhz`, `clocks` e
`show_arrays`. Os caminhos dos arquivos gerados: `assembly_path()`,
`verilog_path()`, `testbench_path()` e `files_read_by_verilog()` (as duas
memórias e a tabela do PC, que o Verilog gerado lê pelo nome e que quem o
lê fora da pasta de saída copia antes); os das portas, `input_path(n)` (na
pasta do processador) e `output_path(n)` (na de saída).

### 3.2 Abrir e criar

| Função | Faz |
|---|---|
| `Project::open(caminho)` | Aceita o `.spf` ou o diretório (usa `<dir>/<nome-do-dir>.spf` ou o único `.spf` dele). Só lê. |
| `Project::discover(caminho)` | Acha o projeto que contém o caminho: o `.spf` dele ou da primeira pasta acima que tenha um, com a regra de `open` em cada pasta. É o que a CLI usa, para os comandos funcionarem de qualquer pasta do projeto. Sem nenhum até a raiz do sistema, `ProjectNotFound`. |
| `project.processor_at(caminho)` | O processador cuja pasta (ou a temporária dele) contém o caminho; `None` fora delas. A CLI usa para `status`, `build`, `check`, `hierarchy`, `sim`, `wave`, `synth` e `proc set` sem nome de processador. |
| `Project::create(pai, nome)` | Cria `<pai>/<nome>/<nome>.spf` vazio, no formato da AURORA. Recusa se já existir. |
| `project.add_processor(&NewProcessor)` | Cria a pasta do processador, o fonte-modelo dentro dela (3.5) e a entrada no `.spf`; a pasta de saída nasce no primeiro `build`. Recusa nome repetido, nome que não passa em `validate_new_processor_name` (3.4), parâmetro que não passa em `NewProcessor::validate` (`InvalidParameter`, abaixo) e fonte que já exista no disco. |
| `project.configure_processor(nome, &ProcessorConfig)` | Grava `clk`, `numClocks` e `showArrays` no `.spf` (campos `None` ficam como estão). Uma entrada no formato antigo (só o nome) vira objeto. Valem a partir do próximo `build`. Frequência de 1 a `MAX_FREQUENCY_MHZ` (500000) e clocks de 1 a `MAX_CLOCKS` (2147483647); fora disso, `InvalidParameter`, e nada é gravado. |
| `project.remove_processor(nome)` | Tira o processador do projeto e grava o `.spf`: a entrada em `structure.processors` (no formato antigo ou de objeto), os arquivos registrados de dentro da pasta dele (o Verilog e o testbench gerados), com a escolha de topo ou de testbench que apontava para eles, e a pasta temporária `.aurora8/Temp/<nome>`. A pasta do processador fica no disco, e os `.v` gerados nela passam a aparecer em `unregistered_verilog`. Devolve `RemovedProcessor`: `name`, `dir` (para quem quiser apagar a pasta; a CLI e o Studio a mandam para a lixeira, se pedido) e `unregistered` (os `ProjectFile` que saíram, com o papel e a marca de topo). Nome que não existe: `ProcessorNotFound`, e nada muda. |
| `project.issues()` | O que está estranho no `.spf` sem impedir de abrir (`Vec<ProjectIssue>`, abaixo). Só lê. |
| `processor.is_built()` | O `<nome>.v` e o testbench existem na pasta de saída. Não diz se estão atualizados em relação ao fonte. |
| `project.legacy_processors()`, `project.convert_layout()` | Os processadores na estrutura do YANC 6.x, e a conversão deles (3.6). |
| `project.buildable_processors()` | Os processadores que têm o fonte no disco, na ordem do `.spf`: os que o botão Wave da AURORA compila antes de simular o projeto. Um na estrutura do YANC 6.x entra, e o build dele recusa com `LegacyLayout`, em vez de a simulação usar o Verilog velho de `Hardware/`. |

Todo método que altera o projeto trava `.aurora8/spf.lock`, relê o `.spf` do
disco, aplica a mudança e grava na hora (gravação atômica: arquivo
temporário e `rename`): duas gravações ao mesmo tempo, como a do Studio e a
da CLI, entram uma depois da outra, e nenhuma perde a da outra. Não existe
"salvar". Fora das gravações, um `Project` aberto não percebe mudanças que
outro programa faça no `.spf`: para ler sem gravar, abra de novo.

`NewProcessor::validate()` confere os parâmetros contra o que o YANC
compila, e é o que `add_processor` aplica. Portas de entrada e de saída: de 0
a `MAX_PORTS` (256, teto da AURORA; o `asmcomp` gera testbench corrompido com
100000). Só em C±: `#NBEXPO` de 2 a 8 (acima disso o `cmmcomp` converte
constantes com o `float` da máquina), `#NBMANT` de pelo menos 2, `#NUBITS`
até 32 e igual a `#NBMANT + #NBEXPO + 1` (o `asmcomp` exige), `#NUGAIN`
potência de dois e `#NDSTAC` e `#SDEPTH`, quando dados, de pelo menos 1
(pilha vazia passa no build e quebra a simulação). O erro é `InvalidParameter`, com o
parâmetro (`#NUBITS`), o valor e a regra. Em C, só as portas contam.

`ProjectIssue` (de `issues()`) tem `kind`, `path` (absoluto), `detail` e
`message` (em inglês). Os tipos (`IssueKind`):

| `kind` | Quando | `detail` |
|---|---|---|
| `rescued_path` | um absoluto de outra máquina que não existe aqui foi achado pela cauda dentro da raiz (4.1); a próxima gravação conserta o `.spf` | o caminho como estava gravado |
| `selection_not_registered` | `topLevelFile` ou `testbenchFile` aponta para um arquivo fora da lista do papel; é ignorado | o campo |
| `testbench_as_top` | `topLevelFile` tem nome de testbench (3.4); é ignorado | o campo |
| `invalid_processor_name` | um processador do `.spf` com nome que o YANC não compila (`x-y`, `void`); o `build` dele recusa com `InvalidName` | o nome |
| `legacy_layout` | um processador ainda tem `Software/`, `Hardware/` ou `Simulation/` com algo dentro; o `build` dele recusa com `LegacyLayout` até `convert_layout` (3.6), a menos que o fonte já esteja no lugar novo | o nome |
| `duplicate_file` | um arquivo repetido numa lista (também por outro caminho, `rtl/../rtl/a.v`, `rtl\a.v`) ou nas duas; conta uma vez (4.1), e a próxima gravação tira as entradas que sobram | a lista |
| `nested_project` | uma pasta acima da raiz tem `.spf`: o projeto está dentro de outro; o `path` é o `.spf` de fora | nenhum |

### 3.3 O `.spf`

| Campo | Lido | Gravado |
|---|---|---|
| `metadata.projectName`, `createdAt`, `computerName`, `appVersion` | não | na criação |
| `metadata.lastModified` | não | em toda gravação |
| `metadata.projectPath`, `structure.basePath` | não (a raiz é o diretório do `.spf`) | em toda gravação, com a raiz atual |
| `structure.processors` | sim | `add_processor`, `configure_processor`, `remove_processor` |
| `structure.synthesizableFiles`, `testbenchFiles` | sim | `add_verilog`, `remove_verilog`, `set_top`, `add_file`, `remove_file`, `reorder_file`, `set_top_level`, `set_testbench`, `move_path`, `record_move`, `register_generated`, `remove_processor`, `convert_layout`; toda gravação conserta os caminhos achados pela cauda e tira as entradas repetidas (4.1) |
| `structure.topLevelFile`, `testbenchFile` | sim | `add_verilog` (o primeiro de cada papel), `remove_verilog`, `set_top`, `set_top_level`, `set_testbench`, `move_path`, `record_move`, `register_generated`, `remove_processor`, `convert_layout` |
| `structure.topLevelModule` (campo da AURORA) | sim | `set_top` com nome de módulo; sai com `set_top_level` e quando o arquivo de topo sai da lista |
| qualquer outro (`commandOverrides`, `folders`, ...) | não | preservado como estava |

A leitura é tolerante como a da AURORA: JSON estrito primeiro; se falhar, tira
comentários `//` e `/* */`, BOM e vírgula antes de `}` ou `]`, e tenta de novo.
A ordem das chaves é preservada ao gravar.

Um tipo errado nos campos de arquivos é `InvalidProjectFile`, com o campo, e
não um campo ignorado (a próxima gravação o trocaria por uma lista vazia):
`synthesizableFiles` e `testbenchFiles` são listas de objetos, `path` é
texto, `isTopLevel` é booleano (ou o texto `"true"`/`"false"`), e
`topLevelFile`, `testbenchFile` e `topLevelModule` são texto. Ausente ou
`null` vale como vazio. `structure` que não é objeto também é
`InvalidProjectFile`.

Cada processador é uma string (formato antigo) ou um objeto:

| Campo | Tipo | Padrão | Uso |
|---|---|---|---|
| `name` | texto | obrigatório | nome do processador |
| `language` | `"cpp"` ou ausente | C± | linguagem |
| `sourceFile` / `cmmFile` | texto | `<nome>.<ext>` | só o nome do arquivo importa; a extensão ajuda a decidir a linguagem. O YANC 7.1 dá ao processador o nome do arquivo, e o `build` recusa outro que não `<nome>.<ext>` |
| `clk` | inteiro positivo (número ou texto) | 100 | `frequency_mhz`, `-f` do `asmcomp` |
| `numClocks` | inteiro positivo | 2000 | `clocks`, `-c` do `asmcomp` |
| `showArrays` | booleano | `false` | `show_arrays`, `--wave-arrays` do `cmmcomp` |

`clk: 12.5` é erro (`InvalidProjectFile`). A AURORA trunca para 12 em
silêncio; a AURORA recusa para não compilar com um valor que ninguém escolheu.
Também é `InvalidProjectFile` ao abrir: `clk` acima de 500000 ou `numClocks`
acima de 2147483647 (as faixas de `configure_processor`), nome de
processador vazio (tomaria a raiz como pasta dele), com `/`, `\` ou `:`, ou
repetido (sem diferença de caixa). Um nome que só não compila (`x-y`) abre,
com aviso em `issues()`, e o `build` recusa.

A linguagem é resolvida nesta ordem: `language` declarada; extensão de
`sourceFile`/`cmmFile`; o arquivo que existir na pasta do processador (ou,
num projeto da estrutura do YANC 6.x, em `Software/`; `.cmm` antes de
`.cpp`); C±.

### 3.4 Nomes

| Nome | Regra | Por quê |
|---|---|---|
| Processador | a regra do YANC 7.1 (`Compilers/common/proc_name.c`): `[A-Za-z_][A-Za-z0-9_]*`, sem começar por `__`, fora as palavras reservadas do Verilog e do SystemVerilog (`module`, `logic`, `int`, `void`, ...); e mais, da AURORA: até `MAX_PROCESSOR_NAME` (64) caracteres, fora os módulos da biblioteca SAPHO (`core`, `processor`, `ula`, ...) e os nomes reservados do Windows (`validate_processor_name`) | vira a pasta, o nome do fonte e o módulo Verilog, e o YANC recusa um fonte com nome que não serve de módulo; `core` instanciaria a si mesmo. Palavras do C± que não são do Verilog (`in`, `out`, `sqrt`) servem: conferido compilando e simulando |
| Módulo de topo | o escolhido pelo nome com `set_top` (campo `topLevelModule`), senão o único módulo do arquivo de topo, ou o que tem o nome do arquivo (`top_module`, 4.1) | vai em `hierarchy -top` da síntese e no `--top-module` do lint |
| Arquivo de topo | qualquer Verilog, inclusive o `<nome>/<nome>/<nome>.v` gerado de um processador, menos nome de testbench: `tb_<nome>.v`, `<nome>_tb.v`, `tb.v` (`verilog::is_testbench_name`) | o testbench não é o design |
| Módulo do testbench | a mesma regra, no testbench (`testbench_module`) | vai em `-s` e `--top-module` da simulação do projeto e do `check` |
| Projeto | letras sem acento (`A-Z`, `a-z`), dígitos, `_` e `-`, começando por letra, até `MAX_PROJECT_NAME` (64) caracteres; fora `CON`, `COM1` e os outros nomes reservados do Windows (`validate_project_name`). `create` também recusa um nome que só difere na caixa de outro da mesma pasta (`Ok1` ao lado de `ok1`), e não deixa pasta para trás se não conseguir gravar o `.spf` | vira a pasta, o `.spf` e parte do caminho de tudo que as ferramentas recebem; espaço e acento exigem que cada script e ferramenta cite o argumento direito |

A AURORA de antes da 7.1 aceitava `-` em nome de processador; o YANC 7.1
recusa o arquivo, e a AURORA também.
`validate_processor_name(nome)` é pública, como a
de projeto, para avisar enquanto o usuário digita. Para um processador
novo, `validate_new_processor_name(nome)` recusa também, sem diferença de
maiúsculas, os nomes das pastas da AURORA (`synth`, `hierarchy`, `lint`,
`wave`, `surfer`, `cocotb`): o processador dividiria a pasta com uma
operação da AURORA (`.aurora8/Temp/<nome>`, ou `wave/`, a das escolhas de
sinais). Um processador que já existe com um desses nomes continua abrindo
e compilando.

A regra de projeto só vale para criar (`Project::create`): um projeto que
já existe com outro nome, como os da AURORA, abre normalmente.
`validate_project_name(nome)` é pública, para uma interface avisar enquanto
o usuário digita; o erro é `InvalidName`, com o motivo em `reason`.

### 3.5 Fonte-modelo de `add_processor`

C± (padrões da AURORA em `NewProcessor::new`), como o YANC 7.1 pede: sem
`#PRNAME` (o processador se chama como o arquivo, e a diretiva é erro), e
sem as diretivas que têm padrão no YANC quando ninguém escolheu outro
valor: `#NDSTAC` e `#SDEPTH` só com `ndstac`/`sdepth` dados (sem elas o
`asmcomp` calcula as pilhas pelo programa), `#NUGAIN` só quando difere de
128.

```
#NUBITS 23
#NUIOIN 1
#NUIOOU 1
#NBMANT 16
#NBEXPO 6

void main()
{
    // out(porta, valor) escreve numa porta de saída
    out(0, 0);
}
```

O modelo traz uma instrução: o `cmmcomp` até a 6.0 recusava função de corpo
vazio, e um corpo só com comentário contava como vazio (conferido na 5.6 e
na 6.0; a 7.1 aceita o bloco vazio). Com `output_ports = 0` não há porta
para o `out`, e o corpo é `int x = 0;`, que compila com um aviso de variável
não usada.

C:

```
#pragma yanc nuioin 1
#pragma yanc nuioou 1

void main(void)
{
}
```

No fluxo C, largura, mantissa, expoente e pilhas são padrões de compilação do
`cppcomp` e não vão para o fonte.

### 3.6 Converter da estrutura do YANC 6.x: `convert_layout`

`project.convert_layout() -> Result<LayoutConversion>` faz o que a AURORA
faz ao abrir um projeto antigo (`js/project/estrutura_antiga.ts`), para que
o projeto convertido abra igual nas duas:

- de `Software/`, `Hardware/` e `Simulation/` de cada processador, o que o
  YANC gera ou gerava (o `.asm`, o `.v`, o testbench, as memórias, a tabela
  do PC, as tabelas e os logs de nome fixo, `output_<n>.txt`,
  `toma_<n>.txt`, a onda) vai para a pasta de saída; o resto, que é da
  pessoa (o fonte, os arquivos de arrays, `input_<n>.txt`, um `.v` posto à
  mão em `Hardware/`, uma subpasta), vai para a pasta do processador;
- nada é apagado nem sobrescrito: um destino que já existe deixa o arquivo
  onde estava (`skipped`, com o motivo), e a subpasta antiga só sai vazia
  (`remaining_dirs`, as que ficaram com algo além do `.DS_Store` e
  parecidos, que não são movidos);
- sem o `.spf` gravado, o que foi movido volta para onde estava, com o
  texto de antes, e a conversão devolve o erro;
- a linha `#PRNAME` (ou `#pragma yanc prname`) sai do fonte (`name_lines`,
  com o nome que ela dava ao módulo Verilog: se difere do processador, quem
  instanciava o módulo pelo nome antigo precisa do novo);
- o `.spf` acompanha (os arquivos registrados, o topo, o testbench
  escolhido), e as entradas dos processadores perdem os campos de pasta da
  AURORA antiga (`softwarePath`, `hardwarePath`, `simulationPath`);
- `invalid_names` lista os processadores com nome que não serve mais (na
  7.1 ele vira o módulo).

Num projeto já convertido, o relatório vem vazio (`is_empty()`) e nada é
gravado. Um arquivo que não pôde ser movido não é erro: fica em `skipped`.

---

## 4. Arquivos do projeto

### 4.1 Verilog e testbenches

Cada arquivo Verilog registrado é sintetizável (`FileRole::Synthesizable`) ou
testbench (`FileRole::Testbench`). `add_verilog` é a forma normal de
registrar: decide o papel pelo conteúdo, cria o arquivo que não existe e
escolhe topo e testbench. `add_file`, `remove_file` e `set_top_level` são a
forma de baixo nível: recebem o papel de quem chama e não classificam nem
criam modelo. Um `.py` só entra como testbench: é um testbench cocotb, como
na AURORA (4.4).

| Método | Faz |
|---|---|
| `add_verilog(toolchain, caminho, testbench) -> Result<AddedFile>` | registra um arquivo (`.v`, `.sv`, ou `.py`, testbench cocotb); se ele não existe, cria a partir do modelo |
| `remove_verilog(caminho) -> Result<bool>` | tira das duas listas e das escolhas de topo e testbench, sem apagar do disco; diz se estava registrado |
| `add_verilog_files(toolchain, caminhos, testbench) -> Result<Vec<AddedFile>>` | registra vários, na ordem, conferindo todos antes de registrar o primeiro: um nome ruim no meio da lista não deixa metade registrada (`aurora add`, `project.add`) |
| `remove_verilog_files(caminhos) -> Result<RemoveOutcome>` | tira vários, na ordem, e diz quais estavam registrados e quais não (`aurora remove`, `project.remove`) |
| `set_top(alvo) -> Result<Utf8PathBuf>` | escolhe o topo por arquivo ou por nome de módulo; devolve o arquivo |
| `top_module() -> Result<Option<String>>` | o nome do módulo de topo |
| `testbench_module() -> Result<Option<String>>` | o nome do módulo do testbench escolhido; num `.py`, o módulo Python dos testes (o nome do arquivo) |
| `unregistered_verilog() -> Vec<Utf8PathBuf>` | os `.v` e `.sv` da pasta do projeto que não estão registrados, e os `.py` com `@cocotb.test` |
| `files(FileRole)` | lista os arquivos de um papel, na ordem do `.spf`, com caminho absoluto; um arquivo repetido conta uma vez (abaixo) |
| `reorder_file(caminho, &ListPosition) -> Result<Vec<ProjectFile>>` | muda a posição do arquivo na lista dele (`ListPosition::First`, `Last`, `Before(arquivo)`, `After(arquivo)`) e devolve a lista; a ordem é a dos compiladores, e um `` `define `` só vale para os que vêm depois |
| `top_level()` | o arquivo de topo: `topLevelFile`, se está na lista dos sintetizáveis e não tem nome de testbench; senão o sintetizável marcado |
| `testbench()` | `testbenchFile`, se está na lista dos testbenches; senão o testbench marcado, senão o primeiro |
| `check_add_verilog(caminho)` | confere, sem mudar nada, se `add_verilog` aceitaria o arquivo (extensão e, se não existe, o nome); quem registra vários confere todos antes |
| `set_testbench(caminho)` | testbench da simulação do projeto: `testbenchFile` + `isTopLevel` exclusivo; registra se não estava |
| `add_file(papel, caminho, conteúdo)` | registra com o papel dado; com `Some(texto)` cria o arquivo (recusa se existir), com `None` exige que exista; só `.v` e `.sv`, e `.py` como testbench (`InvalidName`, sem gravar nada) |
| `remove_file(papel, caminho)` | tira de uma lista, não apaga do disco |
| `set_top_level(caminho)` | arquivo de topo: `topLevelFile` + `isTopLevel` exclusivo; registra se não estava; recusa nome de testbench (3.4) |
| `resolve_path(texto)` | converte um caminho como está no `.spf` para absoluto |
| `move_path(de, para) -> Result<MovedPath>` | move ou renomeia um arquivo ou pasta do projeto; os arquivos registrados continuam registrados no lugar novo |
| `record_move(de, para) -> Result<MovedPath>` | registra no `.spf` um movimento que já aconteceu fora da AURORA, sem tocar no disco |
| `replace_refusal(de, destino) -> Option<&str>` | por que o que está em `destino` não pode sair para dar lugar a `de`, num `move_path` que achou o destino ocupado (`PathExists`), ou `None` se pode: a pergunta de quem oferece substituir o destino (o Studio, que o manda para a lixeira); o Core nunca sobrescreve |
| `register_generated(builds) -> Result<Vec<AddedFile>>` | registra o `<nome>.v` e o `<nome>_tb.v` que cada build bem-sucedido gerou na pasta de saída e o projeto ainda não tem (5.1.1) |

`add_verilog(toolchain, caminho, testbench)`:

- **Arquivo que existe:** o papel vem de `verilog::classify` (4.2); com
  `testbench`, é testbench. Se ele estava na outra lista, muda de lista: um
  arquivo fica numa lista só.
- **Arquivo que não existe:** é testbench com `testbench` ou se o nome
  tiver `tb`, `test` ou `testbench` como palavra (`alu_tb.v`, `tb_alu.v`,
  `test_alu.v`; `contest.v` não); senão, módulo, com
  `verilog::module_template(<nome do arquivo>)`. O testbench é
  `verilog::testbench_template` e instancia o módulo `X` de `X_tb.v` ou
  `tb_X.v` se algum sintetizável registrado o declara; senão, o módulo de
  topo; senão, nenhum. As portas vêm de `verilog::read_interfaces` com a
  `toolchain` recebida: com `None`, ou sem o Yosys instalado, o leitor de
  portas embutido.
- **`.py`:** é sempre testbench. Novo, sai de `cocotb::testbench_template`
  (4.4), com o módulo testado pela mesma regra (`test_alu.py` testa `alu`)
  na diretiva `# aurora-toplevel:`; o nome do arquivo precisa ser
  identificador do Python (`InvalidName`, sem criar nada).
- O primeiro sintetizável registrado vira o topo, a não ser que tenha nome
  de testbench (3.4) ou não declare módulo (um arquivo só de `` `define ``);
  o primeiro testbench, o testbench escolhido.

`caminho` é absoluto ou relativo à raiz do projeto. A CLI sempre passa
absoluto, resolvido a partir do diretório atual do shell.

`AddedFile`:

| Campo | Conteúdo |
|---|---|
| `path` | caminho absoluto |
| `role` | `synthesizable` ou `testbench`: em qual lista ficou |
| `created` | o arquivo não existia e foi criado a partir do modelo |
| `selected` | virou o topo (era o primeiro sintetizável) ou o testbench escolhido |

`set_top(alvo)` usa `alvo` como arquivo (absoluto ou relativo à raiz) se ele
existir, registrando-o como sintetizável (um arquivo que estava entre os
testbenches muda de lista). Qualquer Verilog serve, inclusive o
`<nome>/<nome>/<nome>.v` que o build de um processador gerou: registrado, ele
continua entrando uma vez só no design, porque `check`, a simulação, a
síntese e a hierarquia tiram os repetidos. Nome de testbench (`tb_<nome>.v`,
`<nome>_tb.v`, `tb.v`) é `InvalidName`, e nada muda; senão, procura um módulo com esse nome nos
sintetizáveis registrados e grava o arquivo e o nome do módulo
(`structure.topLevelModule`, campo da AURORA que a AURORA ignora), para
arquivos com vários módulos. Se nenhum declara, é `ModuleNotFound`, com os
módulos do projeto em `available` (sem repetição); se mais de um declara, é
`AmbiguousModule`, com os arquivos, e o topo se escolhe pelo arquivo. Um
alvo que é arquivo mas não `.v` nem `.sv` é `InvalidName`.

`top_module()` é o módulo de `topLevelModule`, se o arquivo de topo ainda o
declara; senão, os módulos do arquivo de topo (`verilog::modules_in`): com um
só, é ele; com vários, o que tem o nome do arquivo; senão, `ModuleNotFound`.
Sem topo, `Ok(None)`. `testbench_module()` é a mesma regra, sem o campo,
sobre `testbench()`, com mais um caso: vários módulos e nenhum com o nome do
arquivo, vale o único que nenhum outro do arquivo instancia
(`bancada_tb.v` com `gerador` e `principal`, que instancia `gerador`,
simula `principal`).

`unregistered_verilog()` procura na pasta do projeto e nas subpastas (até 4
níveis), menos `.aurora8/`, as ocultas, as pastas dos processadores e a
`TopLevel/` legada, cujos `.v` já entram sozinhos.

`move_path(de, para)` move ou renomeia um arquivo ou uma pasta dentro do
projeto, como arrastar na árvore de uma IDE, e mantém o `.spf` em dia.
`para` é o caminho final, e não a pasta de destino (`rtl/a.v` para
`src/a.v` move; `a.v` para `b.v` renomeia); as pastas que faltam nele são
criadas. Os dois são absolutos ou relativos à raiz.

- Cada entrada do `.spf` que estava em `de` (o próprio arquivo, ou os de
  dentro da pasta) troca de caminho e de nome na mesma posição da lista, e
  `topLevelFile`, `testbenchFile` e `topLevelModule` acompanham. O papel e a
  marca de topo ou de testbench escolhido não mudam.
- Ficam onde estão, porque o Core os acha pelo lugar: o `.spf`, a pasta
  `.aurora8` e, de cada processador, a pasta, a pasta de saída e o fonte
  (`CannotMove`). O resto de dentro delas (entradas, saídas, gerados) pode
  mudar de lugar. Nada vai para dentro de `.aurora8`, e
  uma pasta não vai para dentro dela mesma (`CannotMove`).
- O destino que já existe não é sobrescrito (`PathExists`), inclusive um
  link quebrado; renomear só a caixa (`a.v` para `A.v`) funciona também num
  sistema que não distingue maiúsculas. Uma origem que não existe é
  `InvalidProject`. Caminho fora da
  pasta do projeto é `OutsideProject`. Em todos os erros, nada mudou.
- O movimento é um `rename`; entre discos diferentes, copiar e apagar. Se o
  `.spf` não puder ser gravado depois, o movimento é desfeito antes do erro.
- `de` igual a `para` não faz nada.

`record_move(de, para)` é o `move_path` para o que já foi movido fora do
AURORA (no explorador de um editor, com o `mv`), como o `hg mv --after`: só o
`.spf` muda, do mesmo jeito. As regras do que fica no lugar são as mesmas;
o disco tem que bater com o movimento: `de` que ainda existe é
`CannotMove`, e `para` que não existe é `InvalidProject`. Só a caixa
mudar (`a.v` para `A.v`) vale também aqui. Se o `.spf` não puder ser
gravado, ele fica como estava.

`MovedPath`:

| Campo | Conteúdo |
|---|---|
| `from` | onde estava, absoluto |
| `to` | onde ficou, absoluto |
| `files` | os `ProjectFile` que mudaram de lugar, já com o caminho novo; vazio para um arquivo que não está no `.spf` |

```rust
let moved = project.move_path("contador.v", "rtl/contador.v")?;
for file in &moved.files {
    println!("{} continua no projeto ({:?})", file.path, file.role);
}
```

Caminhos são comparados depois de normalização léxica: `rtl/../top.v` e
`top.v` são o mesmo arquivo. Um `.spf` editado à mão com o mesmo arquivo
repetido numa lista conta uma vez, marcado se alguma das entradas está;
nas duas listas, conta como testbench se o nome indica (`_tb`, `tb_`,
`test`), e como sintetizável senão. A marca de testbench escolhido aceita
também o `isMarkedTestbench` legado da AURORA, que ela lê como
`isTopLevel`; escolher um testbench tira a marca legada dos outros. O `.spf` guarda, sempre com `/`:

- relativo à raiz, quando o arquivo está dentro dela, como a AURORA;
- relativo com `..` (`../../rtl/x.v`), quando está fora dela mas no mesmo
  repositório git (a primeira pasta acima da raiz com `.git`), para o
  projeto funcionar em qualquer clone;
- absoluto, no resto.

Na leitura aceita `\` e `/`. Um absoluto que não existe nesta máquina (de
outra, `C:\Users\...` num `.spf` aberto no Linux) é procurado pela cauda
dentro da raiz, como a AURORA (`resgatarPelaCauda`): `C:\velho\p\rtl\x.v`
tenta `velho/p/rtl/x.v`, `p/rtl/x.v`, `rtl/x.v` e `x.v`, e o primeiro que
existir vale para todas as operações. O caminho achado é um aviso
(`rescued_path` em `issues()`), e a próxima gravação do `.spf` o troca pelo
caminho de hoje.

### 4.2 O módulo `verilog`

`aurora_core::verilog` trabalha sobre texto. Só `read_interfaces` executa
uma ferramenta, o Yosys, e só quando ele está instalado.

| Função | Devolve |
|---|---|
| `classify(texto, nome_do_arquivo) -> FileRole` | sintetizável ou testbench, pela regra da AURORA |
| `modules_in(texto) -> Vec<String>` | os nomes dos módulos declarados, na ordem, sem os que estão em comentário |
| `read_interfaces(toolchain, arquivos) -> Result<Vec<ModuleInterface>>` | as portas de cada módulo declarado nos arquivos |
| `module_template(nome) -> String` | o módulo-modelo de um arquivo novo |
| `testbench_template(testbench, dut) -> String` | o testbench-modelo, instanciando `dut` quando há um |
| `is_testbench_name(nome_do_arquivo) -> bool` | o nome é de testbench pela regra do topo: `tb_<nome>`, `<nome>_tb` ou `tb`, com qualquer extensão; mais estreita que a dica de nome do `classify` (`test_alu.v` não conta) |

`classify` segue `js/project/verilog_classifier.ts` da AURORA: soma pontos, e
3 ou mais é testbench. Cada indício conta uma vez.

| Indício | Pontos |
|---|---|
| `$dumpfile`, `$dumpvars`, `$dumpon`, `$dumpoff`, `$dumpall`, `$dumplimit` ou `$dumpflush` | 3 |
| `$finish` ou `$stop` | 3 |
| módulo sem portas (`module tb;`, `module tb();`, também com `#(...)` de parâmetros) | 3 |
| `initial` | 2 |
| `$display`, `$write`, `$monitor`, `$strobe`, `$time`, `$realtime`, `$random`, `$sformat` ou `$sformatf` | 1 |
| atraso `#<dígito>` (`#10`; o `#(` de parâmetro não conta) | 1 |
| nome do arquivo com `tb`, `test` ou `testbench` como palavra (`alu_tb.v`, `test_alu.v`) | 2 |

Comentários e strings não contam. Texto vazio é sintetizável. Na dúvida,
sintetizável: `initial` sozinho, que aparece em módulo sintetizável para
inicializar memória, não basta; o nome sozinho também não.

`read_interfaces` lê só sintetizáveis, porque o Yosys não lê testbench. Com
o Yosys do bundle, roda `read_verilog -sv -lib` e `write_json`; sem ele
(`toolchain` `None` ou componente não instalado), usa um leitor de portas no
estilo ANSI, as declaradas no cabeçalho do módulo. `ModuleInterface` tem
`name`, `file` e `ports`, na ordem da declaração; cada `Port` tem `name`,
`direction` (`input`, `output`, `inout`), `width` em bits, com os parâmetros
nos valores padrão, e `signed`.

`module_template(nome)` dá um módulo com `input wire [7:0] a`,
`output wire [7:0] y` e `assign y = a;`: compila, e tem portas para não ser
classificado como testbench.

`testbench_template(testbench, dut)`, com `dut`, instancia o módulo com
todas as portas (`reg` para as entradas, `wire` para as saídas), gera clock
se houver uma porta `clk` ou `clock` e reset se houver `rst` ou `reset`,
grava a onda com `$dumpfile("<testbench>.fst")` (o Verilator grava
`<testbench>.vcd`, 5.2) e
`$dumpvars(0, <testbench>)` e termina com `$finish`. Sem `dut`, o testbench
não instancia nada: grava a onda com o mesmo `$dumpfile` e `$dumpvars`,
espera `#100` e chama `$finish`.

### 4.3 Entradas e saídas da simulação

O testbench que o `asmcomp` gera lê `<proc>/input_<n>.txt`, na pasta do
processador, e escreve `<proc>/<proc>/output_<n>.txt`, na de saída, por
caminho relativo a ela (`../input_<n>.txt`, `output_<n>.txt`), onde a
simulação roda. Os dois têm um valor decimal com sinal por linha.

| Método de `Processor` | Faz |
|---|---|
| `input_path(n)`, `output_path(n)` | os caminhos |
| `write_input_values(n, &[i64])` | grava a entrada, um valor por linha (substitui a anterior) |
| `write_input(n, texto)` | grava a entrada como texto livre |
| `read_output_values(n)` | lê a saída como `Vec<i64>` |
| `read_output(n)` | lê a saída como texto |

`read_data_file(caminho)` lê qualquer arquivo nesse formato. Uma linha que não
seja inteiro é erro (`InvalidDataFile`, com a linha), nunca descartada: um
valor perdido mudaria o resultado da simulação.

A porta `n` corresponde ao endereço usado em `in(n)` / `out(n)` no programa.
O testbench só lê uma entrada se o arquivo existir; arquivo ausente não é
erro, e a porta simplesmente não recebe dado. `missing_inputs(processador)`
lista os que faltam antes de simular.

`simulate` (5.2) confere as entradas que existem antes de rodar, porque o
testbench do YANC lê com `$fscanf` sem olhar o resultado: uma linha que não
é inteiro é `InvalidDataFile`, e a simulação não roda; arquivo vazio, ou
valor que não cabe em `#NUBITS` bits (com ou sem sinal), é aviso, e a
simulação roda. Um arquivo com menos valores do que o programa lê não é
conferido (o número de leituras só se sabe rodando; o último valor se
repete).

### 4.4 O módulo `cocotb`

Um `.py` em `testbenchFiles` é um testbench cocotb: só os testes
(`@cocotb.test()`), e a AURORA monta a simulação (5.3.2). O módulo que os
testes recebem como `dut` vem da diretiva da AURORA, uma linha só de
comentário `# aurora-toplevel: <módulo>` (com `:` ou `=`, sem distinção de
caixa), ou, sem ela, do topo do projeto, com um aviso. O nome do arquivo é o
módulo Python que o cocotb importa.

| Função ou tipo | Faz |
|---|---|
| `is_testbench(caminho) -> bool` | o arquivo é testbench cocotb (termina em `.py`) |
| `toplevel_directive(texto) -> Option<String>` | o módulo da diretiva `# aurora-toplevel:`, se houver |
| `testbench_template(dut) -> String` | o testbench-modelo: com `dut`, a diretiva e um teste que liga o clock (`clk`, 10 ns), segura o reset nos primeiros 20 ns (`rst`, ou `rst_n` ativo em baixo), põe as outras entradas em zero e escreve as saídas no log; sem, um teste que só espera. `Timer` e `Clock` com a unidade posicional, que vale no cocotb 1 e no 2 |
| `TestReport` | o resultado dos testes: `results` (o `results.xml`), `cases` e as contagens `passed`, `failed`, `skipped` |
| `TestCase` | um teste: `name` (`<módulo>.<teste>`), `status` (`TestStatus`: `passed`, `failed`, `skipped`), `message`, `file`, `line` (numa falha, a linha mais funda do traceback dentro do `.py`) |

---

## 5. Operações

Todas bloqueiam até as ferramentas terminarem, exceto `open_waveform`. As
que executam ferramentas recebem por último um `&Control`;
`&Control::default()` roda até o fim sem cancelar nem avisar nada. Cancelar,
pôr prazo e acompanhar a saída estão em 5.8.

### 5.1 `build(&Toolchain, &Processor, &BuildOptions, &Control) -> Result<BuildResult>`

Do fonte ao Verilog sintetizável, com o YANC 7.1 ou mais novo.

Antes de executar qualquer coisa:

1. o YANC do bundle precisa ser o 7.1 ou mais novo (`YancTooOld`): até a
   6.x a linha de comando e as pastas eram outras;
2. o processador precisa estar na estrutura do YANC 7.1: com o fonte fora
   do lugar novo e `Software/`, `Hardware/` ou `Simulation/` com algo
   dentro, `LegacyLayout` (ver 3.6);
3. o fonte precisa existir e se chamar `<processador>.<ext>`: o YANC dá ao
   processador o nome do arquivo (`InvalidSource`);
4. nenhum caminho pode passar de `YANC_PATH_LIMIT` (259 no Windows, 1000
   nos outros sistemas; ver 9.1);
5. a pasta de saída e a temporária são criadas.

Comandos, com `D` = pasta do processador e `O` = pasta de saída (`D/<nome>`):

| Passo | Comando | CWD |
|---|---|---|
| `compile` (C±) | `cmmcomp D/<nome>.cmm -o O [--wave-arrays] --lang=en` | `D` |
| `compile` (C) | `cppcomp D/<nome>.cpp -o O --lang=en` | `D` |
| `assemble` | `asmcomp O/<nome>.asm -f <MHz> -c <clocks> --lang=en` | `O` |

`build_steps(Language)` dá essa lista, com a ferramenta de cada passo; a CLI
a usa para dizer o que não chegou a rodar.

Cada ferramenta acha `SAPHO/`, `Macros/` e `Header/` ao lado do próprio
executável (a AURORA confere antes que o bundle os tem). O compilador grava em
`O` o `.asm`, `prog_log.txt` (com a pasta do fonte, `srcdir`, onde o
`asmcomp` lê os arquivos que inicializam arrays), `src_lines.txt` e
`pc_<nome>_mem.txt`; o `asmcomp`, ao lado do `.asm`, o Verilog, as
memórias, o testbench, `app_log.txt` e `trad_opcode.txt`. O `cppcomp`
pré-processa o C na memória e procura os `#include` na pasta do fonte e,
por último, no `Header/` do bundle.

O Verilog e o testbench citam os arquivos pelo caminho relativo a `O`: as
memórias, a tabela do PC, `output_<n>.txt` e `toma_<n>.txt` pelo nome,
`input_<n>.txt` e `cade_<n>.txt` como `../input_<n>.txt`. Nada é copiado
nem reescrito: o testbench roda em `O` (5.2), e a simulação do projeto, a
síntese e a placa, que leem o Verilog de outro lugar, copiam antes as
memórias e a tabela (`Processor::files_read_by_verilog`).

Um `#PRNAME` ou `#pragma yanc prname` que sobrou de um fonte da 6.x é erro
do compilador, que diz para apagar a linha; a AURORA deixa a mensagem vir
dele, com a linha, como qualquer erro do fonte.

A AURORA pede as mensagens em inglês (`--lang=en`), mesmo com `YANC_LANG=pt`
no ambiente, para que o parser trabalhe sobre um idioma só.

`BuildOptions` sobrepõe `frequency_mhz`, `clocks` e `show_arrays` só neste
build, sem gravar no `.spf`.

Artefatos, todos na pasta de saída. Obrigatórios: `assembly`, `verilog`,
`data_memory`, `instruction_memory`, `testbench`. Intermediários:
`program_counter_map`, `source_translation`, `opcode_translation`,
`compiler_log`, `pre_assembler_log`.

### 5.1.1 `build_processors(&Toolchain, processadores, &BuildOptions, OnFailure, &Control, on_result) -> Result<Vec<BuildResult>>`

Compila vários processadores em sequência. `on_result` é chamado depois de
cada um, para a interface mostrar o progresso. Um build cancelado encerra a
sequência mesmo com `OnFailure::Continue`.

| `OnFailure` | Uso |
|---|---|
| `Continue` | "compilar tudo": mostra todos os erros de uma vez |
| `Stop` | antes de simular ou sintetizar: para no primeiro que falhar |

O fluxo do botão Wave da AURORA é:

```rust
let control = Control::default();
let builds = build_processors(&tc, project.buildable_processors(), &BuildOptions::default(),
                              OnFailure::Stop, &control, |r| mostrar(r))?;
project.register_generated(&builds)?;
if builds.iter().all(BuildResult::succeeded) {
    let options = SimulationOptions::new(Simulator::Icarus);
    let sim = simulate_project(&tc, &project, &options, &control)?;
}
```

Depois do build, `project.register_generated(&builds)` põe no `.spf` o que
cada processador gerou e o projeto ainda não tem: o `<nome>/<nome>/<nome>.v`
entre os sintetizáveis e o `<nome>/<nome>/<nome>_tb.v` entre os testbenches,
no fim de cada lista. É o que a AURORA faz ao atualizar a árvore de
arquivos, e a CLI e o Studio chamam depois de todo build. Só os builds que
terminaram bem entram. Num projeto sem sintetizáveis, o `.v` do primeiro
processador vira o topo; num sem testbench, o testbench dele vira o
escolhido. O que já está registrado, em qualquer lista, fica como está; o
que foi tirado com `remove_file` volta no build seguinte. Devolve um
`AddedFile` por arquivo registrado (`created` é sempre `false`). As
operações já usavam o `<nome>.v` gerado sem ele no `.spf` e tiram o
repetido quando está; o testbench registrado passa a ser elaborado pelo
`check` e mostrado pela `hierarchy`, uma vez só.

### 5.2 `simulate(&Toolchain, &Processor, &SimulationOptions, &Control) -> Result<SimulationResult>`

Simula um processador com o testbench que o `build` gerou. Duração e
frequência já estão no testbench (vêm de `clocks` e `frequency_mhz` do build).

Com `<SAPHO>` = a biblioteca SAPHO (`yanc/SAPHO/` do bundle), `O` = a pasta
de saída do processador e `T` = a temporária:

| Simulador | Passo | Comando | CWD |
|---|---|---|---|
| Icarus | `elaborate` | `iverilog -y <SAPHO> -s <nome>_tb -o T/<nome>_tb.vvp O/<nome>.v T/instr_<nome>_tb.v` | `O` |
| Icarus | `simulate` | `vvp -n [-i] T/<nome>_tb.vvp -fst`, com `-i` quando o `Control` tem receptor de eventos (5.8); sem `-fst` só quando a onda não vai para um `.fst` (o formato da onda, abaixo) | `O` |
| Verilator | `verilate` | `perl verilator --binary --main --trace -j 0 ... --top-module <nome>_tb -Mdir T/obj_dir_<nome>_tb -y <SAPHO> <arquivos>` (no Windows, `verilator_bin.exe` no lugar de `perl verilator`) | `T` |
| Verilator | `simulate` | `T/obj_dir_<nome>_tb/V<nome>_tb` | `O` |

O CWD é a pasta de saída, como o YANC 7.1 pede: o `.v` do processador lê as
memórias e `pc_<nome>_mem.txt` pelo nome, e o testbench lê as entradas na
pasta do processador (`../input_<n>.txt`) e grava as saídas e a onda
(`<nome>_tb.fst`, `.vcd` no Verilator) ali.

Uma pasta com acento, outro alfabeto ou emoji funciona: o YANC 7.1 cita
memórias e entradas pelo nome relativo. No Windows, o Icarus, o `vvp`, o
Yosys, o Verilator e o `dot` abrem arquivos pela página de código ANSI, e um
caminho com caractere fora do ASCII chega a eles relativo ao CWD do passo
(`process::tool_path`): o trecho com acento, que é a pasta do projeto ou a do
usuário, fica de fora. O Verilator, que monta o caminho absoluto de cada
fonte a partir do CWD lido pela página de código, nasce no nome curto (8.3)
da pasta. Os diagnósticos voltam com o caminho absoluto. Um nome de arquivo
com acento dentro do projeto (`rtl/módulo.v`) continua chegando assim à
ferramenta. Conferido em 2026-10-27 em pastas com acento, espaço, símbolos,
grego, CJK e emoji. O aviso do `vvp` sobre nome de arquivo
("contains non-printable characters"), se aparecer, vira erro.

Em toda chamada do Icarus (simulação, `check`, `hierarchy`), um `include`
procura primeiro na pasta do arquivo que inclui (`-grelative-include`) e
depois na raiz do projeto (`-I <raiz>`), a mesma ordem da síntese.

O `-n` faz `$stop` e Ctrl-C terminarem a simulação, em vez de abrir o prompt
interativo do `vvp`.

Antes de rodar, as entradas do processador são conferidas (4.3), e a
simulação usa uma cópia do testbench (`T/instr_<nome>_tb.v`) que confere o
resultado de cada `$fscanf` da entrada: na primeira leitura além do fim do
arquivo, o `vvp` escreve `WARNING: <entrada>:<linha>: the program reads more
values than this file has (N)`, que vira aviso com o arquivo e a linha
seguinte à última (antes, a porta repetia o último valor em silêncio). A
cópia tem as mesmas linhas, e os diagnósticos dela apontam para o
testbench original. A mesma cópia leva a extensão da onda trocada (o formato
da onda, abaixo); um testbench sem o trecho esperado e com a extensão certa
roda como está.
Depois, se
a simulação deu certo mas o testbench não escreveu `end of program` (o aviso
que o testbench do `asmcomp` dá quando o programa termina), um diagnóstico de
aviso diz que os clocks acabaram antes do fim do programa e que as saídas
param ali; o status continua `succeeded`.

Um build (5.1) ou uma simulação de um processador trava
`.aurora8/Temp/<nome>.lock` enquanto roda, e `simulate_project` trava a do
projeto (`.aurora8/Temp/.project.lock`) e a de cada processador. Uma segunda
operação no mesmo processador, de outro processo (o Studio e a CLI) ou da
mesma, não espera: é `OperationInProgress`, e nada é alterado. Sem a trava,
as duas regravariam a pasta de saída e a temporária ao mesmo tempo e
falhariam juntas.

A biblioteca SAPHO entra por `-y`: o simulador carrega só os módulos que
faltarem. Ela vem do YANC (`Toolchain::sapho_library`) e só entra quando há
processador SAPHO: sempre em `simulate`; em `simulate_project`, `check` e
`synthesize`, só se o projeto tem processadores. Um projeto só de Verilog
não usa a biblioteca e não precisa do YANC; um projeto com processador e sem
o YANC instalado dá `ComponentMissing("yanc")`.

Flags do Verilator (as da AURORA, sem o arquivo `.vlt` de monitores dela, e
com três mudanças): `--binary --main --trace -j 0
-MAKEFLAGS OBJCACHE= -MAKEFLAGS PYTHON3=<Python do bundle> -Wno-fatal
-Wno-TIMESCALEMOD -Wno-DECLFILENAME -Wno-STMTDLY --timing --x-assign fast
--no-trace-top --autoflush +define+YANC_TRACE -CFLAGS -O3 -CFLAGS -march=native -CFLAGS
-fstrict-aliasing -CFLAGS -pipe -CFLAGS -Wno-attributes` (sem `-march=native`
no macOS). As três mudanças:

- `--trace` (VCD) em vez de `--trace-fst`: no Linux e no macOS, o FST do
  Verilator do bundle compila contra lz4 e zlib do sistema, fora da exceção
  do compilador; o Windows segue o mesmo formato;
- `PYTHON3=`: o `make` gerado chama `python3` pelo nome, e sem isso usaria o
  do sistema;
- `--autoflush`: o modelo escreve o `$display` na hora, e não em blocos no
  fim, o que permite mostrar a saída enquanto roda (5.8). Entra com ou sem
  receptor de eventos, menos na simulação rápida.

Na simulação rápida saem `--trace`, `--no-trace-top` e `--autoflush`
(abaixo).

No Linux e no macOS, o script `verilator` roda pelo Perl do
`SystemCompiler`, com `PATH` = o diretório do script (`bin/` do OSS CAD
Suite), depois os diretórios do compilador, e `LC_ALL=C`. No Windows a AURORA
não passa pelo script: roda direto o `verilator_bin.exe` de `ucrt64/bin`, com
`PATH` = esse diretório, depois os do compilador e o `System32`, `LC_ALL=C` e
o `VERILATOR_ROOT` que o script deduziria do próprio caminho. Pelo script, o
`system()` passava cada argumento pelo `cmd.exe` com o escape de shell do
Unix, e um fonte numa pasta com espaço ou acento virava outro caminho; e o
`verilator_bin`, chamado sem o `.exe`, anotava entre as entradas do modelo
(`__verFiles.dat`) um nome que não existe, e o `--skip-identical` nunca
reconhecia um modelo em dia. A primeira compilação do modelo leva dezenas de
segundos. Depois, com os fontes e as flags iguais, o Verilator não gera o
C++ de novo, o `make` não recompila nada e a simulação só roda o executável
(medido no Windows em 2026-10-07, num contador: 0,4 s, contra 9 s antes); o
`verilated_model` conta como `fresh`, porque está em dia.

`SimulationOptions`:

| Campo | Padrão | Efeito |
|---|---|---|
| `simulator` | (obrigatório em `new`) | `Icarus` ou `Verilator` |
| `build_jobs` | `None` (`-j 0`, todos os núcleos) | paralelismo da compilação C++ do Verilator |
| `timeout` | `None` (sem limite) | prazo do passo `simulate` (o `vvp` ou o modelo do Verilator), sem contar elaboração e compilação. Passou dele, a AURORA encerra a simulação e o resultado vem com `status: timed_out` e o que o testbench escreveu até ali (5.8) |
| `fast` | `false` | a simulação rápida, sem onda (abaixo) |

O formato da onda não é opção: o Icarus grava FST (`vvp -fst`) e o
Verilator, VCD (`--trace`), e a onda sai com o nome do `$dumpfile` do
testbench e a extensão do formato. Um `$dumpfile` com outra
extensão é trocado numa cópia do testbench, com as mesmas linhas: o
`<nome>_tb.vcd` que o testbench do `asmcomp` pede vira `<nome>_tb.fst` no
Icarus, e um `saida.fst` vira `saida.vcd` no Verilator. A cópia é a que o
AURORA já simula, `T/instr_<nome>_tb.v` num processador e
`.aurora8/Temp/instr_<testbench>` no projeto (5.3), e o arquivo do usuário não
muda. Uma chamada de `$dumpfile` quebrada em linhas não é trocada, e o
Icarus grava VCD no nome que ela der. `SimulationResult.waveform` traz o
caminho e o formato.

**A simulação rápida.** Com `fast`, a simulação é o Fast Sim da AURORA
(`runFastSim`): roda sem gravar onda, para ver a saída do testbench, as
portas do processador e os testes cocotb na velocidade do simulador. O
testbench Verilog, de `simulate` ou de `simulate_project`, roda no
Verilator, qualquer que seja o `simulator`: o passo `verilate` não leva
`--trace`, `--no-trace-top` nem `--autoflush`, o `-Mdir` é `obj_dir_fast_<topo>`, ao lado
do `obj_dir_<topo>` da simulação com onda (alternar entre as duas não
recompila tudo a cada vez), e o modelo ignora os `$dumpfile` e `$dumpvars`
do testbench (escreve na saída `$dumpvar ignored, as Verilated without
--trace`). A AURORA não injeta o dump padrão nem troca a extensão do
`$dumpfile`; o `+define+YANC_TRACE` continua, porque o testbench que o YANC
gera lê os sinais de simulação do processador para achar o fim do programa.
Um testbench cocotb roda os testes no `simulator` (5.3.2). O resultado vem
com `fast: true`, o simulador que rodou em `simulator` e `waveform: null`.
A AURORA comenta os `$dumpfile` e `$dumpvars` numa cópia do testbench; o
AURORA não precisa: o Verilator 5 do bundle os ignora sozinho. Sem
`--autoflush`, como na AURORA, o `$display` chega em blocos, e não linha a
linha: num testbench que imprime a cada ciclo, o flush de cada linha chegava
a dobrar o tempo da simulação (medido no Linux em 2026-10-12: 1 milhão de
linhas em 1,6 s sem a flag e 3,3 a 3,9 s com ela). O preço é que uma
simulação rápida cancelada, ou que passou do prazo, perde o que ainda
estava no buffer do modelo.

O `vvp` sai com código 0 depois de um `$error`. Por isso as linhas `ERROR:`
e `FATAL:` que ele escreve (as de `$error` e `$fatal`) viram diagnóstico de
erro e a simulação termina como `failed`, com `failed_step: simulate`.
Sem elas, uma simulação que chega ao `$finish` com código 0 é `succeeded`,
mesmo que o circuito esteja errado: conferir o comportamento é olhar as
saídas e a onda. O stdout do testbench (`$display`) fica no `stdout` do passo
`simulate`.

### 5.3 `simulate_project(&Toolchain, &Project, &SimulationOptions, &Control) -> Result<SimulationResult>`

Simula o testbench do projeto, como o botão Wave da AURORA.

1. O testbench é `project.testbench()`. O módulo simulado (`-s`,
   `--top-module`) é `project.testbench_module()`: vem do conteúdo do
   testbench, não só do nome do arquivo.
2. Arquivos: os sintetizáveis do `.spf`, depois o `<nome>.v` gerado de cada
   processador compilado que não esteja na lista, e o testbench por último.
3. Roda na raiz do projeto, ou, se o testbench é o que o YANC gerou para um
   processador (`<nome>/<nome>/<nome>_tb.v`), na pasta de saída dele, a
   única em que ele roda certo (a regra da AURORA). Copia para essa pasta as
   memórias e o `pc_<nome>_mem.txt` de cada processador compilado, que o
   `.v` dele lê pelo nome (`Processor::files_read_by_verilog`).
4. Copia para lá os arquivos que o testbench lê por nome relativo
   (`$readmemb("x")`, `$readmemh("x")`, `$fopen("x", "r")`), a partir da pasta
   do testbench, com duas restrições:
   - nada é escrito fora da raiz: um `$readmemh("../dados/x.hex")` lê, como
     o Icarus faz, o caminho relativo à raiz, e se ele não existe entra em
     `missing_inputs`;
   - um arquivo do usuário na raiz não é sobrescrito: se a raiz já tem um
     `x` diferente, ele fica, a simulação lê esse, e sai um aviso. A AURORA só
     atualiza as cópias que ele mesmo fez, anotadas em
     `.aurora8/Temp/data_copies.txt`.
5. O `$dumpfile` vale com texto entre aspas ou com um `localparam`,
   `parameter` ou `` `define `` de texto (`$dumpfile(ONDA)`). Com uma
   expressão que a AURORA não resolve, nada é injetado, a onda não entra no
   resultado, e `waveform_path` dá `InvalidProject` (antes, o dump injetado e
   o `-fst` dele gravavam FST no `.vcd` do usuário). Um `$dumpfile` sem
   `$dumpvars` dá um aviso: nenhum sinal é gravado e a onda não sai. Um
   `$dumpfile` com a extensão de outro formato vai para uma cópia em
   `.aurora8/Temp/instr_<testbench>`, com a extensão trocada (5.2): `saida.vcd`
   vira `saida.fst` no Icarus.
   Se o testbench não tem `$dumpfile`, simula uma cópia em
   `.aurora8/Temp/instr_<testbench>` com `$dumpfile("<tb>.fst");
   $dumpvars(0, <tb>);` antes do `endmodule` do módulo do testbench
   (comentário e texto entre aspas não contam), em que `<tb>` é esse módulo
   (`<tb>.vcd` no Verilator). O bloco entra sem quebrar linha, e os
   diagnósticos da cópia apontam para o arquivo do usuário, que não muda. A
   profundidade 0 grava todos os sinais da hierarquia, inclusive os do
   módulo testado. A onda injetada não é exigida: um testbench que chega ao
   `$finish` no tempo 0, antes do bloco injetado, termina como sucesso, sem
   onda. Na simulação rápida (5.2), nada disso: o testbench roda como está,
   e não há onda.
6. Roda os mesmos comandos de 5.2, com CWD na raiz e `.vvp`/`obj_dir` em
   `.aurora8/Temp/`, e o mesmo `timeout`. A onda fica na raiz, com o nome do
   `$dumpfile` e a extensão do formato.

Os processadores não são compilados aqui. Chame `build` para cada um antes.

Uma onda VCD do Icarus acima de 100 MB vem com um aviso: um `$dumpfile`
terminado em `.fst` grava os mesmos sinais várias vezes menor. Só acontece
quando a AURORA não troca a extensão do `$dumpfile` (a chamada quebrada em
linhas).

### 5.3.2 Testbench cocotb

Com um testbench `.py`, `simulate_project` roda os testes cocotb no Icarus
ou no Verilator, pelos passos de sempre.
Com `<B>` = `<raiz>/.aurora8/Temp/cocotb/<módulo de teste>` e `<dut>` o módulo
da diretiva (ou o topo):

| Simulador | Passo | Comando | CWD |
|---|---|---|---|
| Icarus | `elaborate` | `iverilog -grelative-include -I <raiz> [-g2012] [-y <SAPHO>] -s <dut> -f <B>/cmds.f -s aurora_cocotb_dump -o <B>/<dut>.vvp <design> <B>/aurora_cocotb_dump.v` | raiz |
| Icarus | `simulate` | `vvp -n [-i] -m <VPI do cocotb> <B>/<dut>.vvp -fst` | raiz |
| Verilator | `verilate` | `perl verilator --cc --exe --build --vpi --public-flat-rw --prefix Vtop -o V<dut> --timescale 1ns/1ps -LDFLAGS <VPI do cocotb> ... --trace --no-trace-top <as flags de 5.2> --top-module <dut> -Mdir <B>/obj_dir_<dut> [-y <SAPHO>] <verilator.cpp do cocotb> <design>` (no Windows, `verilator_bin.exe` no lugar de `perl verilator`) | `<B>` |
| Verilator | `simulate` | `<B>/obj_dir_<dut>/V<dut> --trace --trace-file <raiz>/<módulo de teste>.vcd` | raiz |

`<design>` é o mesmo da simulação de um testbench Verilog (5.3, passo 2),
sem o testbench, e os `pc_<nome>_mem.txt` dos processadores também vão para
a raiz. O `cmds.f` traz `+timescale+1ns/1ps`, o padrão do runner do cocotb,
para os módulos sem `` `timescale `` (no Verilator, `--timescale`). O
`aurora_cocotb_dump` grava `<raiz>/<módulo de teste>.fst` com
`$dumpvars(0, <dut>)`.

No Verilator, o modelo tem o `main` do cocotb (`share/lib/verilator/verilator.cpp`),
que carrega a VPI dele e deixa os testes dirigirem o tempo, e grava a onda
com `--trace` em `<raiz>/<módulo de teste>.vcd`. A VPI do cocotb entra na
ligação: no Windows a estática, `libcocotbvpi_verilator.a`, com `-L<libs>
-lgpi`; no Linux e no macOS a compartilhada, com `-Wl,-rpath,<libs>
-L<libs>`, como no `Makefile.verilator` do cocotb. No Windows, o
`PYTHONPATH` começa por `.aurora8/Temp/cocotb/site/`, com um
`sitecustomize.py` que põe a pasta do Python entre as que o Windows procura
ao carregar DLLs (`os.add_dll_directory`): o Python dentro do modelo não
acha as DLLs das extensões dele pelo `PATH` (o `binascii` carrega a do
`zlib`), e o modelo, ao contrário do `vvp`, não fica na pasta do Python.

Na simulação rápida (5.2) os testes rodam sem onda: no Icarus sem o
`aurora_cocotb_dump` e com `vvp ... -none`; no Verilator com o modelo sem
`--trace`, em `<B>/obj_dir_fast_<dut>`.

O que o simulador precisa para carregar o cocotb vem de uma sonda
(`cocotb_probe.py`) rodada com o Python do componente `cocotb` e guardada
em `.aurora8/Temp/cocotb/probe.json` até o bundle mudar: a VPI para o Icarus,
a VPI e o `verilator.cpp` para o Verilator, a biblioteca do Python, o ponto
de entrada e o `sys.path`. Com isso o simulador recebe `PYGPI_PYTHON_BIN`, `PYTHONPATH` (a pasta do `.py`, a raiz e o
`sys.path` do Python), `TOPLEVEL_LANG=verilog`, `COCOTB_RESULTS_FILE`, as
variáveis do cocotb 2 (`GPI_USERS`, `COCOTB_TOPLEVEL`,
`COCOTB_TEST_MODULES`) ou do 1 (`LIBPYTHON_LOC`, `TOPLEVEL`, `MODULE`), a
pasta das bibliotecas do cocotb no `PATH`, `PYTHONUTF8=1` e
`PYTHONPYCACHEPREFIX` na pasta de cache do usuário: os `.pyc` não vão para o
bundle nem para o projeto.

O `results.xml` vira `SimulationResult::tests` (`TestReport`, 4.4). Cada
teste que falha é um diagnóstico de erro do simulador (`vvp`, ou
`verilator` no Verilator) no `.py`, na linha mais funda do traceback dentro
dele, e a simulação termina `failed`, com `failed_step: simulate`; um `.py`
sem teste nenhum também. Um diagnóstico `info` resume as contagens. A onda
vem em `waveform` mesmo com teste falhando, desde que a simulação tenha ido
até o fim: é nela que se vê a falha.

Erros: `NoCocotbToplevel` sem diretiva e sem topo; `InvalidName` se o nome
do `.py` não é identificador do Python; `ComponentMissing` sem o
componente `cocotb`; `CocotbUnavailable` se a sonda falha ou o cocotb não
traz a biblioteca do simulador. Testado no Windows com o cocotb 2.1.0 e o
Verilator 5.050 do bundle, nos dois simuladores; no Linux e no macOS, com o
2.1.0.dev0 do OSS CAD Suite, não foi rodado.

### 5.3.1 `waveform_path(&Project, Option<&Processor>) -> Result<Utf8PathBuf>`

Onde a simulação grava a onda, sem simular: é o que `aurora wave` abre sem
argumento. Com um processador, a do testbench que o `asmcomp` gerou
(`<nome>/<nome>/<nome>_tb.fst` ou `.vcd`, na pasta de saída); sem, a do
testbench do projeto: o nome do `$dumpfile` dele, ou, sem `$dumpfile`, a
onda que a AURORA injeta na pasta onde ele roda (5.3, passos 3 e 5). Das duas extensões, `.fst` do Icarus e
`.vcd` do Verilator, vale a mais recente (a do Icarus se nenhuma existe
ainda). Com um testbench cocotb, `<raiz>/<módulo de teste>.fst` (Icarus)
ou `.vcd` (Verilator), a mais recente (5.3.2).

Erros: `NoTestbench` sem testbench; `NotBuilt` se o processador ainda não
foi compilado; `InvalidProject` se o `$dumpfile` do testbench é uma
expressão que a AURORA não resolve (5.3, passo 5).

### 5.4 `check(&Toolchain, &Project, &CheckOptions, &Control) -> Result<CheckResult>`

Confere se o Verilog do projeto elabora, sem simular nem sintetizar. Não
gera nada. O design é o conjunto dos sintetizáveis do `.spf`, do
`<nome>.v` gerado de cada processador e dos `.v` de `<raiz>/TopLevel/`
(legado da AURORA), sem testbenches. A biblioteca SAPHO entra por `-y` só se
o projeto tem processadores.

Sem `file`, com `<design>` = esses arquivos:

| Passo | Comando | Quando |
|---|---|---|
| `check_syntax` | `iverilog [-g2012] -tnull -Wall [-y <SAPHO>] <design>` | sempre |
| `check_syntax` | `iverilog [-g2012] -tnull -Wall [-y <SAPHO>] -s <módulo do testbench> <design> <testbench>` | um por testbench registrado |
| `lint` | `verilator --lint-only -Wall --quiet -Wno-fatal [--top-module <topo>] [-y <SAPHO>] <design>` | com `lint` |

O primeiro passo não tem `-s`: o Icarus elabora cada módulo que ninguém
instancia como raiz, então um módulo solto também é verificado, e o projeto
não precisa ter topo. A exceção é a recursão parametrizada (um módulo que se
instancia num `generate`): aí nenhum módulo sobra como raiz, o Icarus diria
"No top level modules", e as raízes vão com `-s`: o topo do projeto, ou os
módulos que só o próprio arquivo cita. As raízes saem de uma contagem de
palavras num passe por arquivo (um projeto de 1500 arquivos leva
milissegundos), e entram em `CheckResult.targets`.

O `iverilog` só avisa de uma macro indefinida ("macro NBITS undefined (and
assumed null)") e segue com a macro vazia; a AURORA trata como erro, com a
explicação de que um `` `define `` só vale para os arquivos depois dele na
lista do projeto, e o passo falha mesmo com o `iverilog` saindo com 0.
Antes, `check` e `sim` passavam com a largura errada, e só a síntese
quebrava. O `-g2012` (SystemVerilog) só entra quando há um `.sv`
no projeto: ele reserva nomes como `bit` e `logic`, que um Verilog-2001 pode
usar como sinal; a simulação segue a mesma regra. Um passo que falha
interrompe os seguintes. O `iverilog` roda com CWD na raiz. O Verilator roda
numa pasta vazia, com `--top-module` quando o projeto tem topo; com
`-Wno-fatal`, os avisos dele não fazem o `check` falhar.

Com `file`, só os módulos daquele arquivo são elaborados como raiz (um `-s`
para cada um), com o design para resolver as instâncias. Se o arquivo é
testbench, o passo é a elaboração dele com o design.

Com `processor`, o design é só o `<nome>.v` gerado do processador, e o
único testbench é o que o build gerou (`<nome>/<nome>/<nome>_tb.v`, se
existe); os passos são os mesmos, e o lint usa o processador como topo. Os
arquivos do `.spf` e os outros processadores ficam de fora. O processador
precisa ter sido compilado (`NotBuilt`).

`CheckOptions` é `#[non_exhaustive]` e implementa `Default`: construa com
`CheckOptions::default()` e atribua os campos.

| Campo | Padrão | Efeito |
|---|---|---|
| `file` | `None` | só os módulos deste arquivo; `None`, o projeto inteiro |
| `processor` | `None` | só este processador e o testbench dele, no lugar do design do projeto |
| `lint` | `false` | acrescenta o `verilator --lint-only` |

```rust
let mut options = CheckOptions::default();
options.lint = true;
let result = check(&toolchain, &project, &options, &Control::default())?;
println!("{}: {:?}", result.targets.join(", "), result.status);
```

`CheckResult.targets` lista os módulos elaborados como raiz: os do design
(ou os do arquivo pedido) e o de cada testbench. O módulo de um testbench é
o de `testbench_module` (4.1): com dois módulos e nenhum com o nome do
arquivo, o único que o outro não instancia.

Com `lint` e sem o Verilator instalado, a verificação roda do mesmo jeito,
sem o passo `lint`, e um diagnóstico de aviso diz que o lint não rodou.

Os processadores não são compilados aqui, nem pelo `aurora check` (5.11):
um processador sem o Verilog gerado num projeto com outro Verilog fica de
fora, com um aviso, e, sem nada para verificar, o erro é `NotBuilt`.

Erros: `EmptyProject` sem nenhum arquivo Verilog registrado e sem
processadores; `InvalidProject` se um sintetizável registrado não existe;
`ComponentMissing` sem o Icarus; `SystemCompilerMissing` com `lint` e sem o
compilador do sistema (Linux e macOS).

### 5.4.1 `hierarchy(&Toolchain, &Project, &HierarchyOptions, &Control) -> Result<HierarchyResult>`

A hierarquia do design depois da elaboração: que módulo instancia qual, com
os nomes das instâncias, os parâmetros resolvidos e os blocos `generate`
expandidos. Elabora com o Icarus os mesmos arquivos do `check` (5.4), mas
grava a imagem (`-o`, sem `-tnull`) e lê dela a árvore. Não compila os
processadores, não grava relatório no histórico (é uma consulta, como
`Project::files`) e não gera nada fora da pasta de trabalho.

Com `<design>` = os arquivos do `check` e `<T>` = `<raiz>/.aurora8/Temp/hierarchy`,
refeita a cada chamada:

| Passo | Comando | Quando |
|---|---|---|
| `elaborate` | `iverilog [-g2012] [-y <SAPHO>] -o <T>/design.vvp <design>` | com design |
| `elaborate` | `iverilog [-g2012] [-y <SAPHO>] -s <módulo do testbench> -o <T>/tb-<n>.vvp <design> <testbench>` | um por testbench registrado |
| `elaborate` | `iverilog [-g2012] -y <SAPHO> -s <nome>_tb -o <T>/proc-<nome>.vvp <saída>/<nome>.v <saída>/<nome>_tb.v` | um por processador compilado |

Como no `check`, o design vai sem `-s`, e o Icarus elabora como raiz cada
módulo que ninguém instancia; com recursão parametrizada, as raízes vão com
`-s`, pela mesma regra. Ao contrário do `check`, uma elaboração que
falha não interrompe as outras: a árvore do design aparece mesmo com um
testbench quebrado. O cancelamento para tudo. Um processador nunca compilado
fica de fora, sem erro.

Com `processor`, o design é só o `<nome>.v` gerado do processador, e o
único testbench é o que o build gerou; o processador precisa ter sido
compilado (`NotBuilt`).

`HierarchyOptions` é `#[non_exhaustive]` e implementa `Default`:

| Campo | Padrão | Efeito |
|---|---|---|
| `processor` | `None` | só este processador e o testbench dele, no lugar do projeto |

`HierarchyResult` tem a estrutura comum (6.1, com um `elaborate` por
elaboração e um `icarus_image` em `artifacts` para cada uma) e:

| Campo | Conteúdo |
|---|---|
| `design` | `Elaboration` do design, ou `null` sem Verilog de design |
| `testbenches` | uma `Elaboration` por testbench: os registrados, na ordem do `.spf`, e depois o de cada processador compilado que não esteja entre eles |

`Elaboration`:

| Campo | Conteúdo |
|---|---|
| `testbench` | o testbench elaborado; `null` no design |
| `processor` | o processador, quando o testbench é o que o build dele gerou |
| `status` | como esta elaboração terminou |
| `roots` | `ModuleInstance[]`: os módulos que ninguém instancia, com o que eles instanciam; vazio numa falha |
| `diagnostics` | os erros e avisos desta elaboração (também no `diagnostics` do resultado) |

`ModuleInstance`:

| Campo | Conteúdo |
|---|---|
| `name` | o nome da instância (`dut`); numa raiz, o do módulo. Dentro de blocos `generate`, `begin` ou `fork`, com o nome deles na frente (`op_add.my_add`): os blocos não viram nós |
| `module` | o módulo instanciado |
| `file`, `line` | onde o módulo é definido |
| `instance_file`, `instance_line` | onde está a instância, no módulo de cima; `null` numa raiz |
| `library` | o módulo vem da biblioteca SAPHO |
| `children` | as instâncias de dentro, na ordem do fonte |

A árvore sai do `.vvp`: cada escopo elaborado é uma linha `.scope` com o
tipo, a instância, o módulo, os índices de arquivo e linha da instância e da
definição e o escopo pai; os arquivos vêm da tabela `:file_names`. Funções e
tarefas ficam de fora. Num processador, a árvore mostra as unidades da
biblioteca SAPHO que o programa realmente usa, porque o processador só
instancia essas.

```rust
let result = hierarchy(&toolchain, &project, &HierarchyOptions::default(), &Control::default())?;
for tb in &result.testbenches {
    for root in &tb.roots {
        println!("{}: {} instâncias", root.module, root.children.len());
    }
}
```

Erros: os do `check` (`EmptyProject`, `InvalidProject`, `ModuleNotFound`
para um testbench sem módulo que dê para usar); `ProcessorNotFound` e
`NotBuilt` com `processor`; `ComponentMissing` sem o Icarus, ou sem o YANC
num projeto com processadores.

### 5.5 `synthesize(&Toolchain, &Project, &DesignTarget, &Control) -> Result<SynthesisResult>`

Síntese de visualização (a do RTLViewer da AURORA), sem mapear para FPGA.

| Alvo | Topo | Arquivos |
|---|---|---|
| `DesignTarget::TopLevel` | o módulo de topo (`project.top_module()`, ver 3.4) | o design de `check` (5.4) |
| `DesignTarget::Processor(nome)` | o processador | `<nome>/<nome>/<nome>.v` |

Quando o projeto tem processadores, todo `.v` da biblioteca SAPHO
(`yanc/SAPHO/` do bundle) que não é testbench entra primeiro. O Yosys roda
na raiz, e o Verilog de cada processador lê as memórias pelo nome, relativo
a ela: antes da síntese, as memórias e o `pc_<nome>_mem.txt` de cada
processador compilado vão para a raiz (`Processor::files_read_by_verilog`).

Script gravado em `.aurora8/Temp/synth/<topo>/yosys_script.ys`:

```
read_verilog -setattr src "<arquivo>"     (um por arquivo)
hierarchy -top <topo>
proc
tee -q -o "<dir>/check.log" check
setundef -zero
opt_clean -purge
write_json "<dir>/hierarchy.json"
tee -q -o "<dir>/stat.json" stat -json -top <topo>
```

Cada `read_verilog` leva `-I "<raiz>"` e, num `.sv`, `-sv`. Execução: `yosys
-q -l <dir>/yosys.log -s <dir>/yosys_script.ys`, com CWD na raiz do projeto,
como a simulação: um `$readmemh("rtl/rom.mif")` relativo à raiz vale nas
duas (o Yosys também procura na pasta do arquivo que lê).
`SynthesisResult.modules` lista os módulos do netlist que dá para desenhar:
sem as caixas-pretas (um módulo vazio vira `blackbox` e fica no netlist
mesmo fora da árvore do topo).

O `check` do Yosys vai só para o `check.log` (`tee -q`): no processador SAPHO
ele avisa de cada fio sem driver da biblioteca. Do `check.log`, viram aviso
(com o arquivo e a linha do primeiro `source:`) só o laço combinacional
("found logic loop") e os drivers em conflito, que nem o Icarus nem a
síntese apontam.

Um módulo do design que se instancia sem `if`, `case` nem `for` no corpo
nunca para: o Icarus recusa, e o Yosys geraria um nível atrás do outro até
ser interrompido. A síntese falha antes de rodar o Yosys, com um erro que
diz o módulo. Se o Yosys cai (sinal 11) e um módulo do design se instancia,
um erro relaciona a queda com a recursão.

O `write_json` do Yosys grava cada byte fora do ASCII como `\uFFFFFFxx`, o que
estraga os caminhos com acento nos atributos `src` e faz o `read_json` do
próprio Yosys recusar o netlist. Depois da síntese, a AURORA devolve esses
bytes ao texto UTF-8 que eram.

`SynthesisResult.statistics` é o que o `stat` do Yosys contou no mesmo
netlist (`SynthesisStatistics`, em `stats.rs`): módulos, fios, bits de fio,
fios e bits com nome do usuário, memórias, bits de memória, processos,
células e as células por tipo, da seção `design` do `stat -json`, que soma
os submódulos (as instâncias de submódulo ficam fora das células). Uma
contagem que o Yosys não informou é `None`, diferente de zero. As células
são as genéricas do Yosys, depois de `proc` e `opt_clean`: nada aqui estima
LUT, DSP, ocupação ou temporização. `None` quando a síntese falhou ou o
`stat.json` desta síntese não existe.

### 5.6 `render_schematic(&Toolchain, &netlist, módulo, &SchematicOptions, &Control) -> Result<SchematicResult>`

1. Confere que o módulo está no netlist; um módulo fora da árvore do topo
   não é sintetizado, e o erro é `ModuleNotFound`, com os módulos do
   netlist em `available` (antes, `InvalidNetlist`, como se o netlist
   estivesse quebrado).
2. Passo `graph`: `yosys -q -s <módulo>.show.ys`, CWD `<dir>`, com o script
   `read_json "<netlist>"` e `show -format dot -prefix <módulo> [-width] <módulo>`.
   Gera `<dir>/<módulo>.dot`.
3. Passo `render`: `dot -Tsvg <módulo>.dot -o <módulo>.svg`, CWD `<dir>`. Uma
   mensagem `Error:` do `dot` conta como falha mesmo com código 0. No Linux,
   a AURORA grava `<dir>/fonts.conf` a partir do modelo do OSS CAD Suite e
   passa `FONTCONFIG_FILE` e `FONTCONFIG_PATH`: o `dot` usa as fontes do
   bundle, com o cache em `<dir>/fontconfig-cache`. Sem isso ele leria a
   configuração de fontes do sistema, e o SVG mudaria de máquina para
   máquina. No macOS e no Windows o pacote não traz fontes e o `dot` usa as
   do sistema.

`SchematicOptions::bus_widths` (padrão: sim) escreve a largura dos
barramentos nas arestas. O visual é o do Graphviz, não o do netlistsvg da
AURORA.

Não há teto de ligações: qualquer módulo desenha. O `dot` fica lento com
muitas ligações e portas compartilhadas (40 instâncias de um registrador de 8
bits ligadas ao mesmo `clk` e à mesma entrada, 160 ligações, levaram mais de
40 s), e o prazo dele é a única proteção:

| Opção | Padrão | O que faz |
|---|---|---|
| `timeout` | `Some(SCHEMATIC_TIMEOUT)`, 60 s | prazo do passo do `dot`; vencido, o passo termina como `timed_out` |

Nomes de módulo com caracteres fora de `[A-Za-z0-9_.-]` (por exemplo
`$paramod\processor\NUBITS=23`) viram `_` no nome dos arquivos.

### 5.6.1 O RTL viewer: o módulo `rtl_viewer`

O esquemático interativo do netlist, o mesmo que o Studio mostra numa aba e
a extensão do VS Code num webview, num HTML autocontido que abre em qualquer
navegador. O viewer é o pacote `packages/rtl-viewer/` do repositório; o build
dele gera o modelo `rtl-viewer.html`, com o código, o CSS e o worker do layout
embutidos, que o componente `rtl-viewer` do bundle traz. Não roda ferramenta.

| Função | O que faz |
|---|---|
| `rtl_viewer::template(&Toolchain) -> Result<Utf8PathBuf>` | o modelo: o de `AURORA_RTL_VIEWER` (`TEMPLATE_ENV`), se definida, ou o do componente. `ComponentMissing("rtl-viewer")` sem o componente; `ToolchainIncomplete` sem o arquivo |
| `rtl_viewer::write(&modelo, &netlist, título) -> Result<Utf8PathBuf>` | preenche o modelo com o netlist (`SynthesisResult::netlist`) e grava `rtl-viewer.html` na pasta dele, sobrescrevendo o de antes. Os `<` do JSON viram `\u003c`, para um `</script>` dentro de uma string do netlist não fechar o elemento. `ToolchainIncomplete` se o modelo não tem os lugares do netlist e da configuração |
| `open_with_system(&caminho) -> Result<()>` | abre um arquivo com o programa padrão do sistema (`xdg-open`, `open`, o Explorer), sem esperar por ele. É uma das exceções à regra de só rodar ferramentas do bundle, com o compilador do Verilator (2.4), o Quartus (2.5) e o `taskkill` (5.8): o navegador do usuário não vem nele. O abridor é chamado pelo nome, sem caminho fixo, e herda o ambiente da AURORA. `Spawn` se o abridor não pôde rodar |

### 5.7 `open_waveform(&Toolchain, &onda, &ViewerOptions) -> Result<RunningProcess>`

`surfer-aurora <onda> [-c <arquivo.sucl> | -s <arquivo.surf.ron>]`, destacado,
CWD na pasta da onda. Retorna na hora. O stdout e o stderr vão para
`<onda>.log` (`contador_tb.fst.log`) numa pasta oculta, nunca ao lado da
onda: `.aurora8/Temp/surfer/` do projeto que contém a onda; com a onda fora de
projeto, a pasta de cache do usuário (`$XDG_CACHE_HOME/aurora/surfer` ou
`~/.cache/aurora/surfer` no Linux, `~/Library/Caches/aurora/surfer` no macOS,
`%TEMP%\aurora\surfer` no Windows). Abrir a mesma onda de novo sobrescreve o
log; `RunningProcess::log_file()` diz onde ele ficou.
Não recebe `Control`: o surfer-aurora não é um passo da operação, e o
cancelamento não o fecha.

O surfer-aurora recebe só as variáveis de ambiente que uma aplicação gráfica precisa
(`DISPLAY`, `WAYLAND_DISPLAY`, `HOME`, `XDG_*`, `DBUS_SESSION_BUS_ADDRESS` no
Linux; `USERPROFILE`, `APPDATA`, `LOCALAPPDATA` no Windows).

`RunningProcess`:

| Método | Faz |
|---|---|
| `id()` | PID |
| `try_wait()` | `Some(Termination)` se já terminou, sem bloquear |
| `wait_timeout(d)` | espera até `d` |
| `ensure_started(d)` | erro `ProcessExitedEarly`, com o fim do log, se o processo terminou dentro de `d` |
| `wait()` | bloqueia até terminar |
| `kill()` | encerra (não é erro se já tinha terminado) |
| `log_file()` | o log |

Soltar o `RunningProcess` não fecha o Surfer. Sem display, o Surfer morre em
menos de um segundo com código 101; `ensure_started` de um ou dois segundos
logo depois de abrir detecta isso (a CLI usa 1,5 s).

`ViewerOptions::working_dir` troca a pasta onde o Surfer roda (padrão: a da
onda). É nela que ele procura `.surfer/mappings/`, os tradutores de valor de
um layout preparado (5.7.1).

### 5.7.1 O layout dos processadores SAPHO: `wave_layout(&onda) -> Result<Option<WaveLayout>>`

Monta, em memória, o estado do Surfer que mostra cada processador SAPHO da
onda como a AURORA mostrava. O YANC já põe na onda as variáveis
do programa, o PC (`valr2`) e a linha do fonte (`linetabs`); o layout dá
nome a esses números e os arruma em grupos:

| Grupo | O que tem |
|---|---|
| `Top-level` | os sinais da raiz do testbench (o `clk` e o `rst`) |
| `<instância> (<processador>)` | um por processador, aberto |
| ↳ (sem grupo) | `clk`, `rst` e `itr` do núcleo |
| ↳ `I/O` | `req_in N`, `input N`, `out_en N`, `output N`, com `N` o número da porta, lido do nome do sinal (`in_sim_2` é `input 2` mesmo que seja a única entrada usada) |
| ↳ `Instructions` | `Assembly (<proc>)`: o `valr2` como a instrução (`LOD_V main_x 2`); `C± (<proc>)`: o `linetabs` como a linha do fonte |
| ↳ `Variables` | `int` com sinal, `float` como o número real que o YANC calcula na simulação, complexo como `re imi`; cada array num grupo fechado |
| ↳ `Flags` | fechado: pilha de dados e de instruções (`pointeri` em degrau), ULA (`delta_int`, `delta_float`) |

O processador é o escopo com `valr2` e `linetabs`. O nome dele sai do
subescopo `p_<nome>.core` que o YANC cria; sem ele, do pai `<nome>_inst`, da
raiz `<nome>_tb` ou da instância. As tabelas vêm da pasta de saída do
processador com esse nome no projeto que contém a onda (fora de projeto, da
pasta da onda, se ela tem `pc_<nome>_mem.txt`):

| Tabela do YANC | Tradutor | Sinal |
|---|---|---|
| `trad_opcode.txt` (`<índice> <mnemônico> <operando>`) | `aurora_asm_<proc>`, `Bits` = largura do `valr2` | `valr2` |
| `src_lines.txt` (`<linha> <texto>`; -1, -2 e -3 viram `0xFFFFF`, `0xFFFFE`, `0xFFFFD` em 20 bits) | `aurora_src_<proc>` | `linetabs` |
| os valores dos complexos no corpo da onda | `aurora_complex`, um `0b<bits> <re> <im>i` por valor | `comp_me3_*`, `comp_arr_me3_*` |

Linha de tabela sem texto sai: o Surfer recusa o tradutor inteiro por ela.
Sem a tabela, o sinal aparece como número (`Unsigned`, `Signed`).

Uma tabela (ou o `pc_<nome>_mem.txt`) gravada depois da onda é de outro
build: o processador foi compilado de novo depois da simulação, e o assembly
e a linha do C± sairiam deslocados. Nenhuma tabela é usada, o PC e a linha
aparecem como números, e `WaveProcessor::outdated` diz por quê, para a
interface avisar ("simule de novo").

O tradutor dos complexos lê o corpo da onda inteiro, só nos sinais
complexos: num VCD, em bytes e só nas linhas `b<bits> <id>` de um id
complexo; num FST, pelo filtro de sinais do leitor. Para em
`MAX_COMPLEX_VALUES` (16384) valores distintos. Um complexo que muda a cada clock passa disso, e os valores
seguintes aparecem em binário. Numa onda de 100 MB com um complexo, a
varredura custa alguns segundos antes de abrir o Surfer.

Detalhes dos nomes: o rótulo da linha do fonte é `C (<proc>)` num
processador em C e `C± (<proc>)` em C±; o elemento de um array leva o índice
lido inteiro depois do último `_e_` (o `asmcomp` usa `%04d`, então o 10000
tem cinco dígitos e fica no mesmo grupo); a variável de uma função com
`_v_` no nome (`get_v_x`) é desfeita pelos pares do `prog_log.txt` do build;
um escopo com `.` no nome (o identificador escapado `\u.pa `) continua um
componente só no caminho do `.surf.ron`; `delta_int` e `delta_float`, que o
Icarus grava como `real`, ficam sem formato de bits.

Uma onda sem processador SAPHO (um projeto só de Verilog) recebe só o grupo
`Top-level`, com os sinais da raiz do testbench, como a AURORA; o resto do
design fica na hierarquia do Surfer. O FST é lido pela crate `fst-reader`,
e a hierarquia dele vira o mesmo texto do cabeçalho de um VCD.
`Ok(None)`: a onda não é VCD nem FST (o GHW abre sem layout), ou não tem
nem sinal na raiz nem processador.
`WaveLayout`: `state` (o `.surf.ron`), `mappings` (`MappingTranslator`:
`name`, `content`), `processors` (`WaveProcessor`: `instance`,
`processor`, `variables`, `assembly`, `source`, `outdated`), `testbench`
(o escopo de cima da onda, que dá o nome dos arquivos em `wave/`) e
`selection` (a escolha de sinais que o layout seguiu).

### 5.7.2 `prepare_wave_layout(&onda) -> Result<Option<PreparedLayout>>`

Grava o layout em `.aurora8/Temp/surfer/<onda sem extensão>-<hash do caminho>/` do projeto que
contém a onda (fora de projeto, na pasta de cache do usuário, como o log do
Surfer): `<onda>.surf.ron` e os tradutores em `.surfer/mappings/<nome>`, sem
os de antes. `ViewerOptions::with_layout(&preparado)` abre com ele:

```rust
if let Some(layout) = aurora_core::prepare_wave_layout(&onda)? {
    open_waveform(&toolchain, &onda, &ViewerOptions::with_layout(&layout))?;
}
```

O formato do `.surf.ron` é o do surfer-aurora do bundle (base Surfer 0.7.0):
os identificadores dos sinais são marcadores, e o Surfer reacha cada sinal
pelo caminho e pelo nome.

Numa onda de testbench do projeto, o estado aberto é o layout salvo no
projeto, `wave/<testbench>.surf.ron` (`PreparedLayout::saved`, 5.7.4), e
não o da pasta temporária; os tradutores continuam nela.

### 5.7.3 A escolha de sinais: `wave_signals(&Toolchain, &Project, &Control) -> Result<WaveSignals>`

A Wave Configuration da AURORA (`js/wave/wave_config_manager.ts` e
`testbench_instrumenter.ts`): por testbench, o que a simulação do projeto
grava e o que o layout mostra. O nome do testbench é o do módulo dele
(`wave_testbench(&projeto)`), que é também o escopo de cima da onda
(`wave_testbench_of(&onda)`).

| Função | Faz |
|---|---|
| `wave_signals(&toolchain, &projeto, &control)` | elabora o testbench do projeto com o Icarus (a elaboração da `hierarchy`, em `.aurora8/Temp/wave/signals.vvp`) e devolve a árvore: escopos (instâncias, blocos `generate` e blocos com nome, cada um com o caminho, o módulo e se é um processador SAPHO) e sinais (nome, caminho, largura, `reg`/`wire`/`integer`/`real`, a direção quando é porta), com a escolha gravada e os itens dela que a árvore não tem (`unknown`). Uma elaboração que falha vem sem árvore, com os erros em `diagnostics` |
| `read_selection(&projeto, módulo)` | a escolha de `wave/<módulo>.json`; vazia sem o arquivo |
| `write_selection(&projeto, módulo, &itens)` | grava a escolha, em ordem e sem o que um escopo dela já cobre; vazia, apaga o arquivo |
| `SignalScope::contains`, `scope`, `without` | conferir um caminho na árvore, achar um escopo e tirar um item da escolha (um escopo escolhido que contém o item dá lugar ao resto dele) |
| `selection_file`, `layout_file` | `wave/<módulo>.json` e `wave/<módulo>.surf.ron` |

```json
// wave/mux_tb.json
{ "schema": 1, "signals": ["mux_tb.dut.u0", "mux_tb.sel"] }
```

Com escolha, `simulate_project` (5.3) elabora o testbench (para conferir os
itens e achar os processadores SAPHO) e simula uma cópia dele: o primeiro
`$dumpvars` vira `begin $dumpvars(0, <itens>); $dumpvars(1, <processador>); end`,
os outros viram comentário, e sem `$dumpvars` a escolha entra depois do
`$dumpfile`, ou no dump que a AURORA injeta. As linhas do arquivo continuam as
mesmas. Um processador que a escolha não cobre entra com o escopo dele
(nível 1: as variáveis, o I/O, o PC e a linha), porque o layout precisa.
Um item que o design não tem mais é um aviso no resultado, com o arquivo da
escolha. A simulação rápida e o testbench cocotb não mudam. `wave_layout`
(5.7.1) lê a mesma escolha e mostra só os sinais dela, um grupo por escopo
(`WaveLayout::selection`).

### 5.7.4 O layout salvo: `saved_layout(&onda, &layout) -> Result<Option<SavedLayout>>`

O layout que o usuário arruma no Surfer fica no projeto,
`wave/<testbench>.surf.ron`. `saved_layout` decide o estado que o Surfer
recebe, para a onda de um testbench do projeto (`None` fora de projeto):

- sem o arquivo, ele nasce com o layout gerado (`layout.state`);
- com o arquivo como a AURORA o gravou da última vez (o SHA-256 fica em
  `.aurora8/wave/generated.json`), ele é refeito com o layout gerado agora;
- com o arquivo mudado, porque o Surfer salvou nele (Ctrl+S), ele fica como
  está: `SavedLayout::customized`, e `state` é o conteúdo dele.

`prepare_wave_layout` abre o Surfer com `-s <arquivo>`, e o Ctrl+S salva
nele. O Surfer carrega o estado e depois a onda passada na linha de comando,
mantendo o layout: o caminho da onda gravado no estado não importa, e o
layout continua valendo se o projeto mudar de pasta. A aba de onda do
Studio faz o mesmo pelo cliente web (`state_save_url_set`).
`reset_saved_layout(&projeto, testbench)` apaga o arquivo e o registro: a
próxima abertura volta ao gerado.

### 5.8 Cancelamento, prazo e saída ao vivo: `Control`

Toda operação que executa ferramentas (`build`, `build_processors`, `check`,
`hierarchy`, `simulate`, `simulate_project`, `synthesize`, `render_schematic`,
`wave_signals`, `fpga::build`, `fpga::program`, `flows::run`,
`flows::choose_signals`) recebe um
`&Control` como último argumento; em `build_processors`, antes de
`on_result`. Ele leva o pedido de cancelamento e, se houver, quem recebe os
eventos. `Control::default()` não cancela e não avisa nada. As consultas
`fpga::cables` (o `quartus_pgm -l`) e `verilog::read_interfaces` (o Yosys,
também dentro de `fpga::prepare` e de `add_verilog`) rodam sem ele, até o fim.

| Tipo | O que é |
|---|---|
| `CancelToken` | o pedido de cancelamento. `cancel()` pede, de qualquer thread; `is_cancelled()` consulta. Os clones são o mesmo pedido. Um pedido feito não se desfaz: para a operação seguinte, crie outro. `CancelToken::from(Arc<AtomicBool>)` liga o pedido a uma flag que outro código marca, como a de um tratador de sinal |
| `Control` | `Control::new()`, `.with_cancel(token)` e `.on_event(receptor)`; `cancel_token()` e `is_cancelled()` consultam |
| `Event` | o que aconteceu, avisado na hora: `StepStarted`, `Output`, `StepFinished` |
| `Stream` | de qual saída veio uma linha: `stdout` ou `stderr` |

`Event` em JSON, com o tipo no campo `event` (schema em
`docs/schema/events.json`):

| `event` | Campos | Quando |
|---|---|---|
| `step_started` | `step`, `tool`, `command` (a `Invocation`) | o processo do passo vai ser iniciado |
| `output` | `step`, `tool`, `stream`, `line`, `diagnostic` | a cada linha que o processo escreve, sem o fim de linha |
| `step_finished` | `step`, `tool`, `termination`, `duration_ms` | o processo do passo terminou |

`diagnostic` diz se a linha volta como `Diagnostic` no resultado (seção 7):
tudo do stderr, todo o stdout dos compiladores (YANC e `iverilog`) e, no
stdout de uma simulação, as linhas do próprio simulador (`$finish called
at`, `VCD info:` ou `FST info:`, o `ERROR:` de um `$error`). `false` é saída
do programa, como o `$display` do testbench. A decisão é linha a linha
(`diagnostics::is_message`). Os eventos só adiantam o que chega depois no
resultado (`StepReport`). As linhas de stdout e de stderr chegam na ordem em
que a AURORA as leu, que pode não ser a ordem exata em que a ferramenta as
escreveu.

O receptor roda na thread da operação, no meio dela: deve ser rápido
(mandar o evento por um canal, escrever uma linha) e não pode chamar outra
operação da AURORA.

Numa GUI, a operação roda numa thread de trabalho, e a interface fica com um
clone do `CancelToken` (`Control`, `CancelToken` e `Event` são
`Send + Sync`):

```rust
use std::sync::mpsc;
use aurora_core::*;

// toolchain e project abertos como na seção 1.
let cancel = CancelToken::new();
let (events, received) = mpsc::channel();
let control = Control::new()
    .with_cancel(cancel.clone())
    .on_event(move |event| {
        let _ = events.send(event.clone());
    });

let worker = std::thread::spawn(move || {
    let options = SimulationOptions::new(Simulator::Icarus);
    simulate_project(&toolchain, &project, &options, &control)
});
// Na thread da interface: mostra cada linha; o botão de parar chama
// cancel.cancel().
for event in received {
    if let Event::Output { line, .. } = event {
        println!("{line}");
    }
}
let result = worker.join().expect("a thread terminou")?;
```

**O que acontece com os processos.** A AURORA lê stdout e stderr linha a
linha, em duas threads, e confere o cancelamento e o prazo a cada 20 ms,
mesmo enquanto a ferramenta escreve sem parar. Encerrar é encerrar tudo o
que a ferramenta iniciou: o `iverilog` roda o `ivlpp` e o `ivl`, o
Verilator roda o `make`, que roda o compilador C++.

- **Unix:** cada passo roda num grupo de processos próprio. A AURORA manda
  SIGTERM ao grupo e, se o processo não sair em 1 s, SIGKILL. Fora do grupo
  da AURORA, os filhos não recebem o Ctrl+C do terminal: quem os encerra é o
  AURORA, ao ver o pedido.
- **Windows:** cada passo roda num Job Object com
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` (`ProcessJob`): encerrar é fechar o
  job, e tudo o que a ferramenta iniciou termina junto. O que sobrar quando o
  passo acaba também termina, e o mesmo vale se o processo da AURORA morrer,
  mesmo à força: o sistema fecha o handle do job. Se o sistema recusar o
  job, o `taskkill /T /F` do `System32` encerra a árvore. O `taskkill.exe` só
  serve para encerrar ([BUNDLE.md](BUNDLE.md), seção 4); os outros programas
  do sistema que a AURORA roda são o compilador do Verilator (2.4), o Quartus
  (2.5) e o abridor de `open_with_system` (5.6.1).
  Nenhuma ferramenta abre janela de console (`hide_console`): se a AURORA tem
  console (um terminal, ou o que quem o criou lhe deu), ela divide esse
  console, e o Ctrl+C chega a ela também; se não tem (o Studio), ela leva
  `CREATE_NO_WINDOW` e ganha um console próprio, sem janela, ao custo de um
  `conhost.exe`, uns 16 ms por ferramenta. Quem cria processo fora do Core,
  como o Studio ao chamar a CLI, usa o mesmo `ProcessJob` e o mesmo
  `hide_console`.

O passo encerrado termina com `Termination::Cancelled` ou
`Termination::TimedOut`, e a operação, com `Status::Cancelled` ou
`Status::TimedOut` e `failed_step` nesse passo. É `Ok`, não `Err`: a
operação rodou, e o resultado traz os passos que chegaram a rodar, com o que
as ferramentas escreveram até ali. Cancelada, nenhum passo seguinte começa;
um pedido feito antes da operação devolve `cancelled` sem passo nenhum, com
`failed_step` no primeiro, que não rodou. Só uma simulação que deu certo
traz `waveform` no resultado: uma cancelada ou que passou do prazo não traz,
mesmo que o arquivo da onda exista.

**Prazo.** A simulação tem prazo (`SimulationOptions::timeout`, 5.2), só no
passo `simulate`: é onde um código do usuário pode não terminar, como um
testbench sem `$finish`; sem prazo por padrão. O esquemático tem o dele, no
passo `render` do `dot` (`SchematicOptions::timeout`, 60 s por padrão, 5.6).

**O custo da saída ao vivo.** Num pipe, o `vvp` guarda o stdout em buffer:
o `$display` só sai quando o buffer enche ou no fim, e se perde se o
processo for morto. Com um receptor de eventos, a AURORA roda o `vvp` com `-i`, que
tira o buffer, ao custo de uma escrita por linha. Sem receptor, fica o
buffer, que é mais rápido. O Verilator compila com `--autoflush` (5.2), com
ou sem receptor, porque trocar a flag conforme o receptor recompilaria o
modelo; a simulação rápida, que tem modelo próprio, compila sem.

A AURORA lê o pipe em blocos de até 64 KiB e passa adiante, de uma vez, as
linhas inteiras de cada bloco, e não uma linha por vez: num testbench que
imprime sem parar, o caminho de cada linha até o receptor custava mais que
a simulação escrevê-la (medido em 2026-10-12, 2 milhões de linhas: 1,28 s
de sobrecarga lendo linha a linha, 0,25 s em blocos). Quem escreve devagar
continua chegando linha a linha. Se o receptor atrasa, a ferramenta espera,
em vez de a fila crescer na memória.

### 5.9 Relatório e histórico: o módulo `history`

Depois de uma operação, a interface chama `history::record` com o que cada
fase devolveu, e a AURORA grava um relatório no histórico do projeto. Nenhuma
operação grava sozinha: quem decide é quem chama. Os fluxos de
`aurora_core::flows` (5.11), que a CLI, o Studio e o `aurora serve` usam,
gravam depois de `build`, `check`, `simulate` e `synthesize`.

```rust
use aurora_core::history::{self, Operation};

let started = std::time::SystemTime::now();
let builds = aurora_core::build_processors(&toolchain, project.buildable_processors(), &opts, OnFailure::Stop, &control, |_| {})?;
let sim = aurora_core::simulate_project(&toolchain, &project, &options, &control)?;
let operation = Operation::new("aurora sim", started)
    .with_builds(&builds)
    .with_simulation(&sim);
let record = history::record(&project, &toolchain, &operation)?;   // run-000042
```

```
<projeto>/.aurora8/reports/
  run-000001/
    report.txt     o relatório, para gente
    record.json    o que a comparação usa (RunRecord)
  sequence         o último número reservado
```

O número só cresce, e o de um relatório apagado não volta. A pasta é
montada com outro nome e renomeada no fim: quem lê nunca vê um relatório
pela metade. Duas operações gravando ao mesmo tempo no mesmo projeto ficam
com números seguidos.

Apagar é em dois passos, para a interface mostrar e confirmar antes:
`plan_cleanup` escolhe, sem apagar, e `remove` apaga exatamente a lista
escolhida. Um relatório gravado entre os dois não sai sem ter sido mostrado.

```rust
use aurora_core::history::{self, Cleanup};

let doomed = history::plan_cleanup(&project, &Cleanup::KeepLatest(10))?;
// mostrar `doomed` e confirmar
let removed = history::remove(&project, &doomed)?;
```

| Função | Faz |
|---|---|
| `record(&Project, &Toolchain, &Operation) -> Result<RunRecord>` | grava e devolve o relatório |
| `list(&Project) -> Result<Vec<RunSummary>>` | os relatórios, do mais novo para o mais antigo; um `record.json` ilegível aparece com `readable: false` |
| `load(&Project, id)`, `latest(&Project)` | um relatório (`run-000042` ou `42`), o mais novo |
| `report_text(&Project, id)`, `report_path(&Project, id)` | o `report.txt` e onde ele fica |
| `view(&Project, id) -> Result<ReportView>` | o relatório pedido, ou o mais novo sem `id`, com o texto e o caminho do `report.txt` (`aurora report show`, `history.show`) |
| `clean(&Project, ids) -> Result<CleanOutcome>` | apaga os relatórios do plano e conta os que ficaram (`aurora report clean`, `history.clean`) |
| `compare_reports(&Project, atual, referência)` | escolhe e compara, como `aurora report compare`; os dois vêm do histórico do projeto, então se comparam mesmo depois de a pasta mudar de lugar; sem parte nenhuma em comum (uma simulação contra uma síntese), `NotComparable` |
| `previous_comparable(&Project, &RunRecord)` | o anterior mais novo que se compara em pelo menos uma parte, de preferência um que terminou bem (uma simulação que estourou o prazo só serve sem outra) |
| `compare(&referência, &atual) -> Result<RunComparison>` | compara dois relatórios; não lê nem roda nada |
| `compare_metric(Option<u64>, Option<u64>)` | compara dois números |
| `parse_id(texto)` | `42` ou `run-000042` para `run-000042` |
| `plan_cleanup(&Project, &Cleanup) -> Result<Vec<String>>` | os relatórios que `Cleanup::All`, `KeepLatest(n)` ou `Reports(ids)` escolhe, do mais antigo para o mais novo; `ReportNotFound` se um id pedido não existe, e então nada é escolhido |
| `remove(&Project, &ids) -> Result<Vec<String>>` | apaga e devolve os que saíram; um que já não existe é pulado. Grava em `sequence` o maior número publicado antes de apagar, e renomeia cada pasta para um nome oculto antes de apagá-la |

`RunRecord` tem `metadata` (`RunMetadata`: comando, status, duração, versão
da AURORA, um hash da raiz do projeto, bundle, topo, a máquina e o contexto da
síntese e da simulação, com hashes dos fontes e das entradas), `synthesis`
(as `SynthesisStatistics` da síntese) e `simulation` (`SimulationTimings`:
compilação, execução e total em milissegundos, e o tempo simulado do
`$finish` do `vvp` em femtossegundos).

A comparação segue a do Alpha-Solar. Relatórios de projetos diferentes não
se comparam (`NotComparable`). A síntese se compara quando os dois têm
estatísticas do mesmo topo; a simulação, quando os dois têm tempos do mesmo
simulador e testbench. O resto que difere vira aviso em `warnings`, sem
invalidar a comparação: fontes, entradas, versões das ferramentas, onda
ligada num só, tempo simulado diferente, a máquina (um hash do sistema, da
arquitetura, do processador, do número de processadores e do nome da
máquina), simulação que não terminou bem, versão da AURORA. Cada número
comparado é um `MetricComparison`: os dois valores, a `Change`
(`unchanged`, `increased`, `decreased`, `added`, `removed`,
`not_comparable`), a diferença e a porcentagem da referência, que é `None`
de zero para outro número. Os tipos de célula vêm da maior mudança para a
menor e, no empate, pelo nome.

A máquina vem do que o sistema informa sem rodar outro programa: o `uname`
no Linux e no macOS, `/etc/os-release`, `/proc/cpuinfo` e `/proc/meminfo` no
Linux. O que não dá para saber assim (o modelo do processador e a memória
fora do Linux) fica `None`.

### 5.10 Placas FPGA: `fpga::prepare`, `fpga::build` e `fpga::program`

As placas, o `fpga.json` e as regras das ligações estão em
[FPGA.md](FPGA.md).

As placas são um recurso experimental (5.10.1). As funções de `fpga`
funcionam com ele desligado, para quem usa o Core como biblioteca e para
os testes; quem as oferece ao usuário (a CLI, o Studio) chama
`experimental::require(Feature::Fpga)` antes.

`fpga::prepare(Option<&Toolchain>, &Project) -> Result<Prepared>` lê o
`fpga.json` e a placa, acha o topo (o do `fpga.json`; senão o do projeto;
senão o processador, se for um só), lê as portas dele do Verilog e confere
as ligações. Devolve as ligações bit a bit (`resolved`, com as notas de
ajuste) e o Verilog do topo da placa (`board_top`).

`fpga::build` compila o projeto para a placa pelo Quartus:

1. `prepare`; os processadores precisam estar compilados (a CLI compila
   antes). O Verilog deles cita as memórias pelo nome, relativo ao
   diretório de quem o compila: as memórias e a tabela do PC vão para a
   pasta da placa, a do projeto do Quartus, antes da compilação. Que o
   Quartus procura ali é suposição, ainda não conferida numa compilação;
2. grava em `fpga::build_dir(projeto, placa)`, `<raiz>/.aurora8/fpga/<placa>/`,
   o topo da placa (`aurora_board_top.v`), o `.qsf` (a FPGA, os fontes da
   síntese mais o topo da placa, um pino por bit com o padrão de I/O, o
   `.rbf` e o `.svf` sem compressão) e o `.sdc` (um `create_clock` por
   clock da placa ligado ao topo);
3. roda, na pasta da placa, os passos `synthesize` (`quartus_map`), `fit`,
   `bitstream` (`quartus_asm`) e `timing` (`quartus_sta`), todos com
   `Tool::Quartus`, os mesmos comandos que a interface do Quartus roda;
4. depois do `fit`, confere o `.pin` contra a placa: cada bit no pino, na
   direção e no padrão de I/O dela, posto pela atribuição e não pelo
   Fitter. Uma diferença reprova o `fit` (um diagnóstico por bit) antes do
   `.sof`;
5. terminada, grava o `aurora-build.json`: o SHA-256 do `.sof` e de cada
   entrada (fontes, memórias `.mif`, `fpga.json`, definição da placa).

`FpgaBuildResult`, além dos campos comuns (6.1): `board`, `top`, `dir`,
`quartus` (a instalação usada), `bitstream` (o `.sof`, quando terminou),
`resources` (as linhas do resumo do Fitter: nome, usado, disponível e se é
detalhe da linha de cima), `timing` (por clock, a frequência pedida, a Fmax
e as folgas de setup e de hold no pior canto, e `met`, sem folga negativa) e
`notes` (os ajustes das ligações). Folga negativa não muda o `status`. Um
diagnóstico repetido igual (o aviso de tempo vem uma vez por canto) aparece
uma vez.

`fpga::program(&Toolchain, &Project, cable: Option<&str>, &Control) ->
Result<FpgaProgramResult>` grava o `.sof` da última `build` pelo Quartus
Programmer: um passo `program`/`quartus`, `quartus_pgm -c <cabo> -m jtag -o
"p;output_files/aurora_board_top.sof@<posição>"` na pasta da placa, com a
posição da FPGA na cadeia JTAG da placa. Sem `cable`, o primeiro de
`fpga::cables(&Toolchain)` (o `quartus_pgm -l`). Antes, confere o
`aurora-build.json` contra o projeto de agora, pelo conteúdo, e recusa com
`StaleBitstream` (cada diferença) um `.sof` velho, trocado ou de outra
placa. `fpga::bitstream_state(&Toolchain, &Project) -> Result<BitstreamState>`
faz a mesma conferência sem gravar: `bitstream`, `built_at_ms` e `reasons`
(vazio: pronto).

`fpga::modules` lista os módulos que podem ir para a placa e
`fpga::top_interface` dá as portas de um (ou do topo do projeto), para
montar as ligações antes de estarem certas. `Prepared::connections` traz
cada ligação como texto (`target`, `source`), como o `aurora fpga check`
mostra.

`fpga::connect(Option<&Toolchain>, &Project, board: Option<&str>,
&Reconnect) -> Result<FpgaConfig>` muda uma ligação a partir da placa e
grava o `fpga.json` (o método `fpga.connect`): `Reconnect { signal, bit,
port }` diz o sinal da placa, o bit (`None`: o sinal inteiro, como um
display) e a porta (`PortChoice { port, bit, invert }`, com o primeiro bit
dela; `None` solta o elemento). Sem `fpga.json`, parte das ligações pelo
nome na `board` dada. A regra fica em `FpgaConfig::reconnect(&Board,
&ModuleInterface, &Reconnect)`: o bit da placa fica só com a porta pedida, a
saída que já ia a outros lugares continua neles, as outras ligações não
mudam e tudo volta escrito em faixas contínuas, com uma lista para a porta
que vai a vários sinais; o resultado passa pelo `resolve` antes de gravar,
e um problema é `InvalidFpgaConfig`, com um por linha.
`FpgaProgramResult` traz `board`, `bitstream`, `cable`, `position`,
`quartus` e os campos comuns, menos `artifacts`.

Conferido em 2026-10-09 com o Quartus Prime 25.1 Lite no Windows e uma
DE2-115 (compilar e gravar). As mensagens e os relatórios também foram
conferidos em relatórios publicados do Quartus II 13.1 e 14.0 e do Quartus
Prime 17.1, 18.0 e 18.1.

### 5.10.2 A placa virtual: o módulo `virtual_board`

O projeto simulado com os periféricos da placa, como se estivesse gravado
nela ([FPGA.md](FPGA.md#placa-virtual-aurora-vboard)). A placa e as ligações
são as de 5.10: `fpga::prepare_with(toolchain, project, config)` é o
`prepare` com uma configuração dada, e `FpgaConfig::by_name(board, top)`
liga as portas do topo com o nome e a direção de um sinal da placa. Cada
placa diz o que é cada sinal em `Board::peripherals` (`fpga::Peripheral`:
`switches`, `buttons`, `leds`, `seven_segment`, `vga`, `ps2`, `uart`),
conferido junto com o resto do JSON dela.

| Função | Faz |
|---|---|
| `virtual_board::layout(Option<&Toolchain>, &Project, &BuildOptions) -> Result<BoardLayout>` | a placa sem compilar: cada periférico, ligado ao projeto ou não, com as larguras e a polaridade dos sinais, a porta do topo em cada bit (`LayoutSignal::ports`: `a`, `b[3]`, `!rst`; vírgulas quando mais de uma porta lê o mesmo bit), os relógios e os ajustes das ligações |
| `virtual_board::build(&Toolchain, &Project, &BuildOptions, &Control) -> Result<BoardBuild>` | grava o topo da placa, o `main` em C++ e o runtime dos periféricos em `.aurora8/Temp/vboard` e compila pelo Verilator (passo `verilate`). Verilog que não compila é `Ok` com `status: failed` e os diagnósticos |
| `virtual_board::run(&Toolchain, &Project, &BoardBuild, &RunOptions, &Receiver<BoardInput>, &mut dyn FnMut(SessionEvent), &Control) -> Result<SessionOutcome>` | roda o executável até o cancelamento ou o `$finish`. As entradas (`BoardInput`: `set`, `key`, `text`, `bytes`, `pause`, `resume`, `step`, `speed`) chegam pelo canal; os `SessionEvent` (`ready`, `state`, `frame`, `output`) vão para quem chama, na thread dele |
| `BoardLayout::check(&BoardInput) -> Result<()>` | confere que a entrada vai para um periférico ligado que a recebe (`virtual_board_device`, com os que recebem) e que o valor de chaves e botões cabe nele (`virtual_board_value`, com a largura) |

`BuildOptions::board` vale num projeto sem `fpga.json`. `RunOptions::new(limit,
fps)`: `limit` 1 é o tempo real, 0 o mais rápido possível; `fps` limita os
quadros de vídeo. O estado (`BoardState`) sai cerca de 30 vezes por segundo,
com o tempo simulado, a velocidade, o brilho de cada LED e segmento (0 a
255) e os bytes que cada UART transmitiu; o quadro (`Frame`) sai em PNG, só
quando a imagem muda. No protocolo são os métodos `vboard.layout`,
`vboard.run` e `vboard.input` e as notificações `vboard_ready`,
`vboard_state`, `vboard_frame` e `vboard_output`
([PROTOCOL.md](PROTOCOL.md)).

Erros novos: `virtual_board_not_built` (`run` sem o executável) e
`virtual_board_device`.

### 5.10.1 Recursos experimentais: o módulo `experimental`

Partes da AURORA que ainda mudam muito ficam desligadas até o usuário ligar.
`Feature` lista os recursos (hoje, `Feature::Fpga`, a placa FPGA, e
`Feature::Assistant`, o assistente de IA: `aurora chat`, `aurora mcp` e a vista
Assistente do Studio), com `id()` (`"fpga"`, `"assistant"`, o identificador
do arquivo e da CLI), `name()` e `description()`, em inglês; `Feature::ALL`
tem todos.

O que está ligado fica em `experimental.json`, na pasta de configuração do
AURORA do usuário (`%APPDATA%\aurora` no Windows, `$XDG_CONFIG_HOME/aurora` ou
`~/.config/aurora` no Linux, `~/Library/Application Support/aurora` no macOS),
como `{"enabled": ["fpga"]}`. `settings_path()` devolve o arquivo (`None`
sem `APPDATA` ou `HOME`).

| Função | Faz |
|---|---|
| `is_enabled(Feature) -> bool` | o recurso está ligado |
| `require(Feature) -> Result<()>` | `ExperimentalDisabled` se ele está desligado |
| `states() -> Vec<FeatureState>` | cada recurso, se está ligado e a descrição |
| `info() -> ExperimentalInfo` | o arquivo e cada recurso (`aurora experimental`, `experimental.list`) |
| `set_enabled(Feature, bool) -> Result<()>` | liga ou desliga, gravando o arquivo de forma atômica e criando a pasta |

A leitura falha fechada: sem a pasta, sem o arquivo ou com ele ilegível,
nada está ligado. Gravar mantém os identificadores que esta versão não
conhece.

### 5.11 Os fluxos compostos, a entrega, a busca do bundle e o contrato

As operações de 5.1 a 5.10 são as peças. O que uma interface faz de fato é
compô-las: compilar os processadores antes de simular, registrar no `.spf`
o que o YANC gerou, ler as portas de saída, gravar o relatório, abrir a
onda. Essa composição morava duas vezes, na CLI e no Studio, e divergia;
agora mora aqui, uma vez: a CLI chama estas funções direto, e o despacho
do `aurora serve`, que o Studio e a extensão usam, chama as mesmas.

**`aurora_core::flows`.** Um [`FlowRequest`] (`build`, `check`, `hierarchy`,
`simulate`, `synthesize`, `schematic`, `fpga_build`, `fpga_program`; em
JSON, etiquetado por `flow`) entra em `flows::run(&Toolchain, &mut
Project, &FlowRequest, &FlowOptions, progress, &Control) ->
Result<FlowOutcome>`. As decisões de interface são parâmetros de
[`FlowOptions`]: o texto do comando para o relatório (`command`), se a
onda abre em janela ou não (`wave: WavePolicy`), quem abre o RTL viewer
(`viewer_opener`). O `progress` recebe [`Progress`]: a fase que começou
(`Phase`), cada processador compilado, a onda aberta. O [`FlowOutcome`]
tem `status` explícito (`succeeded`, `failed`, `cancelled`, `timed_out`,
`crashed`, `incomplete`) e `succeeded`, e as partes que o fluxo produziu
(`builds`, `registered`, `check`, `lint_missing`, `hierarchy`, `not_built`,
`simulation`, `outputs`, `synthesis`, `schematic`, `schematic_error`,
`rtl_viewer`, `rtl_viewer_error`, `wave`, `wave_error`, `report`,
`report_error`, `fpga`, `fpga_program`). As regras
que antes divergiam estão fixas: o `build` continua depois de um
processador que falha e mostra todos os erros; o `check` não compila
processadores; a simulação para no primeiro build que falhar
(`simulation: null`); um módulo inexistente no esquemático é
`schematic_error`, não falha do fluxo; a onda que não abre é `wave_error`.
`flows::run_with` é o `run` com o bundle pedido só depois das conferências
que não precisam dele: o recurso experimental do fluxo, os processadores
que o pedido nomeia e o `build` de um projeto sem processadores, que nem
precisa do bundle. Quem chama passa como acha o bundle, e o erro dele sai
com o tipo de quem chama; é o que a CLI e o despacho do `aurora serve`
chamam, para seguirem a mesma ordem. `flows::open_wave_with` faz o mesmo
com a onda: confere que ela existe antes de pedir o bundle.
`flows::open_wave` abre uma onda com a política de layout
(`generated`, `none`, `reset`, `file`) e devolve [`WaveOpened`]; uma onda
que não existe, do projeto ou um arquivo, é `no_waveform`.
`flows::choose_signals` muda a escolha de sinais: `all` a esvazia, `add` e
`remove` acrescentam e tiram, `set` troca a escolha inteira; os caminhos
são conferidos na árvore do testbench elaborado. O `aurora learn` tem o seu
em `aurora_learn::flows`, e `aurora_learn::layout::with_exercise_layout` dá à
onda de um exercício o layout da correção, quando o pedido quer o gerado.
`FlowRequest::Learn` é a correção de um exercício como fluxo, com o
resultado em `FlowOutcome::learn`: quem a roda é o servidor da AURORA, que
linka o aurora-learn, e o `flows::run` do core responde `unsupported`.

**`aurora_core::delivery`.** A entrega da saída ao vivo para uma interface
com fila: o `Dispatcher` recebe os eventos do `Control` e o `Progress` e
entrega [`Notification`] em lotes (`phase`, `events`, `output_omitted`,
`build`, `install`), no máximo a cada 30 ms ou 500 eventos, com um teto de
linhas (20 000 de uma vez e depois 5 000 por segundo; o que passa vira
`output_omitted`, no lugar onde estaria) e um canal limitado: com a fila cheia, a ferramenta espera o cliente até
500 ms por linha (`Limits::send_timeout`) e, se a entrega continuar parada,
a linha fica de fora e entra em `output_omitted`, em vez de a memória crescer. Era o código de
`jobs.rs` do Studio; agora o Studio, o `aurora serve` e o `aurora mcp` usam o
mesmo.

**`aurora_core::locate`.** A busca do bundle a partir de outro programa, com
a ordem que o Studio usava: a pasta declarada (`settings`), a variável
(`environment`), o bundle que contém o executável (`exe`, onde o instalador
põe o Studio), as pastas padrão do instalador, o `aurora` do `PATH` e, por
último, o bundle ao lado do executável, como a CLI. Quem chama monta o [`Lookup`] com o que
leu do ambiente e das preferências (o core não lê o ambiente para achar o bundle), e `resolve`
devolve o bundle com a origem (`Found`); `describe` monta o
[`ToolchainInfo`] da tela de ferramentas do Studio e do `toolchain.info` do
`aurora serve`: o que o `aurora tools` mostra, em dados (o `aurora tools` monta a
tela dele a partir daqui).
`Project::status()` devolve o retrato do projeto ([`ProjectStatus`]: o
`aurora status` e a árvore do Studio) e `Processor::output_ports` as portas
de saída lidas pelo nome do arquivo.

**O contrato, no crate `aurora-protocol`.** Os pedidos e as respostas do
`aurora serve` moram num crate próprio, acima do core, do `aurora-learn` e do
`aurora-installer`; o core não conhece o protocolo. O crate tem [`Request`]
(um método por operação, com grupo e ponto: `project.status`, `flow.run`,
`history.list`, `update.check`, ...), [`Response`] (o valor em si, sem
etiqueta: os tipos do core, o `UpdateReport` e o `InstallReport` do
instalador e os tipos do `aurora-learn`), [`ServerInfo`], [`ShutdownReason`]
e os tipos de resposta que nenhuma função do core devolve (`TopOutcome`,
`NameCheck`, `ProcessorRemoval`, `WaveLayoutInfo`). Os resultados de
funções do core ficam no core, ao lado delas: `RemoveOutcome` em `files`,
`ReportView` e `CleanOutcome` em `history`, `ExperimentalInfo` em
`experimental`.
`request_schema()` gera o schema dos pedidos na vista de quem desserializa
(um campo com padrão é opcional) e `response_schema()` o das respostas e
notificações; a CLI os grava em `docs/schema/protocol-request.json` e
`protocol-response.json`, e os dois `gen-aurora-types` (Studio, VS Code)
geram os tipos TypeScript deles. A macro `methods!` declara cada método
uma vez, com o nome, o tipo do resultado e as marcações (se só lê, se mexe
na instalação, se vira ferramenta de agente): dela saem `METHODS`, a lista
dos métodos, `METHOD_INFO` e `result_schema()`, o schema do `result` de
cada um, que um teste do `aurora serve` confere contra o que o servidor
devolve; uma variante de `Request` sem linha nela não compila. Os três schemas geram a
referência legível, [PROTOCOL.md](PROTOCOL.md). O `aurora
serve` é o transporte deste contrato por stdio, e o `aurora mcp` o
adaptador para um agente ([CLI.md](CLI.md#a-api-para-outros-programas-aurora-serve-e-aurora-mcp)).

**`ErrorInfo`.** O erro pronto para atravessar uma fronteira: `code` (o
`AuroraError::code()`, ou um código de quem chamou), `message`, `hint` (o
que fazer, quando quem mostra sabe; o core não o preenche) e `data` (os
campos da variante, quando há: `available`, `files`, `path`, `reason`,
...), os dois últimos omitidos no JSON quando não há. É o `error` do
`--json` da CLI, o `error.data` do `aurora serve` e o que vai dentro de um
`FlowOutcome` (`schematic_error`, `rtl_viewer_error`, `wave_error`).

### 5.12 Controle de versão: o crate `aurora-git`

Fica fora do core porque roda programas do sistema, o git e o `curl`, que
não vêm no bundle (a regra está no CONTRIBUTING.md). É o recurso
experimental `git`; no protocolo, os métodos `git.*` e `github.*`.

**O git.** `Git::locate(&pastas)` acha o `git` nas pastas que quem chama
passa (o `aurora serve` passa as do `PATH` e, no Windows, as do instalador
do Git for Windows) e confere a versão (2.26 ou mais nova). Cada operação
recebe o `&Git` e a pasta do projeto e vale para o repositório inteiro que
a contém; os caminhos são relativos à pasta de topo, com `/`.

| Função | O que faz |
|---|---|
| `status` | ramo, upstream, à frente e atrás, arquivos mudados (os do projeto primeiro), identidade, remotos e operação pela metade; `repository: None` fora de repositório |
| `diff` | o diff unificado desde o último commit, cortado em 1 MiB |
| `read_file` | o conteúdo de um arquivo no índice, no `HEAD` ou num commit, com corte em 4 MiB e aviso de binário |
| `commit` | o commit de exatamente os arquivos escolhidos (`CommitOptions.paths`), no estado do disco; o que estava preparado e não foi escolhido continua preparado |
| `discard` | volta arquivos ao último commit; a versão descartada vai para a lixeira do sistema |
| `init` | cria o repositório na pasta do projeto, com `.aurora8/` e a pasta de saída de cada processador no `.gitignore` |
| `log`, `commit_files` | o histórico em páginas, com os commits que só existem nesta máquina marcados, e os arquivos de um commit |
| `branches`, `switch` | os ramos; trocar, criar e seguir um ramo remoto, recusando quando a troca apagaria mudança sem commit |
| `fetch`, `pull`, `push` | o remoto do ramo (ou o `origin`): o pull só em avanço direto, o push publica o ramo que ainda não tem remoto |
| `set_identity`, `set_remote`, `remove_remote` | nome e e-mail dos commits, global ou do repositório; criar um remoto ou corrigir o endereço dele, e tirá-lo |

Nenhuma chamada espera alguém digitar: sem credencial, o git falha, e a
falha volta como `Error::Authentication`; um remoto que não existe no
endereço, não responde ou não dá acesso à conta, como
`Error::RemoteUnreachable`, com o endereço, para a interface oferecer a
correção. Os erros têm código próprio (`git_missing`,
`git_identity_missing`, `git_switch_blocked`, `git_remote_unreachable`,
`git_push_rejected`, `git_diverged`, ...), com os campos em `Error::data`.

**O GitHub.** `github::login_start` e `github::login_poll` fazem o login
pelo fluxo de dispositivo (o aplicativo OAuth "sapho" da nipscernlab, o
mesmo da AURORA 7, ou o de `AURORA_GITHUB_CLIENT_ID`). O token vai para o
gerenciador de credenciais do git (`git credential approve`), e a AURORA
não o grava em lugar nenhum; `github::account` o pede de volta ao git e
pergunta ao GitHub de quem ele é, e `github::logout` o tira. As chamadas
ao GitHub vão pelo `curl` do sistema, com o token no stdin.

---

## 6. Resultados

Todo resultado implementa `Serialize`, e o JSON da CLI é essa serialização.
O JSON Schema de cada tipo é gerado do próprio tipo e fica em
`docs/schema/`, nos `$defs` do schema do comando da CLI que o escreve:
`BuildResult` em `build.json`, `CheckResult` em `check.json`,
`SimulationResult` em `sim.json`, `SynthesisResult` e `SchematicResult` em
`synth.json`, `HierarchyResult` na raiz de `hierarchy.json` (não em `$defs`), `MovedPath` em
`move.json`, `Event` em `events.json`. Um teste confere que os arquivos são
o que os tipos geram.

### 6.1 Estrutura comum

| Campo | Tipo | Conteúdo |
|---|---|---|
| `status` | `Status` | `succeeded`, `failed`, `crashed`, `incomplete`, `cancelled`, `timed_out` |
| `failed_step` | `Option<Step>` | o passo que falhou ou que a AURORA encerrou; num cancelamento pedido antes de um passo começar, esse passo, que não rodou |
| `steps` | `Vec<StepReport>` | um por programa executado, na ordem |
| `diagnostics` | `Vec<Diagnostic>` | mensagens interpretadas de todos os passos |
| `artifacts` | `Vec<Artifact>` | arquivos esperados, com `fresh` |
| `duration_ms` | `u64` | quanto a operação levou, do começo ao fim, com a preparação da AURORA |

`CheckResult` não tem `artifacts` e tem `targets`, os módulos elaborados
como raiz (5.4). `HierarchyResult` tem `design` e `testbenches`, e o
`status` de cada elaboração dentro deles (5.4.1).

`Status`:

| Valor | Quando |
|---|---|
| `succeeded` | todos os passos com código 0 e todo artefato obrigatório gerado agora |
| `failed` | um passo terminou com código diferente de 0, ou o `vvp` escreveu `ERROR:` ou `FATAL:` (5.2) |
| `crashed` | um passo morreu por sinal (Unix) ou exceção NTSTATUS (Windows): defeito da ferramenta |
| `incomplete` | os passos deram certo, mas falta artefato obrigatório |
| `cancelled` | o cancelamento foi pedido (`CancelToken`): o passo que rodava foi encerrado e os seguintes não rodaram (5.8) |
| `timed_out` | um passo passou do prazo e a AURORA o encerrou; numa simulação, é o testbench que não chega ao `$finish` (5.8) |

Os passos param no primeiro que falhar e quando o cancelamento é pedido. A
exceção é `hierarchy`, em que cada elaboração roda mesmo que uma anterior
tenha falhado (5.4.1).

### 6.2 Passos por operação

| Operação | `Step` / `Tool` |
|---|---|
| `build` C± | `compile`/`cmmcomp`, `assemble`/`asmcomp` |
| `build` C | `compile`/`cppcomp`, `assemble`/`asmcomp` |
| `simulate*` Icarus | `elaborate`/`iverilog`, `simulate`/`vvp` |
| `simulate*` Verilator | `verilate`/`verilator`, `simulate`/`verilator` |
| `check` | `check_syntax`/`iverilog` (um para o design, um por testbench), `lint`/`verilator` (com `lint`) |
| `hierarchy` | `elaborate`/`iverilog` (um para o design, um por testbench) |
| `synthesize` | `synthesize`/`yosys` |
| `render_schematic` | `graph`/`yosys`, `render`/`dot` |
| `fpga::build` | `synthesize`/`quartus` (`quartus_map`), `fit`/`quartus` (`quartus_fit`), `bitstream`/`quartus` (`quartus_asm`), `timing`/`quartus` (`quartus_sta`) |
| `fpga::program` | `program`/`quartus` (`quartus_pgm`) |

`preprocess`/`cpppp` e `pre_assemble`/`appcomp` só aparecem nos relatórios
de builds feitos com o YANC até a 6.x.

`StepReport` guarda `command` (programa, argumentos, CWD, ambiente),
`termination`, `stdout`, `stderr` e `duration_ms`. Com ele dá para reproduzir
qualquer passo à mão.

`Termination` em JSON: `{"kind": "exited", "value": 1}`,
`{"kind": "signaled", "value": 11}`, `{"kind": "exception", "value": ...}`,
`{"kind": "cancelled"}`, `{"kind": "timed_out"}`, `{"kind": "unknown"}`.
`cancelled` e `timed_out` são processos que a própria AURORA encerrou (5.8);
`signaled` é um processo que morreu por um sinal que não veio da AURORA.

### 6.3 Artefatos

`fresh` compara o horário de modificação antes e depois da operação: um
arquivo que sobrou de uma execução anterior aparece com `fresh: false`.
Obrigatórios aparecem sempre; intermediários, só se existirem.

| `ArtifactKind` | Arquivo | Produzido por |
|---|---|---|
| `assembly` | `O/<nome>.asm` (`O` = `<nome>/<nome>/`, a pasta de saída) | `build` |
| `verilog` | `O/<nome>.v` | `build` |
| `data_memory` | `O/<nome>_data.mif` | `build` |
| `instruction_memory` | `O/<nome>_inst.mif` | `build` |
| `testbench` | `O/<nome>_tb.v` | `build` |
| `preprocessed_source` | `pp.cpp` (só nos relatórios do YANC até a 6.x) | `build` C |
| `program_counter_map` | `O/pc_<nome>_mem.txt` | `build` |
| `source_translation` | `O/src_lines.txt` | `build` |
| `opcode_translation` | `O/trad_opcode.txt` | `build` |
| `compiler_log` | `O/prog_log.txt` | `build` |
| `pre_assembler_log` | `O/app_log.txt` | `build` |
| `icarus_image` | `<trabalho>/<topo>.vvp`; na hierarquia, `hierarchy/design.vvp`, `hierarchy/tb-<n>.vvp`, `hierarchy/proc-<nome>.vvp` | `simulate*` Icarus, `hierarchy` |
| `verilated_model` | `<trabalho>/obj_dir_<topo>/V<topo>`; na simulação rápida, `obj_dir_fast_<topo>` | `simulate*` Verilator |
| `waveform` | nome do `$dumpfile`, ou o da onda injetada (5.3) | `simulate*` |
| `simulation_output` | `O/output_<n>.txt` | `simulate*` |
| `netlist` | `synth/<topo>/hierarchy.json` | `synthesize` |
| `synthesis_statistics` | `synth/<topo>/stat.json` (não obrigatório) | `synthesize` |
| `schematic_graph` | `synth/<topo>/<módulo>.dot` | `render_schematic` |
| `schematic` | `synth/<topo>/<módulo>.svg` | `render_schematic` |
| `board_top` | `.aurora8/fpga/<placa>/aurora_board_top.v` (não obrigatório) | `fpga::build` |
| `quartus_project` | `.aurora8/fpga/<placa>/aurora_board_top.qsf` (não obrigatório) | `fpga::build` |
| `sram_object` | `.aurora8/fpga/<placa>/output_files/aurora_board_top.sof` | `fpga::build` |
| `raw_binary` | `output_files/aurora_board_top.rbf` (não obrigatório) | `fpga::build` |
| `serial_vector_format` | `output_files/aurora_board_top.svf` (não obrigatório) | `fpga::build` |

### 6.4 Exemplo: build com erro

Um elemento de `results` na saída real de
`aurora -C com_erro build -p conta --json` (caminhos encurtados, bundle em
`/opt/aurora/toolchain`, com o YANC 7.1, artefatos e o `duration_ms` cortados):

```json
{
  "processor": "conta",
  "language": "cmm",
  "status": "failed",
  "failed_step": "compile",
  "frequency_mhz": 100,
  "clocks": 2000,
  "steps": [
    {
      "step": "compile",
      "tool": "cmmcomp",
      "command": {
        "program": "/opt/aurora/toolchain/yanc/bin/cmmcomp",
        "args": ["/home/eu/projetos/com_erro/conta/conta.cmm",
                 "-o", "/home/eu/projetos/com_erro/conta/conta", "--lang=en"],
        "cwd": "/home/eu/projetos/com_erro/conta",
        "env": [],
        "inherit": []
      },
      "termination": { "kind": "exited", "value": 1 },
      "stdout": "",
      "stderr": "Error on line 15: c'mon dude, declare the variable 'total' properly!\n",
      "duration_ms": 3
    }
  ],
  "diagnostics": [
    {
      "tool": "cmmcomp",
      "severity": "error",
      "message": "c'mon dude, declare the variable 'total' properly!",
      "file": "/home/eu/projetos/com_erro/conta/conta.cmm",
      "line": 15,
      "column": null,
      "raw": "Error on line 15: c'mon dude, declare the variable 'total' properly!"
    }
  ],
  "artifacts": [
    { "kind": "assembly", "path": "/home/eu/projetos/com_erro/conta/conta/conta.asm", "required": true, "fresh": true },
    { "kind": "verilog", "path": "/home/eu/projetos/com_erro/conta/conta/conta.v", "required": true, "fresh": false }
  ]
}
```

O `.asm` aparece como `fresh` porque o `cmmcomp` o abre para escrita antes de
achar o erro; o `.v` não, porque o `asmcomp` nem rodou.

### 6.5 Exemplo: simulação

Exemplo de `aurora -C soma sim -p soma --json`, campo `simulation` (caminhos
encurtados, bundle em `/opt/aurora/toolchain`, sem o `duration_ms`). No Linux e no macOS, o
`iverilog` e o `vvp` do bundle são lançadores em bash, que a AURORA roda pelo
`/bin/bash` com `PATH=/usr/bin:/bin` (9.1):

```json
{
  "top": "soma_tb",
  "simulator": "icarus",
  "fast": false,
  "status": "succeeded",
  "failed_step": null,
  "steps": [
    {
      "step": "elaborate",
      "tool": "iverilog",
      "command": {
        "program": "/bin/bash",
        "args": ["/opt/aurora/toolchain/oss-cad-suite/bin/iverilog",
                 "-grelative-include", "-I", "/home/eu/projetos/soma/soma/soma",
                 "-y", "/opt/aurora/toolchain/yanc/SAPHO", "-s", "soma_tb", "-o", "/home/eu/projetos/soma/.aurora8/Temp/soma/soma_tb.vvp",
                 "/home/eu/projetos/soma/soma/soma/soma.v", "/home/eu/projetos/soma/.aurora8/Temp/soma/instr_soma_tb.v"],
        "cwd": "/home/eu/projetos/soma/soma/soma",
        "env": [["PATH", "/usr/bin:/bin"]],
        "inherit": []
      },
      "termination": { "kind": "exited", "value": 0 },
      "stdout": "",
      "stderr": "",
      "duration_ms": 144
    },
    {
      "step": "simulate",
      "tool": "vvp",
      "command": {
        "program": "/bin/bash",
        "args": ["/opt/aurora/toolchain/oss-cad-suite/bin/vvp", "-n", "/home/eu/projetos/soma/.aurora8/Temp/soma/soma_tb.vvp", "-fst"],
        "cwd": "/home/eu/projetos/soma/soma/soma",
        "env": [["PATH", "/usr/bin:/bin"]],
        "inherit": []
      },
      "termination": { "kind": "exited", "value": 0 },
      "stdout": "FST info: dumpfile soma_tb.fst opened for output.\nInfo: end of program!\n/home/eu/projetos/soma/.aurora8/Temp/soma/instr_soma_tb.v:61: $finish called at 985000 (1ps)\n",
      "stderr": "",
      "duration_ms": 66
    }
  ],
  "diagnostics": [
    { "tool": "vvp", "severity": "info", "message": "FST info: dumpfile soma_tb.fst opened for output.",
      "file": null, "line": null, "column": null, "raw": "FST info: dumpfile soma_tb.fst opened for output." },
    { "tool": "vvp", "severity": "info", "message": "$finish called at 985000 (1ps)",
      "file": "/home/eu/projetos/soma/soma/soma/soma_tb.v", "line": 61, "column": null,
      "raw": "/home/eu/projetos/soma/.aurora8/Temp/soma/instr_soma_tb.v:61: $finish called at 985000 (1ps)" }
  ],
  "artifacts": [
    { "kind": "icarus_image", "path": "/home/eu/projetos/soma/.aurora8/Temp/soma/soma_tb.vvp", "required": true, "fresh": true },
    { "kind": "waveform", "path": "/home/eu/projetos/soma/soma/soma/soma_tb.fst", "required": true, "fresh": true },
    { "kind": "simulation_output", "path": "/home/eu/projetos/soma/soma/soma/output_0.txt", "required": false, "fresh": true }
  ],
  "waveform": { "path": "/home/eu/projetos/soma/soma/soma/soma_tb.fst", "format": "fst" },
  "outputs": ["/home/eu/projetos/soma/soma/soma/output_0.txt"],
  "missing_inputs": [],
  "tests": null
}
```

---

## 7. Diagnósticos

`Diagnostic` tem `tool`, `severity`, `message`, `file`, `line`, `column` e
`raw` (o texto original, com as linhas de continuação do Verilator). Os três
campos de localização são opcionais.

| Ferramenta | Formato reconhecido | Arquivo |
|---|---|---|
| `cmmcomp`, `asmcomp` (e o `appcomp` dos relatórios antigos) | `Error on line N: ...`, `Syntax error on line N. ...`, `Heads up on line N: ...`, `Warning on line N: ...`, `Error: ...`, `Info: ...`, `Error in function f: ...` (sem linha); `asmcomp`: `Error: line N of file 'x' ...` | o fonte do processador (preenchido pela AURORA), ou o arquivo de dados citado |
| `cppcomp` | `<arquivo>:<linha>: error: ...`, `<ferramenta>: ...` | o fonte, ou o cabeçalho incluído onde está o erro |
| `iverilog`, `vvp` | `<arquivo>:<linha>: error: ...`, `<arquivo>:<linha>: syntax error`, `<arquivo>: No such file or directory`, `ERROR: ...`, `FATAL: ...` | o que a ferramenta cita |
| `verilator` | `%Error: <arquivo>:<linha>:<coluna>: ...`, `%Warning-<CÓDIGO>: ...` (o código vai para o fim da mensagem entre colchetes); erros do `g++` no formato C | o que a ferramenta cita |
| `yosys` | `<arquivo>:<linha>: ERROR: ...`, `ERROR: ...`, `Warning: ...`, com ou sem espaço depois dos dois-pontos; a gravidade no começo da linha vem antes do local (o aviso de latch traz um `proc_dlatch.cc:542:` do Yosys no meio da mensagem) | o que a ferramenta cita |
| `dot` | `Error: <arquivo>: syntax error in line N ...`, `Warning: ...` | o `.dot` citado |
| `quartus` | `Error (N): ... File: <arquivo> Line: <linha>`, `Critical Warning (N): ...`, `Warning (N): ...`; o número vai para o fim da mensagem entre colchetes (`[10161]`, `[critical 332148]`), sem o parágrafo da base de conhecimento da Intel; sem `File:`, o `at x.v(12)` do texto | o que a ferramenta cita; no `at`, só o nome do arquivo |

Regras:

- Linha do stderr que não casa com nada vira `severity: unknown`. Nunca é
  descartada.
- Linha do stdout que não casa só vira diagnóstico nas ferramentas cujo
  stdout é feito de mensagens (YANC e `iverilog`). No `vvp`, no modelo do
  Verilator, no Yosys e no `dot`, o stdout é saída do programa (o
  `$display` do testbench, o log do `make`) e fica só no `StepReport`.
- Resumos (`2 error(s) during elaboration.`, `%Error: Exiting due to ...`)
  e `$finish called` são `info`.
- No `vvp`, `ERROR:` e `FATAL:` (de `$error` e `$fatal`) são erro e fazem a
  simulação falhar, mesmo com código 0 (5.2). O `$error` do Verilog sempre
  traz o local (`ERROR: tb.v:12: ...`); um `ERROR:` sem local é o testbench
  escrevendo (`$display("ERROR: ...")`) e fica como saída dele, sem
  reprovar. A linha seguinte do `vvp` (`Time: 1000  Scope: tb`) entra na
  mensagem.
- No `iverilog`, a continuação com local de um aviso
  (`x.v:5:        : Padding 4 high bits of the port.`) entra na mensagem
  anterior, e o bloco `*** These modules were missing:` ... `***` vira um
  resumo `info` só.
- No `iverilog`, a macro indefinida é erro (5.4), e um erro faz o passo
  falhar mesmo com código 0.
- As mensagens do YANC vêm em inglês, porque a AURORA pede `--lang=en`.
- No Quartus, os `Info` (centenas) ficam de fora, as linhas indentadas
  entram no `raw` da mensagem de cima, e o resumo de uma ferramenta que
  falhou (`Error: Quartus Prime Fitter was unsuccessful. ...`, com os
  `Error:` indentados de memória e tempo) é `info`.

---

## 8. Erros

`AuroraError` é para quando a operação não pôde rodar. `code()` devolve um
identificador estável, o mesmo que a CLI põe em `error.code` no JSON.

| Variante | `code()` | Quando | O que fazer |
|---|---|---|---|
| `BundleNotFound` | `bundle_not_found` | não há `toolchain/` com `bundle.json` ao lado do executável | reinstalar a AURORA |
| `InvalidBundle` | `invalid_bundle` | manifesto ausente ou inválido, formato diferente de `BUNDLE_SCHEMA`, outra plataforma, componente ausente ou fora do bundle, executável que é symlink para fora do bundle | reinstalar, ou montar o bundle certo |
| `UnsupportedPlatform` | `unsupported_platform` | fora de Linux x64, macOS arm64, Windows x64 | não há bundle |
| `ComponentMissing` | `component_missing` | o componente da ferramenta não foi instalado; a mensagem não diz como instalar, que é de cada interface | `aurora install <componente>`, ou o botão Instalar do Studio |
| `ToolchainIncomplete` | `toolchain_incomplete` | o componente está lá, mas falta o executável (ou o `verilator_bin`), ou um diretório do YANC | reinstalar |
| `SystemCompilerMissing` | `system_compiler_missing` | Linux e macOS: Verilator (simulação ou `check` com `lint`) sem compilador C++, `make` e Perl do sistema | instalar (ver BUNDLE.md) ou, na CLI, declarar com `--compiler <DIR>` ou `AURORA_COMPILER` |
| `InvalidProject` | `invalid_project` | diretório ou arquivo do projeto ausente ou em conflito | ver `path` e `reason` |
| `ProjectExists` | `project_exists` | `create` sobre um `.spf` existente | abrir em vez de criar |
| `OutsideProject` | `outside_project` | `move_path` com um caminho fora da pasta do projeto | usar um caminho do projeto |
| `CannotMove` | `cannot_move` | `move_path` do `.spf`, de `.aurora8`, da pasta de um processador, das pastas dele ou do fonte; para dentro de `.aurora8`; de uma pasta para dentro dela mesma; `record_move` de uma origem que ainda existe | ver `reason` |
| `PathExists` | `path_exists` | o destino de `move_path` já existe | outro destino, ou tirar o que está lá |
| `ProcessorExists` | `processor_exists` | `add_processor` com nome repetido | outro nome |
| `ProcessorNotFound` | `processor_not_found` | nome inexistente | ver `available` |
| `InvalidName` | `invalid_name` | nome de projeto, processador ou topo da síntese fora das regras de 3.4; arquivo com nome de testbench como topo; arquivo que não é `.v` nem `.sv` nas listas, ou `.py` fora dos testbenches; `.py` novo ou simulado com nome que não é identificador do Python; `check` de um `.py`; `build` de processador do `.spf` com nome que não compila | outro nome, ou outro arquivo |
| `InvalidProjectFile` | `invalid_project_file` | `.spf` ilegível, campo com tipo errado (3.3), processador com nome vazio, com barra ou repetido, ou `clk`/`numClocks` fora da faixa | corrigir o `.spf` |
| `InvalidSource` | `invalid_source` | fonte ausente, ou com outro nome que não `<processador>.<ext>` | corrigir o fonte |
| `LegacyLayout` | `legacy_layout` | `build` de um processador na estrutura de pastas do YANC 6.x (`Software/`, `Hardware/`, `Simulation/`) | `convert_layout` (3.6) |
| `YancTooOld` | `yanc_too_old` | `build` com um bundle cujo YANC é anterior à 7.1 | instalar o bundle atual |
| `NotBuilt` | `not_built` | simular, sintetizar, verificar (`check` com `processor`) ou elaborar a hierarquia de processador sem `build`; `check` de um projeto em que nada se verifica porque nenhum processador foi compilado; um arquivo registrado que o build de um processador gera e ainda não existe; `waveform_path` de processador não compilado | rodar `build` |
| `NoTestbench` | `no_testbench` | `simulate_project` ou `waveform_path` sem testbench | `add_verilog` de um testbench, ou `set_testbench` |
| `NoCocotbToplevel` | `no_cocotb_toplevel` | `simulate_project` de um testbench cocotb sem a diretiva `# aurora-toplevel:` num projeto sem topo | a diretiva no `.py`, ou `set_top` |
| `CocotbUnavailable` | `cocotb_unavailable` | o Python do componente `cocotb` não carregou o cocotb (a sonda de 5.3.2 falhou), ou o cocotb dele não traz a biblioteca do simulador; `reason` traz o fim da saída da sonda ou a biblioteca que falta | reinstalar o componente |
| `NoTopLevel` | `no_top_level` | `synthesize(TopLevel)` sem topo | `set_top` |
| `EmptyProject` | `empty_project` | `check` sem nenhum arquivo Verilog registrado e sem processadores; `hierarchy` sem nada para elaborar | `add_verilog` ou `add_processor` |
| `ModuleNotFound` | `module_not_found` | `set_top` com um nome que nenhum sintetizável registrado declara; `top_module` ou `testbench_module` num arquivo sem um módulo que dê para usar; `render_schematic` de um módulo fora do netlist | ver `available` |
| `AmbiguousModule` | `ambiguous_module` | `set_top` com um nome que mais de um sintetizável declara | escolher pelo arquivo (`files`) |
| `InvalidParameter` | `invalid_parameter` | `add_processor` com parâmetro que o YANC não compila (`NewProcessor::validate`); `configure_processor` com frequência ou clocks fora da faixa | ver `name`, `value` e `reason` |
| `OperationInProgress` | `operation_in_progress` | outra operação da AURORA usa os mesmos arquivos de trabalho, de outro processo ou do mesmo: o build ou a simulação do mesmo processador; a simulação do projeto, a síntese ou o build da placa com o build de um processador dele; duas simulações do projeto ou sínteses; duas verificações com lint, duas hierarquias, duas árvores de sinais ou dois builds da mesma placa | esperar a outra terminar |
| `InvalidNetlist` | `invalid_netlist` | netlist ilegível | ver `modules` da síntese |
| `InvalidDataFile` | `invalid_data_file` | linha de `input_<n>.txt`/`output_<n>.txt` que não é inteiro | corrigir a linha indicada |
| `ProcessExitedEarly` | `process_exited_early` | o Surfer fechou logo ao abrir | ver o fim do log (`tail`) |
| `NonUtf8Path` | `non_utf8_path` | caminho não UTF-8 | renomear |
| `PathTooLong` | `path_too_long` | caminho acima de `YANC_PATH_LIMIT` (259 no Windows, 1000 nos outros) | mover o projeto |
| `Spawn` | `spawn` | processo não iniciou | ver permissão e caminho |
| `Io` | `io` | leitura ou escrita da AURORA falhou | ver `context` e `path` |
| `ProjectNotFound` | `project_not_found` | `Project::discover` sem `.spf` na pasta nem nas de cima | rodar de dentro de um projeto, ou criar um |
| `NoReports` | `no_reports` | `latest` ou `compare_reports` num projeto sem relatório guardado | rodar uma operação e gravar |
| `ReportNotFound` | `report_not_found` | `load`, `report_text` com um identificador que não existe | ver `list` |
| `InvalidReport` | `invalid_report` | `record.json` ilegível, que não é JSON, ou de outro formato | ver `path` e `reason` |
| `NotComparable` | `not_comparable` | `compare` de projetos diferentes; `compare_reports` sem estatísticas nem tempos, sem anterior compatível, ou entre dois sem parte em comum | escolher outro relatório |
| `BoardNotFound` | `board_not_found` | `fpga::board` com um identificador que não existe | ver `available` |
| `InvalidBoard` | `invalid_board` | o JSON embutido de uma placa é inválido (os testes impedem) | é bug |
| `NoFpgaConfig` | `no_fpga_config` | `fpga::prepare` num projeto sem `fpga.json` | escrever o `fpga.json` |
| `InvalidFpgaConfig` | `invalid_fpga_config` | `fpga.json` que não é JSON válido, ou ligações que não batem com a placa e o topo (um problema por linha em `reason`) | corrigir as ligações |
| `NoBitstream` | `no_bitstream` | `fpga::program` sem o `.sof` da compilação para a placa | `fpga::build` antes |
| `NoCable` | `no_cable` | `fpga::program` sem `cable` e sem cabo de gravação ligado | ligar a placa pela porta do USB-Blaster, e o driver dele |
| `StaleBitstream` | `stale_bitstream` | `fpga::program` com um `.sof` que não descreve o projeto de agora: um fonte, uma memória, o `fpga.json` ou a placa mudou depois da compilação, o `.sof` foi trocado, ou a compilação não terminou | `fpga::build` de novo |
| `QuartusMissing` | `quartus_missing` | `fpga::build`, `fpga::program` ou `tool(Tool::Quartus)` sem Quartus (2.5) | instalar o Quartus Prime Lite com o suporte à família da placa, ou declarar a pasta dele |
| `ExperimentalDisabled` | `experimental_disabled` | `experimental::require` com o recurso desligado (5.10.1) | ligar o recurso: `aurora experimental enable <recurso>`, ou Preferências > Experimental no Studio |
| `Unsupported` | `unsupported` | `flows::run` com `FlowRequest::Learn`: a correção de um exercício roda no servidor da AURORA, que linka o aurora-learn | pedir pelo `aurora serve` (`flow.run`), ou corrigir com `aurora learn check` |
| `NoWaveform` | `no_waveform` | `flows::open_wave` de uma onda do projeto (ou de um processador) que ainda não existe, porque a simulação não rodou ou não chegou ao fim, ou de um arquivo que não existe | simular antes (`aurora sim [-p X]`, o botão Simular) |
| `SignalNotFound` | `signal_not_found` | `flows::choose_signals` com um caminho que a árvore do testbench elaborado não tem; `similar` traz os parecidos | ver `similar`, ou `wave_signals` |
| `SignalNotChosen` | `signal_not_chosen` | tirar da escolha um caminho que não está na escolha gravada nem no design | ver a escolha (`wave_signals`) |
| `TestbenchNotElaborated` | `testbench_not_elaborated` | a escolha de sinais precisa da árvore do testbench, e a elaboração dele pelo Icarus falhou; `reason` é a primeira mensagem | corrigir o testbench (`aurora check`) |

O enum é `#[non_exhaustive]`: todo `match` precisa de `_`. Para atravessar
uma fronteira (JSON, outro processo), um `AuroraError` vira [`ErrorInfo`]
(`ErrorInfo::from(&error)`; 5.11).

---

## 9. Garantias e limites

Garantias:

- **Sem I/O de console.** O Core não escreve no terminal, não chama `exit`,
  não entra em pânico em erro esperado. O log sai por `tracing`, com spans
  por operação (`build`, `simulate`, `simulate_project`, `check`,
  `hierarchy`, `synthesize`, `render_schematic`, `wave_signals`, `fpga_build`,
  `fpga_program`); o cliente configura o
  subscriber.
- **Só o bundle.** Toda ferramenta sai do bundle instalado com a AURORA, num
  caminho fixo por plataforma. Nada do `PATH`, nada configurável. As exceções
  são a do Verilator no Linux e no macOS (compilador, `make` e Perl do
  sistema, em locais fixos ou numa pasta declarada; no Windows os três vêm
  no bundle) e a do Quartus Prime, para as placas Intel (2.5), de pastas
  padrão ou declaradas, nunca do `PATH`.
- **Ambiente explícito.** Toda ferramenta parte de um ambiente vazio (o abridor de `open_with_system` e o `taskkill` herdam o da AURORA). No Windows
  recebe `SystemRoot`, `SystemDrive`, `windir`, `ComSpec`, `ProgramData`, `TEMP`,
  `TMP`; as ferramentas do OSS CAD
  Suite recebem `PATH` com `bin;lib` do pacote, e as do `msys/`, com
  `ucrt64/bin`. No Linux e no macOS, os lançadores do pacote recebem
  `PATH=/usr/bin:/bin`. O Verilator recebe o diretório do script (no Windows, o
  do `verilator_bin.exe`) e os do compilador, `LC_ALL=C` e, no Windows,
  `VERILATOR_ROOT`; o
  surfer-aurora recebe as variáveis de display. O `StepReport` registra o que
  foi definido.
- **stdin fechado.** Todo filho recebe stdin nulo, e o `vvp` roda com `-n`:
  um `$stop` num testbench termina a simulação em vez de esperar entrada.
- **Cancelar não deixa processo para trás.** Uma operação cancelada ou que
  passou do prazo encerra o processo do passo com tudo o que ele iniciou:
  no Unix, o grupo de processos do passo (SIGTERM, depois SIGKILL); no
  Windows, o Job Object do passo, que também encerra a árvore quando o
  processo da AURORA morre (5.8).
- **Nunca sobrescreve código.** `add_processor` e `add_file` com conteúdo
  recusam se o arquivo já existir; `add_verilog` só cria o arquivo que não
  existe e registra como está o que existe. `move_path` recusa um destino
  que já existe. `write_input` substitui a entrada, que é estímulo.
- **Threads.** `Toolchain`, `Project`, os resultados, `AuroraError`,
  `Control`, `CancelToken` e `Event` são `Send + Sync`; `RunningProcess` é
  `Send`. Uma GUI roda as operações numa thread de trabalho e cancela da
  thread da interface (5.8).
- **FFI.** A superfície pública só expõe tipos da AURORA, da `std`, de
  `camino` (caminhos), de `serde`, de `serde_json` (o `Value` de
  `ErrorInfo::data`, `AuroraError::data`, `Notification::Install` e
  `FlowOutcome::learn`) e de `schemars` (derive). Os enums são
  `#[non_exhaustive]`, menos os da 0.9.0 em `flows`, `delivery` e `locate`
  (`FlowRequest`, `WavePolicy`, `Progress`, `WaveSource`, `LayoutPolicy`,
  `WaveChoice`, `Notification`, `Origin`).

Limites desta versão:

- As operações bloqueiam até o fim, menos `open_waveform`. Cancelar e
  acompanhar a saída é pelo `Control` (5.8), de outra thread.
- Só a simulação (`SimulationOptions::timeout`) e o desenho do esquemático
  (`SchematicOptions::timeout`, 60 s por padrão) têm prazo. Build, `check`,
  hierarquia e síntese rodam até o fim ou até o cancelamento.
- No Unix, um SIGKILL no processo que usa a biblioteca não tem como
  encerrar os filhos: eles rodam num grupo de processos próprio e continuam
  rodando. O mesmo vale para um SIGTERM que o processo não trata. Quem
  encerra um cliente da AURORA de fora deve mandar SIGTERM, e o cliente deve
  transformá-lo em `cancel()`, como a CLI faz (`cancel_on_signals`, em
  `crates/aurora-cli/src/main.rs`). No Windows os filhos estão no Job Object
  do passo e terminam junto com o processo, de qualquer jeito que ele
  morra.
- Sem build incremental: cada chamada roda tudo de novo (o modelo do
  Verilator em dia é reaproveitado pelo próprio Verilator e pelo `make`,
  5.2).
- O `StepReport` guarda de cada pipe os primeiros e os últimos 2 MiB; o meio
  de uma saída maior (um testbench que imprime sem parar) sai, e uma linha
  `[AURORA: N bytes of output left out here]` marca o corte. A saída ao vivo do
  `Control` (5.8) passa inteira. Os diagnósticos saem do que foi guardado.
- Plataformas: ver 9.1.

### 9.1 Plataformas

A AURORA é escrito para Linux, macOS e Windows. O que foi conferido e como:

| Verificação | Linux | Windows | macOS |
|---|---|---|---|
| compila, `clippy -D warnings` | sim | sim (MSVC e GNU, cruzado) | sim (Intel e Apple Silicon, cruzado) |
| testes de unidade | sim | sim, no Wine | não |
| build com o YANC real, regressão do YANC (66 + 10 casos), CLI | sim | sim, no Wine, com o YANC compilado para Windows | não |
| Icarus, Verilator, Yosys, `dot`, surfer-aurora do bundle | sim (bundle 2026.09.29, com o OSS CAD Suite 2026-09-29 e o surfer-aurora v0.7.0-nips.10: Icarus 14.0 devel, Verilator 5.053 devel, Yosys 0.69+156, Graphviz 2.43); o bundle atual, 2026.10.22, com o OSS CAD Suite 2026-10-22 (Yosys 0.70+190), ainda não foi rodado no Linux | não | não |

O Wine executa a API do Windows (criação de processo, caminhos `C:\`,
`.exe`, ambiente), mas não é o Windows. A verificação completa nos três
sistemas, com todas as ferramentas, é à mão: os testes contra uma
instalação ([CONTRIBUTING.md](../CONTRIBUTING.md)).

O que muda por sistema, e como a AURORA trata:

| Assunto | Tratamento |
|---|---|
| extensão de executável | `.exe` no Windows para o YANC e para o modelo do Verilator (`EXE_SUFFIX`) |
| caminhos | `camino` em toda a API; `dunce` tira o prefixo `\\?\` que os compiladores C não entendem; `.spf` aceita `\` e `/` |
| limite de caminho | 259 caracteres no Windows (MAX_PATH dos `.exe` do YANC), 1000 nos outros (buffers do YANC); conferido por arquivo antes de compilar |
| nomes reservados | `CON`, `PRN`, `AUX`, `NUL`, `COM0-9`, `LPT0-9` recusados em todo sistema |
| maiúsculas | processadores que diferem só na caixa são recusados (Windows e macOS não distinguem) |
| ambiente do filho | Windows: `SystemRoot`, `SystemDrive`, `windir`, `ComSpec`, `ProgramData`, `TEMP`, `TMP`. Surfer: `HOME`, `DISPLAY`, `WAYLAND_DISPLAY`, `XDG_*`, `TMPDIR` (Unix) ou `USERPROFILE`, `APPDATA`, `LOCALAPPDATA` (Windows) |
| término de processo | sinal no Unix (`Signaled`), NTSTATUS no Windows (`Exception`) |
| encerrar um passo (cancelamento, prazo) | Unix: grupo de processos próprio por passo, SIGTERM ao grupo e SIGKILL depois de 1 s. Windows: Job Object por passo (`KILL_ON_JOB_CLOSE`), que encerra a árvore também quando a AURORA morre; `taskkill /T /F` do `System32` se o sistema recusar o job |
| janela de console | Windows: nenhuma (`hide_console`); com console o filho divide o da AURORA, sem console leva `CREATE_NO_WINDOW` |
| ferramentas do OSS CAD Suite | lançadores bash via `/bin/bash` (Linux, macOS); `.exe` com `PATH=bin;lib` (Windows) |
| Verilator | Linux e macOS: o script Perl pelo Perl do sistema; Windows: o `verilator_bin.exe` direto, com `VERILATOR_ROOT`; Python do bundle no `make`; sem `-march=native` no macOS |
| fins de linha | CRLF aceito em toda leitura (`.spf`, fontes, saídas); no Windows o YANC grava CRLF |

---

## 10. Diferenças em relação à AURORA

| Assunto | AURORA | AURORA |
|---|---|---|
| Abrir projeto | regrava o `.spf` (`lastOpened`, `exists`) | só lê |
| `clk` fracionário | trunca | recusa |
| Diretório temporário | `<raiz>/.aurora/Temp` | `<raiz>/.aurora8/Temp` |
| Idioma do YANC | o da interface (`--lang=pt` ou `--lang=en`) | sempre inglês (`--lang=en`) |
| Ambiente do filho | herda o da AURORA | vazio + o mínimo |
| Classificação de arquivos | por conteúdo, refeita a cada carga do projeto | por conteúdo, com a mesma regra, quando `add_verilog` registra; o papel fica gravado no `.spf` |
| Verificação | `iverilog -tnull -s <topo>` | `iverilog` sem `-s` (todas as raízes), um por testbench e, com `lint`, `verilator --lint-only` |
| Hierarquia | montada pelo Yosys | elaborada pelo Icarus, do design e de cada testbench, com a biblioteca SAPHO (`hierarchy`) |
| Nome de projeto | aceita espaço e acento | só letras sem acento, dígitos, `_` e `-`, começando por letra; os projetos que já existem abrem |
| Formato da onda (Icarus) | `vvp -fst` sempre, no nome do `$dumpfile` (um `.vcd` com FST dentro) | FST, com a extensão `.fst` trocada numa cópia do testbench |
| Testbench cocotb | o runner Python do cocotb, num processo, no Icarus ou no Verilator | os passos `elaborate` (ou `verilate`) e `simulate` da AURORA, no Icarus ou no Verilator, com os testes em `SimulationResult::tests` |
| Simulação rápida (Fast Sim) | Verilator sem `--trace-fst`, com os `$dumpfile`/`$dumpvars` comentados numa cópia do testbench; o botão só habilita com o Verilator escolhido; cocotb no simulador escolhido | Verilator sem `--trace`, com o testbench como está (o Verilator ignora o dump), qualquer que seja o simulador escolhido; cocotb no simulador escolhido (`SimulationOptions::fast`) |
| Testbench sem `$dumpfile` | injeta `$dumpvars(1, <topo>)`: só o nível do testbench | injeta `$dumpvars(0, <tb>)`: todos os sinais, inclusive os do módulo testado |
| Simulação de processador | só pelo modo projeto | `simulate` direto, além de `simulate_project` |
| Ferramentas | `components/` baixados pela AURORA | bundle versionado instalado com a AURORA |
| Esquemático | netlistsvg (fork próprio), dentro do processo, ~50 skins | `show` do Yosys + `dot` do Graphviz; o Studio desenha o próprio, do `hierarchy.json`, com o ELK |
| Onda do Verilator | FST | VCD (o FST do Verilator do bundle exige lz4 e zlib do sistema) |
| Layout do Surfer | gera `.surf.ron` e tradutores na pasta de configuração do usuário | gera o mesmo layout (`wave_layout`), com os tradutores em `.aurora8/Temp/surfer/` |
| Sinais fora dos processadores no layout | todos os escopos | só os da raiz do testbench; com escolha de sinais (5.7.3), os escolhidos, um grupo por escopo |
| Teste de hardware (THTEST) | harness C++ próprio | não implementado |
