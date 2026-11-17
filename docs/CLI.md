# `aurora`: a linha de comando

A CLI é uma casca fina sobre o `aurora-core` (ver [API.md](API.md)): cada
comando abre o projeto, chama uma função do Core e formata o resultado, em
texto ou no `--json` do comando. O despacho do `aurora serve`, que o Studio e
a extensão do VS Code usam, chama as mesmas funções, e as regras de cada
operação moram no Core, então os dois fazem o mesmo. Os comandos que rodam
ferramentas (`build`, `check`, `hierarchy`, `sim`, `synth`, `fpga build` e
`fpga program`) montam um `FlowRequest` e chamam `aurora_core::flows::run_with`,
que confere o pedido antes de procurar o bundle, e o `wave` chama
`flows::open_wave_with` e `flows::choose_signals`; a coluna "Função do Core"
das tabelas abaixo diz o que esses fluxos chamam por dentro.

```
cargo install --path crates/aurora-cli     # instala o binário `aurora`
aurora --help                              # visão geral
aurora <comando> --help                    # detalhes de um comando
```

## Primeiro uso

Um projeto da AURORA é um projeto Verilog. Processadores SAPHO, quando
existem, são uma etapa a mais: o YANC os compila antes de verificar, simular
ou sintetizar. Os dois fluxos usam os mesmos comandos e cabem no mesmo
projeto.

Verilog:

```
aurora tools                       # confere o bundle instalado
aurora new contador
cd contador
aurora add contador.v              # cria o módulo-modelo; o primeiro módulo vira o topo
# edite contador.v
aurora add contador_tb.v           # cria o testbench, que já instancia contador
aurora check                       # Icarus: o design inteiro e cada testbench
aurora hierarchy                   # a árvore de instâncias do design e do testbench
aurora sim                         # simula o testbench e mostra o que ele escreve
aurora sim --fast                  # no Verilator, sem gravar onda
aurora wave                        # abre a onda da simulação no surfer-aurora
aurora synth --svg                 # Yosys + Graphviz: o esquemático do topo
```

Processador SAPHO:

```
aurora new demo
cd demo
aurora proc add soma               # cria soma/soma.cmm
# edite soma/soma.cmm
# ponha os valores de entrada em soma/input_0.txt, um por linha
aurora build                       # C± -> Verilog, memórias e testbench, pelo YANC
aurora sim -p soma                 # simula e mostra as saídas
aurora sim -p soma --fast          # as mesmas saídas, no Verilator e sem onda
aurora wave -p soma                # abre a onda da simulação do processador
```

### Regras

**De onde rodar:** de qualquer pasta do projeto, não só da raiz. A AURORA
procura o `.spf` na pasta atual e, se não achar, nas de cima, como o git
acha o repositório. Fora de qualquer projeto, o erro é `project_not_found`.
`-C` aponta outro lugar para começar a busca.

**Dentro da pasta de um processador** (`soma/`, a pasta de saída
`soma/soma/` ou `.aurora8/Temp/soma/`), os comandos que
recebem um processador e não receberam nenhum agem sobre ele: `build`,
`check`, `sim`, `wave`, `synth` e `proc set`. No resto do projeto, eles agem
sobre o projeto inteiro (`proc set` pede o nome). `aurora status` marca o
processador da pasta. Um nome dado (`-p`, `proc set NOME`), um `ARQUIVO` do
`check` ou um `TESTBENCH` do `sim` vale de qualquer pasta e vence a
pasta; para agir sobre o projeto inteiro de dentro de um processador, use
`-C` com a raiz.

**Caminhos:** os da linha de comando são relativos ao diretório atual do
shell, não à raiz do projeto. De fora da pasta do projeto:
`aurora -C contador add contador/rtl/alu.v`.

