# Mudanças

## Não publicada

- O `flow.run` do `aurora serve` aceita `options.port_values_limit`, quantos
  valores de cada porta de saída o resultado leva, e cada porta ganha
  `count`, com quantos valores ela tem no `output_<n>.txt`. Sem a opção,
  vêm todos, como antes. A AURORA IDE pede 0 na simulação rápida: um
  processador simulado por 25 milhões de clocks mandava 4 milhões de
  valores, 30 MiB de JSON, que ela não mostra.

## 1.1.0 (2026-11-09)

A primeira release do repositório novo, `nipscernlab-sandbox/aurora`, com a
AURORA IDE 8.1.0, que agora tem repositório e releases próprios
([nipscernlab-sandbox/aurora-ide](https://github.com/nipscernlab-sandbox/aurora-ide)).
Quem tem a 1.0.0 chega nela pelo `aurora update`, pelo endereço antigo, que
o GitHub redireciona. Entram, experimentais, a placa virtual e o controle de
versão. A extensão do VS Code vai para a 0.2.1.

- A AURORA IDE sai deste repositório: o código e a documentação dela estão
  em [nipscernlab-sandbox/aurora-ide](https://github.com/nipscernlab-sandbox/aurora-ide),
  com o histórico da pasta `studio/`. O bundle traz a IDE pronta da release
  de lá, conferida pelo SHA-256, agora na 8.1.0, com a placa virtual e o
  controle de versão experimental. Para montar com uma IDE que ainda não
  saiu, `AURORA_IDE_SRC` aponta o `scripts/bundle.py` para um clone do
  aurora-ide.
- O repositório mudou para `nipscernlab-sandbox/aurora`, e o do bloco de
  Windows para `nipscernlab-sandbox/aurora-toolchain`. O `aurora update`, o
  `install.sh`, o `install.ps1` e o assistente do Windows procuram as
  releases no endereço novo. Os antigos (`ART3121/lace` e
  `ART3121/lace-toolchain`) continuam funcionando pelo redirecionamento do
  GitHub, e é por eles que a 1.0.0 acha esta versão.
- No `aurora update --check`, a versão upstream da AURORA IDE é a da última
  release do aurora-ide; antes era a da AURORA.
- O `install.sh`, numa plataforma sem instalador, dizia "Na AURORA
  installer" (desde a 1.0.0).
- A extensão do VS Code 0.2.1: o endereço do repositório no manifesto e os
  comentários das gramáticas; o código é o mesmo da 0.2.0.
- Controle de versão, no recurso experimental novo `git`: o crate
  `aurora-git`, sobre o git instalado na máquina, e os métodos `git.status`,
  `git.diff`, `git.read_file`, `git.commit`, `git.discard`, `git.init`,
  `git.log`, `git.commit_files`, `git.branches`, `git.switch`, `git.fetch`,
  `git.pull`, `git.push`, `git.set_identity`, `git.set_remote` (cria o
  remoto ou corrige o endereço) e `git.remove_remote`. O login no
  GitHub é pelo fluxo de dispositivo (`github.login_start`,
  `github.login_poll`), com o token no gerenciador de credenciais do git;
  `github.account` diz quem entrou, e `github.logout` sai. No `aurora mcp`,
  as ferramentas `git_status`, `git_diff`, `git_log`, `git_commit_files`,
  `git_branches` e `git_commit`.
- A placa virtual, no recurso experimental `fpga`: o projeto roda no
  computador com os periféricos da placa simulados nos pinos (chaves,
  botões, LEDs com brilho de PWM, displays de 7 segmentos, teclado PS/2,
  tela VGA e serial), pelo Verilator do bundle, com o mesmo topo da placa do
  `aurora fpga`. O JSON das placas ganhou a lista `peripherals`; as três
  placas têm chaves, botões, LEDs e displays, e a DE2-115 também VGA, PS/2
  e serial, com os 37 pinos das tabelas 4-16, 4-18 e 4-19 do manual. Novos:
  `aurora vboard run`, os métodos `vboard.layout`, `vboard.run` e
  `vboard.input` com as notificações `vboard_*`, e no core o módulo
  `virtual_board`, `fpga::prepare_with`, `FpgaConfig::by_name` e os erros
  `virtual_board_not_built` e `virtual_board_device`.
- `fpga.connect` (e a ferramenta MCP `fpga_connect`): muda uma ligação a
  partir da placa, um bit de um sinal ou o sinal inteiro numa porta do topo,
  e grava o `fpga.json` com as outras ligações intactas, reescritas em
  faixas. No core, `fpga::connect` e `FpgaConfig::reconnect`; o
  `LayoutSignal` da placa virtual traz a porta de cada bit.
- `fpga.json`: uma saída que vai a vários sinais da placa pode vir numa
  lista (`"seg": ["HEX0", "HEX1"]`). O esquema do `connect` passou de texto
  para texto ou lista; um arquivo sem lista lê e grava como antes.

- Windows: o assistente que substitui o Lace põe a AURORA no PATH antes de
  rodar o desinstalador do Lace, e não depois, e mostra o andamento enquanto
  ele apaga o bundle do Lace, o que leva minutos. Na 1.0.0, se o assistente
  fosse fechado nessa espera, sem sinal de andamento na tela, a AURORA ficava
  instalada e o Lace saía, mas a pasta da AURORA ficava fora do PATH, e o
  `aurora` não abria num terminal novo. Quem passou por isso volta a ter o
  `aurora` no PATH rodando o instalador da AURORA de novo.
- Verilator (simulação rápida, simulação com onda, cocotb e placa virtual):
  uma compilação interrompida no meio, cancelada ou morta junto com a
  AURORA, deixava um `.o` vazio com data nova na pasta do modelo, e todas as
  compilações seguintes do mesmo topo falhavam no link
  (`undefined reference to VerilatedContext::VerilatedContext()`), porque o
  `make` tomava o objeto por pronto. A AURORA marca a compilação em
  andamento e, achando a marca de uma que não terminou ou um `.o` vazio,
  apaga a pasta do modelo e compila do zero.
- Placa virtual: os processadores SAPHO desatualizados (o fonte ou um arquivo
  da pasta dele mais novo que o Verilog gerado) são compilados antes da
  placa, como na simulação. Antes, a placa rodava o programa do último build
  sem aviso. O `BoardBuild` ganhou `processors`, e o `layout` fica `null`
  quando um processador não compila. No core, `Processor::is_out_of_date`.
- Placa virtual: a saída do projeto (`$display`) sai em lotes, um por estado
  (cerca de 30 por segundo), com no máximo 100 linhas cada e a contagem das
  que ficaram de fora. Um `$display` a cada ciclo mandava 141 mil
  notificações por segundo e derrubava os estados de 30 para menos de 10 por
  segundo. Muda a notificação `vboard_output` (`lines` e `omitted` no lugar
  de `line`), o `SessionEvent::Output` e o relatório JSON do
  `aurora vboard run` (`output_omitted`).
- Verilator (placa virtual, simulação rápida, com onda e cocotb): mudar uma
  linha do projeto recompilava também a biblioteca do Verilator
  (`verilated.o` e `verilated_threads.o`), porque o `verilated.mk` a faz
  depender do makefile do modelo, que o Verilator regrava a cada verilação.
  O `make` recebe `--old-file=<prefixo>.mk`, e uma marca na pasta do modelo
  (versão da AURORA, `bundle.json` e flags) a apaga quando algo disso muda.
  Medido: de 21 s para 5 s na placa virtual e de 12 s para 4,6 s na
  simulação rápida, numa mudança de uma linha.
- Verilator (placa virtual, simulação rápida, com onda e cocotb): o
  `include` procura na pasta do arquivo que inclui, depois na do testbench
  original e na raiz do projeto (`--relative-includes` e `-I`), como no
  Icarus. Um `include "defs.vh"` ao lado do arquivo passava no Icarus e dava
  "Cannot find include file" no Verilator.
- Portas do topo (placa FPGA e placa virtual): o Yosys que lê as portas
  roda na raiz do projeto, como na síntese. Rodava na pasta dos arquivos, e
  um `$readmemh("dados/rom.hex")` relativo à raiz o fazia falhar; o leitor
  de texto que entrava no lugar não expande macros, e uma porta
  `` [`LARGURA-1:0] `` saía com 1 bit, sem aviso: só o bit 0 chegava à
  placa. Novo no core: `verilog::read_interfaces_in`, com a raiz.
- Placa virtual: um valor de chaves ou botões maior que o periférico
  (`--set SW=0x40000` nas 18 chaves, `vboard.input` com `1 << 40`) é
  recusado com o erro `virtual_board_value`, que diz a largura e o maior
  valor; antes os bits a mais eram cortados sem aviso. O `--set` com um
  valor que não é número diz o formato, em vez do erro do Rust.
- Placa virtual no Windows: compilar com uma placa do mesmo projeto ligada
  (na IDE e pela CLI, ou em duas janelas) falhava no link, com o executável
  em uso. Ele é renomeado para o lado antes da compilação, e a placa que
  roda segue com ele; a próxima compilação apaga o antigo.
- `aurora vboard run --time` para no tempo exato: a placa recebe a ordem
  `until` e para sozinha. Antes a CLI parava pelo estado, umas 30 vezes por
  segundo de relógio, e 5 ms viravam 9,7 ms, 20 ms viravam 46 ms; o mesmo
  comando dava resultados diferentes. No core, `RunOptions::until`.
- Placa virtual: uma serial do projeto em outra velocidade (9600 numa placa
  de 115200) dava bytes vazios, sem aviso. A UART conta os quadros com o
  bit de parada em 0 e, depois de dois, passa a ler na velocidade padrão
  mais próxima do pulso mais curto do TX; o estado traz `uart_baud`, e a
  CLI e a IDE mostram a velocidade detectada. Os primeiros bytes se perdem.
- `aurora vboard run`: um LED em PWM mostra o brilho (`PWM: [0] 25%`), e o
  grupo de displays aparece como `HEX7..HEX0`, e não `HEX7`.
- `aurora vboard run`: avisa os caracteres do `--keys` que o teclado
  americano não digita (`ç`, `€`), que antes sumiam; recusa o `--png` num
  projeto sem vídeo, como o `--keys` e o `--uart` sem teclado ou serial; e,
  num projeto sem `fpga.json`, a dica cita o `--board`. No core,
  `virtual_board::untypeable`.

## 1.0.0 (2026-11-05)

A primeira release com o nome AURORA: o Lace vira a AURORA API 1.0.0, e o
Lace Studio, a AURORA IDE 8.0.0, numerada depois da AURORA 7, a de
Electron. A extensão do VS Code passa a se chamar AURORA, na 0.2.0. Quem
tem o Lace passa para ela pelo `lace update`, que instala a AURORA com os
mesmos aplicativos e tira o Lace
([docs/INSTALL.md, Quem tem o Lace](docs/INSTALL.md#quem-tem-o-lace)).

### Quebras de compatibilidade

- O Lace dá lugar à AURORA, outro programa. Os arquivos da release passam a
  se chamar `aurora-<versão>-<plataforma>` (os instaladores, o índice e os
  pedaços do bundle), e o empacotador recebe o executável em `--aurora`. O
  instalador do Windows passa a ser `installer/windows/aurora.iss`: outro
  programa para o Windows, AURORA, com identificação própria e a pasta
  `Programs\AURORA` (`C:\AURORA` como administrador). O que fazer: rodar o
  `lace update`, ou a atualização do Lace Studio, do Lace 0.4 em diante. A
  release publica os instaladores da AURORA também com os nomes que o Lace
  procura (`lace-<versão>-<plataforma>.tar.gz` e
  `lace-<versão>-windows-x64-setup.exe`), e eles trocam o Lace pela AURORA,
  com os mesmos aplicativos. No Linux e no macOS, o `install` põe a AURORA
  na pasta dela, com o atalho `aurora` onde estava o `lace`, e tira o Lace
  pelo `uninstall.sh` dele. No Windows, o assistente marca os aplicativos
  do Lace e, no fim, o desinstala (a tarefa Desinstalar o Lace, que vem
  marcada também quando a AURORA é instalada à parte); se a atualização
  veio do Lace Studio, a AURORA IDE abre no fim.
- O comando passa a ser `aurora`, e os crates, `aurora-core`, `aurora-cli`,
  `aurora-protocol`, `aurora-installer`, `aurora-learn` e `aurora-agent`
  (`LaceError` vira `AuroraError`). O empacotador passa a ser `aurora-pack`,
  e o instalador de terminal, `aurora-installer`. As trilhas de exercícios
  ficam em `aurora-learn/`, e a pasta de exercícios usa `.aurora-learn.json`
  e `.aurora-learn/`: a pasta de exercícios feita pelo Lace não é
  reconhecida. O que fazer: trocar `lace` por `aurora` nos scripts e
  começar outra pasta de exercícios com `aurora learn init`.
- As variáveis de ambiente passam de `LACE_*` para `AURORA_*`
  (`AURORA_TOOLCHAIN`, `AURORA_COMPILER`, `AURORA_QUARTUS`,
  `AURORA_LEARN_DIR`...); a do programa que a IDE abre, `LACE_STUDIO_LACE`,
  vira `AURORA_IDE_CLI`. As pastas de configuração e de cache passam a se
  chamar `aurora`, e a instalação de terminal vai para
  `~/.local/share/aurora`, com o atalho em `~/.local/bin/aurora` (como
  root, `/opt/aurora`). O que fazer: renomear as variáveis e ligar de novo
  os recursos experimentais com `aurora experimental enable`.
- Protocolo 2: os campos `lace` viram `aurora` (a versão no `initialize`, o
  relatório do `update.check`), `lace_version` vira `aurora_version` (os
  relatórios gravados pelo Lace continuam legíveis), e o servidor MCP passa
  a se chamar `aurora`, com as ferramentas `mcp__aurora__*`. A IDE recusa
  um servidor do protocolo 1. O que fazer: um cliente do `aurora serve`
  passa a falar o protocolo 2, e a configuração de um agente troca o
  `lace mcp` pelo `aurora mcp`.
- O `--reopen-studio` do `aurora update` passa a ser `--reopen-ide`, e o
  `reopen_studio` do `update.apply`, `reopen_ide`. O executável da IDE no
  bundle passa a ser `aurora-ide` (no macOS, `AURORA IDE.app`), e o atalho
  do Linux, `aurora-ide.desktop`.
- A pasta do projeto que é só nossa (intermediários, relatórios, projetos
  de placa e a trava do `.spf`) passa de `.lace/` para `.aurora8/`. O 8 a
  separa da `.aurora/`, que é da AURORA 7 e pode estar no mesmo projeto.
  Abrir um projeto que tem `.lace/` a renomeia para `.aurora8/`, com o que
  tinha dentro. O registro da compilação para a placa passa de
  `lace-build.json` para `aurora-build.json`: uma placa compilada pelo Lace
  precisa ser compilada de novo, porque sem o registro a gravação recusa com
  `stale_bitstream`.
- Extensão do VS Code 0.2.0: passa a se chamar AURORA, e as configurações,
  os comandos e os contextos passam de `lace.*` para `aurora.*` (`lace.path`
  vira `aurora.path`). Ela fala com o `aurora` pelo protocolo 2, e o
  `.vsix` da release passa a se chamar `aurora-vscode-<versão>.vsix`. O
  pacote do RTL viewer passa a ser `@aurora/rtl-viewer`, e o ícone da
  extensão na barra de atividades, o camaleão. O que fazer: instalar o
  `.vsix` novo, desinstalar a extensão do Lace e passar as configurações
  `lace.*` para `aurora.*`.
- Um processador não pode mais ter o nome de um módulo novo da biblioteca
  SAPHO do YANC 9.1 (`myDCFIFO`, `myKeyPS2`, `myVGAText`, `div_array`,
  `ula_idiv`, `ula_lzc`). O que fazer: dar outro nome ao processador
  (`aurora proc remove` e `aurora proc add`).

### Outras mudanças

- O nome Lace sai do texto: a documentação, as mensagens da CLI, os textos
  da IDE, da extensão e do instalador passam a dizer AURORA. Ficam com o
  nome antigo só as menções ao que veio do Lace (a pasta `.lace` que a
  abertura renomeia, o `settings.json` do Lace Studio que a IDE copia, os
  atalhos antigos que o instalador apaga).
- Os instaladores trocam a marca do Lace pela da AURORA. No assistente do
  Windows, a faixa da esquerda tem o camaleão azul e o nome AURORA no fundo
  grafite do tema Boreal, e o canto das outras páginas, o ícone da AURORA
  IDE. No instalador de terminal, o camaleão aparece em braille, e o azul
  do tema vai para os títulos e a barra de progresso. O nome AURORA
  desenhado em curvas fica em `studio/public/brand/aurora-wordmark.svg`.
- A AURORA API e a AURORA IDE passam a ter versões próprias. A primeira
  release com o nome novo é a API 1.0.0 com a IDE 8.0.0; o `release.yml`
  confere só a versão da API na tag, e o `bundle.py` continua conferindo a
  da IDE contra o `bundle/versions.json`.

- O Lace Studio passa a se chamar AURORA IDE nos textos da CLI, do
  instalador e da documentação. O atalho do menu Iniciar e o da área de
  trabalho no Windows e o do menu de aplicativos no Linux trocam de nome; o
  instalador do Windows apaga os atalhos com o nome antigo. O componente
  continua `studio`. As mudanças da IDE estão em studio/CHANGELOG.md.

- Simulação, síntese, hierarquia e escolha de sinais funcionam com o
  projeto numa pasta com acento, outro alfabeto ou emoji, como
  `OneDrive\Área de Trabalho` ou a cópia `teste - Cópia` que o Explorer
  cria. No Windows, o Icarus, o `vvp`, o Yosys, o Verilator e o `dot` abrem
  arquivos pela página de código ANSI: um caminho com caractere fora do
  ASCII chega a eles relativo ao CWD do passo, e o Verilator nasce no nome
  curto da pasta. Os diagnósticos voltam com o caminho absoluto. Sai o erro
  `non_ascii_path`, que recusava a simulação de processador nessas pastas;
  a síntese falhava com `Can't open log file`, e a leitura das portas pelo
  Yosys caía no leitor de texto.
- A hierarquia e a árvore de sinais da onda leem o `.vvp` na página de
  código do Windows quando ele não é UTF-8: numa pasta com acento, o
  `iverilog` grava nele o caminho em Windows-1252, e a leitura falhava com
  `stream did not contain valid UTF-8`.

- Protocolo: o contrato sai do `lace-core` para um crate próprio, o
  `lace-protocol`, acima do core, do `lace-learn` e do instalador. Com
  isso as respostas de `update.check`, `update.apply`, `install.add`,
  `learn.check`, `learn.tracks`, `learn.status`, `learn.init` e
  `learn.set_current` ganham tipo no schema (`UpdateReport`,
  `InstallReport`, `LearnCheckOutcome`, `LearnTracks` e
  `WorkspaceSnapshot`), e o `docs/PROTOCOL.md` e os tipos gerados do Studio
  e da extensão passam a descrevê-las. O JSON que o `lace serve` manda não
  muda. Em Rust, quem usava `lace_core::protocol` passa a usar
  `lace_protocol`.

- Protocolo: cada método declara na linha dele em `methods!` se só lê, se
  mexe na instalação e se vira ferramenta de agente; o `is_read_only`, o
  `is_installation` e a tabela `METHOD_INFO` saem dali, e um teste confere
  a lista do `lace mcp` contra essas marcações. O despacho do `lace serve`
  precisa de um braço para cada método, exigido pelo compilador: sai o
  braço que respondia `unsupported`.

- CLI e `lace serve`: as operações de mais de um passo viram funções do
  core, e a CLI e o despacho do `lace serve` chamam as mesmas:
  `Project::add_verilog_files` e `remove_verilog_files`, `flows::run_with`
  e `flows::open_wave_with` (que conferem o pedido antes de procurar o
  bundle), `history::view`, `history::clean` e `experimental::info`. O
  `--json` não muda. O que se vê de diferente: um processador que não
  existe é erro antes do bundle também no `check`, no `hierarchy`, no `sim`
  e no `synth` (era só no `build`); um arquivo de onda que não existe é
  `no_waveform` antes do bundle; o `lace add` escreve os arquivos depois de
  registrar todos; e a pasta que não foi para a lixeira no
  `lace proc remove --delete` é erro `io`, e não `cli`.
- `lace serve`: o `flow.run` confere o recurso experimental e os
  processadores do pedido antes de procurar o bundle, e um `build` num
  projeto sem processadores não precisa dele; o `wave.open` confere a onda
  antes do bundle.

- Protocolo: métodos novos para o que o Studio ainda fazia linkando o core,
  o lace-learn e o instalador: `processor.defaults`, `flow.command`,
  `wave.layout` (o layout gerado, o salvo no projeto e o de um exercício,
  para um visualizador próprio), `verilog.modules`,
  `verilog.testbench_names`, `fpga.top` e os exercícios do `lace learn`
  (`learn.tracks`, `learn.status`, `learn.init`, `learn.set_current`,
  `learn.reset`). O `flow.run` corrige um exercício como fluxo
  (`{"flow": "learn"}`, com a correção em `learn`) e corta a saída de cada
  passo no resultado com `step_output_limit`. Os dois `PortDirection` do
  core viram um só.
- `lace serve`: um `--compiler` ou um `--quartus` que não serve não
  derruba mais o bundle da sessão. Antes, todo pedido que usa o bundle
  falhava com o erro da CLI; agora só a operação que precisa dele falha,
  com o erro dela, e o `toolchain.info` mostra o motivo, a regra que o
  Studio já seguia.
- `lace serve`: o `update.apply` ganha `reopen_studio`, o `--reopen-studio`
  da CLI, para o Studio pedir a atualização ao servidor; o `update.check`
  tem prazo de 90 s em cada consulta de versão, o que o Studio usava; o
  `install.add` recusa a lista vazia e um nome que não é do bundle antes de
  qualquer download, como o `lace install`; e a notificação `install` leva
  a linha do console em `line`, com os nomes dos aplicativos, que o
  `lace mcp` usa como mensagem de progresso.
- `lace wave <arquivo>` abre a onda de um exercício do `lace learn` com o
  layout da última correção, como o Studio já fazia, e um arquivo de onda que
  não existe é `no_waveform`, e não mais `invalid_project`. O `wave.open` do
  protocolo segue as duas regras.
- Protocolo: `project.move` ganha `overwrite`, que manda o destino que
  existe para a lixeira do sistema e move (menos o que o Lace mantém no
  lugar, `cannot_replace`), e `wave.choose` ganha `{"kind": "set"}`, que
  troca a escolha inteira, com os caminhos conferidos como no `add`. Eram
  comandos só do Studio.
- `lace serve` também fala MCP. O primeiro pedido diz o idioma: um cliente
  que se apresenta como MCP no `initialize`, com `protocolVersion`, recebe a
  mesma sessão do `lace mcp`, com as mesmas ferramentas, o projeto do `-C`
  como padrão e `--allow-update` para as que mudam a instalação. O idioma
  MCP exige o recurso experimental `assistant`, como antes. O `lace mcp`
  continua, como o mesmo servidor sempre em MCP, para as configurações dos
  agentes que já o têm.
- `lace serve`: fechar a entrada não cancela mais o que já foi pedido. O
  servidor responde cada pedido que recebeu e só então sai, e espera a
  última resposta ser escrita antes de sair; num `printf ... | lace serve`,
  os pedidos depois do `initialize` voltavam cancelados. Se o cliente
  fechou também a saída, o que roda é cancelado, como antes.
- `lace serve` e `lace mcp`: um parâmetro que o método ou a ferramenta não
  tem é recusado, com o nome mais parecido na mensagem, em vez de ignorado
  em silêncio; no `flow.run`, também um campo que o fluxo pedido não tem.
  Um BOM no começo de uma linha, que o PowerShell manda, é ignorado.
- `lace serve`: o `processor.check_name` confere o nome com a mesma regra
  do `processor.add`, que recusa os nomes das pastas do Lace (`wave`,
  `synth`, ...); antes ele aceitava esses nomes e a criação falhava
  depois. Os métodos `fpga.*` exigem o recurso experimental `fpga` ligado,
  como a CLI e o Studio já exigiam. Um projeto cujo caminho não existe
  responde o código `spf_missing`, o mesmo do Studio.
- `docs/PROTOCOL.md`: a referência do protocolo do `lace serve` e do
  `lace mcp`, com cada método, os parâmetros, o resultado, as
  notificações, os erros e os tipos. É gerada dos tipos de
  `lace_protocol` pelo mesmo comando do `docs/schema/`
  (`LACE_UPDATE_SCHEMA=1 cargo test -p lace-cli schema`), e um teste falha
  quando ela não confere.
- `lace_protocol`: a macro `methods!` declara cada método com o tipo
  do resultado, de onde saem `METHODS` e o novo `result_schema()`; uma
  variante de `Request` sem declaração não compila. Um teste do `lace
  serve` roda 23 dos 42 métodos num projeto temporário e confere cada
  resultado contra o tipo declarado. Os arquivos de `docs/schema/` não
  mudam.
- O bundle passa a 2026.10.22. O YANC vai da v7.1 à v9.1: a divisão leva 5
  ciclos (4 em float), o que sobe o clock máximo do processador, a condição
  em float segue o C nas duas linguagens, a biblioteca SAPHO traz o teclado
  PS/2 (`myKeyPS2`) e a tela VGA de texto (`myVGAText`), e o C++ ganha
  `<iostream>`, `<cstdio>` e `<cstring>`. O OSS CAD Suite vai de 2026-09-29
  a 2026-10-22: Yosys 0.70+190 e openFPGALoader 1.2.0 em todos os sistemas,
  e, no Linux e no macOS, Icarus e Verilator mais novos do `master`. No
  Windows, o bloco MSYS2 passa ao `ucrt64-v2`, com as mesmas versões de
  Icarus, Verilator, Perl, Python e cocotb e revisões do gcc 16.2 e das
  bibliotecas. Um processador não pode mais ter o nome de um módulo novo da
  biblioteca SAPHO (`myDCFIFO`, `myKeyPS2`, `myVGAText`, `div_array`,
  `ula_idiv`, `ula_lzc`).
- Com o YANC 9.1, um processador com três ou mais portas de saída e
  memória de dados pequena volta a escrever na porta 0: o número da porta
  saía cortado na largura do endereço de dados (era uma pendência externa
  em docs/PENDENCIAS.md).

- Linux: o instalador e o `lace install studio` dizem o comando que instala
  o webkit2gtk 4.1 pelo gerenciador de pacotes do sistema (`apt`, `dnf` ou
  `pacman`) quando ele falta, e a instalação guiada pergunta se o roda na
  hora. Antes, o aviso citava só `apt` e `dnf` e passava despercebido no fim
  da instalação, e o `lace install studio` nem avisava. O atalho do menu
  abre o `lace-studio-launch`, que confere as bibliotecas antes do Studio e,
  faltando alguma, mostra o que falta numa janela (`kdialog`, `zenity`,
  `notify-send` ou `xmessage`); antes, o Studio fechava sem mensagem. O
  README e o INSTALL.md dizem, antes do comando de instalação, que o KDE
  Plasma, o Arch e os ambientes leves em geral não trazem o webkit2gtk. A
  falta do compilador do Verilator diz também o comando do Arch.

- A correção do `lace learn` mostra a primeira amostra errada: o instante
  (e o ciclo, num circuito sequencial), o valor de cada entrada e cada
  saída, com o esperado e o do aluno, em binário até 8 bits e em
  hexadecimal acima, com o decimal ao lado. O testbench gerado escreve as
  linhas novas `first`, `first_in` e `first_out` do resumo; um `tb.v`
  escrito à mão pode escrevê-las também. O JSON do `lace learn check` traz
  o `counterexample`.
- Com o Verilator instalado, a correção acrescenta os avisos do lint sobre
  o arquivo do aluno (sem os de estilo), e, com o Yosys, avisa do latch
  inferido, com o sinal e a linha (`latch_inferred`), também num exercício
  resolvido. Nenhum dos dois muda o veredito; `"advice": false` no
  `exercise.json` desliga os dois. O `lace learn dev check` passa a exigir
  a solução sem latch e sem aviso.
- A trilha de Verilog do Lace Learn passa de 25 para 51 exercícios, em
  cinco capítulos novos: circuitos combinacionais (multiplexador,
  decodificador, codificador de prioridade, comparador, display de sete
  segmentos, código Gray), circuitos com memória (flip-flop, reset síncrono
  e assíncrono, registrador, contador, registrador de deslocamento,
  detector de borda), máquinas de estado (paridade serial, detectores de
  sequência de Moore e de Mealy, semáforo), aritmética (soma com saturação
  e com estouro, multiplicador, contagem de uns, acumulador, média móvel) e
  memórias (ROM, banco de registradores, RAM síncrona). Uma pasta de
  exercícios criada antes recebe os novos na primeira vez que é aberta.
- Na cópia da referência, uma porta ou um fio com o mesmo nome do módulo
  deixa de ganhar `_ref`: só a declaração e as instâncias mudam. Antes, um
  módulo `gray` com a saída `gray` não compilava no testbench.

## 0.9.0 (2026-10-20)

- O core passa a ser a API do orquestrador de fato. A composição dos
  fluxos (compilar antes, registrar o que o YANC gerou, ler as portas de
  saída, gravar o relatório, abrir a onda), que a CLI e o Studio mantinham
  em cópia e que divergia em quatro pontos, mora em `lace_core::flows`
  (`run`, `open_wave`, `choose_signals`), com um `FlowOutcome` de `status`
  explícito; a entrega da saída ao vivo em lotes com teto de linhas, em
  `lace_core::delivery`; a busca do bundle a partir de outro programa, em
  `lace_core::locate`; o retrato do projeto, em `Project::status()`. A CLI
  e o Studio viraram cascas sobre isso, com a mesma saída de antes
  ([API.md, 5.11](docs/API.md)).
- `lace serve`: a API do Lace para outro programa, por stdin e stdout, em
  JSON-RPC 2.0 com uma mensagem por linha (`lace_core::protocol`, com o
  schema em `docs/schema/protocol-request.json` e
  `protocol-response.json`). Cada pedido traz o projeto; os do mesmo
  projeto rodam na ordem, os que só leem em paralelo; `$/cancel` cancela;
  as notificações (`phase`, `events`, `output_omitted`, `build`,
  `install`) saem em lotes com o `id` do pedido. `lace serve --schema`
  escreve o schema ([CLI.md](docs/CLI.md)).
- `lace mcp`: o mesmo contrato como servidor MCP para um agente, uma
  ferramenta por método (`build`, `check`, `sim`, `synth`,
  `project_status`, `history_show`, ...), com o `inputSchema` tirado do
  schema do protocolo. Cada ferramenta devolve um resumo em texto (o
  status, os erros com arquivo e linha, o relatório) e o JSON inteiro com
  a saída dos passos cortada; o projeto é opcional, com o padrão da pasta
  atual (ou do `-C`), e `project_find` acha os `.spf`; as
  chamadas rodam fora da thread de leitura, com `notifications/progress` e
  `notifications/cancelled`. `lace mcp setup claude-code|claude-desktop|print`
  registra o servidor no cliente. `update_apply` e `install_add` só com
  `--allow-update`.
- `lace chat`: o assistente do Lace no terminal, e a vista Assistente no
  Studio, na barra lateral da direita ou numa aba. Quem responde é o
  Claude Code instalado na máquina (a conta ou a chave dele; o Lace não
  guarda chave), aberto como processo filho com o `lace mcp` como
  ferramentas: o assistente compila, verifica, simula, sintetiza e lê os
  relatórios do projeto aberto, e mostra cada ferramenta que chamou. Por
  padrão só usa as ferramentas do Lace e lê arquivos; `--allow-edits` (ou
  a preferência do Studio) deixa editar os arquivos do projeto. O motor é
  o crate novo `lace-agent`.
- O assistente de IA é o componente experimental `assistant`, desligado
  por padrão como a placa FPGA: `lace mcp`, `lace chat` e a vista
  Assistente do Studio só existem depois de `lace experimental enable
  assistant` (ou Preferências > Experimental > Assistente de IA, no
  Studio). Tudo o que passa pelo `lace-agent` ainda vai mudar até a 1.0.
  O `lace serve` não é experimental.
- O `lace update`, o `lace install` e o `lace uninstall` viraram
  biblioteca (`lace-installer`: `update::run`, `add`, `uninstall`, com os
  erros com código `network`, `no_installation`, `install_in_progress`,
  `checksum_mismatch`, `installer_failed`, `wizard_failed`, `cancelled`),
  que a CLI, o Studio e o `lace serve` chamam em processo: a regra do que
  é atualizar mora uma vez só. Os comandos, as flags e o JSON não mudam;
  o `error.code` do `--json` desses comandos deixa de ser sempre `cli`.
  Cancelar vale nas consultas de versão, nos downloads (mata o `curl`) e
  entre os pedaços; a troca dos arquivos segue até o fim, coberta pelo
  diário. O progresso tem eventos novos (`fallback`, `fallback_download`,
  `wizard_selection`, `installer_output`, `warning`).
- A extensão do VS Code fala com um `lace serve` em segundo plano quando o
  Lace instalado é a 0.9.0 ou mais nova (`lace.transport`: `auto`, `serve`
  ou `cli`), um processo só em vez de um por comando, e volta à CLI com um
  Lace mais antigo.
- Um `ErrorInfo` só, no core (`code`, `message`, `hint`, `data`): o
  `error.json` da CLI ganha o campo opcional `data`, com os campos da
  variante. Erros novos: `no_waveform`, `signal_not_found`,
  `signal_not_chosen`, `testbench_not_elaborated`. `NewProcessor` em JSON
  aceita só `name` e `language`, com os outros campos no padrão da CLI.

## 0.8.0 (2026-10-15)

- A instalação começa por um perfil, no lugar de Recommended e Advanced:
  **SAPHO** (o recomendado e padrão, todos os aplicativos), **Verilog**
  (todos menos o YANC) e **Custom** (uso avançado, para quem sabe de quais
  componentes precisa). Cada perfil diz para quem serve e mostra a lista
  inteira dos aplicativos, com os que ele instala marcados e travados; só o
  Custom deixa marcar. O Lace (CLI & Core) vem sempre marcado. Na
  instalação guiada do Linux e do macOS, o perfil sob o cursor mostra cada
  aplicativo com `[x]` ou `[ ]`. O padrão muda: o SAPHO traz também o
  Verilator, o cocotb, o openFPGALoader e o Lace Learn, que a Recommended
  deixava de fora. Os perfis ficam em `bundle/components.json`
  (`profiles`), que o `bundle.py` confere; o campo `recommended` dos
  componentes saiu, e o índice da release continua a gravá-lo (o do perfil
  padrão) para o `lace update` das versões anteriores.
- O assistente do Windows começa perguntando o idioma: inglês ou português
  do Brasil, com o do Windows já escolhido (`/LANG=en` ou `/LANG=pt`). As
  descrições dos aplicativos e dos perfis em português ficam em
  `bundle/components.json` (`translations`), que o `bundle.py` confere.
- `./install --yes --profile <perfil>` instala um perfil sem perguntar, e
  `./install --list` mostra cada perfil com o tamanho, o público e os
  aplicativos. No assistente do Windows, `/TYPE=` recebe o perfil (`sapho`,
  `verilog`) ou `avancada`, o Custom; o `/TYPE=recomendada` de
  antes cai no perfil padrão.
- O instalador com a cara do Lace. O assistente do Windows usa o ícone do
  Lace Studio, segue o modo claro ou escuro do Windows, traz na página de
  boas-vindas a marca e o nome sobre o azul do Lace e é mais alto, para a
  lista de aplicativos caber sem rolar; as imagens saem da marca por
  `scripts/installer_art.py`. A instalação guiada do Linux e do macOS abre
  com a marca desenhada em braille e usa as cores dela.
- `lace proc remove <nome>` tira um processador do projeto: a entrada no
  `.spf`, o Verilog e o testbench gerados das listas de arquivos (com a
  escolha de topo e de testbench que apontava para eles) e a pasta
  temporária do Lace. A pasta do processador fica no disco; com `--delete`,
  ela vai para a lixeira do sistema, depois de perguntar (sem terminal,
  com `--yes`). No Core, `Project::remove_processor`; o `--json` é o
  `docs/schema/proc-remove.json`.
- As placas FPGA viraram um componente experimental, desligado por padrão:
  desligado, todo `lace fpga` sai com 2 e diz como ligar. O comando novo
  `lace experimental` lista os componentes experimentais, e `lace
  experimental enable fpga` e `disable fpga` ligam e desligam. O que fica
  ligado vai para `experimental.json`, na pasta de configuração do Lace
  (`%APPDATA%\lace` no Windows, `~/.config/lace` no Linux), e vale também
  para o Lace Studio, que liga e desliga o mesmo arquivo em Preferências >
  Experimental. No Core, o módulo `experimental` e o erro
  `experimental_disabled` ([docs/CLI.md](docs/CLI.md), "Componentes
  experimentais").
- O Lace avisa quando há versão nova. O Lace Studio procura uns segundos
  depois de abrir e, aberto, a cada 12 horas (`lace update --check`), e
  oferece a atualização num diálogo; o indicador na barra de status fica
  até ela. Atualizado, o Studio grava os arquivos abertos e reinicia sozinho
  na versão nova. No Windows, quando a atualização cai no assistente de
  instalação, o Studio fecha para ele trocar os arquivos e o assistente o
  abre de novo no fim (`lace update --reopen-studio`, que o Studio passa, e
  o `/REOPENSTUDIO=1` do assistente). Vale a partir desta versão: um Studio
  0.7.0 não procura nem reinicia.
- O `lace`, num terminal, consulta uma vez por dia a última release
  enquanto o comando roda e, se há Lace mais novo, avisa no fim, no stderr,
  uma vez por dia por versão ([docs/CLI.md](docs/CLI.md), "Bundle"). Não
  consulta com `--json` ou `--events`, fora de um terminal, num `lace` que
  não veio do instalador, com `CI` ou com `LACE_NO_UPDATE_CHECK=1`.
- A placa Terasic Cyclone V GX Starter Kit (C5G, Cyclone V GX
  5CGXFC5C6F27C7), com `"board": "c5g"` no `fpga.json`: os quatro clocks de
  50 MHz, KEY, o CPU_RESET_n, SW, LEDR, LEDG e os quatro displays. Os 65
  pinos vêm do manual da Terasic e foram conferidos com o golden top e com
  a demonstração de fábrica do CD da placa, 65 de 65, e o perfil compila
  no Quartus 25.1. HEX2 e HEX3 dividem os pinos com o GPIO e pedem as
  chaves DIP S1 e S2 em ON ([docs/FPGA.md](docs/FPGA.md)). Ainda não foi
  gravado numa placa.
- `lace fpga boards` alinha as colunas pelo maior id e pelo maior nome;
  antes, um nome com mais de 20 letras desalinhava a tabela.
- O bundle traz o YANC v7.1 (`6c32d94`, bundle `2026.10.13`), e o Lace
  segue as regras dele, como a AURORA 7.1. A 7.1 mudou a linha de comando,
  os compiladores e as pastas: o fonte é o argumento sem opção, o
  processador se chama como o arquivo (o `#PRNAME` virou erro), cada
  ferramenta acha `SAPHO/`, `Macros/` e `Header/` ao lado do próprio
  executável, o `asmcomp` faz as duas passadas e o `cppcomp` pré-processa
  (não há mais `appcomp` nem `cpppp`), e tudo o que é gerado vai para uma
  pasta de saída, com os arquivos citados pelo caminho relativo a ela. O
  build roda `cmmcomp <proc>.cmm -o <proc>/<proc> --lang=en` (ou o
  `cppcomp`) e `asmcomp <proc>/<proc>/<proc>.asm`. Um bundle com o YANC até a
  6.x é recusado com `yanc_too_old`.
- **Estrutura de pastas do YANC 7.1.** O processador é a pasta `<proc>/`,
  com o fonte `<proc>.cmm` (ou `.cpp`) e os arquivos da pessoa (os de
  arrays, `input_<n>.txt`, `cade_<n>.txt`, os cabeçalhos do C); tudo o que
  o YANC gera fica em `<proc>/<proc>/`: o `.asm`, o Verilog, as memórias, o
  testbench, as tabelas da onda (`src_lines.txt`, `prog_log.txt`) e o que a
  simulação grava (`output_<n>.txt`, a onda). `Software/`, `Hardware/` e
  `Simulation/` saíram. A simulação de um processador roda na pasta de
  saída, como o YANC pede; a do projeto roda na raiz (ou na pasta de saída,
  com o testbench gerado de um processador) e copia para lá as memórias e o
  `pc_<proc>_mem.txt` de cada processador, como a síntese e a compilação
  para a placa (que as copia para a pasta da placa; não conferido no
  Quartus). Nada é reescrito no Verilog gerado, e o projeto pode mudar de
  pasta sem compilar de novo. `Processor` ganhou `output_dir` e
  `assembly_path`, `verilog_path`, `files_read_by_verilog`; saíram
  `software_dir`, `hardware_dir` e `simulation_dir`.
- **Conversão do projeto antigo.** Um processador com `Software/`,
  `Hardware/` ou `Simulation/` não compila (`legacy_layout`, e o aviso em
  `lace status`). `lace convert` (`Project::convert_layout`) faz a
  conversão da AURORA: o que o YANC gerava vai para a pasta de saída, o
  resto para a do processador, nada é apagado nem sobrescrito, a linha
  `#PRNAME` sai do fonte e o `.spf` acompanha, com um relatório do que
  mudou, do que ficou e dos módulos que mudaram de nome. O Lace Studio
  pergunta ao abrir um projeto antigo.
- **Processador novo.** O fonte-modelo sai sem `#PRNAME`, sem `#NDSTAC` e
  `#SDEPTH` se elas não forem dadas (o YANC calcula as pilhas pelo
  programa) e sem `#NUGAIN` se ele for 128; a pasta de saída nasce no
  primeiro build. O nome segue a regra do YANC 7.1: a lista de palavras
  reservadas do Verilog e do SystemVerilog dele, e nada de `__` no começo;
  palavras do C± que não são do Verilog (`in`, `out`, `sqrt`) passaram a
  valer. `NewProcessor::ndstac` e `sdepth` viraram `Option`.
- No fluxo C, os erros apontam o fonte (ou o cabeçalho incluído) e a linha
  escrita, e não mais o `pp.cpp`; a onda mostra cada linha como foi
  escrita. Sai das pendências o `externo-cpppp-sem-line`. No macOS, a 7.1
  parece não achar as pastas da biblioteca (deduzido do código do YANC,
  não conferido num Mac: [docs/PENDENCIAS.md](docs/PENDENCIAS.md),
  `externo-yanc-exe-dir-macos`).
- `lace tools` e a aba Toolchain do Studio não listam mais o `appcomp` e o
  `cpppp`; o resumo de um build que falhou mostra os passos que não rodaram
  com os da 7.1 (`build_steps`). Os exemplos de `examples/` estão na
  estrutura nova, e os testes que comparam o `.asm` com os goldens do YANC
  usam os casos de teste da 7.1.

## 0.7.0 (2026-10-12)

- A simulação rápida (`lace sim --fast`, o Fast Sim do Studio) compila o
  modelo do Verilator sem `--autoflush`, como a AURORA: o `$display` chega
  em blocos, e não linha a linha. Num testbench que imprime a cada ciclo, o
  flush de cada linha chegava a dobrar o tempo da simulação. Uma simulação
  rápida cancelada perde o que ainda estava no buffer. A primeira depois da
  atualização recompila o modelo.
- O Lace lê a saída das ferramentas em blocos e passa as linhas de cada
  bloco de uma vez, e não uma linha por vez: o custo de cada linha até o
  terminal, o Studio ou o `--events` caiu para cerca de um quinto, o que
  pesa num testbench que imprime sem parar.
- O `install.ps1` (o `irm ... | iex` do Windows) mostra o progresso do
  download do instalador: a barra, a porcentagem, os MiB, a velocidade e o
  tempo que falta, numa linha que encolhe para caber na janela. Antes, o
  download de 126 MiB passava sem nenhum sinal. Uma conexão que cai no meio
  ganha uma segunda tentativa. Como o script sai da `main`, isto vale também
  para quem instala a 0.6.0.
- `lace update` por componentes ([docs/INSTALL.md](docs/INSTALL.md),
  "Atualizar por componentes"): em vez de baixar o instalador inteiro (126
  MiB no Windows) e reinstalar tudo, ele compara o manifesto de arquivos da
  instalação com o da release nova, mostra o que muda e quanto baixar, e
  baixa só os pedaços com arquivos que mudaram. Os arquivos trocam no lugar,
  o próprio `lace` também (no Windows, por renomeação do executável em
  uso), e o bundle é conferido; uma falha no meio devolve a instalação ao
  que era. No Windows, o assistente não abre mais, o que o `lace install`
  acrescentou fica, e a lista de programas instalados passa a mostrar a
  versão nova. `--full` usa o instalador inteiro, como antes, e é também a
  reserva quando a atualização por componentes não dá. `--from` atualiza a
  partir de um instalador no disco, sem rede. Vale a partir da 0.7.0: a
  instalação precisa do manifesto, que a 0.6.0 não grava, então a
  atualização de 0.6 para 0.7 ainda é pelo instalador.
- O manifesto de arquivos: a release publica
  `lace-<versão>-<plataforma>-files.json` (o SHA-256 de cada arquivo, por
  pedaço), e toda instalação guarda o da versão dela em
  `toolchain/files.json`. O índice dos pedaços passa a dizer o tamanho de
  cada um para baixar (`download`).
- `lace install` e `lace update` mostram a barra de cada pedaço baixado,
  com a porcentagem e os MiB. `lace install --from` aceita também uma pasta
  com os arquivos que a release publica.
- No Windows, o pedaço sempre instalado traz o `lace` como `bin/lace.exe`
  (antes, `bin/lace`, que só o Linux e o macOS usavam).
- A escolha dos sinais da onda, a Wave Configuration da AURORA
  ([docs/CLI.md](docs/CLI.md), "Onda"): `lace wave signals` mostra a árvore
  do testbench do projeto, elaborada pelo Icarus, e `lace wave select` e
  `unselect` gravam a escolha em `wave/<testbench>.json`, na pasta do
  projeto. A simulação do projeto passa a gravar só ela, numa cópia do
  testbench (o arquivo dele não muda), com cada processador SAPHO do design
  sempre junto; o layout mostra só os escolhidos, um grupo por escopo. Sem
  escolha, a onda grava todos os sinais, como antes. No JSON,
  `wave-signals` e `wave-select`.
- O layout da onda salvo no projeto: a onda de um testbench abre com
  `wave/<testbench>.surf.ron`, e o Ctrl+S do Surfer salva nele. O Lace o
  refaz a cada abertura até você salvar; depois, ele fica, e
  `lace wave --reset-layout` volta ao gerado. O `--json` do `lace wave`
  ganha `saved_layout`.
- O assistente web do Windows, `lace-<versão>-windows-x64-web-setup.exe`,
  com alguns MB: ao clicar em Instalar, ele baixa da release só os
  aplicativos marcados, com a barra de cada download, confere cada um pelo
  SHA-256 e os extrai (o `lace setup`, que troca o `toolchain\` antes de o
  assistente copiar qualquer arquivo). O `irm ... | iex` passa a usar o web
  quando a release o publica; `LACE_FULL_SETUP=1` volta ao completo, que
  continua na release para instalar sem rede. `/MIRROR=<url>` baixa de um
  espelho do laboratório.
- O assistente do Windows, completo ou web, numa pasta que já tem o Lace: a
  primeira página diz de que versão para qual ele atualiza, e os aplicativos
  que a pasta tem vêm marcados, inclusive os que o `lace install`
  acrescentou (antes, reinstalar pela Recommended os tirava). O aviso de
  falta do WebView2 aparece antes de instalar, e não depois.
- O instalador de Linux e macOS, numa pasta que já tem o Lace: o tipo
  Recommended (e o `./install --yes` sem `--components`) mantém os
  aplicativos que a instalação tem, junto dos recomendados; antes, voltava
  só aos recomendados e tirava o resto sem avisar. A tela de boas-vindas
  lista os aplicativos do próprio instalador (antes, uma lista fixa que não
  tinha o cocotb, o openFPGALoader nem o Studio).
- `LACE_RELEASE_URL` (o espelho de releases do laboratório) pode trazer um
  arquivo `latest` com a versão mais nova, que o `lace update` lê no lugar
  do GitHub.
- `lace synth --open` mostra o netlist da síntese no RTL viewer, o
  esquemático interativo do Studio, no navegador: o Lace grava
  `rtl-viewer.html` ao lado do netlist (`.lace/Temp/synth/<alvo>/`), um
  arquivo só que abre sem servidor e sem rede, e o abre pelo abridor do
  sistema ([docs/CLI.md](docs/CLI.md), "Compilar, verificar, simular,
  sintetizar"). O `--json` ganha `rtl_viewer`. Com `LACE_OPENER` definida, só
  grava o arquivo (a extensão do VS Code a define no terminal integrado e
  abre o netlist numa aba). `LACE_RTL_VIEWER` aponta outro modelo.
- O componente `rtl-viewer` no bundle, na instalação Recommended: o modelo
  do HTML do `lace synth --open`, que exige o Yosys. O RTL viewer saiu do
  Studio para um pacote do Lace, `packages/rtl-viewer/`, que o Studio, a
  extensão do VS Code e o `lace synth --open` usam; no Studio, nada muda no
  uso.
- O build põe no `.spf` o que cada processador gerou, como a AURORA faz:
  depois de `lace build`, `sim`, `synth` e `fpga build` (e dos fluxos do
  Studio), o `Hardware/<nome>.v` entra como módulo e o
  `Simulation/<nome>_tb.v` como testbench, se ainda não estavam. Num projeto
  sem módulos, o do primeiro processador vira o topo; sem testbench, o dele
  vira o simulado. Um arquivo tirado com `lace remove` volta no build
  seguinte. O `--json` desses comandos ganha `registered`; no Core,
  `Project::register_generated`. Antes, o Lace usava o `.v` gerado sem
  registrá-lo, e um projeto aberto depois na AURORA, que só compila o que
  está no `.spf`, dependia de ela o descobrir. O `lace hierarchy` não mostra
  mais duas vezes o testbench de um processador registrado.
- `lace move --after` registra no `.spf` um movimento que já aconteceu fora
  do Lace (no explorador de um editor, com o `mv`), como o `hg mv --after`:
  os registrados que estavam na origem passam ao destino, com o topo e o
  testbench simulado, e o disco não é tocado. No Core,
  `Project::record_move`.
- A extensão do VS Code, em `editors/vscode/` (versão própria, 0.1.0;
  [README](editors/vscode/README.md)), fora do Marketplace por enquanto: o
  `lace-vscode-0.1.0.vsix` vai anexado a esta release e se instala no VS Code
  em Extensões > `...` > Install from VSIX. Fala com o Lace pela CLI com `--json`
  e `--events`: a vista Fontes do projeto (`lace status`), com o topo, o
  testbench simulado e os processadores em destaque; os botões Build,
  Check, Sim, Wave e Synth na barra de status (`lace build`, `check`, `sim`,
  `wave` e `synth`), com a saída ao vivo, o cancelamento e os erros do check
  e do YANC no painel Problemas; o RTL viewer numa aba depois de cada
  síntese; Definir como topo (`lace top`); Novo projeto (`lace new`), num
  botão da vista quando a pasta não tem `.spf`; criar e configurar
  processadores (`lace proc add` e `set`); realce e snippets de C± e do
  assembly do SAPHO; e os arquivos do projeto: pôr (`lace add`, inclusive um
  arquivo novo do modelo, o aviso de `.v` novo fora do projeto e arrastar do
  explorador), tirar (`lace remove`), a ordem de compilação (`lace order`,
  pelo menu ou arrastando) e renomear (`lace move`); o que o explorador
  renomeia ou apaga acompanha no `.spf` (`lace move --after`, `lace
  remove`). Precisa do Lace 0.2.0 ou mais novo (check, sim e synth, da
  0.3.0).

## 0.6.0 (2026-10-09)

- Placas FPGA ([docs/FPGA.md](docs/FPGA.md)): o Lace
  conhece a Terasic DE2-115 (Cyclone IV E EP4CE115F29C7) e a DE10-Nano
  (Cyclone V 5CSEBA6U23I7), com os pinos tirados dos manuais da Terasic. O
  `fpga.json`, ao lado do `.spf`, diz a placa e liga as portas do topo aos
  sinais dela (`"rst": "!KEY[0]"`, `"out[7:0]": "LEDR[7:0]"`), e o Lace gera
  o topo da placa, que inverte, corta e completa larguras. `lace fpga
  boards` mostra as placas e os pinos; `lace fpga check` confere o
  `fpga.json` contra a placa e as portas do topo, com todos os problemas de
  uma vez.
- O Quartus Prime, que compila para as placas Intel, é a segunda exceção à
  regra do bundle: `--quartus <DIR>` (ou `LACE_QUARTUS`), senão
  `QUARTUS_ROOTDIR`, senão as pastas padrão do instalador. `lace tools`
  mostra o que achou; no JSON, `quartus`, e a ferramenta `quartus`.
- `lace fpga build` compila os processadores e depois o projeto para a
  placa pelo Quartus, sem abrir a interface: grava o projeto do Quartus em
  `.lace/fpga/<placa>/` e roda `quartus_map`, `quartus_fit`, `quartus_asm` e
  `quartus_sta`, os passos `synthesize`, `fit`, `bitstream` e `timing`. O
  resultado traz os erros e avisos do Quartus com arquivo, linha e número da
  mensagem, o `.sof` (mais o `.rbf` e o `.svf` sem compressão, para o
  openFPGALoader), os recursos da FPGA e a Fmax e as folgas de cada clock.
  No JSON, `fpga-build`.
- `lace fpga program` grava na placa o `.sof` da última compilação pelo
  Quartus Programmer (`quartus_pgm`), pelo primeiro cabo ligado ou pelo de
  `--cable`; `--list` mostra os cabos. No JSON, `fpga-program` e
  `fpga-cables`. Os dois comandos foram conferidos com o Quartus Prime 25.1
  Lite e uma DE2-115.
- A segurança da placa ([docs/FPGA.md](docs/FPGA.md), "Segurança da placa"):
  depois do Fitter, o Lace confere no `.pin` cada pino, direção e padrão de
  I/O contra a placa e reprova a compilação antes do `.sof` se algum ficou
  fora do lugar; os pinos sem uso ficam escritos no `.qsf` como entrada em
  alta impedância; e a gravação recusa (`stale_bitstream`) um `.sof` que
  não descreve o projeto de agora, comparando pelo conteúdo os fontes, as
  memórias dos processadores, o `fpga.json` e a definição da placa
  (`lace-build.json`). Os pinos das duas placas foram conferidos também com
  o `.qsf` da Terasic, um a um, e a DE10-Nano ganhou os LEDs 6 e 7, que vêm
  dele.
- O componente opcional `openfpgaloader`, com o openFPGALoader do OSS CAD
  Suite nas três plataformas, para gravar as placas pelo cabo USB. No JSON,
  a ferramenta se chama `openfpgaloader`.
- Lace Studio: a aba Placa FPGA, com as ligações, a compilação e a
  gravação, e o console Placa (os detalhes estão em `studio/CHANGELOG.md`).
- O Lace passa a ter licença: MIT ([LICENSE](LICENSE)).
- O OSS CAD Suite 2026-09-29 não traz o `nextpnr-mistral` (Cyclone V) em
  nenhuma plataforma, e o fluxo aberto para a DE10-Nano ficou para depois
  do Quartus.

## 0.5.0 (2026-10-08)

- `lace learn`: exercícios de Verilog no estilo do rustlings
  ([docs/LEARN.md](docs/LEARN.md)). `lace learn init` cria a pasta de
  exercícios, um projeto Lace por exercício, e `lace learn` entra no modo
  watch: a cada gravação, o Lace roda o `check` e a simulação no Icarus e
  compara cada saída do módulo do aluno com a de uma solução de referência,
  amostra a amostra. A correção diz em quantas amostras cada saída errou, o
  instante do primeiro erro e, quando dá, a causa (porta renomeada, saída
  sem atribuição, reset que só erra quando ativo), e a onda abre com as
  saídas ao lado das da referência e o primeiro erro marcado. Os
  subcomandos `check`, `list`, `hint`, `reset`, `wave` e `dev check` têm
  `--json`. A primeira trilha, `verilog`, tem 25 exercícios, de primeiros
  passos a hierarquia, e vem no componente opcional `lace-learn`, que os
  instaladores oferecem na instalação Advanced (no Windows, embaixo do
  Icarus) e o `lace install lace-learn` acrescenta depois. O código é o
  crate novo `lace-learn`, que o Studio também usa.
- Lace Studio: a vista Exercícios, a aba do enunciado e a correção a cada
  gravação (os detalhes estão em `studio/CHANGELOG.md`).

## 0.4.0 (2026-10-07)

- Simulação rápida, o Fast Sim da AURORA: `lace sim --fast` roda sem gravar
  onda, para ver a saída do testbench, as saídas do processador e os testes
  cocotb na velocidade do simulador. O testbench Verilog, do projeto ou de um
  processador (`-p`), roda no Verilator compilado sem `--trace`, num
  `obj_dir_fast_<topo>` à parte, e o Verilator ignora os `$dumpfile` e
  `$dumpvars` dele; um testbench cocotb roda os testes sem a onda, no Icarus
  (`vvp -none`) ou, com `--verilator`, no Verilator. No JSON, a simulação
  traz `fast`; na API, `SimulationOptions::fast` e `SimulationResult::fast`.
- O cocotb também roda no Verilator: `lace sim --verilator` com um testbench
  `.py` compila o design com o `main` e a VPI do cocotb e grava a onda em
  `<módulo de teste>.vcd`. `cocotb_needs_icarus` deixou de existir; um
  cocotb sem a biblioteca do simulador dá `cocotb_unavailable`. Testado no
  Windows; no Linux e no macOS, não.
- YANC 6.0 no bundle (era a 5.6; a 5.7 ficou no meio). Pelo CHANGELOG dele:
  `DIV`, `MOD` e `F_DIV` levam três ciclos, e cada divisão vira
  `<divisão> x; NOP; <leitura>`, com as leituras novas `QUO`, `REM` e
  `F_QUO` (o Fmax do `sapho_all` foi de 9,43 para 21,9 MHz na DE10-Nano); o
  `asmcomp` recusa um `.asm` escrito à mão com divisão sem essa sequência.
  O código que nenhum caminho alcança sai do programa e do hardware, a pilha
  sem `#NDSTAC`/`#SDEPTH` fica com a profundidade que o programa usa, e
  recursão em C± é erro. No Studio, o assembly conhece as três leituras.
- Windows: o Verilator reaproveita o modelo em dia. Ele anotava o
  `verilator_bin` sem `.exe` entre as entradas do modelo, não reconhecia
  nada como em dia, gerava o C++ de novo e recompilava tudo a cada simulação,
  a biblioteca dele junto; agora, com o design igual, a compilação leva
  menos de 1 s (num contador, 9 s antes). Sai também o "o sistema não pode
  encontrar o caminho especificado" que o script dele escrevia a cada
  chamada, e os avisos do g++ sobre a biblioteca do próprio Verilator viram
  informação.
- Lace Studio (os detalhes estão em `studio/CHANGELOG.md`):
  - o RTLViewer (F10) desenha o esquemático no próprio Studio, a partir do
    netlist da síntese, com as cores do tema, a hierarquia, a busca e o
    destaque das redes; a CLI continua com o Graphviz (`lace synth --svg`);
  - a Rápida (F9) é a simulação rápida, e o item do Explorador que marca o
    testbench só marca, sem simular;
  - layout da janela: cada vista vai para qualquer região, há uma barra
    lateral direita, e os layouts com nome guardam a janela inteira;
  - barra de título integrada, com os menus, o nome do projeto e os botões
    da janela numa faixa só;
  - as Preferências têm uma página por assunto, e o tema Atlas passou ao
    azul do ATLAS.

## 0.3.0 (2026-10-06)

Windows:

- Nenhuma ferramenta abre janela de console: no Lace Studio, cada passo de
  build, verificação, simulação e síntese abria uma, e fechá-la matava a
  ferramenta. A saída continua nos consoles do Studio e no terminal.
- As ferramentas não ficam mais órfãs. Cada passo roda num Job Object, e o
  que a ferramenta iniciou termina com ela: ao cancelar, no fim do passo e
  quando o Lace ou o Studio morrem, mesmo encerrados pelo Gerenciador de
  Tarefas (antes, o `iverilog` e o `vvp` seguiam rodando). Quando o Lace tem
  console, como num terminal, as ferramentas continuam nele, sem o custo de
  um console novo por ferramenta (`ProcessJob` e `hide_console` no
  `lace-core`).
- `lace update` abre o assistente da versão nova com os componentes
  instalados marcados, inclusive os que o `lace install` acrescentou, que
  antes saíam na atualização. O assistente e o desinstalador abrem sem a
  saída do `lace` e fora do job de quem o chamou: o Studio não fica mais
  ocupado até o assistente fechar.
- No Studio, cancelar um `lace install` encerra também o download em
  andamento, e "Novo arquivo" ou "Nova pasta" numa pasta que ainda não
  existe (`rtl/modulo.v` num projeto sem `rtl/`) não dá mais "fora do
  projeto".

Todos os sistemas:

- Um `include` relativo à pasta de quem o faz falhava no `check` e no `sim`
  no Windows (o Icarus recebia o caminho com `\`) e, em qualquer sistema, no
  `sim` de um testbench sem `$dumpfile`, cuja cópia com o dump injetado fica
  em `.lace/Temp`. Os diagnósticos e a hierarquia trazem os caminhos com o
  separador do sistema, iguais aos do projeto.
- No Studio, um caminho com `..` passando por uma pasta que não existe não
  escapa mais da pasta do projeto.
- Testbench em Python, com o cocotb: um `.py` entra no projeto como
  testbench (`lace add test_alu.py`, criado a partir de um modelo se não
  existe), e `lace sim` roda os testes no Icarus. Cada teste sai na saída da
  CLI e em `simulation.tests` no JSON; o que falha é um erro na linha do
  `.py` e reprova a simulação, e a onda sai do mesmo jeito. O módulo testado
  vem da linha `# aurora-toplevel: <módulo>` do `.py`, como na AURORA, ou do
  topo do projeto. Com o Verilator o Lace recusa (`cocotb_needs_icarus`); o
  `check` e o `hierarchy` deixam o `.py` de fora.
- `lace check` não compila mais os processadores: verifica o Verilog que
  está no disco, como o nome diz. Um processador que nunca foi compilado fica
  de fora, com um aviso; se não sobra nada para verificar, sai com
  `not_built` e a dica de compilar. O JSON do `check` não tem mais
  `builds`.
- O esquemático não recusa mais um módulo com mais de 120 ligações
  (`schematic_too_large` deixou de existir); continua o prazo de 60 s do
  `dot`. `--no-schematic-limit` é aceito e ignorado.
- A onda do Icarus sai sempre em FST, com a extensão `.fst`: o
  `$dumpfile("<proc>_tb.vcd")` do testbench que o YANC gera vira
  `<proc>_tb.fst` numa cópia do testbench, e o modelo do `lace add` grava
  `.fst`. O Verilator continua em VCD, agora sempre com a extensão `.vcd`
  (antes, um `$dumpfile("x.fst")` gravava VCD em `x.fst`). O layout do
  Surfer dos processadores SAPHO lê também o FST.
- No Studio, o terminal acompanha o projeto aberto (o shell entra na pasta
  do projeto novo), e a aba Lace saiu do painel inferior: a saída do
  `lace install` e do `lace update` aparece na tela de Ferramentas.

## 0.2.0 (2026-10-05)

A 0.1.0 saiu com o nome Solar; esta é a mesma linha com o nome Lace (o
executável `lace`, a pasta `~/.local/share/lace`, `LACE_*` no lugar de
`SOLAR_*`). O Lace não troca o Solar: quem tem o Solar 0.1.0 o desinstala e
instala o Lace (INSTALL.md, "Quem tem o Solar 0.1.0").

Lace Studio e instaladores:

- O Lace Studio, o ambiente gráfico, passa a morar neste repositório
  (`studio/`), com a versão do Lace, e a sair no instalador como o
  componente `studio`, marcado na instalação Recommended. Fica no bundle
  (`toolchain/studio/`), usa o bundle em que está e ganha atalho no menu de
  aplicativos: o `lace-studio.desktop` no Linux, o `Lace Studio.app` em
  `~/Applications` no macOS, o menu Iniciar (e, se marcado, a área de
  trabalho) no Windows. `lace install studio` o instala depois, e o
  `uninstall.sh` tira o atalho (`lace_installer::desktop`).
- Os instaladores avisam quando falta o WebView do Studio: o webkit2gtk 4.1
  no Linux, o WebView2 no Windows.
- O cocotb também é componente no Windows, do bloco MSYS2 do lace-toolchain
  (o Python e a VPI do Verilator compilados lá). Com isso, todo aplicativo
  do bundle é selecionável nas três plataformas; só a linha de comando é
  sempre instalada.
- O bloco de Windows (`msys`) vem do lace-toolchain `ucrt64-v1`.
- No Windows, o `lace.exe`, o instalador, o Studio e o surfer-aurora levam
  o runtime do Visual C++ dentro (`+crt-static`, `.cargo/config.toml`): não
  pedem mais o `VCRUNTIME140.dll`. O Lace cria a pasta `tmp/` do MSYS antes
  do Verilator, cujo `make` avisava `could not find /tmp` a cada chamada.
- Mínimos documentados (INSTALL.md, Requisitos): no Linux, a glibc 2.35
  (Ubuntu 22.04, Debian 12, Fedora 36), porque o CI compila no
  `ubuntu-22.04` (antes, `ubuntu-24.04`, que exigia a 2.39); no macOS, o 13,
  com `MACOSX_DEPLOYMENT_TARGET=13.0` no build.
- O `release.yml` publica os pedaços do bundle na raiz da release (antes eles
  ficavam em `apps/` e o passo das somas falhava) e confere que o Studio
  tem a versão do Lace.
- Windows, achados no primeiro CI com o instalador: a hierarquia (`lace
  hierarchy`, a árvore do Studio) perdia as `\` dos caminhos dos arquivos (o
  `.vvp` do Icarus os guarda sem escapar; o `iverilog` da hierarquia passa a
  recebê-los com `/`); abrir o projeto enquanto outro processo grava o `.spf`
  falhava na troca do arquivo (`Project::open` canonicaliza a pasta e tenta de
  novo por alguns milissegundos, e a gravação também); o `docs/schema` vai em
  LF no checkout. Limitação registrada: no Windows, a simulação encerrada pelo
  `--timeout` perde o que ainda estava no buffer de saída do simulador.

O Lace passa a cobrir o desenvolvimento em Verilog e o de processadores
SAPHO num fluxo só. Um projeto é Verilog; os processadores, quando existem,
são compilados pelo YANC antes de verificar, simular ou sintetizar. Um
projeto só de Verilog não precisa do YANC: a biblioteca SAPHO só entra
quando o projeto tem processadores (antes, entrava sempre que o YANC estava
instalado).

Comandos e opções novos:

- `lace add <arquivo.v>... [--tb]` registra arquivos Verilog e decide pelo
  conteúdo, com a regra da AURORA, se cada um é módulo ou testbench. O
  arquivo que não existe é criado a partir de um modelo, e o testbench-modelo
  já instancia o módulo testado. O primeiro módulo vira o topo; o primeiro
  testbench, o simulado.
- `lace remove <arquivo.v>...` tira do projeto, sem apagar do disco.
- `lace top [arquivo|módulo]` mostra o topo ou o escolhe, por arquivo ou
  por nome de módulo. Qualquer Verilog pode ser o topo, inclusive o
  `Hardware/<proc>.v` que o build de um processador gerou; a exceção é o
  nome de testbench (`tb_<nome>.v`, `<nome>_tb.v`, `tb.v`,
  `verilog::is_testbench_name`), recusado com `invalid_name`. `lace add`
  não escolhe sozinho um arquivo com nome assim como topo.
- `lace check <arquivo>` verifica só os módulos de um arquivo;
  `lace check --lint` acrescenta o `verilator --lint-only -Wall`.
- `lace sim <testbench>` escolhe o testbench e simula.
- `lace wave`, sem argumento, abre a onda da simulação do projeto;
  `lace wave -p <proc>`, a do processador.
- `lace wave` e `lace sim --open` abrem a onda de um processador SAPHO
  com o layout da AURORA: as variáveis do programa, a instrução de assembly
  e a linha do C± de cada ciclo, em grupos (I/O, instruções, variáveis,
  flags). As tabelas do YANC (`trad_opcode.txt`, `trad_cmm.txt`) viram
  tradutores do Surfer em `.lace/Temp/surfer/`, sem tocar na configuração
  do usuário. `--no-layout` abre a onda crua. No Core, `wave_layout` e
  `prepare_wave_layout`.
- O bundle traz o cliente web do surfer-aurora (`surfer-aurora/web/`, da
  mesma tag do executável), para interfaces que mostram a onda numa página;
  `Toolchain::surfer_web_dir` o acha.
- `lace status` mostra o módulo de topo e os `.v` da pasta que não estão
  registrados.
- `lace move <origem>... <destino>` move ou renomeia arquivos e pastas do
  projeto, com os argumentos do `mv`, e mantém o `.spf` em dia: os arquivos
  registrados continuam registrados no lugar novo, na mesma posição, e o
  topo, o testbench escolhido e o módulo de topo acompanham. O `.spf`, a
  `.lace` e as pastas e o fonte dos processadores ficam no lugar; o destino
  que existe não é sobrescrito (`Project::move_path`, erros novos
  `outside_project`, `cannot_move` e `path_exists`).
- `lace hierarchy [-p <proc>]` mostra a árvore de instâncias do design e de
  cada testbench (os registrados e o de cada processador compilado), como o
  Icarus as elabora: parâmetros resolvidos, blocos `generate` expandidos e
  as unidades da biblioteca SAPHO que o processador usa. Uma elaboração que
  falha não esconde as outras. Não compila nem grava relatório
  (`lace_core::hierarchy`, `docs/schema/hierarchy.json`).

Nome de projeto:

- `lace new` (e `Project::create`) só aceita letras sem acento, dígitos,
  `_` e `-`, começando por letra: o nome vira a pasta, o `.spf` e parte do
  caminho de tudo que as ferramentas recebem. Antes aceitava espaço e
  acento. Projetos que já existem, como os da AURORA, continuam abrindo.
  `validate_project_name` é pública, para uma interface avisar enquanto o
  usuário digita.

Comandos e opções que saíram:

| Saiu | No lugar |
|---|---|
| `lace file add` (`--testbench`, `--create`) | `lace add` (`--tb`); o arquivo que não existe é criado |
| `lace file remove` | `lace remove` |
| `lace file top` | `lace top` |
| `lace file testbench` | `lace sim <testbench>` |
| `lace file list`, `lace proc list` | `lace status` |
| `lace input` | escrever `<proc>/Simulation/input_<n>.txt`, um valor por linha |
| `lace output` | `lace sim -p` mostra as saídas; o arquivo continua em `<proc>/Simulation/output_<n>.txt` |
| `lace config` (`show`, `path`, `set-compiler`, `unset-compiler`) e o arquivo de configuração | `--compiler <DIR>` ou a variável `LACE_COMPILER`, em qualquer comando |
| `--config`, `LACE_CONFIG` | nenhum: não há mais arquivo de configuração |
| `proc set --show-arrays` | `proc set --arrays` |
| `build --freq`, `--clocks`, `--show-arrays`; `sim --freq`, `--clocks` | `proc set --freq`, `--clocks`, `--arrays`, que gravam no `.spf` |
| `sim --simulator verilator` | `sim --verilator` |
| `sim --vcd` | a extensão do `$dumpfile` do testbench |
| `sim --jobs` | nenhum: o Verilator usa todos os núcleos |
| `sim --no-build`, `synth --no-build` | nenhum: os processadores são sempre compilados antes |
| `synth --no-widths` | nenhum: o esquemático sempre traz as larguras |
| `wave --view`, `wave --wait` | nenhum |

`lace completions` continua, mas não aparece mais na ajuda.

De qualquer pasta do projeto:

- Todo comando funciona de qualquer pasta do projeto, não só da raiz: o Lace
  procura o `.spf` na pasta atual e nas de cima (`Project::discover`). Fora
  de um projeto, o erro é `project_not_found`.
- Dentro da pasta de um processador (`soma/`, `soma/Software/`,
  `soma/Simulation/`), `build`, `check`, `sim`, `wave`, `synth` e
  `proc set` sem nome valem para ele; no resto do projeto, para o projeto
  inteiro (`proc set` pede o nome). `lace status` marca o processador da
  pasta, e o JSON dele ganha `here`.
- `lace check -p <proc>` verifica só o Verilog do processador e o testbench
  gerado pelo YANC (`CheckOptions::processor`).

Saída em texto:

- Num terminal, uma barra mostra que cada passo está rodando (`Simulating
  (vvp)`, `Compiling the model (verilator)`), com o tempo decorrido. É
  indeterminada, porque os simuladores não informam quanto falta, e só
  aparece depois de 0,3 s.
- O fim de cada operação diz o que ela fez e como terminou (`finished in
  0.09 s`, `failed after 2 ms: cmmcomp exited with code 1`), lista cada
  passo (`done`, `failed`, `not run`) e mostra os artefatos com o papel e o
  caminho: `Generated`, ou, numa falha, `Written before it stopped` e `Not
  generated`. Quando um build falha antes, `check`, `sim` e `synth` dizem que
  a fase seguinte não rodou. Depois da simulação, a dica de abrir a onda.

Testbench dos processadores:

- O build copia o testbench que o YANC gera na pasta temporária para
  `<processador>/Simulation/<processador>_tb.v`, como a AURORA, e a
  simulação roda essa cópia. Antes ele ficava só em
  `.lace/Temp/<processador>/`, e parecia que o YANC não o gerava.

Relatórios:

- Cada `lace build`, `check`, `sim` e `synth` grava um relatório no
  histórico do projeto (`.lace/reports/run-NNNNNN/`): status, comando,
  máquina, ferramentas do bundle com versão e caminho, projeto, tempo de
  cada fase e de cada ferramenta, estatísticas de síntese, artefatos e, numa
  falha, o erro e o fim da saída do passo. É a função de relatório do
  Alpha-Solar, trazida para o Core.
- `lace report` mostra o mais novo; `lace report show <ID>`, um deles;
  `lace report list [--limit N]`, todos; `lace report compare [ID]
  [--against ID] [--summary]` compara as estatísticas de síntese e os tempos
  de simulação de dois, com avisos quando o contexto difere.
- `lace report clean` apaga relatórios: todos, todos menos os `N` mais novos
  (`--keep N`) ou os pedidos pelo identificador. Apagar em massa pergunta
  antes; sem terminal, exige `--yes`. O número de um relatório apagado não
  volta.
- A síntese grava o `stat -json` do Yosys, e o `SynthesisResult` traz as
  estatísticas em `statistics`.
- Todo resultado de operação ganha `duration_ms`, do começo ao fim; o JSON
  de `build`, `check`, `sim` e `synth` ganha `report`, o relatório gravado.

Bundle de Windows:

- No Windows, o Icarus e o Verilator passam a vir do bloco MSYS2 UCRT64 do
  repositório lace-toolchain (o pacote `msys` de `bundle/versions.json`),
  com o g++, o `make` e o Perl que o Verilator usa e o Python com o cocotb.
  O Verilator funciona sem instalar o MSYS2, e o Lace não procura mais o
  MSYS2 em `C:\msys64`; `--compiler` continua trocando o compilador, para
  desenvolvimento. O Yosys continua vindo do OSS CAD Suite. No Linux e no
  macOS nada muda: tudo do OSS CAD Suite, com o compilador do sistema.
- `lace tools` diz se o compilador do Verilator é do bundle ou do sistema;
  no `--json`, `system_compiler.bundled`.
- `lace update` compara o Icarus e o Verilator do Windows com as releases
  do lace-toolchain.

Bundle de Linux e macOS:

- Componente novo, `cocotb`, fora da Recommended: o cocotb do OSS CAD Suite
  com o Python que o roda e o pytest, para testbenches em Python no Icarus
  (exigido) e no Verilator. Acrescenta 115 MiB no Linux e 76 MiB no macOS
  (42 e 25 MiB com o Verilator, que traz o mesmo Python). O Lace ainda não
  tem fluxo de cocotb, e no Windows o componente ainda não existe.
- O empacotamento segue o `$ORIGIN` do RUNPATH das bibliotecas do Linux, e
  aceita exceções ao `exclude` (`keep` em `bundle/components.json`).

Instalar aplicativos do bundle:

- `lace install` instala aplicativos do bundle (Verilator, Yosys, ...) na
  instalação, sem reinstalar o Lace. Sem nomes, abre no terminal a lista,
  com os instalados travados; `lace install verilator` instala direto. Só
  os pedaços dos aplicativos novos são baixados e extraídos em `toolchain/`,
  e uma falha no meio desfaz o que entrou.
- A release passa a publicar o bundle em pedaços
  (`lace-<versão>-<plataforma>-<pedaço>`, com o índice), conferidos pelo
  `SHA256SUMS`: é de lá que o `lace install` baixa. `--from` usa um
  instalador no disco; `LACE_RELEASE_URL`, um espelho.
- `lace-pack tui --assets <DIR>` grava esses pedaços com os nomes da
  release.

Atualizar:

- `lace update` compara o Lace, o bundle e cada aplicativo instalado com
  a última release e com a última versão upstream, e marca o que é mais
  novo; `--check` só mostra. Com um Lace mais novo, pergunta (`--yes` não
  pergunta) e reinstala pelo instalador da release, conferido pelo
  `SHA256SUMS`, com os mesmos aplicativos, a mesma pasta e o mesmo atalho; no
  Windows, abre o assistente. Ferramenta mais nova upstream não é instalada
  direto: chega num bundle novo, numa release nova do Lace.

Desinstalar:

- `lace uninstall` remove a instalação de onde ele roda, com o bundle
  inteiro: no Linux e no macOS roda o `uninstall.sh` da pasta; no Windows
  abre o desinstalador. Pergunta antes; `--yes` não pergunta. Apaga também o
  arquivo de configuração da 0.1.0.
- O `uninstall.sh` apaga também as sobras de uma instalação interrompida
  (`.instalando-*`, `.antigo-*`), que antes deixavam a pasta para trás.

Cancelamento, prazo e saída ao vivo:

- O Ctrl+C (e o SIGTERM, e no Unix o SIGHUP) cancela a operação: o Lace
  encerra a ferramenta que roda, com tudo o que ela iniciou, mostra o que
  chegou a rodar e sai com o código 130. Um segundo Ctrl+C sai na hora.
  Antes, o sinal matava só o Lace, e um `vvp` de testbench sem `$finish`
  continuava rodando sozinho.
- `lace sim --timeout <SEGUNDOS>` encerra a simulação depois do prazo
  (código de saída 1, status `timed_out`), com a dica do `$finish`.
- Em texto, o `$display` do testbench sai enquanto a simulação roda, e não
  mais só no fim. Com `-v`, saem o comando e toda a saída de cada passo,
  também enquanto rodam.
- `--events`, opção global: o stdout tem um objeto JSON por linha, um evento
  por passo que começa, por linha que as ferramentas escrevem e por passo
  que termina, e na última linha `{"event": "result", "result": ...}` com o
  mesmo objeto do `--json`.

Contrato do `--json`:

- O JSON Schema da saída de cada comando está em `docs/schema/`, gerado dos
  tipos e conferido por teste contra a saída real.
- `lace proc add` escreve o processador criado, o mesmo objeto de
  `lace proc set`; antes, `{message, path}`.
- `lace sim` escreve sempre `outputs` e `surfer_pid`, também quando um
  processador não compila.
- Em `lace tools`, as ferramentas de `tools` saem em ordem alfabética.

Comportamento:

- `check` elabora todas as raízes do design, sem `-s`: um módulo que ninguém
  instancia também é verificado, e o projeto não precisa ter topo. Depois,
  elabora cada testbench registrado com o design. Antes, só elaborava a
  partir do topo e deixava os testbenches de fora.
- `check` e `synth` compilam antes os processadores, como o `sim` já fazia.
- `sim` sempre mostra a saída do testbench (`$display`, `$monitor`); antes,
  só com `-v`.
- `sim` falha quando o testbench chama `$error` ou `$fatal`. O `vvp` sai com
  0 nesse caso, e antes isso passava como sucesso. O `vvp` roda com `-n`:
  `$stop` termina a simulação.
- `sim -p` mostra os valores de cada porta de saída e diz qual arquivo de
  entrada criar quando falta um.
- Num testbench sem `$dumpfile`, o Lace injeta `$dumpvars(0, <tb>)`: a onda
  traz todos os sinais, inclusive os do módulo testado. Antes, o nível 1 só
  trazia os do testbench.
- O formato da onda segue a extensão do `$dumpfile`. Antes, o Icarus gravava
  FST mesmo num arquivo `.vcd`; agora a onda de um processador
  (`<nome>_tb.vcd`) é VCD, e a injetada é `<tb>.fst` no Icarus e `<tb>.vcd`
  no Verilator.
- O módulo do testbench vem do conteúdo do arquivo, não do nome.
- Caminhos na linha de comando são relativos ao diretório atual do shell.
  Antes, os de `lace file` eram relativos à raiz do projeto, e um caminho
  relativo em `lace wave` não abria.
- `build` num projeto sem processadores avisa e sai com 0; antes, era erro.
- O log do Surfer sai de perto da onda: vai para `.lace/Temp/surfer/` do
  projeto (`<onda>.log`) ou, com a onda fora de projeto, para a pasta de
  cache do usuário. Antes, `<onda>.surfer.log` ficava ao lado da onda, na
  raiz do projeto no caso da onda injetada.
- Os erros de projeto sem topo, sem testbench ou vazio vêm com o comando que
  resolve.

Saída em inglês:

- O que o `lace`, o instalador, o `install.sh` e o `install.ps1` mostram
  passa a ser em inglês, e toda mensagem começa com maiúscula. `erro:` e
  `aviso:` viram `Error:` e `Warning:`; os erros de argumento do `clap` saem
  do mesmo jeito. Nos diagnósticos, a gravidade continua em minúscula, como
  no gcc (`arquivo:linha: error: ...`), porque é o que os editores
  reconhecem.
- As confirmações de `lace update` e `lace uninstall` pedem `[y/N]` e
  aceitam `y` ou `yes`.
- No `--json` e no `--events`, os textos de `message` e `hint` mudaram
  junto; os `code`s e as chaves continuam os mesmos. As mensagens de
  `LaceError` no `lace-core` também passam a ser em inglês.
- `lace --help` ficou mais curto e em inglês, junta as opções globais num
  grupo só e não mostra mais o `--toolchain`, que é de desenvolvimento.
- Os tipos de instalação se chamam Recommended e Advanced. No assistente do
  Windows, `/TYPE=recomendada` e `/TYPE=avancada` continuam valendo.

`lace-core`:

- `Control`, `CancelToken`, `Event` e `Stream`. Toda operação que
  executa ferramentas recebe `&Control` como último argumento: `build`,
  `build_processors` (antes do `on_result`), `check`, `simulate`,
  `simulate_project`, `synthesize` e `render_schematic`. Com
  `&Control::default()`, nada muda.
- `Status::Cancelled` e `Status::TimedOut`; `Termination::Cancelled` e
  `Termination::TimedOut`. `SimulationOptions::timeout`.
- Cada passo roda num grupo de processos próprio (Unix); cancelar encerra o
  grupo, ou a árvore com o `taskkill` no Windows.
- Com quem receba os eventos, o `vvp` roda com `-i` (stdout sem buffer). O
  Verilator compila o modelo com `--autoflush`: como a linha de comando
  mudou, a primeira simulação depois da atualização pode compilar o modelo
  de novo.
- Os resultados derivam `schemars::JsonSchema`.
- `check(toolchain, project, &CheckOptions)` substitui `check_syntax`;
  `CheckResult` ganhou `targets`, e `Step` ganhou `Lint`.
- `SimulationOptions` perdeu o campo `fst`; `waveform_path` diz onde a
  simulação grava a onda.
- Módulo `lace_core::verilog`: `classify`, `modules_in`, `read_interfaces`,
  `module_template`, `testbench_template`.
- `Project::add_verilog`, `remove_verilog`, `set_top`, `top_module`,
  `testbench_module`, `unregistered_verilog`, e o tipo `AddedFile`.
- `LaceError::EmptyProject` (`empty_project`) e `ModuleNotFound`
  (`module_not_found`). As mensagens de `NoTopLevel` e `NoTestbench` não
  citam mais comandos.

Correções do teste de fogo (2026-10-04):

- Processador num caminho com acento ("Área de Trabalho") simulava "com
  sucesso" sem programa nem entradas, porque o `vvp` não abre nome de
  arquivo com acento. Agora `sim` recusa antes com `non_ascii_path`, e o
  aviso do `vvp` ("contains non-printable characters") vira erro.
- O esquemático falhava em qualquer projeto com acento no caminho: o
  `write_json` do Yosys estraga os bytes fora do ASCII, e o Lace agora os
  conserta no `hierarchy.json` depois da síntese.
- O esquemático de um módulo com muitas ligações deixava o `dot` rodando por
  minutos e consumindo centenas de MB. Agora `synth --svg` recusa acima de
  120 ligações (`schematic_too_large`, `--no-schematic-limit` desliga) e o
  `dot` tem prazo de 60 s (`SchematicOptions::max_connections` e `timeout`).
- Duas gravações simultâneas do `.spf` (o Studio e a CLI, dois `lace add`)
  perdiam alterações. Cada mudança agora trava `.lace/spf.lock`, relê o
  `.spf` e grava com um temporário de nome único.
- A simulação do projeto sobrescrevia, na raiz, um arquivo do usuário com o
  mesmo nome de um arquivo de dados do testbench, e com `../` chegava a
  escrever fora da pasta do projeto. Agora não sai da raiz nem sobrescreve o
  que não foi copiado pelo próprio Lace (`.lace/Temp/data_copies.txt`); o
  que não foi copiado vira aviso.
- `synth` de arquivo `.sv` falhava: o Yosys agora lê com `-sv`.
- Um testbench sem `$dumpfile` que chega ao `$finish` no tempo 0 era
  reprovado por falta da onda injetada; a onda injetada não é mais exigida.
  O dump entra no `endmodule` do módulo do testbench (antes, no último do
  arquivo, inclusive dentro de comentário), e os diagnósticos apontam para o
  testbench do usuário, não para a cópia em `.lace/Temp`.
- Uma saída de processador com `x` (divisão por zero) derrubava `sim -p` com
  código 2, sem resumo nem relatório. Agora a porta sai como "unreadable",
  com o motivo em `error` no JSON, e o resto continua.
- `include` e `$readmem` relativos: o Icarus procura o `include` na pasta do
  arquivo e depois na raiz (`-grelative-include -I <raiz>`), o Yosys recebe
  `-I <raiz>` e roda com CWD na raiz, como a simulação. O mesmo projeto passa
  a valer em `check`, `sim`, `hierarchy` e `synth`.
- A versão mínima do Rust sobe para 1.89 (`File::lock` da biblioteca padrão).

Correções médias do teste de fogo (2026-10-04):

- Caminhos no `.spf`. Um arquivo de fora da pasta
  do projeto é gravado com `..` quando está no mesmo repositório git (o
  `rtl/` do HITS), e absoluto quando não. Um absoluto de outra máquina
  (`C:\Users\...`) é procurado pela cauda dentro da pasta, como a AURORA, e
  a próxima gravação conserta o `.spf`.
- `Project::issues()` (e `issues` no `lace status --json`): avisos do `.spf`
  que não impedem de abrir, como caminho achado pela cauda, `topLevelFile`
  fora da lista e processador com nome que não compila.
- O `.spf` com tipo errado nos campos de arquivos (`synthesizableFiles`
  objeto, `topLevelFile: 42`), com `structure` que não é objeto, ou com
  processador de nome vazio, com barra ou repetido agora é
  `invalid_project_file`, com o campo; antes era ignorado e apagado na
  gravação seguinte, ou tomava a raiz como pasta do processador.
- `topLevelFile` ou `testbenchFile` fora da lista, ou topo com nome de
  testbench, é ignorado por todas as operações (antes `status`, `top`,
  `synth` e `check` discordavam).
- Só `.v` e `.sv` entram nas listas: `lace top README.md` e
  `lace sim notas.txt` recusam sem gravar nada. `lace add` com vários
  arquivos confere todos antes de registrar o primeiro
  (`Project::check_add_verilog`). Um arquivo só de `` `define `` não vira
  topo.
- `lace top MÓDULO` com o módulo em dois arquivos é `ambiguous_module`, com
  os arquivos; a lista de `module_not_found` não repete nomes.
- `proc add` recusa nome que não compila (palavras do C± e do Verilog,
  módulos da biblioteca SAPHO, mais de 64 caracteres;
  `validate_processor_name`) e parâmetro fora do que o YANC compila
  (`invalid_parameter`, `NewProcessor::validate`). `--nubits` e os outros
  de C± com `--lang cpp` recusam; só `--nbmant` ou `--nbexpo` ajusta o
  `--nubits`. `proc set` recusa frequência acima de 500000 MHz e clocks
  acima de 2147483647 com o motivo (antes o erro vinha depois, do `asmcomp`
  ou do Icarus). Um processador do `.spf` com nome que não compila (`x-y`)
  abre com aviso, e o `build` recusa com `invalid_name` antes do YANC.
- A verificação acha as raízes num passe por arquivo (1500 arquivos: de
  1 s para milissegundos), e o título diz quantas são e nomeia cinco. Com
  recursão parametrizada, `check` e `hierarchy` passam as raízes com `-s`
  (antes: "No top level modules").
- Testbench com dois módulos e nenhum com o nome do arquivo usa o único que
  o outro não instancia, em `check`, `sim` e `hierarchy`.
- Macro indefinida no ponto de uso é erro no `check` e na simulação, com a
  explicação da ordem dos arquivos; antes passava com a largura errada.
- O parser do Yosys reconhece `ERROR:` sem espaço e o aviso de latch
  (antes, `unknown` sem arquivo). O erro `Error in function f: where's the
  return...` do `cmmcomp` vira erro com o fonte.
- `lace hierarchy` marca cada passo pelo próprio resultado (um testbench que
  elaborou não aparece mais como `failed`).
- `check --lint` sem o Verilator roda a verificação e avisa que o lint não
  rodou; a CLI sugere `lace install <componente>` para todo
  `component_missing` (o Core não cita mais o instalador).
- `synth --svg --module X` com X fora da árvore do topo grava o relatório e
  sai com `module_not_found` e os módulos sintetizados (antes
  `invalid_netlist`, sem relatório). `schematic_too_large` também grava.
- Simulação: `$dumpfile(ONDA)` com `localparam`, `parameter` ou `` `define ``
  usa o nome deles; com uma expressão, o Lace não injeta outro dump (antes o
  `-fst` dele gravava FST no `.vcd` do usuário). `$dumpfile` sem `$dumpvars`
  dá aviso.
- Simulação de processador: uma entrada que não é inteiro recusa
  (`invalid_data_file`); arquivo vazio ou valor que não cabe em `#NUBITS`
  dá aviso. Programa que não chega ao fim nos clocks dá aviso para subir os
  clocks.
- Build e simulação travam o processador (`.lace/Temp/<nome>.lock`) e a
  simulação do projeto trava o projeto: duas ao mesmo tempo, do Studio e da
  CLI, não se quebram mais; a segunda recusa com `operation_in_progress`.
- A saída guardada de cada passo tem teto: 2 MiB do começo e 2 MiB do fim
  (um testbench que imprime sem parar crescia sem limite na memória).
- O histórico usa nomes únicos por operação também dentro do mesmo processo.
- Layout da onda: os rótulos de I/O usam o número da porta (`in_sim_2` é
  `input 2`); tabelas mais novas que a onda não são aplicadas
  (`WaveProcessor::outdated`, e a CLI avisa para simular de novo); onda sem
  processador SAPHO ganha o grupo Top-level com os sinais do testbench; a
  varredura dos complexos lê em bytes, e o teto de 16384 valores está
  documentado.

Correções baixas do teste de fogo (2026-10-04):

- O mesmo arquivo repetido numa lista do `.spf` (também por outro caminho)
  ou nas duas conta uma vez, com aviso (`duplicate_file`); a próxima
  gravação tira as entradas que sobram. O `isMarkedTestbench` legado da
  AURORA escolhe o testbench, como nela.
- `Project::reorder_file` e `lace order`: muda a posição de um arquivo na
  lista (a ordem dos compiladores, que decide quem vê cada `` `define ``).
- `lace new`: nome até 64 caracteres (com 248, a pasta ficava vazia para
  trás); nome que só difere na caixa de outro da mesma pasta recusa; projeto
  dentro da pasta de outro avisa (`nested_project`), e o de fora não lista
  mais os arquivos do de dentro como não registrados.
- `lace move` não sobrescreve um link quebrado no destino, renomeia só a
  caixa num sistema que não distingue maiúsculas, e uma origem que não
  existe é `invalid_project` (antes, `io`).
- A CLI não repete a causa de um erro de I/O ("No such file or directory"
  duas vezes).
- Texto: síntese que falhou não diz mais "(0 modules)"; o prazo aparece
  como `stopped at the 3 s limit`; a continuação `: Padding ...` do Icarus
  entra no aviso anterior; `*** These modules were missing` vira um resumo
  só; na tabela de ferramentas do relatório, o passo que estourou o prazo
  aparece como `TIMEOUT`, e não `FAIL`.
- Simulação: `$display("ERROR: ...")` do testbench fica como saída dele e
  não reprova (só `$error` e `$fatal`); a linha `Time: ... Scope: ...` do
  `$error` entra na mensagem; o programa que lê mais valores do que um
  `input_<n>.txt` tem gera aviso com o arquivo e a linha; VCD acima de
  100 MB sugere FST.
- Síntese: o `check` do Yosys aponta laço combinacional e drivers em
  conflito; módulo vazio (caixa-preta) sai da lista de módulos do
  esquemático; um módulo que se instancia sem condição falha antes de rodar
  o Yosys (que rodava sem fim), e uma queda do Yosys com recursão vem
  explicada.
- `lace report compare`: a referência automática prefere uma execução que
  terminou bem; relatórios de um projeto movido continuam se comparando;
  dois sem parte em comum dão `not_comparable`.
- Layout da onda: processador em C rotulado `C`; array com mais de 9999
  elementos num grupo só, com o índice certo; função com `_v_` no nome
  desfeita pelo `cmm_log.txt`; escopo escapado com `.` num componente só;
  `delta_*` real sem formato de bits; uma pasta de layout por onda (duas
  `pa_tb.vcd` não dividem mais a pasta).

O que o teste de fogo achou e ainda não foi corrigido está em
`docs/PENDENCIAS.md`.

## 0.1.0 (2026-09-29)

Primeira versão. Substitui a orquestração da AURORA para o SAPHO.

- `lace-core`: API para criar projetos e processadores (formato `.spf` da
  AURORA), compilar com o YANC (C± e C), simular com Icarus Verilog e
  Verilator, verificar sintaxe e sintetizar com Yosys, desenhar o esquemático
  com o `show` do Yosys e o Graphviz, e abrir ondas no surfer-aurora.
- `lace`: a linha de comando sobre o `lace-core`, com saída em texto ou
  JSON.
- Bundle de ferramentas `2026.09.29`: OSS CAD Suite de 2026-09-29, YANC v5.6,
  surfer-aurora v0.7.0-nips.10 e, no Windows, Graphviz 16.1.0. O Lace só
  executa ferramentas desse bundle; a exceção declarada é o compilador C++, o
  `make` e o Perl do sistema, para o Verilator.
- O OSS CAD Suite é dividido por ferramenta: o bundle completo de Linux tem
  300 MiB, contra os 2,5 GB do pacote inteiro.
- Instaladores: assistente (Inno Setup) no Windows, instalação guiada no
  terminal no Linux e no macOS. Tipo Recomendada por padrão; Avançada para
  escolher os componentes.

Plataformas: Linux x64, macOS Apple Silicon, Windows 10 e 11 x64.