**Arquivos Verilog:** entram no projeto por `aurora add`, que os registra no
`.spf`. O papel de um arquivo que já existe vem do conteúdo, pela regra da
AURORA: `$dumpfile`, `$finish`, módulo sem portas, `initial`, `$display` e
atrasos como `#10` somam pontos de testbench (a tabela está em
[API.md, seção 4.2](API.md#42-o-módulo-verilog)). `--tb` registra como
testbench. Um arquivo fica numa lista só: registrado de novo com outro
papel, ele muda de lista. Um `.py` é sempre testbench: um testbench cocotb,
como na AURORA (ver [Testbench cocotb](#testbench-cocotb)).

**Arquivo novo:** `aurora add` o cria a partir de um modelo. É testbench com
`--tb` ou quando o nome tem `tb`, `test` ou `testbench` como palavra
(`alu_tb.v`, `tb_alu.v`, `test_alu.v`; `contest.v` e `latest.v` não);
senão é um módulo com o nome do arquivo e portas de exemplo que compilam. O
testbench-modelo instancia o módulo testado com todas as portas (com clock e
reset, se o módulo tiver `clk`/`clock` e `rst`/`reset`), grava a onda com
todos os sinais e termina com `$finish`. O módulo testado é o `X` de
`X_tb.v` ou `tb_X.v`, se algum módulo registrado o declara; senão, o módulo
de topo; sem nenhum dos dois, o testbench não instancia nada: grava a onda,
espera `#100` e chama `$finish`. Um `.py` novo sai do modelo cocotb, com o
módulo testado pela mesma regra (`test_alu.py` testa `alu`) na linha
`# aurora-toplevel: alu` e um teste que liga o clock, aplica o reset, põe
as outras entradas em zero e escreve as saídas no log. O nome do `.py` vira
o nome do módulo Python: só letras, dígitos e `_`, sem começar por dígito.

**Topo:** o primeiro módulo registrado vira o topo, e o primeiro testbench,
o testbench simulado. `aurora top` troca o topo por arquivo
(`aurora top rtl/alu.v`) ou por nome de módulo (`aurora top alu`);
`aurora sim <testbench>` troca o testbench.

**Processadores:** a biblioteca SAPHO do YANC só entra na verificação, na
simulação e na síntese quando o projeto tem processadores. Um projeto só de
Verilog não precisa do YANC.

## Opções globais

Valem em qualquer comando, antes ou depois dele.

| Opção | Padrão | Efeito |
|---|---|---|
| `-C, --project <CAMINHO>` | `.` | o `.spf` do projeto, ou uma pasta dentro dele: a AURORA sobe até achar o `.spf` |
| `--json` | não | um objeto JSON no stdout, nada mais ([JSON](#json)) |
| `--events` | não | um objeto JSON por linha no stdout: os eventos das ferramentas enquanto rodam e, na última linha, o resultado ([Eventos](#eventos)). Com `--json` junto, vale o `--events` |
| `-v`, `-vv` | não | `-v`: o comando e toda a saída de cada passo enquanto roda, mensagens `info`, artefatos intermediários, módulos da síntese e log `info` no stderr; `-vv`: log de depuração |

Bundle (o `--compiler` e o `--quartus` ficam entre as opções globais do `--help`; o `--toolchain`, de desenvolvimento, não aparece na ajuda):

| Opção | Variável | Efeito |
|---|---|---|
| `--toolchain <DIR>` | `AURORA_TOOLCHAIN` | outro bundle no lugar do instalado (desenvolvimento) |
| `--compiler <DIR>` | `AURORA_COMPILER` | onde está o compilador para o Verilator, quando não está no local padrão; no Windows, troca o do bundle (veja abaixo) |
| `--quartus <DIR>` | `AURORA_QUARTUS` | onde está o Quartus Prime, para as placas Intel, quando não está nas pastas padrão do instalador (veja abaixo) |

## De onde vêm as ferramentas

Do bundle instalado com a AURORA (`toolchain/` ao lado de `bin/`), e só dele.
`aurora tools` mostra o bundle, as versões e onde está cada ferramenta;
`aurora tools --verify` confere os hashes. Como instalar: [INSTALL.md](INSTALL.md). O que o bundle traz e como montá-lo:
[BUNDLE.md](BUNDLE.md).

A exceção é o compilador do sistema, que o Verilator usa no Linux e no
macOS. A AURORA o procura nos locais padrão ([BUNDLE.md](BUNDLE.md), seção 4)
e nunca no `PATH`. Fora deles, `--compiler <DIR>` diz onde ele está: o
diretório com `perl`, `make` e `g++` ou `clang++`. No Windows o compilador
vem no bundle, com o Verilator, e `--compiler` recebe a raiz de um MSYS2
para usá-lo no lugar, só para desenvolvimento. Um diretório sem os três é
erro (código 2), e não volta em silêncio aos locais padrão. Para não
repetir a opção, defina a variável `AURORA_COMPILER` no perfil do shell:

```sh
export AURORA_COMPILER=/opt/gcc/bin                    # bash, zsh: no ~/.bashrc ou ~/.zshrc
set -Ux AURORA_COMPILER /opt/gcc/bin                   # fish: vale para as próximas sessões
```

```powershell
[Environment]::SetEnvironmentVariable("AURORA_COMPILER", "D:\msys64", "User")
```

`aurora tools` mostra qual compilador foi encontrado e se ele é do bundle ou
do sistema. O compilador e o bundle não têm arquivo de configuração: o único
arquivo de configuração da AURORA é o `experimental.json`
([Componentes experimentais](#componentes-experimentais-aurora-experimental)).

A segunda exceção é o Quartus Prime, que compila para as placas Intel e não
pode ir no bundle. A AURORA usa o de `--quartus <DIR>` (ou `AURORA_QUARTUS`),
que é erro se não tiver o `quartus_sh`; senão o de `QUARTUS_ROOTDIR`, que o
instalador da Intel cria; senão o das pastas padrão do instalador, a versão
mais nova ([FPGA.md](FPGA.md)). `aurora tools` mostra o que achou.

## Comandos

### Projeto

| Comando | Faz | Função do Core |
|---|---|---|
| `aurora new <NOME> [--dir DIR]` | cria `<DIR>/<NOME>/<NOME>.spf`. O nome usa só letras sem acento, dígitos, `_` e `-`, começando por letra, até 64 caracteres, fora os nomes reservados do Windows (`CON`, `PRN`, `AUX`, `NUL`, `COM0` a `COM9`, `LPT0` a `LPT9`, recusados em todo sistema), e não pode diferir só na caixa de outro da mesma pasta. Criado dentro da pasta de outro projeto, avisa | `Project::create`, `Project::issues` |
| `aurora status` | os avisos do `.spf` (`warning: ...`), os módulos registrados (o topo marcado com o nome do módulo), os testbenches (o simulado marcado), os `.v`, os `.sv` e os testbenches cocotb (`.py` com `@cocotb.test`) da pasta que não estão registrados, com a dica `aurora add <arquivo>`, e os processadores (linguagem, frequência, clocks, arrays, se está compilado, e `(this folder)` no da pasta atual). Num projeto vazio, os primeiros passos dos dois fluxos | `Project::open`, `Project::issues`, `Project::files`, `Project::top_module`, `Project::unregistered_verilog`, `Processor::is_built`, `Project::processor_at` |

Os arquivos não registrados são procurados na pasta do projeto e nas
subpastas, até 4 níveis, menos `.aurora8/`, as ocultas, as pastas dos
processadores, a `TopLevel/` legada da AURORA e as de outro projeto, com
`.spf` próprio.

Os avisos do `.spf` (`issues` no JSON) são o que está estranho sem impedir
de abrir: um caminho de outra máquina (`C:\...`) achado pela cauda dentro da
pasta do projeto, que a próxima mudança grava como o caminho de hoje; um
`topLevelFile` ou `testbenchFile` fora da lista, ou um topo com nome de
testbench, que é ignorado; um processador com nome que não compila.

### Arquivos Verilog e VHDL

| Comando | Faz | Função do Core |
|---|---|---|
| `aurora add <ARQUIVO>... [--tb]` | registra cada arquivo (`.v`, `.sv`, `.vhd`, `.vhdl`, ou `.py`, um testbench cocotb); o que não existe é criado a partir do modelo. Confere todos antes de registrar o primeiro: um nome que não serve recusa o comando inteiro, e nada é criado nem registrado | `Project::check_add_hdl`, `Project::add_hdl` |
| `aurora remove <ARQUIVO>...` | tira do projeto, e do topo ou do testbench simulado se era um deles; não apaga do disco | `Project::remove_hdl` |
| `aurora top` | mostra o arquivo de topo e o módulo de topo, ou diz que não há topo e como escolher | `Project::top_level`, `Project::top_module` |
| `aurora top <ARQUIVO\|MÓDULO>` | escolhe o topo: se o argumento é um arquivo, ele (qualquer `.v` ou `.sv`, inclusive o `<proc>/<proc>/<proc>.v` gerado; registrado se não estava); senão, o arquivo registrado que declara esse módulo. Recusa nome de testbench (`tb_<nome>.v`, `<nome>_tb.v`, `tb.v`), arquivo que não é Verilog (`invalid_name`) e módulo declarado em mais de um arquivo (`ambiguous_module`, com os arquivos); nada é gravado | `Project::set_top` |
| `aurora move [--after] <ORIGEM>... <DESTINO>` | move ou renomeia arquivos e pastas do projeto; os registrados continuam no projeto, com o topo e o testbench simulado acompanhando. Com `--after`, só registra no `.spf` o que já foi movido | `Project::move_path`, `Project::record_move` |
| `aurora order <ARQUIVO> (--first \| --last \| --before <OUTRO> \| --after <OUTRO>)` | muda a posição do arquivo na lista dele e mostra a lista numerada; os compiladores leem nessa ordem, e um `` `define `` só vale para os de baixo | `Project::reorder_file` |

Para cada arquivo, `add` diz se o criou ou só o registrou, se ele ficou como
módulo ou como testbench e se virou o topo ou o testbench simulado. Um
arquivo sem módulo (só `` `define ``) não vira o topo. Um arquivo de fora da
pasta do projeto é gravado no `.spf` com caminho relativo (`../../rtl/x.v`)
quando está no mesmo repositório git do projeto, e absoluto quando não.
As portas
que o testbench-modelo instancia são lidas pelo Yosys, quando ele está
instalado; sem ele, por um leitor de portas no estilo ANSI da própria AURORA.

`remove` avisa quais arquivos não estavam registrados e sai com código 2 se
nenhum estava.

`move` segue o `mv`: se `DESTINO` é uma pasta que existe, ou termina com
`/`, cada origem vai para dentro dela; senão, a única origem passa a ter o
caminho `DESTINO` (renomear). As pastas que faltam são criadas. Com várias origens,
`DESTINO` precisa ser uma pasta (código 2). Para cada origem, diz de onde
para onde foi e quais arquivos continuam no projeto, com o papel de cada um.
Ficam no lugar o `.spf`, a pasta `.aurora8` e, de cada processador, a pasta,
a pasta de saída e o fonte (`cannot_move`); o destino
que existe não é sobrescrito (`path_exists`). As origens anteriores à que
falhou já foram movidas.

`move --after` é para o que já foi movido fora da AURORA, no explorador de um
editor ou com o `mv`: o disco fica como está e só o `.spf` acompanha, como
o `hg mv --after`. A origem não pode mais existir (`cannot_move`) e o
destino tem que existir (`invalid_project`). Como uma pasta no destino pode
ser a própria origem renomeada (`aurora move --after rtl hdl`), ela só é a
pasta de destino se terminar com `/`, se houver várias origens ou se a
origem já estiver dentro dela. É o que a extensão do VS Code chama quando
um arquivo do projeto é renomeado no explorador.

O módulo de topo é o único módulo do arquivo de topo; num arquivo com vários
módulos, o que tem o nome do arquivo. Sem nenhum dos dois, é erro
(`module_not_found`, com os módulos do projeto na mensagem).

Um arquivo VHDL segue as mesmas regras, com a entidade no lugar do módulo e
sem diferenciar maiúsculas (`aurora top CONTADOR` acha `entity contador`).
O modelo de `add` é VHDL-2008: a entidade com uma entrada e uma saída de 8
bits; o testbench com uma constante para cada genérico, um sinal para cada
porta, a instância por `entity work.<entidade>`, o relógio de 10 ns, o
reset nos primeiros 20 ns e `std.env.finish`. O nome de um arquivo novo
precisa servir de entidade: letras, dígitos e `_` simples, fora das palavras
reservadas do VHDL. Um projeto pode ter Verilog e VHDL; cada simulação fica
numa linguagem: um testbench Verilog roda no Icarus ou no Verilator com os
arquivos Verilog, e um VHDL roda no GHDL com os arquivos VHDL. Um testbench
VHDL que instancia um módulo Verilog (um processador SAPHO, por exemplo) é
erro, com a explicação. `check` confere o VHDL pelo GHDL (`ghdl -i` e
`ghdl -m` de cada raiz e de cada testbench), e `hierarchy` mostra a árvore
dele, com a arquitetura de cada entidade. A simulação de VHDL tem três
passos (`analyze`, `elaborate`, `simulate`) e grava a onda em
`<testbench>.ghw` na raiz do projeto; uma asserção de nível `error` ou
`failure` reprova a simulação, e a onda vem mesmo assim.

### Processadores: `aurora proc`

| Comando | Faz | Função do Core |
|---|---|---|
| `proc add <NOME> [--lang cmm\|cpp] [--inputs N] [--outputs N] [--nubits N] [--nbmant N] [--nbexpo N] [--nugain N] [--ndstac N] [--sdepth N]` | cria a pasta `<NOME>/`, o fonte-modelo `<NOME>/<NOME>.cmm` (ou `.cpp`) e a entrada no `.spf`; a pasta de saída, `<NOME>/<NOME>/`, nasce no primeiro build | `Project::add_processor` |
| `proc set [NOME] [--freq MHZ] [--clocks N] [--arrays true\|false]` | grava frequência, clocks e exportação de arrays no `.spf`; valem a partir do próximo build. Sem `NOME`, o processador da pasta | `Project::configure_processor` |
| `proc remove <NOME> [--delete [--yes]]` | tira o processador do projeto: a entrada no `.spf`, o Verilog e o testbench gerados das listas (com a escolha de topo e de testbench que apontava para eles) e a pasta temporária da AURORA. A pasta do processador fica no disco, e os `.v` gerados nela passam a aparecer como não registrados no `aurora status`. Com `--delete`, a pasta vai para a lixeira do sistema; antes, pergunta no terminal, e sem terminal exige `--yes` | `Project::remove_processor` |

As opções `--nubits` a `--sdepth` são só de C±; com `--lang cpp`, o comando
recusa (código 2). Os padrões são os da AURORA (23, 16, 6, 128). O
fonte-modelo segue o YANC 7.1: sem `#PRNAME` (o processador se chama como o
arquivo), sem `#NDSTAC` e `#SDEPTH` se elas não forem dadas (o YANC calcula
as pilhas pelo programa) e sem `#NUGAIN` se ele for 128. Com
só `--nbmant` ou `--nbexpo`, `--nubits` acompanha (`#NBMANT + #NBEXPO + 1`,
o único que o `asmcomp` aceita). O Core recusa, antes de criar qualquer
coisa, nome que o YANC 7.1 recusa como módulo (palavra do Verilog ou do
SystemVerilog, começo com dígito ou com `__`), módulo da biblioteca SAPHO,
nome reservado do Windows, nome de uma pasta da AURORA (`synth`, `hierarchy`,
`lint`, `wave`, `surfer`, `cocotb`, em qualquer caixa), mais de 64
caracteres, e parâmetro fora do que o YANC
compila (`invalid_parameter`: `#NUBITS` diferente da soma ou acima de 32,
`#NBEXPO` fora de 2 a 8, `#NUGAIN` que não é potência de dois, pilha 0, mais
de 256 portas). `proc set` recusa `--freq` fora de 1 a 500000 e `--clocks`
fora de 1 a 2147483647, com o motivo. Os processadores do projeto aparecem
no `aurora status`.

### Projeto da estrutura antiga: `aurora convert`

Até o YANC 6.x cada processador tinha `Software/` (o fonte e o `.asm`),
`Hardware/` (o Verilog e as memórias) e `Simulation/` (o testbench e as
entradas e saídas). O YANC 7.1 não compila mais assim: o fonte e os
arquivos da pessoa ficam em `<proc>/`, e tudo o que ele gera em
`<proc>/<proc>/`. Num projeto antigo, o `build` recusa com `legacy_layout`,
e o `aurora status` avisa.

`aurora convert` faz a conversão que a AURORA faz ao abrir um projeto antigo
(`Project::convert_layout`): o que o YANC gerava (o `.asm`, o `.v`, o
testbench, as memórias, as tabelas, as saídas e a onda) vai para a pasta de
saída; o resto (o fonte, os arquivos de arrays, `input_<n>.txt`, um `.v`
posto à mão em `Hardware/`, uma subpasta) vai para a pasta do processador.
Nada é apagado nem sobrescrito: um destino que já existe deixa o arquivo
onde estava, com um aviso, e a subpasta antiga só sai vazia. A linha
`#PRNAME` (ou `#pragma yanc prname`) sai do fonte; se ela dava ao módulo
Verilog outro nome que não o do processador, um aviso lembra de atualizar
quem o instancia. O `.spf` passa a apontar para os lugares novos. Num
projeto já convertido, não faz nada. Com `--json`, o `convert.json`.

### Compilar, verificar, simular, sintetizar

| Comando | Faz | Função do Core |
|---|---|---|
| `aurora build [-p NOME]...` | compila os processadores pedidos; sem `-p`, o da pasta ou, fora deles, todos. Num projeto sem processadores, avisa e sai com 0 | `build_processors` (`Continue`) |
| `aurora check [ARQUIVO] [--lint]` | verifica o Verilog (ver abaixo), sem compilar os processadores: o Verilog que o YANC gerou entra como está no disco. Dentro da pasta de um processador, sem `ARQUIVO`, é o mesmo que `-p` com ele | `check` |
| `aurora check -p NOME [--lint]` | verifica só o Verilog do processador e o testbench gerado pelo YANC, sem recompilá-lo | `check` (`CheckOptions::processor`) |
| `aurora sim [TESTBENCH] [--verilator] [--fast] [--timeout S] [--open]` | compila os processadores que têm fonte e simula o testbench do projeto. Com `TESTBENCH`, ele passa a ser o testbench simulado (e é registrado, se não estava). Dentro da pasta de um processador, sem `TESTBENCH`, é o mesmo que `-p` com ele | `set_testbench`, `buildable_processors`, `build_processors` (`Stop`), `simulate_project` |
| `aurora sim -p NOME [--verilator] [--fast] [--timeout S] [--open]` | compila o processador e o simula com o testbench gerado pelo YANC | `build_processors` (`Stop`), `simulate` |
| `aurora synth [--svg [--module M]] [--open]` | compila os processadores e sintetiza o módulo de topo do projeto; com `--svg`, desenha o esquemático (Yosys `show` + `dot`); com `--open`, abre o RTL viewer no navegador (veja abaixo). Dentro da pasta de um processador, é o mesmo que `-p` com ele | `build_processors` (`Stop`), `synthesize`, `render_schematic`, `rtl_viewer::write` |
| `aurora synth -p NOME [--svg [--module M]] [--open]` | compila o processador e o sintetiza sozinho | `build_processors` (`Stop`), `synthesize`, `render_schematic`, `rtl_viewer::write` |

Depois de compilar, `build`, `sim`, `synth` e `fpga build` põem no `.spf`
o que cada processador gerou e o projeto ainda não tem, como a AURORA: o
`<nome>/<nome>/<nome>.v` como módulo e o `<nome>/<nome>/<nome>_tb.v` como
testbench, no fim de cada lista (`Module added: soma/soma/soma.v`).
Num projeto sem módulos, o do primeiro processador vira o topo; sem
testbench, o testbench dele vira o simulado. O que já está registrado fica
como está, e o que foi tirado com `aurora remove` volta no build seguinte.
No `--json`, o campo `registered` lista o que entrou. Uma falha ao gravar
o `.spf` vira aviso, e o build vale do mesmo jeito.

`TESTBENCH` e `-p` não andam juntos, nem `ARQUIVO` e `-p` no `check`. `--verilator` simula com o Verilator no
lugar do Icarus; ele usa o compilador do sistema no Linux e no macOS e o
do bundle no Windows (ver [BUNDLE.md](BUNDLE.md)) e grava a onda sempre em
VCD (ver [Onde fica a onda](#onde-fica-a-onda)).
`--timeout <SEGUNDOS>` (inteiro, a partir de 1) encerra a simulação que
passar desse tempo, sem contar a elaboração e a compilação; sem ele, não há
limite, e um testbench sem `$finish` roda até o Ctrl+C. No Linux e no
macOS, o prazo pede ao simulador que saia (SIGTERM), e o que o testbench
escreveu até ali aparece. No Windows não há esse pedido: a AURORA encerra o
simulador na hora, e o que ainda estava no buffer de saída dele se perde
(um `$fflush` no testbench o escreve antes). `--open` abre a onda
no surfer-aurora ao terminar. O esquemático sempre traz a largura dos
barramentos. Para ver os nomes de módulo que `--module` aceita, rode
`aurora synth -v`.

`--fast` é a simulação rápida, o Fast Sim da AURORA: roda sem gravar onda,
para ver a saída do testbench, as portas do processador e os testes cocotb
na velocidade do simulador. O testbench Verilog, do projeto ou de um
processador (`-p`), roda no Verilator, com ou sem `--verilator`, compilado
sem `--trace` num `obj_dir_fast_<topo>` à parte do da simulação com onda;
os `$dumpfile` e `$dumpvars` dele não gravam nada (o Verilator avisa, na
saída, que os ignora), e a AURORA não injeta o dump padrão. Como na AURORA, o
`$display` do testbench chega em blocos, e não linha a linha: o flush de
cada linha chegava a dobrar o tempo de um testbench que imprime a cada
ciclo. Um testbench cocotb roda os testes sem o módulo que grava a onda: no Icarus, com `vvp
-none`, ou no Verilator, com `--verilator`. O título diz `Fast
simulation`, o JSON traz `simulation.fast: true` e `waveform: null`, e
`--open` não anda junto. A primeira simulação rápida compila o modelo (de
segundos a minutos, conforme o design); as seguintes, com o design igual,
só rodam o executável que já existe.

```
$ aurora sim --fast
Fast simulation of contador_tb (Verilator): finished in 383 ms
    done      verilate      verilator    346 ms
    done      simulate      verilator     34 ms
```

`--open` mostra o netlist da síntese no RTL viewer, o esquemático
interativo que o Studio tem numa aba: entrar nos submódulos, buscar, destacar
uma rede, exportar o SVG. A AURORA grava `rtl-viewer.html` ao lado do netlist
(`.aurora8/Temp/synth/<alvo>/`), um arquivo só, com o viewer e o netlist dentro,
que abre em qualquer navegador sem servidor e sem rede, e o abre no navegador
padrão pelo abridor do sistema (`xdg-open`, `open`, o Explorer). Fora do
Studio e do VS Code, clicar numa célula mostra o arquivo e a linha, mas não
abre o editor. Precisa do componente `rtl-viewer` (`aurora install
rtl-viewer`); `AURORA_RTL_VIEWER` aponta outro modelo no lugar do do bundle (o
`packages/rtl-viewer/dist/rtl-viewer.html` de uma build de desenvolvimento).
Com `AURORA_OPENER` definida, o `--open` só grava o HTML e diz onde: a extensão
do VS Code a define (`vscode`) no terminal integrado e abre o netlist numa
aba dela. Uma síntese que falha não abre nada.

O esquemático desenha qualquer módulo, sem teto de ligações; num módulo
grande o Graphviz pode levar minutos, e o passo do `dot` tem prazo de 60 s.
O `--no-schematic-limit` das versões anteriores ainda é aceito e não faz
nada. Um `--module` fora da árvore do topo (que a síntese não inclui) sai
com código 2 (`module_not_found`, com os módulos sintetizados na mensagem),
depois de gravar o relatório da síntese.

Um build ou uma simulação de um processador que já está em outro build ou
simulação (outro terminal, o Studio) recusa na hora com
`operation_in_progress`, código 2.

`sim` e `synth` compilam os processadores antes e param no primeiro build
que falhar, com código 1, sem simular nem sintetizar. `build` compila todos
os pedidos mesmo que um falhe, para mostrar todos os erros de uma vez.
`check` não compila. Um processador com fonte que ainda não foi compilado
fica de fora da verificação, com um aviso; se não sobra nada para verificar,
ou se o Verilog ou o testbench que o build dele gera está registrado no
projeto e ainda não existe, o comando sai com `not_built` (código 2) e a
dica de rodar `aurora build`.

#### O que `check` verifica

Sem `ARQUIVO`, três passos:

1. `iverilog -tnull -Wall` (com `-g2012` se houver `.sv`) sobre o design inteiro (os módulos
   registrados, o Verilog dos processadores e o `TopLevel/*.v` legado da
   AURORA), sem escolher topo. O Icarus elabora cada raiz, então um módulo
   que nenhum outro instancia também é verificado.
2. Um `iverilog` por testbench registrado, com o design e `-s` no módulo do
   testbench.
3. Com `--lint`, `verilator --lint-only -Wall` sobre o design, com o módulo
   de topo, se houver. O Verilator acha o que o Icarus deixa passar, como
   larguras que não batem e sinais sem uso. Sem o Verilator instalado, os
   dois primeiros passos rodam do mesmo jeito, um aviso diz que o lint não
   rodou e a CLI sugere `aurora install verilator`.

Uma macro indefinida no ponto de uso (o `` `define `` num arquivo que vem
depois na lista do projeto) faz o `check` falhar, com a explicação; o
Icarus sozinho só avisa e compila com a macro vazia. Com recursão
parametrizada (um módulo que se instancia num `generate`), as raízes vão com
`-s`. Com muitas raízes, o título diz quantas e nomeia as cinco primeiras.

Com `ARQUIVO`, só os módulos daquele arquivo são elaborados como raiz, com o
resto do design para resolver as instâncias; um testbench é elaborado com o
design.

Com `-p NOME`, ou dentro da pasta do processador, o design é só o
`<NOME>/<NOME>/<NOME>.v` que o build gerou, e o testbench é o dele,
`<NOME>/<NOME>/<NOME>_tb.v`; os passos são os mesmos, e o lint usa o
processador como topo. Os outros processadores não são compilados nem
verificados.

A saída lista os módulos elaborados como raiz e os diagnósticos de cada
passo. Avisos, do Icarus ou do Verilator, não fazem o `check` falhar. O
`check` não gera artefato, só o relatório do histórico
([Relatórios](#relatórios)), e sai com 1 se algum passo falhou.

#### O que `sim` mostra

- enquanto roda, o que o testbench escreve (`$display`, `$monitor`), linha a
  linha, cada uma depois de `|`, sem precisar de `-v`. As linhas que o
  próprio simulador escreve (`$finish called at`, `VCD info:` ou
  `FST info:`, o `ERROR:` de um `$error`) não saem aí: ficam para o resumo,
  como diagnósticos, e as `info` só aparecem com `-v`;
- com `-v`, enquanto roda, o comando de cada passo (depois de `$`) e tudo o
  que ele escreve, inclusive as mensagens das ferramentas;
- depois, o resumo: a linha de título, os diagnósticos e a onda;
- com `-p`, depois de uma simulação que deu certo, os valores de cada porta
  de saída, uma linha por porta: `Output 0: 5 -2 26`;
- quando falta um arquivo de entrada do processador, qual criar:
  `Warning: Create soma/input_0.txt with one value per line (the testbench reads it)`;
- quando passa do `--timeout`, o título
  `timed out, vvp stopped at the 1 s limit` e a dica
  `Does the testbench reach $finish? Without it the simulation never ends.
  Otherwise, raise --timeout`;
- com `-p`, um aviso quando o programa não chega ao fim nos clocks
  simulados (as saídas param ali; suba com `aurora proc set <nome> --clocks N`),
  e outro para um arquivo de entrada vazio ou com valor que não cabe em
  `#NUBITS` bits. Uma linha de entrada que não é inteiro recusa a simulação
  antes de rodar (`invalid_data_file`, com o arquivo e a linha);
- com `-p`, um aviso quando o programa lê mais valores do que um
  `input_<n>.txt` tem, com o arquivo e a linha (dali em diante a porta
  repete o último valor);
- sem `-p`, um aviso quando o Icarus grava a onda em VCD e ela passa de
  100 MB, com a sugestão de `$dumpfile("<nome>.fst")`. Só acontece quando o
  AURORA não consegue trocar a extensão do `$dumpfile` (a chamada quebrada em
  linhas);
- uma linha `ERROR: ...` que o testbench escreve com `$display` sai como as
  outras (`| ERROR: ...`) e não reprova; só o `$error` e o `$fatal` do
  Verilog reprovam;
- sem `-p`, um aviso quando o testbench chama `$dumpfile` sem `$dumpvars`
  (a onda não sai). Um `$dumpfile(ONDA)` com `localparam`, `parameter` ou
  `` `define `` de texto vale como o nome entre aspas.

```
  | q = 10
Simulation of contador_tb (Icarus): finished in 0.09 s
    done      elaborate     iverilog       65 ms
    done      simulate      vvp            20 ms
  Generated:
    Icarus image        .aurora8/Temp/contador_tb.vvp
    waveform            contador_tb.fst
  Open the waveform with: aurora wave
Report run-000002: aurora report show 2
```

A simulação falha, com código 1, quando o Verilog não elabora, quando o
simulador sai com código diferente de 0 e quando o `vvp` escreve uma linha
`ERROR:` ou `FATAL:`, que é o que `$error` e `$fatal` fazem no Icarus. O
`vvp` sai com 0 depois de um `$error`; a AURORA conta isso como falha. Passar
do `--timeout` também sai com 1.

#### Onde fica a onda

O nome vem do `$dumpfile` do testbench, relativo à raiz do projeto; a
extensão é a do formato que o simulador grava: `.fst` no Icarus, `.vcd` no
Verilator (o FST do Verilator exige a lz4, que o bundle não traz). Um
`$dumpfile("saida.vcd")` simulado no Icarus grava `saida.fst`: a AURORA troca
a extensão numa cópia do testbench, e o arquivo do usuário não muda. O
testbench-modelo do `aurora add` grava `<testbench>.fst`. Num testbench sem
`$dumpfile`, a AURORA simula uma cópia com `$dumpfile("<tb>.fst")` (`<tb>.vcd`
com o Verilator) e `$dumpvars(0, <tb>)`, em que `<tb>` é o módulo do
testbench: a onda fica na raiz, com todos os sinais, inclusive os do módulo
testado. Com as duas extensões no disco, `aurora wave` abre a mais recente.
Com um `$dumpfile` que é uma expressão que a AURORA não resolve, a onda vai
para onde o testbench mandar, em VCD, e `aurora wave` pede o arquivo.

Com `-p`, o testbench é o que o YANC gerou, `<NOME>/<NOME>/<NOME>_tb.v`, e
a simulação roda nessa pasta de saída, como o YANC 7.1 pede e a AURORA faz:
o testbench lê as entradas na pasta do processador (`../input_<n>.txt`) e
grava ali as saídas (`output_<n>.txt`) e a onda (`<NOME>_tb.fst`, ou `.vcd`
com o Verilator). Escolhido como testbench do projeto (`aurora sim
<NOME>/<NOME>/<NOME>_tb.v`), ele também roda na pasta de saída; qualquer
outro testbench roda na raiz, para onde vão as memórias e o
`pc_<proc>_mem.txt` de cada processador, que o Verilog gerado lê pelo nome.

#### Testbench cocotb

Um `.py` registrado como testbench é um testbench cocotb: só os testes
(`@cocotb.test()`), e a AURORA monta a simulação, como a AURORA. Os testes
recebem como `dut` o módulo da linha `# aurora-toplevel: <módulo>` do
arquivo (a diretiva da AURORA, um comentário para o Python) ou, sem ela, o
módulo de topo do projeto, com um aviso. `aurora sim test_alu.py` (ou `aurora
sim`, com ele como testbench do projeto) roda no Icarus os passos de sempre:
o `iverilog` elabora o design com o `dut` na raiz, e o `vvp` carrega a VPI
do cocotb, que roda os testes no Python do bundle. Com `--verilator`, o
Verilator compila o design com o `main` e a VPI do cocotb, e o executável
roda os testes. Precisa do componente `cocotb` (`aurora install cocotb`).

```
    done      elaborate     iverilog      108 ms
    failed    simulate      vvp           659 ms
  test_alu.py:19: error: Test test_alu.vai_um failed: y = 16
    passed    test_alu.soma
    failed    test_alu.vai_um
  Tests: 1 of 2 passed
  Open the waveform with: aurora wave
```

Cada teste que falha é um erro na linha do `.py` em que falhou e reprova a
simulação (código 1). A onda sai do mesmo jeito, em `<módulo de teste>.fst`
na raiz (`.vcd` no Verilator), com todos os sinais do `dut`, e `--open` a
abre: é nela que se vê a falha. Um `.py` sem nenhum `@cocotb.test()` também reprova. O log do
cocotb sai como a saída do testbench, e no JSON os testes estão em
`simulation.tests` (`cases`, com nome, `status`, mensagem, arquivo e
linha, e as contagens `passed`, `failed` e `skipped`).

Com `--fast`, os testes rodam sem gravar onda, no Icarus ou, com
`--verilator`, no Verilator.
O `check` e o `hierarchy` deixam o `.py` de fora, porque elaboram Verilog,
e `aurora check test_alu.py` recusa (`invalid_name`). A primeira simulação
cocotb da máquina leva alguns segundos a mais: o Python compila o cocotb, e os
`.pyc` ficam na pasta de cache do usuário, não no projeto nem no bundle.

### Hierarquia

| Comando | Faz | Função do Core |
|---|---|---|
| `aurora hierarchy` | a árvore de instâncias do design e de cada testbench (os registrados e o de cada processador compilado que não esteja entre eles), como o Icarus as elabora. Dentro da pasta de um processador, é o mesmo que `-p` com ele | `hierarchy` |
| `aurora hierarchy -p NOME` | a do processador e do testbench gerado pelo YANC | `hierarchy` (`HierarchyOptions::processor`) |

Cada linha é `instância: módulo`, ou só o nome quando os dois são iguais,
recuada pela profundidade; instâncias dentro de blocos `generate` levam o
nome do bloco na frente (`op_add.my_add`). Os módulos da biblioteca SAPHO
saem marcados `SAPHO` e fechados, com o número de instâncias dentro; `-v`
abre todos e mostra o arquivo e a linha de cada definição. No fim, o resumo
das elaborações, como no `check`.

`hierarchy` não compila os processadores nem grava relatório: mostra o que
está no disco. Um processador ainda não compilado fica de fora, com um
aviso de como compilar. Uma elaboração que falha (código 1) não esconde as
outras: o design aparece mesmo com um testbench quebrado, e o erro sai com
arquivo e linha.

### Onda

| Comando | Faz | Função do Core |
|---|---|---|
| `aurora wave` | abre no surfer-aurora a onda da simulação do projeto; dentro da pasta de um processador, a dele | `waveform_path`, `open_waveform` |
| `aurora wave -p NOME` | abre a onda da simulação do processador | `waveform_path`, `open_waveform` |
| `aurora wave <ONDA>` | abre um arquivo de onda (VCD, FST, GHW), relativo ao diretório atual; a onda de um exercício do `aurora learn` (em `.aurora-learn/`) abre com o layout da última correção | `open_waveform`, `aurora_learn::layout::with_exercise_layout` |
| `aurora wave --no-layout` | abre a onda crua, sem layout | `open_waveform` |
| `aurora wave --reset-layout` | apaga o layout salvo (`wave/<testbench>.surf.ron`) e abre com o gerado | `reset_saved_layout`, `prepare_wave_layout` |
| `aurora wave signals` | a árvore de sinais do testbench do projeto, com `*` no que a onda grava por escolha | `wave_signals` |
| `aurora wave select CAMINHO...` | acrescenta sinais ou escopos à escolha (`wave/<testbench>.json`); `--all` apaga a escolha | `write_selection` |
| `aurora wave unselect CAMINHO...` | tira sinais ou escopos da escolha | `SignalScope::without`, `write_selection` |

Numa onda com processador SAPHO, o Surfer abre com o layout da AURORA: as
variáveis do programa, a instrução de assembly e a linha do C± de cada
ciclo, em grupos por processador (`wave_layout` e `prepare_wave_layout`,
API.md 5.7.1 e 5.7.2). O
texto lista cada processador do layout. `aurora sim --open` faz o mesmo.

**O layout salvo.** Numa onda de testbench do projeto, o layout fica em
`wave/<testbench>.surf.ron`, na pasta do projeto: o Surfer abre com ele e
salva nele (Ctrl+S, ou File > Save state). Na primeira abertura o arquivo
nasce com o layout gerado, e a AURORA o refaz a cada abertura enquanto você
não salva (a escolha de sinais ou os processadores podem ter mudado).
Depois que você salva, ele fica como está; `aurora wave --reset-layout` o
apaga e volta ao gerado. O texto diz qual dos dois abriu.

**A escolha de sinais**, a Wave Configuration da AURORA: por testbench, o
que a simulação do projeto grava e o que o layout mostra. Fica em
`wave/<testbench>.json`, e cada item é um sinal ou um escopo pelo caminho na
hierarquia (`mux_tb.dut`, `mux_tb.dut.sel`); um escopo vale por tudo o que
há dentro dele.

```
aurora wave signals                          # a árvore, da elaboração do Icarus
aurora wave select mux_tb.sel mux_tb.dut.u0  # só estes
aurora wave unselect mux_tb.dut.u0.t         # tira um sinal de dentro do escopo
aurora sim                                   # grava só a escolha
aurora wave select --all                     # volta a gravar tudo
```

Sem escolha, a onda grava todos os sinais, como antes. Com escolha, a
simulação grava só ela, numa cópia do testbench: o primeiro `$dumpvars`
dele vira o da escolha, os outros viram comentário, e sem `$dumpvars` a
escolha entra no dump que a AURORA injeta; o arquivo do testbench não muda.
Um processador SAPHO do design entra sempre com os sinais do escopo dele,
que o layout das instruções e das variáveis usa. O layout mostra só os
escolhidos, um grupo por escopo. Um item que o design não tem mais fica de
fora da onda, com um aviso. O `select` confere os caminhos na árvore e
sugere o certo para um nome que existe em outro lugar; o `unselect` de um
sinal dentro de um escopo escolhido deixa os outros sinais do escopo. No
Verilator, que ignora a lista do `$dumpvars`, a onda grava tudo e o layout
mostra só a escolha. A escolha vale para a simulação do projeto; a de um
processador (`aurora sim -p`) grava o que o testbench gerado pelo YANC escolhe.

Sem `ONDA`, a AURORA abre o arquivo em que a simulação grava a onda (ver
[Onde fica a onda](#onde-fica-a-onda)); ele não simula. O comando retorna logo, sem esperar o Surfer
fechar. Se o Surfer fechar em menos de 1,5 s (sem display, onda inválida), o
comando falha com código 2 (`process_exited_early`) e mostra o fim do log do
Surfer (`RunningProcess::ensure_started`).

O log do Surfer não fica ao lado da onda: vai para `.aurora8/Temp/surfer/` do
projeto, como `contador_tb.fst.log`, e cada abertura da mesma onda
sobrescreve o anterior. Uma onda fora de projeto (`aurora wave` com um arquivo
de outra pasta) deixa o log na pasta de cache do usuário, `~/.cache/aurora/surfer`
no Linux, `~/Library/Caches/aurora/surfer` no macOS e `%TEMP%\aurora\surfer` no
Windows.

### Relatórios

Cada `aurora build`, `check`, `sim` e `synth` que roda alguma ferramenta grava
um relatório no histórico do projeto, `.aurora8/reports/run-NNNNNN/`, e diz no
fim qual foi (`Report run-000042: aurora report show 42`). A operação que
falha também grava; a que nem começa (projeto inválido, componente que
falta) não. Se o relatório não puder ser gravado, a operação não falha por
isso: sai um aviso.

| Comando | Faz | Função do Core |
|---|---|---|
| `aurora report` | mostra o relatório mais novo | `history::latest`, `history::report_text` |
| `aurora report show ID` | mostra um relatório; `ID` é `run-000042` ou só `42` | `history::load`, `history::report_text` |
| `aurora report list [--limit N]` | lista os relatórios, do mais novo para o mais antigo, com o que cada um tem para comparar | `history::list` |
| `aurora report compare [ID] [--against ID] [--summary]` | compara as estatísticas de síntese e os tempos de simulação de dois relatórios | `history::compare_reports` |
| `aurora report clean [ID...] [--keep N] [--yes]` | apaga relatórios: sem argumento, todos; com `--keep N`, todos menos os `N` mais novos; com `ID`, só esses | `history::plan_cleanup`, `history::remove` |

Nenhum desses comandos roda ferramenta: eles só leem o histórico, menos o
`clean`, que apaga. Apagar todos, ou todos menos os mais novos, pergunta
antes (`[y/N]`, no stderr); sem terminal, exige `--yes`. Os relatórios
pedidos pelo `ID` saem sem pergunta, e um `ID` que não existe faz o comando
falhar sem apagar nenhum. O número de um relatório apagado não volta: depois
de apagar o `run-000012`, o próximo é o `run-000013`, mesmo com o histórico
vazio.

O relatório tem o status e o comando; a máquina (sistema, processador,
memória); as ferramentas do bundle que rodaram, com a versão e o caminho; o
projeto; o tempo de cada fase e de cada ferramenta; as estatísticas
genéricas de síntese, quando houve síntese; os artefatos; e, numa falha, o
erro e o fim da saída do passo que falhou.

As estatísticas de síntese saem do `stat` do Yosys sobre o netlist que a
síntese gera: módulos, fios, bits, memórias, processos, células e células
por tipo. São células genéricas do Yosys, não mapeadas para FPGA: não há
estimativa de LUT, DSP, ocupação nem temporização. Uma contagem que o Yosys
não informou aparece como `not reported`, e é diferente de zero.

`compare` sem `ID` compara o relatório mais novo; com `ID`, aquele. Sem
`--against`, a referência é o relatório mais novo, entre os anteriores, que
se compara com ele em pelo menos uma parte, de preferência uma que terminou
bem (uma simulação que estourou o prazo só vira referência sem outra); com
`--against`, é exatamente o pedido. Dois relatórios sem parte em comum (uma
simulação contra uma síntese) saem com `not_comparable`, código 2. Os
relatórios continuam se comparando depois de a pasta do projeto mudar de
lugar. A síntese se compara quando os dois têm estatísticas do mesmo topo;
a simulação, quando os dois têm tempos do mesmo simulador e testbench. O que
difere além disso (fontes, entradas, versões das ferramentas, a máquina)
aparece em `CONTEXT WARNINGS`, sem impedir a comparação. Uma mudança é só
descrita: tempo maior não é chamado de regressão. A porcentagem é da
referência; de zero para outro número, aparece `new`.

Os tempos são de relógio, em milissegundos: a compilação (`iverilog` ou
`verilator --binary`), a execução (`vvp` ou o modelo) e a simulação inteira,
medida à parte. O tempo simulado é outra grandeza, o quanto o tempo andou
dentro do modelo: vem do `$finish called at` do `vvp`, e o Verilator não o
informa. Uma rodada só não é medida de desempenho.

### Exercícios: `aurora learn`

Os exercícios de Verilog do componente `aurora-learn`, no estilo do
rustlings: `aurora learn init` cria a pasta de exercícios, e `aurora learn`,
dentro dela, entra no modo watch, que corrige o exercício atual a cada
gravação. A correção roda o `check` e a simulação no Icarus do projeto do
exercício e compara cada saída com a de uma solução de referência; ela não
grava relatório.

| Comando | Faz | Função |
|---|---|---|
| `aurora learn` | o modo watch (precisa de terminal) | `aurora_learn::grade` a cada gravação |
| `aurora learn init [PASTA] [--track ID]` | cria a pasta de exercícios (padrão `aurora-learn`) | `Workspace::init` |
| `aurora learn check [NOME] [--all]` | corrige e grava se está resolvido; 0 se tudo pedido está | `grade`, `Workspace::record` |
| `aurora learn list` | os exercícios, o atual e os resolvidos | `Workspace::statuses` |
| `aurora learn hint [NOME]` | as dicas | `Exercise::hints` |
| `aurora learn reset NOME [--yes]` | volta o arquivo do exercício ao começo | `Workspace::reset` |
| `aurora learn wave [NOME]` | abre a onda da última correção, com as saídas ao lado das da referência e o primeiro erro marcado | `open_waveform` com o `.sucl` da correção |
| `aurora learn dev check [TRILHA]` | confere uma trilha: o arquivo inicial não passa, a solução passa | `check_track` |

`AURORA_LEARN_DIR` aponta a pasta das trilhas no lugar do componente. As
teclas do modo watch, a pasta de exercícios e o formato das trilhas estão em
[LEARN.md](LEARN.md).

### Placas FPGA: `aurora fpga`

As placas FPGA são um componente experimental, desligado por padrão. Com
ele desligado, todo `aurora fpga` sai com 2 e o aviso de como ligar:

```
$ aurora fpga build
Error: The experimental component fpga (FPGA boards) is disabled
  Enable it with: aurora experimental enable fpga (in AURORA IDE: Settings > Experimental)
```

Ligado com `aurora experimental enable fpga` (ou no Studio, em Preferências >
Experimental), vale para a CLI e para o Studio.

A placa e a ligação das portas do topo aos sinais dela ficam no
`fpga.json`, ao lado do `.spf` ([FPGA.md](FPGA.md)).

| Comando | Faz | Função |
|---|---|---|
| `aurora fpga boards [PLACA]` | as placas conhecidas; com uma, os sinais, os pinos, o padrão de I/O e quais são ativos em nível baixo | `fpga::boards`, `fpga::board` |
| `aurora fpga check [--show-top]` | confere o `fpga.json` contra a placa e as portas do topo e mostra o que vai em cada porta e em cada sinal da placa, com os ajustes de largura; com `--show-top`, o Verilog do topo da placa | `fpga::prepare` |
| `aurora fpga build` | confere o `fpga.json`, a placa e o Quartus, compila os processadores, grava o projeto do Quartus em `.aurora8/fpga/<placa>/` e roda `quartus_map`, `quartus_fit`, `quartus_asm` e `quartus_sta` (passos `synthesize`, `fit`, `bitstream` e `timing`). Depois do Fitter confere cada pino no `.pin` contra a placa, e reprova antes do `.sof` se algum ficou fora do lugar. Mostra os erros e avisos do Quartus com o número da mensagem, os recursos da FPGA e a Fmax e as folgas de cada clock. Folga negativa é aviso, não falha | `fpga::build` |
| `aurora fpga program [--cable NOME] [--list]` | grava na placa o `.sof` da última `aurora fpga build` pelo Quartus Programmer (`quartus_pgm -c <cabo> -m jtag -o "p;output_files/aurora_board_top.sof@<posição>"`), pelo primeiro cabo ligado ou pelo de `--cable`. Recusa, antes de procurar o cabo, um `.sof` que não descreve o projeto de agora (`stale_bitstream`: um fonte, uma memória, o `fpga.json` ou a placa mudou, ou o `.sof` foi trocado). A gravação vai para a SRAM da FPGA e some ao desligar a placa. Com `--list`, só os cabos (`quartus_pgm -l`) | `fpga::program`, `fpga::cables` |

A compilação e a gravação para a placa não entram no histórico
(`aurora report`).

A placa virtual também é do recurso `fpga` ([FPGA.md](FPGA.md#placa-virtual-aurora-vboard)):

| Comando | Faz | Core |
|---|---|---|
| `aurora vboard run [--board PLACA] [--time MS] [--set NOME=VALOR]... [--press NOME=MÁSCARA]... [--keys TEXTO] [--uart TEXTO] [--png ARQUIVO] [--speed FATOR]` | compila o projeto com os periféricos da placa simulados (o topo da placa do `fpga.json`; sem ele, as portas com o nome de um sinal da `--board`), roda `--time` milissegundos simulados (padrão 50) com as chaves de `--set` e os botões de `--press` (valor em decimal, `0x` ou `0b`), o texto de `--keys` digitado no teclado PS/2 e o de `--uart` na serial, e mostra os LEDs, os dígitos dos displays e o que a serial recebeu; `--png` grava o último quadro de vídeo; `--speed 1` limita ao tempo real (padrão 0, o mais rápido possível) | `virtual_board::build`, `virtual_board::run` |

### Componentes experimentais: `aurora experimental`

Partes da AURORA que ainda mudam muito ficam desligadas até o usuário ligar.
Hoje há três: `fpga` (as placas FPGA: `aurora fpga` e, no Studio, a aba Placa
FPGA), `assistant` (o assistente de IA: `aurora chat`, `aurora mcp` e, no
Studio, a vista Assistente) e `git` (o controle de versão: os métodos `git.*` e
`github.*` do `aurora serve` e, no Studio, as vistas Controle de versão e
Perfil).

| Comando | Faz |
|---|---|
| `aurora experimental` | lista os componentes experimentais, se cada um está ligado (`on`) ou não (`off`), e o arquivo onde isso fica |
| `aurora experimental enable <componente>` | liga o componente, para a CLI e para o Studio |
| `aurora experimental disable <componente>` | desliga |

O que está ligado fica em `experimental.json`, na pasta de configuração do
AURORA (`%APPDATA%\aurora` no Windows, `$XDG_CONFIG_HOME/aurora` ou
`~/.config/aurora` no Linux, `~/Library/Application Support/aurora` no macOS):
`{"enabled": ["fpga"]}`. A página Experimental das Preferências do Studio
liga e desliga o mesmo arquivo. Sem o arquivo, ou com ele ilegível, nada
está ligado.

### Bundle

| Comando | Faz |
|---|---|
| `aurora tools [--verify]` | o bundle (identificador, plataforma, componentes instalados com versão e origem, e os não instalados), cada ferramenta (`OK`, `--` se o componente dela não foi instalado ou, no `quartus`, se não há Quartus, `!!` se falta o executável), o compilador do Verilator, com `(bundle)` ou `(system)`, e o Quartus Prime, se encontrado (no macOS, onde não existe, a linha some). Com `--verify`, confere o SHA-256 de cada executável e sai com 1 se algum não conferir |
| `aurora install [APLICATIVO...] [--from CAMINHO]` | instala aplicativos do bundle (yanc, icarus, verilator, cocotb, yosys, graphviz, surfer-aurora, openfpgaloader, studio, aurora-learn, rtl-viewer) na instalação de onde este `aurora` roda, sem reinstalar a AURORA e sem tirar nenhum. Sem nomes, abre no terminal a lista com os instalados travados; com nomes, instala direto, com o que eles exigem. Só os pedaços dos aplicativos novos são baixados (da release desta versão, conferidos pelo `SHA256SUMS`; `AURORA_RELEASE_URL` troca a origem por um espelho) ou lidos de `--from` (a pasta, o `.tar.gz` ou o `payload/` de um instalador, ou uma pasta com os arquivos que a release publica), com a barra de cada download. Uma falha no meio desfaz o que entrou. O `studio` ganha o atalho no menu de aplicativos (Linux e macOS). Recusa um `aurora` que não foi instalado pelo instalador, um nome que não é do bundle e um bundle diferente do instalado |
| `aurora update [--check] [--yes] [--full] [--from CAMINHO]` | compara a AURORA, o bundle e cada aplicativo instalado com a última release (o `bundle/versions.json` da tag dela) e com a última versão upstream (releases do OSS CAD Suite, do aurora-toolchain, do YANC e da AURORA IDE no GitHub, tags do surfer-aurora e releases do Graphviz no GitLab), e marca `(new)` o que é mais novo; `?` é uma fonte que não respondeu. Com `--check`, só mostra. Sem ele, se há AURORA mais nova, atualiza por componentes: compara o manifesto de arquivos da instalação (`toolchain/files.json`) com o da release, mostra o que muda e quanto baixar, pergunta (sem terminal, exige `--yes`) e baixa só os pedaços com arquivos que mudaram, conferidos pelo `SHA256SUMS`. Os arquivos trocam no lugar, o próprio `aurora` também, e o bundle é conferido; uma falha no meio devolve a instalação ao que era. Sem o manifesto (uma instalação anterior à 0.7.0), com `--full` ou se a atualização por componentes falha, baixa o instalador inteiro da release: no Linux e no macOS reinstala com os mesmos aplicativos, a mesma pasta e o mesmo atalho; no Windows abre o assistente. `--from` atualiza por componentes a partir de um instalador no disco (como no `aurora install --from`), sem rede. Ferramenta mais nova upstream não é instalada: chega num bundle novo, numa release nova da AURORA. Sem rede ou com o GitHub fora, sai com 2. Recusa atualizar um `aurora` que não foi instalado pelo instalador |
| `aurora uninstall [--yes]` | remove a instalação de onde este `aurora` roda, com o bundle inteiro: no Linux e no macOS roda o `uninstall.sh` da pasta (sai `toolchain/`, `bin/aurora`, o atalho e a pasta), no Windows abre o desinstalador do Inno (que também tira a pasta do PATH). Pergunta antes; sem terminal, exige `--yes`. Recusa um `aurora` que não foi instalado pelo instalador. Também apaga o `~/.config/aurora/config.json` de um build anterior à 0.2.0, e tira o atalho do Studio no menu. Os projetos ficam |

O compilador do sistema para o Verilator (Linux e macOS), quando está fora
do local padrão, é declarado com `--compiler <DIR>` ou `AURORA_COMPILER`
([De onde vêm as ferramentas](#de-onde-vêm-as-ferramentas)).

Um `aurora` instalado pelo instalador procura sozinho, uma vez por dia, se há
AURORA mais nova publicado (a mesma consulta do `aurora update`). A consulta
roda enquanto o comando roda, e no fim, se há versão nova, uma linha no
stderr avisa:

```
AURORA 0.8.0 is available (this is 0.7.0): update with aurora update
```

O aviso de uma versão sai uma vez por dia até a atualização. Se a consulta
não voltou até o fim do comando, o comando espera por ela no máximo 1,5 s
e o aviso fica para a próxima vez; sem rede, ela tenta de novo uma hora
depois. A hora da última consulta, a versão achada e a do último aviso
ficam em `aurora/update-check.json` na pasta de cache do usuário
(`$XDG_CACHE_HOME` ou `~/.cache` no Linux e no macOS, `%LOCALAPPDATA%` no
Windows); o `aurora update` também grava ali o que consultou. Não há consulta
nem aviso com `--json` ou `--events`, fora de um terminal, no `aurora update`,
no `aurora uninstall`, no `aurora setup`, no `aurora serve`, no `aurora mcp`, no
`aurora chat` e no `aurora completions`, num `aurora` que não veio do instalador, com a
variável `CI` definida ou com `AURORA_NO_UPDATE_CHECK=1`.

`aurora setup --payload <pasta> --prefix <pasta> [--components a,b]` instala
o bundle desses aplicativos em `<prefix>/toolchain`, a partir dos arquivos
da release baixados em `<payload>`: extrai os pedaços, confere os
executáveis e troca o `toolchain/` que houver. É o que o assistente web do
Windows roda antes de copiar o `aurora.exe` ([INSTALL.md](INSTALL.md),
"Windows"), com uma linha `[i/n] ...` por pedaço. Não aparece na ajuda.

`aurora completions <bash|zsh|fish|elvish|powershell>` gera o script de
autocompletar. Ele não aparece na ajuda. Para instalar no bash:

```
aurora completions bash > ~/.local/share/bash-completion/completions/aurora
```

### A API para outros programas: `aurora serve` e `aurora mcp`

Um programa que não é Rust (a extensão do VS Code, um agente, um script)
fala com a AURORA por um processo vivo em vez de um processo por comando. O
Studio é cliente do `aurora serve`, como a extensão; a CLI chama direto as
mesmas funções do Core que o despacho do servidor chama
([API.md, seção 5.11](API.md#511-os-fluxos-compostos-a-entrega-a-busca-do-bundle-e-o-contrato)).

| Comando | Faz |
|---|---|
| `aurora serve [--schema] [--allow-update]` | o servidor da AURORA: atende pedidos JSON-RPC 2.0 por stdin e stdout, uma mensagem JSON por linha (o enquadramento do MCP, não o `Content-Length` do LSP). Cada pedido traz o caminho do projeto; não há projeto atual. O primeiro pedido diz o idioma da sessão: um cliente MCP, cujo `initialize` traz `protocolVersion`, recebe a mesma sessão do `aurora mcp`, com as mesmas ferramentas, o projeto do `-C` como padrão e `--allow-update` para as que mudam a instalação. Esse idioma é o do assistente de IA e exige o recurso experimental `assistant`; desligado, o `initialize` volta com o erro e a dica. `--schema` escreve o JSON Schema do protocolo (os pedidos e as respostas) e sai |
| `aurora mcp [--allow-update]` | o mesmo servidor, sempre em MCP, e o nome que os agentes já têm na configuração: o mesmo contrato como servidor MCP (Model Context Protocol, versão `2025-06-18`) para um agente: `initialize`, `tools/list`, `tools/call`, `ping`, `notifications/cancelled`. Uma ferramenta por método que serve a um agente (`project_status`, `history_show`, ...), com o `inputSchema` tirado do schema do protocolo, e o `flow.run` desdobrado em `build`, `check`, `hierarchy`, `sim` e `synth`, mais `project_find`. Não viram ferramenta o `project.check_name`, o `processor.check_name`, o `wave.open`, o `wave.reset_layout`, o `history.plan_cleanup`, o `history.clean`, o `experimental.set` e o `fpga.cables`, nem os fluxos `schematic`, `fpga_build` e `fpga_program`. `update_apply` e `install_add`, que mudam a instalação, só existem com `--allow-update` |
| `aurora mcp setup <claude-code\|claude-desktop\|print> [--scope local\|user\|project] [--allow-update] [--default-project PATH]` | registra o servidor no cliente, com o caminho deste `aurora` e, se o bundle não é o instalado ao lado dele, a variável `AURORA_TOOLCHAIN`. `claude-code` roda `claude mcp add` (ou grava o `.mcp.json` da pasta com `--scope project`; sem o `claude` no `PATH`, imprime o comando); `claude-desktop` mescla a entrada no `claude_desktop_config.json`, mantendo o resto do arquivo; `print` só imprime o trecho JSON |
| `aurora chat [--model M] [--resume ID] [--allow-edits] [--allow-update] [--claude PATH]` | o assistente da AURORA no terminal: uma tela com a conversa, o que o assistente faz (cada ferramenta chamada, com o resultado) e o campo de texto. Quem responde é o Claude Code instalado na máquina, aberto como processo filho com o `aurora mcp` desta pasta (ou do `-C`) como ferramentas; quem autentica é ele (a conta, ou `ANTHROPIC_API_KEY`), e a AURORA não guarda chave. Sem o Claude Code, diz como instalar. `--allow-edits` deixa o assistente editar e criar arquivos na pasta do projeto, além de usar as ferramentas da AURORA e ler; `--allow-update` expõe `update_apply` e `install_add`. No fim imprime o `session_id`, que `--resume` retoma. Enter envia, Esc interrompe a resposta, PgUp e PgDn rolam, Ctrl+Q sai |

Os dois são processos filhos que o cliente abre quando precisa e fecha
quando termina, como um editor faz com um servidor de linguagem: sem
serviço do sistema, porta ou arquivo de pid. Só stdio. O stdout é só do
protocolo; o log vai para o stderr, como nos outros comandos.

O `aurora mcp` e o `aurora chat` são o componente experimental `assistant`,
desligado por padrão: sem ligar, os dois param com `experimental_disabled`
e a dica para ligar (`aurora experimental enable assistant`, ou Preferências
> Experimental no Studio). O `aurora serve` não é experimental.

Uma sessão do `serve`:

```
→ {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"client":"meu-script 1.0"}}
← {"jsonrpc":"2.0","id":1,"result":{"aurora":"0.9.0","protocol":1,"prefix":"/opt/aurora","experimental":{"assistant":false,"fpga":false},"methods":["initialize","shutdown","project.create",...]}}
→ {"jsonrpc":"2.0","id":2,"method":"flow.run","params":{"project":"/home/eu/soma/soma.spf","request":{"flow":"simulate","simulator":"icarus"}}}
← {"jsonrpc":"2.0","method":"phase","params":{"id":2,"phase":"build"}}
← {"jsonrpc":"2.0","method":"events","params":{"id":2,"events":[{"event":"step_started",...},...]}}
← {"jsonrpc":"2.0","method":"build","params":{"id":2,"result":{"processor":"soma","status":"succeeded",...}}}
← {"jsonrpc":"2.0","method":"phase","params":{"id":2,"phase":"simulate"}}
← {"jsonrpc":"2.0","id":2,"result":{"flow":"simulate","command":"aurora sim","status":"succeeded","succeeded":true,"builds":[...],"simulation":{...},"report":"run-000003",...}}
→ {"jsonrpc":"2.0","id":3,"method":"shutdown"}
← {"jsonrpc":"2.0","id":3,"result":null}
← {"jsonrpc":"2.0","method":"shutdown","params":{"reason":"requested"}}
```

- **Métodos.** Os do `aurora-protocol`, com um grupo e um ponto:
  `project.*` (`create`, `status`, `add`, `remove`, `top`, `testbench`,
  `order`, `move`, `convert`, `check_name`), `processor.*` (`check_name`,
  `add`, `set`, `remove`, `outputs`, `defaults`), `flow.run` (o fluxo em
  `request`: `build`, `check`, `hierarchy`, `simulate`, `synthesize`,
  `schematic`, `fpga_build`, `fpga_program` e `learn`, a correção de um
  exercício; o resultado é um `FlowOutcome` com `status`, e
  `step_output_limit` corta o `stdout` e o `stderr` de cada passo no
  resultado), `flow.command` (o comando de um fluxo, como vai para o
  histórico), `wave.*` (`open`, `signals`, `choose`, `reset_layout`,
  `layout`), `verilog.*` (`modules`, `testbench_names`), `history.*`
  (`list`, `show`, `compare`, `plan_cleanup`, `clean`), `toolchain.*`
  (`info`, `verify`), `experimental.*` (`list`, `set`), `fpga.*` (com
  `top` e `connect`), `vboard.*` (`layout`, `run`, `input`), `update.check`, `update.apply`, `install.add`, `learn.*`
  (`check`, `tracks`, `status`, `init`, `set_current`, `reset`), `git.*` (`status`, `diff`,
  `read_file`, `commit`, `discard`, `init`, `log`, `commit_files`, `branches`, `switch`, `fetch`,
  `pull`, `push`, `set_identity`, `set_remote`, `remove_remote`) e `github.*` (`account`, `login_start`,
  `login_poll`, `logout`), estes dois no recurso experimental `git`,
  `initialize` e `shutdown`. O `initialize` lista os que o servidor atende.
  Cada um, com os parâmetros, o resultado e os tipos:
  [PROTOCOL.md](PROTOCOL.md), gerado dos tipos.
  Os schemas: `docs/schema/protocol-request.json` (os pedidos, com os
  campos com padrão opcionais) e `protocol-response.json` (as respostas,
  as notificações, o `ErrorInfo`).
- **Bundle.** O `--toolchain`, o `--compiler` e o `--quartus` valem para a
  sessão inteira. Um compilador ou um Quartus declarado que não serve não
  derruba o bundle, ao contrário de um comando da CLI: a sessão segue sem
  ele, a operação que o exige falha com o erro dela (`quartus_missing`, por
  exemplo), e o `toolchain.info` mostra o motivo (`compiler_error`,
  `quartus_error`).
- **Notificações**, enquanto um pedido roda, com o `id` dele em
  `params.id`: `phase` (a fase que começou), `events` (os eventos do Core
  em lotes, no máximo a cada 30 ms ou 500), `output_omitted` (linhas que
  passaram do teto: 20 000 de uma vez e depois 5 000 por segundo), `build`
  (um processador compilou) e `install` (o progresso de `update.*` e
  `install.add`: o evento do instalador e, quando há, a linha do console em
  `line`, com os nomes dos aplicativos). Elas saem antes da resposta. Com
  `"output": false` no `flow.run`, não há `events`, e o `vvp` roda com
  buffer, que é mais rápido.
- **Filas.** Os pedidos do mesmo projeto rodam um de cada vez, na ordem
  (um worker por raiz canônica); os que só leem (`project.status`,
  `history.*` de consulta, `toolchain.*`, `fpga.boards`, ...) rodam em
  paralelo, numa thread própria, também durante um fluxo longo do mesmo
  projeto; `update.*` e `install.add` rodam num worker só, com a trava da
  instalação. Projetos diferentes rodam em paralelo.
- **Cancelar.** A notificação `$/cancel` com `{"id": N}`. Um pedido que
  ainda não começou responde o erro `-32001`; um fluxo em andamento para e
  devolve o resultado com `status: "cancelled"`, como o Ctrl+C na CLI.
- **Erros.** O `error.code` do JSON-RPC: `-32700` JSON inválido, `-32600`
  pedido malformado, `-32601` método desconhecido, `-32602` parâmetros
  inválidos, `-32000` um erro da AURORA, com o `ErrorInfo` (`code`,
  `message`, `hint`, `data`) em `error.data`, `-32001` cancelado antes de
  começar, `-32002` sessão não inicializada, `-32003` servidor encerrando.
  Um parâmetro que o método não tem é `-32602`, com o nome mais parecido
  na mensagem (`unknown field `form` (did you mean `from`?)`); no
  `flow.run`, também um campo que o fluxo pedido não tem. Um projeto cujo
  caminho não existe é `-32000` com o código `spf_missing`. Os métodos
  `fpga.*` respondem `experimental_disabled` com o recurso `fpga` desligado.
- **BOM.** Um BOM no começo de uma linha é ignorado: o PowerShell o põe na
  primeira coisa que escreve para um programa.
- **Encerrar.** Fechar a entrada: o que já foi pedido roda até o fim e é
  respondido, e o servidor sai depois, com a notificação `shutdown`; se o
  cliente fechou também a saída, o que roda é cancelado. Com `shutdown` ou
  um sinal, o servidor cancela o que roda, espera até 3 s e sai, com a
  notificação `shutdown` antes (`reason`: `requested`, `signal` ou
  `updated`). Depois de um `update.apply` que trocou os arquivos, ele
  responde e encerra sozinho, porque o binário dele é outro: o cliente o
  abre de novo.

Em Python, sem biblioteca:

```python
import json, subprocess
aurora = subprocess.Popen(["aurora", "serve"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
def call(id, method, params=None):
    aurora.stdin.write(json.dumps({"jsonrpc": "2.0", "id": id, "method": method, "params": params}) + "\n")
    aurora.stdin.flush()
    while True:
        msg = json.loads(aurora.stdout.readline())
        if msg.get("id") == id:
            return msg
        # uma notificação (phase, events, ...) do pedido em andamento
call(1, "initialize", {"client": "exemplo"})
sim = call(2, "flow.run", {"project": "/home/eu/soma/soma.spf", "request": {"flow": "simulate", "simulator": "icarus"}, "output": False})
print(sim["result"]["status"], sim["result"]["report"])
aurora.stdin.close()
```

A extensão do VS Code usa o `serve` quando a AURORA instalada é a 0.9.0 ou
mais nova (`aurora.transport`); com uma AURORA mais antiga continua chamando um
processo por comando (`--json` e `--events`), que seguem como estão.

O `aurora mcp`, para um agente:

- **O projeto.** As ferramentas de projeto aceitam `project` (o `.spf` ou
  a pasta, absoluto ou relativo à pasta atual) e, sem ele, usam o projeto
  padrão do servidor: o da pasta em que ele foi aberto, ou o do `-C`
  global (`aurora -C <projeto> mcp`). Sem padrão e sem o campo, a ferramenta
  responde um erro que diz isso; `project_find` lista os `.spf` abaixo de
  uma pasta (3 níveis por padrão, fora `.aurora8`, `.git`, `node_modules`,
  `target` e `.claude`). O `initialize` diz qual é o padrão.
- **O resultado.** `content` traz um resumo em texto para o agente: o
  fluxo e o status, cada processador compilado, os erros como
  `arquivo:linha: mensagem`, a simulação (topo, simulador, onda, saídas,
  testes), a síntese (netlist, células), o relatório gravado; o retrato do
  projeto; a lista de relatórios; o bundle. `structuredContent` traz o JSON
  inteiro do resultado, com o `stdout` e o `stderr` de cada passo cortados
  aos últimos 2 KiB. Um erro da AURORA vem com `isError`, a mensagem, o
  código e a dica.
- **Progresso e cancelamento.** As chamadas rodam uma de cada vez, fora da
  thread que lê a entrada: `ping` responde durante uma simulação longa.
  Uma chamada com `_meta.progressToken` recebe `notifications/progress`
  (`phase: build`, `build soma: succeeded`, os pedaços de uma instalação);
  `notifications/cancelled` com o `requestId` cancela a chamada, que
  devolve o resultado com `status: "cancelled"` (ou o erro `-32800`, se
  ainda não tinha começado).

Registrar no Claude Code, para esta pasta:

```bash
aurora mcp setup claude-code
```

No Claude Desktop (reiniciar depois):

```bash
aurora mcp setup claude-desktop
```

O `aurora chat` e a vista Assistente do Studio usam o mesmo motor
(`crates/aurora-agent`): o Claude Code em modo `--print`, com entrada e saída
em JSON por linha, o `aurora mcp` configurado como servidor de ferramentas e
as instruções da AURORA no prompt do sistema. Só as ferramentas da AURORA e a
leitura de arquivos são permitidas por padrão; a edição, só com
`--allow-edits` (ou a preferência do Studio). O Claude Code só inicia a
sessão quando recebe a primeira mensagem, e a AURORA a manda em seguida; a
sessão fica aberta entre as mensagens e fecha ao sair, com o que o Claude
Code tiver iniciado.

## Saída

### Texto

```
Build of conta (C±, 100 MHz, 2000 clocks): failed after 2 ms: cmmcomp exited with code 1
    failed    compile       cmmcomp         2 ms
    not run   assemble      asmcomp
  /home/eu/projetos/com_erro/conta/conta.cmm:15: error: c'mon dude, declare the variable 'total' properly!
  Written before it stopped:
    assembly            conta/conta/conta.asm
  Not generated:
    processor Verilog   conta/conta/conta.v
    data memory         conta/conta/conta_data.mif
    instruction memory  conta/conta/conta_inst.mif
    testbench           conta/conta/conta_tb.v
Simulation not run: the build of conta did not finish
Report run-000001: aurora report show 1
```

- Uma linha de título por operação, com o que ela fez e como terminou:
  `finished in <tempo>`, `failed after <tempo>: <ferramenta> exited with
  code N`, `crashed` (a ferramenta morreu por sinal ou exceção),
  `incomplete` (as ferramentas rodaram, mas faltou arquivo), `cancelled
  after <tempo>, <ferramenta> stopped` (Ctrl+C; sem nenhum passo rodado,
  `cancelled before starting`) ou `timed out, <ferramenta> stopped at the
  N s limit` (`--timeout`).
- Embaixo, um passo por linha: `done`, `failed`, `crashed`, `cancelled`,
  `timed out`, com a ferramenta e o tempo; os passos que a falha impediu
  aparecem como `not run`.
- Na simulação, o que o testbench escreve sai antes do título, enquanto
  roda (ver [O que `sim` mostra](#o-que-sim-mostra)).
- Um diagnóstico por linha, no formato `arquivo:linha[:coluna]: gravidade:
  mensagem`, que editores e terminais transformam em link. A gravidade fica
  em minúscula, como no gcc (`error`, `warning`, `info`), porque é o que os
  editores reconhecem. Mensagens `info` só aparecem com `-v`.
- Os artefatos com o papel e o caminho: `Generated` lista os obrigatórios
  (com `-v`, também os intermediários); numa falha, `Written before it
  stopped` lista o que saiu, talvez pela metade, e `Not generated`, o que
  faltou, com `(left from an earlier run)` quando sobrou um de antes.
- Quando um build falha antes de `sim` ou `synth`, uma linha diz
  que a fase seguinte não rodou e por quê.
- Num terminal, uma barra mostra que um passo está rodando: o que ele faz
  (`Simulating (vvp)`, `Compiling the model (verilator)`), uma animação e o
  tempo decorrido. Ela é indeterminada, porque nem o `vvp` nem o modelo do
  Verilator informam quanto falta, e só aparece depois de 0,3 s, para os
  passos rápidos não piscarem. Fica no stderr e sai da frente quando o
  testbench escreve; num pipe, num log ou com `--json`, não há barra.
- Cores só num terminal, e nunca com `NO_COLOR` definido.

### Erros e dicas

Um erro que impede o comando de rodar sai no stderr como `Error: <mensagem>`,
com as causas depois de `: `, sem repetir uma causa que a mensagem já
termina com ela.
Os avisos começam com `Warning:`.
As mensagens do Core são neutras, sem nome de comando; para alguns códigos, a
CLI acrescenta o comando que resolve:

| `error.code` | Dica |
|---|---|
| `no_top_level` | `Choose it with: aurora top <file\|module>` |
| `no_testbench` | `Create it with: aurora add <name>_tb.v, or a cocotb one with: aurora add test_<name>.py` |
| `no_cocotb_toplevel` | `` Add a line `# aurora-toplevel: <module>` to the testbench, or choose the top with: aurora top <file\|module> `` |
| `cocotb_unavailable` | `Reinstall it with: aurora install cocotb` |
| `empty_project` | `Add one with: aurora add <file.v>, or create a processor with: aurora proc add <name>` |
| `not_built` | sem o Verilog do processador (o `check` não compila), `Build it with: aurora build -p <processor>`; sem o testbench, `Build and simulate with: aurora sim -p <processor>` |
| `processor_not_found` | num projeto sem processadores, `Create it with: aurora proc add <name>`; com processadores, nenhuma: a mensagem lista os que existem |
| `system_compiler_missing` | `Or set its location with --compiler <DIR> or AURORA_COMPILER` |
| `module_not_found` | nenhuma: a mensagem já lista os módulos do projeto |
| `ambiguous_module` | `Choose it by file: aurora top <arquivo>` |
| `component_missing` | `Install it with: aurora install <componente>` |
| `legacy_layout` | `Convert it with: aurora convert (nothing is deleted or overwritten)` |
| `yanc_too_old` | `Install the current bundle with: aurora update` |
| `project_not_found` | `Create one with: aurora new <name>, or point to it with -C <folder>` |
| `no_reports` | `aurora build, check, sim and synth each store a report` |
| `report_not_found` | `List them with: aurora report list` |
| `board_not_found` | `List them with: aurora fpga boards` |
| `no_fpga_config` | um exemplo de `fpga.json` e `see the signals with: aurora fpga boards <board>` |
| `invalid_fpga_config` | nenhuma: a mensagem traz um problema por linha |
| `invalid_board` | `This is a bug in AURORA; please report it` |
| `no_bitstream` | `Build for the board first with: aurora fpga build` |
| `stale_bitstream` | `Build for the board again with: aurora fpga build, then program` |
| `no_cable` | `Connect the board through its USB-Blaster port, turn it on and set its RUN/PROG switch to RUN; on Windows the USB-Blaster driver is in the drivers folder of Quartus, and on Linux the cable needs a udev rule (see docs/FPGA.md)` |
| `quartus_missing` | `Install Quartus Prime Lite (Windows or Linux) with the device support of the board, or set its folder with --quartus <DIR> or AURORA_QUARTUS` |
| `experimental_disabled` | `Enable it with: aurora experimental enable <componente> (in AURORA IDE: Settings > Experimental)` |
| `no_waveform` | `Simulate first with: aurora sim`; na onda de um processador, `Simulate first with: aurora sim -p <processador>` |
| `signal_not_found` | sem caminho parecido no design, `List them with: aurora wave signals`; com, nenhuma: a mensagem sugere os caminhos |
| `testbench_not_elaborated` | `See the errors with: aurora check` |
| `process_exited_early` | no `aurora sim --open`, `See the end of the log: <log>`; no `aurora wave`, nenhuma: a mensagem já traz o fim do log |

### JSON

Com `--json`, o stdout tem exatamente um objeto JSON e nada mais. O `serve`
e o `mcp` são a exceção, com o stdout do protocolo, e o `chat` e o
`mcp setup` não leem o `--json`. Cada comando escreve um tipo só, com JSON Schema em `docs/schema/`, gerado dos
tipos e conferido por teste. Os campos estão
no schema; os resultados de operação são os tipos do Core serializados
([API.md, seção 6](API.md#6-resultados)).

| Comando | Schema | O objeto |
|---|---|---|
| `new` | `new.json` | o projeto criado: o `.spf` e a pasta |
| `status` | `status.json` | o projeto inteiro: arquivos registrados, topo, testbench escolhido, `.v` não registrados e processadores, cada um dizendo se está compilado |
| `add` | `add.json` | o `.spf` e um `AddedFile` por arquivo, na ordem da linha de comando |
| `remove` | `remove.json` | o `.spf`, os arquivos tirados e os que não estavam registrados |
| `top` | `top.json` | o arquivo e o módulo de topo e, quando o módulo não dá para saber, o erro |
| `move` | `move.json` | o `.spf` e um `MovedPath` por origem: de onde, para onde e os arquivos do projeto que foram junto |
| `order` | `order.json` | o `.spf` e a lista do arquivo, na ordem nova |
| `proc add`, `proc set` | `proc.json` | o `Processor` criado ou atualizado |
| `proc remove` | `proc-remove.json` | o que saiu (`removed`: `name`, `dir` e os arquivos tirados das listas em `unregistered`) e se a pasta foi para a lixeira (`trashed`) |
| `convert` | `convert.json` | o `.spf` e o `LayoutConversion`: os processadores convertidos, o que mudou de lugar, o que ficou e por quê, os fontes que perderam a linha do nome, os nomes que não servem e as subpastas que ficaram |
| `build` | `build.json` | o `.spf` e um `BuildResult` por processador compilado; nenhum num projeto sem processadores |
| `check` | `check.json` | o `CheckResult` em `check` e o relatório gravado em `report` |
| `hierarchy` | `hierarchy.json` | o `HierarchyResult`: o design e cada testbench, com a árvore de instâncias e o status de cada elaboração |
| `sim` | `sim.json` | os builds feitos antes, o `SimulationResult`, os valores das portas de saída (com `-p`, depois de uma simulação que deu certo; uma porta que não pôde ser lida, como o `x` de uma divisão por zero, vem com `values` vazio e o motivo em `error`) e o PID do surfer-aurora (com `--open`) |
| `wave` | `wave.json` | a onda aberta, o PID do surfer-aurora e o log dele; o `.surf.ron` com que a onda abriu, o salvo no projeto ou o gerado (`layout`, `null` com `--no-layout`, numa onda sem sinal para mostrar ou que não é VCD nem FST), o layout salvo no projeto (`saved_layout`, `null` numa onda fora de projeto) e os processadores do layout (`processors`) |
| `wave signals` | `wave-signals.json` | a árvore de sinais do testbench do projeto, a escolha gravada e os itens dela que o design não tem mais |
| `wave select`, `wave unselect` | `wave-select.json` | o arquivo da escolha e a escolha depois da mudança |
| `synth` | `synth.json` | os builds feitos antes, o `SynthesisResult` (com as estatísticas do Yosys em `statistics`), com `--svg`, o `SchematicResult` e, com `--open`, o HTML do RTL viewer (`rtl_viewer`) |
| `report`, `report show` | `report.json` | o identificador, o caminho do `report.txt`, o que a comparação usa (`record`: contexto, estatísticas e tempos) e o texto |
| `report list` | `report-list.json` | um resumo por relatório, do mais novo para o mais antigo |
| `report compare` | `report-compare.json` | os dois relatórios, a síntese e a simulação comparadas (`null` quando não se comparam) e os avisos de contexto |
| `report clean` | `report-clean.json` | os relatórios apagados, do mais antigo para o mais novo, e quantos ficaram |
| `tools` | `tools.json` | o bundle, os componentes instalados e os que faltam, cada ferramenta (o caminho ou o erro), o compilador do Verilator (`system_compiler`, com `bundled` verdadeiro no Windows), o Quartus Prime (`quartus`, `null` se não encontrado) e, com `--verify`, os executáveis que não conferem |
| `learn` e os subcomandos | `learn-*.json` | ver [LEARN.md](LEARN.md) |
| `fpga boards` | `fpga-boards.json` | as placas, cada uma com o FPGA, o cabo e os sinais com os pinos |
| `fpga check` | `fpga-check.json` | o `fpga.json`, as ligações bit a bit (`resolved`, com as notas de ajuste) e o Verilog do topo da placa (`board_top`) |
| `fpga build` | `fpga-build.json` | os processadores compilados antes (`builds`) e a compilação para a placa (`fpga`, `null` se um processador falhou): os passos do Quartus, os diagnósticos, os artefatos (o `.sof`, o `.rbf`, o `.svf`), os recursos (`resources`) e o tempo de cada clock (`timing`) |
| `fpga program` | `fpga-program.json` | a gravação: a placa, o `.sof`, o cabo, a posição JTAG, o passo `program` e os diagnósticos |
| `fpga program --list` | `fpga-cables.json` | os cabos ligados (`cables`), como o `quartus_pgm -l` os escreve |
| `experimental` (e `enable`, `disable`) | `experimental.json` | o arquivo do que está ligado (`path`) e cada componente (`components`: `feature`, `enabled`, `description`), já com a mudança |
| `install` | `install.json` | a pasta, os aplicativos instalados depois e os que entraram. Exige nomes: com `--json` não há lista |
| `update` | `update.json` | a AURORA (instalado, último publicado, se é mais novo), o bundle (instalado e o da última release), cada aplicativo com a versão instalada, a da release e a upstream (`null` quando a consulta falhou), e o que o comando fez (`checked`, `up_to_date`, `updated`, `wizard_opened`). O que o instalador escreve vai para o stderr |
| `uninstall` | `uninstall.json` | a pasta da instalação e se ela já saiu (`false` no Windows, onde o desinstalador termina depois que o `aurora` sai). O que o `uninstall.sh` escreve vai para o stderr |
| qualquer erro (código 2) | `error.json` | o erro, com o código estável e, quando há, a dica |
| cada linha do `--events` | `events.json` | um evento do Core, ou a linha final com o resultado ([Eventos](#eventos)) |

O que o schema não mostra:

- `build`, `check`, `sim` e `synth` trazem em `report` o relatório que
  gravaram (`run-000042`), ou `null` se nada rodou ou se ele não pôde ser
  gravado.
- Em `sim` e `synth`, se um build falha ou é cancelado antes, o
  resultado da operação sai `null`, e `sim` escreve `outputs: []` e
  `surfer_pid: null`: os campos aparecem sempre.
- Em `tools`, as chaves de `tools` (o nome de cada executável) saem em ordem
  alfabética.
- O erro dentro de `top` (`module_error`) e o de uma ferramenta em `tools`
  (`tools.<nome>.error`) têm o mesmo formato do erro de um comando, com
  `hint` quando há dica.
- Um comando cancelado (código 130) também escreve o objeto, com
  `status: cancelled` no resultado que estava rodando.
- O objeto sai indentado; o de erro, numa linha só.

`error.code` é o `AuroraError::code()` do Core (tabela em
[API.md, seção 8](API.md#8-erros)), ou `"cli"` para erros da própria linha de
comando (`--compiler` sem os três programas, `remove` sem nenhum arquivo
registrado, `proc set` sem nada para mudar), ou `"cancelled"` para um
`install`, `update` ou `uninstall` interrompido pelo Ctrl+C antes de mudar a
instalação. Um erro da biblioteca do instalador, no `install`, no `update`,
no `uninstall` ou no `setup`, traz o código dela (`no_installation`,
`install_in_progress`, `interrupted_update_undone`, `network`,
`curl_missing`, `no_latest_version`, `checksum_mismatch`,
`installer_failed`, `wizard_failed`, `invalid_mirror` ou `install`), e um do
`aurora learn`, o do `aurora-learn` (`no_learn_workspace`,
`learn_workspace_exists`, `unknown_exercise`, `track_not_found`,
`invalid_track`, `invalid_learn_state`, `learn_component_missing`,
`no_waveform` ou `io`).

A exceção são os erros de argumento (opção desconhecida, valor inválido): o
`clap` os detecta antes de o comando começar e escreve a mensagem no stderr,
em texto, com código 2.

Com `--json`, a AURORA não acompanha a saída das ferramentas enquanto rodam:
o `vvp` fica com o buffer do stdout, que é mais rápido, e o `$display` do
testbench só aparece no fim, no `stdout` do passo `simulate`. Em texto e com
`--events`, o `vvp` roda com `-i`, sem buffer, para cada linha sair na hora.
Para ler a saída enquanto roda a partir de outro programa, use `--events`.

### Eventos

Com `--events`, o stdout tem um objeto JSON por linha, e nada mais. Enquanto
as ferramentas rodam, cada linha é um evento do Core: `step_started`,
`output` (uma por linha que a ferramenta escreve) e `step_finished`
([API.md, seção 5.8](API.md#58-cancelamento-prazo-e-saída-ao-vivo-control)).
A última linha é `{"event": "result", "result": <objeto>}`, em que
`<objeto>` é o mesmo que o comando escreveria com `--json`. Um erro que
impede o comando de rodar vira essa linha final, com o objeto de erro em
`result`. Um comando que não roda ferramenta (`new`, `status`, `add`)
escreve só a linha final.

Cada linha confere com `docs/schema/events.json`. Em `output`, `diagnostic`
separa o que é mensagem da ferramenta (volta interpretado em `diagnostics`
no resultado) do que é saída do programa, como o `$display` do testbench.

`aurora --events sim` no projeto `contador` (caminhos encurtados, resultado
cortado):

```
{"event":"step_started","step":"elaborate","tool":"iverilog","command":{"program":"/bin/bash","args":["..."],"cwd":"/home/eu/contador","env":[["PATH","/usr/bin:/bin"]],"inherit":[]}}
{"event":"step_finished","step":"elaborate","tool":"iverilog","termination":{"kind":"exited","value":0},"duration_ms":111}
{"event":"step_started","step":"simulate","tool":"vvp","command":{"program":"/bin/bash","args":["/opt/aurora/toolchain/oss-cad-suite/bin/vvp","-n","-i","/home/eu/contador/.aurora8/Temp/contador_tb.vvp","-fst"],"cwd":"/home/eu/contador","env":[["PATH","/usr/bin:/bin"]],"inherit":[]}}
{"event":"output","step":"simulate","tool":"vvp","stream":"stdout","line":"FST info: dumpfile contador_tb.fst opened for output.","diagnostic":true}
{"event":"output","step":"simulate","tool":"vvp","stream":"stdout","line":"q = 10","diagnostic":false}
{"event":"output","step":"simulate","tool":"vvp","stream":"stdout","line":"/home/eu/contador/.aurora8/Temp/instr_contador_tb.v:15: $finish called at 112000 (1ps)","diagnostic":true}
{"event":"step_finished","step":"simulate","tool":"vvp","termination":{"kind":"exited","value":0},"duration_ms":49}
{"event":"result","result":{"builds":[],"simulation":{"top":"contador_tb","simulator":"icarus","status":"succeeded","...":"..."},"outputs":[],"surfer_pid":null}}
```


### Interromper

Ctrl+C (SIGINT), SIGTERM e, no Unix, SIGHUP (o do terminal que fecha)
cancelam o comando: a AURORA encerra a ferramenta que está rodando, com tudo
o que ela iniciou, não começa os passos seguintes e mostra o que chegou a
rodar, com o título `cancelled after <tempo>, <ferramenta> stopped`. O código
de saída é 130. Com `--json` e `--events`, o objeto do resultado sai do
mesmo jeito. Um segundo sinal sai na hora, também com 130. No `aurora serve`,
no `aurora mcp` e no `aurora chat`, o segundo sinal não sai na hora; o `serve` e
o `mcp`, no primeiro, cancelam o que roda, esperam até 3 s e saem com 0.

`install`, `update` e `uninstall` param no primeiro sinal antes de
mudar a instalação (entre um download e outro, ou depois da pergunta),
com `Cancelled with Ctrl+C; nothing was changed` e código 130; o download
que o sinal interrompeu não leva o `update` ao instalador inteiro. Sem
`--yes`, o `update` pergunta de novo antes de passar ao instalador inteiro
quando a atualização por partes falha. O `setup` não confere o sinal:
termina o que começou e sai com 130, e um segundo sinal o encerra na hora.

```
  | comecou
Simulation of infinito_tb (Icarus): cancelled after 2.31 s, vvp stopped
```

Quem roda o `aurora` a partir de outro programa e precisa pará-lo deve
mandar SIGTERM no Linux e no macOS. Lá, um SIGKILL mata só o `aurora`: a
ferramenta roda num grupo de processos próprio e continua rodando sozinha.
No Windows, encerrar o processo do `aurora` (o `child.kill()` do Node, o
Gerenciador de Tarefas) encerra também a ferramenta, que roda num Job Object
preso a ele, e nenhuma ferramenta abre janela de console.

### Códigos de saída

| Código | Significado |
|---|---|
| 0 | deu certo |
| 1 | a operação rodou e falhou: erro de compilação, Verilog que não elabora, simulação que falhou (inclusive por `$error`) ou que passou do `--timeout` |
| 2 | a AURORA não conseguiu rodar: bundle, projeto, arquivo, opção inválida |
| 130 | cancelado: a AURORA recebeu Ctrl+C, SIGTERM ou, no Unix, SIGHUP durante o comando; o `aurora serve` e o `aurora mcp` encerram pelo sinal e saem com 0 |

### Log

O log do Core sai no stderr: avisos por padrão, `info` com `-v`, `debug` com
`-vv`. `RUST_LOG` tem precedência (por exemplo `RUST_LOG=aurora_core=debug`).
