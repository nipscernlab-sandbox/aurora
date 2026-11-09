# Referência do IPC

O que a interface chama no backend com `invoke`, com o que recebe e o que
devolve. São duas coisas:

- os comandos do Studio, um por linha das tabelas abaixo: as janelas, os
  arquivos, o terminal, a vigia da pasta, as preferências, a onda numa aba e
  as operações longas;
- o `aurora_call`, que leva um método do protocolo da AURORA ao servidor do
  AURORA (`aurora serve`, aberto pelo Studio como processo filho) e devolve a
  resposta dele. O projeto, os processadores, o histórico, os sinais da
  onda, a placa FPGA, os exercícios e o controle de versão passam por aqui.

Na interface, cada coisa tem uma função em `src/ipc/api.ts`, menos
`log_frontend` e `dev_smoke_script`, chamados com `invoke` direto; os tipos
estão em `src/ipc/types.ts` (os do Studio) e `src/ipc/aurora-types.ts` (os do
protocolo, gerados do schema da AURORA).

Convenções:

- Os argumentos de primeiro nível de um comando vão em `camelCase` no
  JavaScript e chegam em `snake_case` no Rust (`expectedModifiedMs` vira
  `expected_modified_ms`). Os campos de um objeto passado inteiro
  (`request`, `settings`) e os parâmetros de um método do protocolo vão em
  `snake_case`, como estão nos tipos.
- Caminhos são absolutos, do sistema (com `\` no Windows).
- Tudo pode falhar com `IpcError { code, message, data? }`. Os códigos da
  AURORA são os de `AuroraError::code()` (API.md da AURORA, seção 8) e os do
  servidor; os do Studio estão em `src-tauri/src/error.rs` e na última seção
  daqui. `data` vem só de um erro do servidor que tem campos (os `data` do
  `ErrorInfo`): os arquivos de `git_switch_blocked`, o stderr de
  `git_failed`.
- Os comandos que tocam o disco, rodam ferramenta ou falam com o servidor
  rodam numa thread de bloqueio (`state::blocking`), fora da thread da
  interface; nenhum comando trava a janela.

## O servidor da AURORA (`server.rs`)

| Comando | Argumentos | Devolve | Equivale a |
|---|---|---|---|
| `aurora_call` | `method` (`project.status`, `history.show`, ...), `params?` | o resultado do método | um pedido ao `aurora serve` |

Cada método, com os parâmetros, o resultado e os tipos, está em
`docs/PROTOCOL.md` da AURORA, gerado do código. O `aurora_call` não aceita os
métodos da sessão (`initialize`, `shutdown`) nem os longos, que têm
progresso: `flow.run`, `learn.check`, `update.check`, `update.apply` e
`install.add` vão pelas operações e pelos comandos do bundle.

O projeto aberto vai sozinho: um método que tem o campo `project` e o
pedido não o traz recebe o `.spf` aberto. Sem projeto aberto, um método que
exige `project` é `no_project`; um que o tem opcional (o `wave.open`) vai
sem ele. Quem não quer o projeto num campo opcional manda `project: null`.

O servidor abre no primeiro pedido e reabre se cair. O `aurora` é o de
`AURORA_IDE_CLI`, o da preferência `aurora_path`, o da instalação do bundle
das preferências, o da instalação em que o Studio está e o do `PATH`, nessa
ordem; ele recebe o bundle (`--toolchain`), o compilador do Verilator
(`--compiler`), o Quartus (`--quartus`) e as trilhas do `aurora learn`
(`AURORA_LEARN_DIR`) das preferências, e mudar uma dessas coisas fecha o
servidor e abre outro no pedido seguinte. Um `aurora` de outra versão do
protocolo é recusado (`aurora_incompatible`). No Windows o servidor roda sem
janela de console, num Job Object que o leva junto quando o Studio fecha.

As funções do `api.ts` que passam pelo `aurora_call` mantêm a forma que a
interface já usava:

| Função | Método | O que muda na ida ou na volta |
|---|---|---|
| `project.snapshot` | `project.status` | |
| `project.addVerilog(path, testbench)` | `project.add` | um arquivo, numa lista de um; volta o `AddedFile` dele |
| `project.removeVerilog(path)` | `project.remove` | volta `true` se o arquivo estava no projeto |
| `project.setTop(target)` | `project.top` | volta o arquivo de topo do `TopOutcome` |
| `project.setTestbench(path)` | `project.testbench` | volta o retrato do projeto |
| `project.reorder(path, position)` | `project.order` | `path` vai em `file` |
| `project.move(from, to, overwrite)` | `project.move` | |
| `project.convertLayout` | `project.convert` | |
| `project.checkName`, `project.checkProcessorName` | `project.check_name`, `processor.check_name` | o `NameCheck` vira `null` (o nome serve) ou o `IpcError` |
| `project.processorDefaults` | `processor.defaults` | |
| `project.addProcessor(request)` | `processor.add` | o pedido vai em `spec` |
| `project.configureProcessor`, `removeProcessor`, `outputValues` | `processor.set`, `processor.remove`, `processor.outputs` | |
| `project.verilogModules(text)` | `verilog.modules` | |
| `app.openWave(processor?, path?)` | `wave.open` | um arquivo vai com `project: null`, para não esperar na fila do projeto aberto |
| `wave.signals`, `wave.resetLayout` | `wave.signals`, `wave.reset_layout` | |
| `wave.setSelection(signals)` | `wave.choose` com `{ kind: 'set', paths }` | volta a escolha como ficou |
| `history.*` | `history.list`, `show`, `compare`, `plan_cleanup`, `clean` | a limpeza da interface (`{ kind, keep }`, `{ kind, ids }`) vai no formato do protocolo |
| `experimental.*` | `experimental.list`, `experimental.set` | |
| `fpga.*` | `fpga.boards`, `config`, `config_set`, `top`, `check`, `modules`, `status`, `cables` | |
| `toolchain.verify` | `toolchain.verify` | |
| `learn.tracks`, `open`, `init`, `setCurrent`, `reset` | `learn.tracks`, `status`, `init`, `set_current`, `reset` | |
| `git.status`, `readFile(revision, path)`, `log(skip, limit)`, `commitFiles(oid)`, `branches` | `git.status`, `git.read_file`, `git.log`, `git.commit_files`, `git.branches` | |
| `git.commit(summary, description, paths)` | `git.commit` | `paths`: exatamente os arquivos marcados na vista |
| `git.discard(paths)`, `init`, `setIdentity(name, email, global)`, `setRemote(name, url)`, `removeRemote(name)` | `git.discard`, `git.init`, `git.set_identity`, `git.set_remote`, `git.remove_remote` | |
| `git.switchTo(branch, create)` | `git.switch` | |
| `git.fetch`, `pull`, `push` | `git.fetch`, `git.pull`, `git.push` | |
| `github.account`, `loginStart`, `loginPoll(deviceCode)`, `logout` | `github.account`, `github.login_start`, `github.login_poll`, `github.logout` | `deviceCode` vai em `device_code`; sem `project` |

O que vale saber de alguns métodos, do lado da interface:

- `wave.open` abre em janela e aplica o layout dos processadores SAPHO,
  qualquer que seja `Settings.wave_viewer`; a onda de um exercício do
  `aurora learn` abre com o layout da correção. Uma onda que ainda não existe
  é `no_waveform`. Espera 1,5 s para conferir que o surfer-aurora não fechou
  logo (`process_exited_early`).
- `project.move` recusa o `.spf`, `.aurora8/`, as pastas e o fonte dos
  processadores e uma pasta para dentro dela mesma (`cannot_move`), caminho
  fora do projeto (`outside_project`) e destino que existe (`path_exists`).
  Com `overwrite`, o destino vai para a lixeira e o movimento é tentado de
  novo, menos o que a AURORA mantém no lugar, a pasta do projeto e uma pasta
  que contém a origem (`cannot_replace`, e nada vai para a lixeira).
- `wave.choose` com `set` confere os caminhos na árvore do testbench
  elaborado pelo Icarus, como `wave.signals`; a lista vazia apaga a escolha
  sem elaborar. O retrato do projeto traz a escolha em `wave_selection` sem
  elaborar nada: é o que os botões Sinais mostram.
- A limpeza do histórico é em duas chamadas: a interface pede o plano
  (`history.plan_cleanup`), mostra a lista, confirma e manda apagar
  exatamente aquela lista (`history.clean`). Um relatório gravado entre as
  duas não sai sem ter sido mostrado.
- Os recursos experimentais ficam no `experimental.json` da pasta de
  configuração da AURORA, e não no `settings.json` do Studio: ligar aqui vale
  também para a CLI, e o contrário. A placa FPGA é um deles: desligada, os
  métodos `fpga.*` respondem `experimental_disabled`.
- O controle de versão (`git.*` e `github.*`) é o recurso experimental `git`:
  desligado, os métodos respondem `experimental_disabled`. A vista Controle
  de versão (`state/git.ts`) relê o `git.status` quando o vigia avisa de uma
  mudança (também do `.git`, ver Eventos), quando a janela volta a ter o
  foco, depois de cada operação dela e ao abrir ou fechar o projeto. Os
  erros que ela trata do jeito dela: `git_no_remote` abre o campo do
  endereço do remoto, `git_switch_blocked` mostra os arquivos (`data.files`)
  no seletor de ramo, `git_authentication` avisa com o botão que mostra a
  vista Perfil e `git_push_rejected`, com o botão Trazer. O login no GitHub é
  pelo código de dispositivo: `github.login_start` e, a cada `interval`
  segundos, `github.login_poll`, até `done`, `expired` ou `denied`. Os de
  leitura (`git.status`, `read_file`, `log`, `commit_files`, `branches`,
  `github.account`) rodam fora da fila do projeto; os que mudam o
  repositório, como o `git.push`, esperam na fila, atrás de uma operação
  longa do mesmo projeto.
- As trilhas do `aurora learn` vêm da preferência `learn_dir`, de
  `AURORA_LEARN_DIR` ou do componente `aurora-learn`, nessa ordem; sem nenhum,
  o erro é `learn_component_missing`. Com `lang` (`en`), os textos que têm
  tradução vêm traduzidos.

## Aplicativo (`commands/app.rs`)

| Comando | Argumentos | Devolve |
|---|---|---|
| `app_info` | | `AppInfo`: nome, versão, sistema, a plataforma do bundle da AURORA (pelo servidor; `null` sem ele), pasta das preferências, versões do Tauri e do motor de páginas, se é build de desenvolvimento |
| `settings_get` | | `Settings` |
| `settings_set` | `settings: Settings` | `Settings` como ficou (a lista de recentes enviada é ignorada) |
| `recent_projects` | | `RecentStatus[]`: `spf`, `name`, `opened_at_ms`, `exists` |
| `recent_forget` | `spf` | |
| `log_frontend` | `level` (`error`, `warn`, `info`), `message` | |
| `dev_smoke_script` | | o roteiro de `AURORA_IDE_SMOKE`, só na build de desenvolvimento; senão `null` |
| `app_exit` | `restart: boolean` | (não volta: o Studio sai) |
| `splash_close` | | (fecha o splash, a janela com o logo que fica na frente enquanto a principal abre escondida; a interface chama logo depois de mostrar a principal) |

`log_frontend` escreve no log do backend, com o alvo `aurora_ide::ui`, o
que a interface manda: erros de JavaScript, `console.error` e todo erro
mostrado num aviso. `dev_smoke_script` alimenta o roteiro de fumaça
(DEVELOPMENT.md, seção 3).

`app_exit` sai do Studio pelo caminho de sempre (a operação rodando é
cancelada, os terminais fecham e o servidor da AURORA recebe `shutdown`) e,
com `restart`, abre de novo o executável com que ele começou, guardado ao
abrir: depois do `aurora update`, é o da versão nova. A interface grava os
arquivos e a sessão antes de chamar. Sem `restart`, só fecha: é o que o
Studio faz no Windows para o assistente de instalação trocar os arquivos
dele.

`Settings.aurora_path` é o `aurora` que o Studio abre como servidor; vazio, o
da instalação. Para desenvolvimento, um build em `target/`.

`Settings.wave_viewer` é `tab` (o padrão) ou `window`: onde a interface
abre a onda. Com `tab` e o cliente web no bundle, a simulação com
`open_wave` não abre a janela: devolve `wave_tab` e a interface abre a aba
(seção "Onda numa aba").

`Settings.terminal_shell` é `powershell` (o padrão) ou `cmd`: o shell do
terminal no Windows. Nos outros sistemas o terminal abre `$SHELL` e o campo
não vale.

`Settings.editor` tem `font_size`, `tab_size`, `word_wrap`, `minimap`,
`auto_save` e `vim_mode`. Campo que falta no arquivo fica com o padrão.

`Settings.zen` tem `fullscreen` (padrão `true`), `center_layout` (`true`),
`show_tabs` (`false`) e `hide_line_numbers` (`false`): as opções do modo
zen. Entrar e sair do zen não passa pelo backend; a tela cheia vem do
`setFullscreen` da janela, pela permissão `core:window:allow-set-fullscreen`.

`Settings.layouts` tem `active` (o id do layout em uso; padrão `default`, o
Padrão) e `saved` (os layouts gravados pelo usuário). Cada item de `saved`
é guardado como a interface o escreveu (`src/state/layoutModel.ts`, com a
versão do formato em `v`) e o backend não o lê: um layout malformado ou de
uma versão mais nova não leva o `settings.json` para o `.bad`, e gravar
outra preferência o devolve como veio. Ao contrário dos recentes, os
layouts enviados em `settings_set` são gravados. A janela de agora (o que o
usuário mexeu depois de aplicar um layout) não passa pelo backend: fica no
`localStorage` da WebView.

## Projeto (`commands/project.rs`)

| Comando | Argumentos | Devolve | Equivale a |
|---|---|---|---|
| `project_default_parent` | | a pasta padrão para criar e abrir projetos (`Projetos` na pasta do usuário), criada se não existe | |
| `project_open` | `path`: o `.spf` ou a pasta | `ProjectSnapshot` | o `project.status` do servidor |
| `project_create` | `parent`, `name` | `ProjectSnapshot`; `invalid_name` se o nome não passar na regra da AURORA (letras sem acento, dígitos, `_` e `-`, começando por letra) | `aurora new`, o `project.create` do servidor |
| `project_close` | | | |
| `project_hierarchy` | | `HierarchyResult` | `aurora hierarchy --json`, o fluxo `hierarchy` do servidor |

Abrir e criar tornam o projeto o aberto: guardam o `.spf`, põem nos
recentes e começam a vigiar a pasta.

`project_hierarchy` é o fluxo `hierarchy` do servidor, sem saída ao vivo: o
design, cada testbench registrado e o testbench de cada processador
compilado, elaborados pelo Icarus. Não compila os processadores nem grava
relatório. Os tipos (`HierarchyResult`, `Elaboration`, `ModuleInstance`)
estão em `src/ipc/aurora-types.ts`; a descrição de cada campo, na API.md do
AURORA, seção 5.4.1.

`ProjectSnapshot` é o `ProjectStatus` do protocolo (`Project::status` do
AURORA), de onde a CLI monta o `StatusReport` do `aurora status --json`:

| Campo | Conteúdo |
|---|---|
| `name`, `spf`, `root` | o projeto |
| `synthesizable`, `testbenches` | `ProjectFile[]`: `role`, `path`, `top_level` |
| `top_level`, `top_module`, `top_module_error` | o arquivo e o módulo de topo; o erro quando o módulo não dá para saber |
| `selected_testbench`, `testbench_module` | o testbench simulado e o módulo dele |
| `wave_selection` | a escolha de sinais da onda desse testbench (`wave/<testbench>.json`); vazia, a onda grava todos. Os botões Sinais mostram quantos itens ela tem |
| `unregistered` | os `.v` e `.sv` da pasta fora do `.spf`, e os `.py` com `@cocotb.test` |
| `processors` | `ProcessorStatus[]`: os campos do `Processor` da AURORA, mais `built`, `inputs` e `outputs` (`{ port, path }`), `missing_inputs`, `waveform` e `generated` (o que o build gerou e existe: `verilog`, `testbench`, `assembly`, `memories[]`, `intermediates[]`, os `.txt` e `.log` da pasta de saída `<proc>/<proc>/`, sem as saídas da simulação) |
| `waveform` | a onda da simulação do projeto, se já existe |
| `top_candidates` | os Verilog que podem ser o topo pela regra da AURORA (qualquer um, menos nome de testbench e testbench cocotb `.py`): registrados, o gerado de cada processador e os de fora do `.spf` |
| `issues` | `ProjectIssue[]` da AURORA (`Project::issues`): `kind` (`rescued_path`, `selection_not_registered`, `testbench_as_top`, `invalid_processor_name`, `duplicate_file`, `nested_project`, `legacy_layout`), `path`, `detail`, `message`; o painel Problemas mostra como aviso, traduzido pelo `kind`. Com `legacy_layout`, o Studio oferece a conversão ao abrir o projeto |

`NewProcessorRequest` (o `spec` do `processor.add`): `name`, `language`
(`cmm` ou `cpp`) e, opcionais, `input_ports`, `output_ports`, `nubits`,
`nbmant`, `nbexpo`, `nugain`, `ndstac`, `sdepth`. Ausente fica com o padrão
da AURORA (`processor.defaults`), e, sem `nubits`, com `nbmant` ou `nbexpo`
dados, `nubits` vira `nbmant + nbexpo + 1`; `ndstac` e `sdepth` ausentes
ficam fora do fonte, e o YANC calcula as pilhas pelo programa. A AURORA
recusa nome e parâmetro que o YANC não compila (`invalid_name`,
`invalid_parameter`).

## Arquivos (`commands/files.rs`)

| Comando | Argumentos | Devolve |
|---|---|---|
| `fs_read_dir` | `path` | `DirEntry[]`: `name`, `path`, `is_dir`, `hidden`, `testbench_name` (nome `tb_<nome>.v` ou `<nome>_tb.v`, que não pode ser topo, pelo `verilog.testbench_names` do servidor; sem servidor, `false`); pastas primeiro |
| `fs_read_text` | `path` | `TextFile`: `content`, `modified_ms`, `binary`, `too_large` (mais de 16 MiB) |
| `fs_write_text` | `path`, `content`, `expectedModifiedMs?` | o novo `modified_ms` |
| `fs_stat` | `path` | `FileStat` (`is_dir`, `modified_ms`, `size`) ou `null` |
| `fs_create_file` | `path`, `content?` | |
| `fs_create_dir` | `path` | |
| `fs_rename` | `from`, `to` | |
| `fs_copy` | `from` (de qualquer lugar), `toDir`, `overwrite` | o caminho novo, dentro de `toDir` |
| `fs_trash` | `path` | |
| `fs_list_files` | | todos os arquivos do projeto (até 20 000), sem pastas ocultas, `node_modules`, `target`, `obj_dir` e `__pycache__` |
| `fs_search` | `query`, `regex`, `caseSensitive`, `maxResults?` (padrão 2000) | `SearchMatch[]`: `path`, `line`, `column`, `length`, `text` |

- `fs_write_text` com `expectedModifiedMs` recusa com `conflict` se o
  arquivo mudou desde a leitura. Sem ele, grava por cima.
- Criar, renomear, copiar e mandar para a lixeira só dentro da pasta do
  projeto (`outside_project`); destino que já existe é `exists`. `fs_copy`
  com `overwrite` manda o que estava no destino para a lixeira antes;
  soltar um arquivo na pasta onde ele já está não faz nada.
- A busca só olha arquivos de texto de até 2 MiB.

## Operações (`jobs.rs`)

| Comando | Argumentos | Devolve |
|---|---|---|
| `flow_start` | `request: FlowRequest`, `channel: Channel<JobMessage>` | o identificador da operação |
| `flow_cancel` | `job?` | `bool`: havia operação para cancelar |
| `flow_running` | | o identificador da operação rodando, ou `null` |

Uma operação por vez: começar outra com uma rodando é `busy`. O fluxo vai
ao servidor pelo `flow.run`, com o comando que o `flow.command` dá
(`aurora-ide sim -p soma`), e cancelar manda o `$/cancel` do pedido. Os
fluxos `fpga_build` e `fpga_program` são da placa FPGA, experimental: com
ela desligada, o servidor recusa com `experimental_disabled`, e a operação
termina em `failed`.

### `FlowRequest`

O `FlowRequest` do protocolo, com o tipo no campo `flow`:

| `flow` | Campos | Equivale a |
|---|---|---|
| `build` | `processors: string[]` (vazio: todos) | `aurora build [-p NOME]...` |
| `check` | `file?`, `processor?`, `lint?` | `aurora check [ARQUIVO] [-p NOME] [--lint]` |
| `hierarchy` | `processor?` | `aurora hierarchy [-p NOME]`: a hierarquia elaborada pelo Icarus, sem compilar nem gravar relatório; a interface usa `project_hierarchy` |
| `simulate` | `processor?`, `testbench?`, `simulator` (`icarus`, `verilator`), `fast?` (a simulação rápida: sem onda, com o testbench Verilog no Verilator; `simulator` só vale para um testbench cocotb), `timeout_s?`, `open_wave?` (ignorado com `fast`) | `aurora sim [TESTBENCH] [-p NOME] [--verilator] [--fast] [--timeout S] [--open]` |
| `synthesize` | `processor?`, `schematic?`, `module?`, `open_viewer?` (grava o HTML do RTL viewer ao lado do netlist e o abre no navegador; a interface não pede) | `aurora synth [-p NOME] [--svg] [--module M] [--open]` |
| `schematic` | `netlist`, `module`, `bus_widths?` (padrão `true`) | desenha outro módulo de um netlist que já existe; sem relatório |
| `learn` | `root` (a pasta de exercícios, ou uma pasta dentro dela), `exercise` | `aurora learn check NOME`: corrige o exercício e grava se está resolvido; sem relatório |
| `fpga_build` | nenhum | `aurora fpga build`: confere o `fpga.json`, a placa e o Quartus (`no_fpga_config`, `board_not_found`, `quartus_missing` antes de compilar), compila os processadores e depois o projeto para a placa; sem relatório, como na CLI |
| `fpga_program` | `cable?` | `aurora fpga program [--cable NOME]`: grava o `.sof` da última compilação (`no_bitstream`, `no_cable` antes de gravar); sem relatório |

A composição é a da AURORA, a mesma da CLI, e roda no servidor; `flows.rs` só
escolhe o programa do comando e se a onda abre em janela ou numa aba.
`simulate`, `synthesize` e `fpga_build` compilam antes os processadores que
têm fonte e param no primeiro build que falhar; `check` não compila,
verifica o que está no disco; `build` compila todos mesmo que um falhe; só
`build`, `check`, `simulate` e `synthesize` gravam um relatório no
histórico.

### `JobMessage`

As mensagens chegam pelo `channel`, nesta ordem, com o tipo no campo
`type`. Fora `started`, `cli_output`, `finished` e `failed`, são as
notificações do servidor, como ele as manda:

| `type` | Campos | Quando |
|---|---|---|
| `started` | `job`, `command` | logo depois de `flow_start` |
| `phase` | `phase`: `build`, `check`, `hierarchy`, `simulate`, `synthesize`, `schematic`, `rtl_viewer`, `wave`, `learn`, `fpga`, `program` | uma fase começou |
| `events` | `events: Event[]` | lotes de eventos da AURORA (`step_started`, `output`, `step_finished`; API.md da AURORA, 5.8), no máximo a cada 30 ms ou 500 eventos |
| `output_omitted` | `step`, `tool`, `lines` | linhas de `output` que ficaram de fora, no lugar delas: passam 20 000 de uma vez e depois 5 000 por segundo, em blocos, com um aviso a cada meio segundo (uma linha longa conta uma a mais a cada 1 KiB); o resultado final ainda traz o fim da saída de cada passo |
| `build` | `result: BuildResult` | um processador terminou de compilar |
| `install` | `event` | o progresso de `aurora_install` e `aurora_update`: um evento do instalador (etiquetado por `event`: `start`, `chunk`, `downloading`, `progress`, `replacing`, `verifying`, `fallback`, `fallback_download`, `wizard_selection`, `installer_output`, `warning`) |
| `cli_output` | `stream`, `line` | só em `aurora_install` e `aurora_update`: a linha do console para o evento, com os nomes dos aplicativos (o `line` da notificação `install`) |
| `finished` | `outcome` | fim: um `FlowOutcome` (ou o resultado de `aurora_install` e `aurora_update`) |
| `failed` | `error: IpcError` | fim: a AURORA não conseguiu rodar |

Toda operação termina com exatamente um `finished` ou um `failed`.

### `FlowOutcome`

O `FlowOutcome` do protocolo, com `wave_tab`, que o Studio acrescenta:

| Campo | Conteúdo |
|---|---|
| `flow`, `command` | o fluxo e o comando equivalente (`aurora-ide sim -p soma`) |
| `status` | como terminou: o pior entre as fases que rodaram (`succeeded`, `failed`, `crashed`, `incomplete`, `timed_out`, `cancelled`) |
| `succeeded` | tudo deu certo (`status` é `succeeded`) |
| `builds` | `BuildResult[]` dos builds feitos antes (ou do próprio build) |
| `registered` | `AddedFile[]`: o `.v` e o testbench que esses builds geraram e entraram no `.spf` agora |
| `lint_missing` | com `lint`, o componente do Verilator que falta quando nenhum passo dele rodou |
| `hierarchy`, `not_built` | a hierarquia do fluxo `hierarchy` e os processadores com fonte e sem build que ficaram de fora |
| `check`, `simulation`, `synthesis`, `schematic` | o resultado de cada fase que rodou, ou `null`; com um testbench cocotb, `simulation.tests` traz os testes (`TestReport` da AURORA) |
| `outputs` | `PortValues[]` (`port`, `path`, `values`, `error`), na simulação de um processador que deu certo |
| `wave`, `wave_error` | a onda aberta em janela, ou por que não abriu |
| `wave_tab` | a onda que a interface abre numa aba (`wave_viewer: tab`), no lugar da janela |
| `schematic_error` | por que o esquemático não saiu depois da síntese (`module_not_found`: o módulo pedido não está no netlist); a síntese vale |
| `rtl_viewer`, `rtl_viewer_error` | o HTML do RTL viewer, com `open_viewer`, ou por que não saiu; a síntese vale |
| `report`, `report_error` | o relatório gravado (`run-000042`), ou por que não foi |
| `learn` | a correção de um exercício (fluxo `learn`): o `Grade` do aurora-learn |
| `fpga` | a compilação para a placa (fluxo `fpga_build`): o `FpgaBuildResult` da AURORA, com os passos do Quartus, os recursos e o tempo |
| `fpga_program` | a gravação na placa (fluxo `fpga_program`): o `FpgaProgramResult` da AURORA |

O `stdout` e o `stderr` de cada passo vêm cortados em 256 KiB, guardando o
fim (o `step_output_limit` do `flow.run`); o que foi cortado já chegou
linha a linha pelos eventos. Um fluxo cancelado antes de começar (na fila
do projeto, atrás de outro pedido) vem como `finished` com `status:
cancelled` e as listas vazias.

## Bundle (`commands/toolchain.rs`)

| Comando | Argumentos | Devolve | Equivale a |
|---|---|---|---|
| `toolchain_info` | | `ToolchainInfo` (nunca falha: sem bundle, ou sem servidor, `found: false` e o erro) | `aurora tools --json`, o `toolchain.info` do servidor |
| `aurora_update_check` | | o `UpdateReport` (`docs/schema/update.json` da AURORA), com `prefix` quando o servidor é o `aurora` de uma instalação do instalador | `aurora update --check --json`, o `update.check` do servidor |
| `aurora_install` | `components: string[]`, `channel` | o identificador da operação | `aurora install <componentes> --json`, o `install.add` do servidor |
| `aurora_update` | `channel` | o identificador da operação | `aurora update --yes --json`, o `update.apply` do servidor |

`ToolchainInfo` (o `aurora_core::locate::ToolchainInfo`): `found`, `error`,
`origin` (`settings`, `environment`, `bundled`, `installation`, `path`,
`beside`), `root`, `bundle`, `platform`, `components`, `not_installed`,
`tools` (`name`, `component`, `path`, `system`, `error`),
`system_compiler`, `compiler_error`, `quartus`, `quartus_error`,
`aurora_cli`, `surfer_web` (o cliente web do Surfer, `surfer-aurora/web/`, ou
`null`). Um compilador ou um Quartus das preferências que não serve não
derruba o bundle: vem em `compiler_error` e `quartus_error`, e só a
operação que precisa dele falha.

Os três do instalador rodam no servidor, com a mesma regra do
`aurora update` e do `aurora install` da CLI, e precisam de rede; cada consulta
de versão do `update.check` tem prazo de 90 s. `aurora_install` e
`aurora_update` são operações como os fluxos: ocupam a vaga e mandam
`install` (cada evento do instalador), `cli_output` (a linha dele para o
console) e `finished` (`{ flow: 'install' | 'update', succeeded,
cancelled, exit_code, result }`). `result` é o relatório (`InstallReport`,
`UpdateReport`), ou `{ error: { code, message, hint } }` quando a operação
não rodou, com `exit_code` o que a CLI daria (0, 2 ou 130). O `code` é o do
instalador (`network`, `no_installation`, `install_in_progress`,
`checksum_mismatch`, `installer_failed`, ...); um nome que não é do bundle
é recusado antes de qualquer download (`invalid_argument`).

A pasta da instalação é a do `aurora` do servidor (`bin/..`); com um `aurora`
de desenvolvimento (`AURORA_IDE_CLI`, `aurora_path`), `aurora_update_check`
responde com `prefix: null` e `aurora_update` falha com `no_installation`.

`aurora_update` não pergunta: quem pergunta é a interface, antes de chamar,
com a versão instalada, a nova e a pasta (o `UpdateReport` de
`aurora_update_check`). No Windows pede ao assistente de instalação, quando a
atualização cai nele, para abrir o Studio de novo no fim
(`reopen_ide`); o servidor roda num Job Object que deixa o assistente
sair dele, e o assistente sobrevive ao Studio, que fecha para ele trocar
os arquivos. A interface chama `aurora_update_check` sozinha ao abrir e a
cada 12 horas (`check_updates` nas preferências), e depois de `updated`
reinicia o Studio com `app_exit`; o servidor, que teve o binário trocado,
sai sozinho depois da resposta. No `result`, `action` diz o que aconteceu:
`updated` (a atualização por componentes trocou os arquivos que mudaram,
`method: "components"`, com o que mudou em `changes`; ou, no Linux e no
macOS, o instalador da release trocou o `aurora` e o bundle, `method:
"installer"`), `wizard_opened` (Windows, quando a atualização por
componentes não dá: o assistente abriu e termina depois que o Studio sai)
ou `up_to_date`. Cancelar (`flow_cancel`) vale até a troca dos arquivos:
um download parado é `cancelled`, e nada muda; durante a troca o pedido é
ignorado, e uma troca interrompida por fora é desfeita na próxima
operação. A interface ainda não oferece o botão de cancelar o update.

## Onda numa aba (`wave_tab.rs`)

| Comando | Argumentos | Devolve |
|---|---|---|
| `wave_tab_open` | `path` | `WaveTab`: `id`, `url` (a página do cliente web para o iframe), `processors` (`WaveProcessor[]` do layout, com `outdated` quando o processador foi compilado de novo depois da simulação), `saved_layout` (o `wave/<testbench>.surf.ron`, ou `null` fora de projeto), `customized` (o usuário salvou o layout) e `selection` (a escolha de sinais que o layout segue) |
| `wave_tab_close` | `id` | |

`wave_tab_open` pede ao servidor o layout da onda (`wave.layout`: o gerado
dos processadores SAPHO, o salvo no projeto e o de um exercício do
`aurora learn`) e o cliente web do bundle (`toolchain.info`), e registra a
onda num servidor HTTP do Studio, só em `127.0.0.1`, que sobe na primeira
aba e serve, numa origem: o cliente web (`/web/`), a onda
(`/wave/<id>/<nome>`), o `.surf.ron` (`/layout/<id>`: o layout salvo no
projeto, ou o gerado), os tradutores e os comandos de partida
(`/doc/<id>/<nome>`). A `url` abre o cliente com `load_url` na onda e
`startup_commands` que carregam os tradutores e o estado e, numa onda de
testbench do projeto, registram `state_save_url_set <origem>/save/<id>`: o
Ctrl+S da aba (que fica com o Surfer) manda o estado num `POST`, e o
servidor local o grava no `wave/<testbench>.surf.ron`, num arquivo ao lado
que depois toma o lugar dele. Erros: `no_waveform`; `surfer_web_missing`,
bundle sem cliente web; `wave_too_large`, onda acima de 256 MB. A interface
oferece a janela nos dois últimos.

`wave_tab_close` faz o servidor local esquecer a aba. A vista chama ao
fechar a aba ou ao recarregar.

## Terminal (`terminal.rs`)

| Comando | Argumentos | Devolve |
|---|---|---|
| `terminal_spawn` | `cwd?` (padrão: a pasta do projeto), `cols`, `rows`, `channel: Channel<TerminalMessage>` | o identificador |
| `terminal_write` | `id`, `data` | |
| `terminal_ack` | `id`, `units` | a interface escreveu no xterm `units` (o `length` das strings) do que o terminal mandou |
| `terminal_cd` | `id`, `path` | digita no shell o `cd` para `path`, na sintaxe dele (PowerShell, `cmd`, POSIX ou fish), depois das teclas que apagam o que estiver na linha; a interface chama ao abrir outro projeto |
| `terminal_resize` | `id`, `cols`, `rows` | |
| `terminal_kill` | `id` | |

`TerminalMessage`: `{ type: 'data', data }` com o que o shell escreveu (UTF-8),
e `{ type: 'exit', code }` quando ele termina. A saída tem controle de fluxo:
a interface confirma com `terminal_ack`, em lotes menores que 16 Ki unidades
UTF-16, o que o xterm já escreveu, e o backend para de ler o shell quando a
interface fica 128 Ki atrás, até o atraso cair para 16 Ki; o programa que
escreve no terminal espera. O shell é o do usuário
(`$SHELL`) no Linux e no macOS e, no Windows, o PowerShell ou o `cmd.exe`, pela preferência `terminal_shell`, com a pasta `bin/`
da instalação da AURORA no começo do `PATH` e `AURORA_IDE=1`.

## Placa virtual (`vboard.rs`)

A placa virtual é do recurso experimental `fpga`. A sessão é o `vboard.run`
do servidor, por um canal próprio, fora das operações (`jobs.rs`): a placa
fica ligada enquanto o usuário compila e simula. As entradas vão pelo
`aurora_call` (`vboard.input`, com o `run` que o canal deu), e a placa sem
compilar pelo `vboard.layout`. O `vboard.run` não passa pelo `aurora_call`.

| Comando | Argumentos | Devolve |
|---|---|---|
| `vboard_start` | `board?` (num projeto sem `fpga.json`), `limit?` (1 é o tempo real, 0 o mais rápido possível), `fps?`, `channel: Channel<VboardMessage>` | nada; a sessão segue numa thread |
| `vboard_stop` | `run` | nada: manda o `$/cancel` do pedido, e o fim chega pelo canal |

`VboardMessage` (`ipc/types.ts`): `started` (com o `run`), as notificações
do pedido com o `type` igual ao método (`phase`, `events` e
`output_omitted` da compilação; `vboard_ready`, `vboard_state`,
`vboard_frame` e `vboard_output` da placa rodando) e, no fim, `finished` (o
`BoardRun`) ou `failed`.

## Assistente (`assistant.rs`)

O assistente é o recurso experimental `assistant`: desligado, a vista, as
ações e a página de Preferências dele somem da janela, e `assistant_start`
responde `experimental_disabled` (conferido pelo `experimental.list` do
servidor).

| Comando | Argumentos | Devolve |
|---|---|---|
| `assistant_info` | | `{ claude, version, aurora, project }`: o Claude Code achado (ou `null`), a versão dele, o `aurora` que vai servir as ferramentas e o `.spf` aberto |
| `assistant_start` | `resume?` (um `session_id` para retomar), `channel: Channel<AgentEvent>` | o identificador da sessão |
| `assistant_send` | `id`, `text` | |
| `assistant_interrupt` | `id` | interrompe o turno em andamento |
| `assistant_stop` | `id` | encerra a sessão (espera o Claude Code sair por até 3 s, depois mata) |

A sessão é um processo filho do Claude Code (`crates/aurora-agent`, o único
crate da AURORA que o Studio linka, até a 1.0), aberto em modo `--print` com
entrada e saída em JSON por linha, o `aurora mcp` como servidor de
ferramentas (a preferência `assistant_aurora`, ou o mesmo `aurora` do servidor
do Studio), o `.spf` aberto como projeto padrão e a pasta do projeto como
pasta de trabalho. O que ele pode fazer vem da preferência
`assistant_allow_edits`: só as ferramentas da AURORA e ler arquivos, ou
também editar e criar na pasta do projeto. Quem autentica é o Claude Code;
o Studio não guarda chave.

`AgentEvent` (`aurora_agent::AgentEvent`, etiquetado por `type`): `ready`
(`session_id`, `model`, `tools`, `aurora_connected`; só vem depois da primeira
mensagem, porque o Claude Code não inicia nada antes dela), `text` (um
pedaço da resposta), `tool_use` (`id`, `name`, `input`), `tool_result`
(`id`, `content`, `is_error`), `done` (`is_error`, `text`, `cost_usd`,
`duration_ms`, `turns`, `session_id`, tokens), `log` (uma linha do stderr
dele) e `exited` (`code`), o último. A interface (`state/assistant.ts`)
monta a transcrição a partir deles; ao trocar de projeto, fecha a sessão,
e a próxima mensagem abre outra. Ao sair, o Studio encerra todas.

## Eventos

| Evento | Carga | Quando |
|---|---|---|
| `studio://fs-changed` | `{ paths: string[] }` | arquivos da pasta do projeto foram criados, gravados, apagados ou renomeados (no máximo um a cada 300 ms; sem `.aurora8/` e sem o `.git/`, menos o `HEAD`, o `index`, o `packed-refs` e as `refs/`, sem as travas `*.lock`, que mudam num commit, numa troca de ramo ou num fetch feitos fora da AURORA; abrir um arquivo para ler não conta) |

## Códigos de erro do Studio

| `code` | Quando |
|---|---|
| `no_project` | o comando ou o método precisa de um projeto aberto |
| `busy` | já há uma operação rodando |
| `outside_project` | criar, renomear ou apagar fora da pasta do projeto |
| `conflict` | o arquivo mudou no disco desde que foi lido |
| `exists` | o arquivo ou a pasta de destino já existe |
| `invalid_argument` | argumento inválido (regex mal formada, método que não passa pelo `aurora_call`, parâmetro que o método não tem) |
| `terminal` | o pseudoterminal não abriu, ou o shell já terminou |
| `unsafe_path` | `terminal_cd` com um caminho que tem caractere de controle: o shell o leria como tecla, e o `cd` não é digitado |
| `terminal_busy` | `terminal_cd` com um programa em primeiro plano no terminal (Linux e macOS): a linha iria para ele, e o `cd` não é digitado |
| `io` | leitura ou escrita do próprio Studio falhou |
| `surfer_web_missing` | `wave_tab_open` num bundle sem o cliente web do Surfer: a interface oferece a janela |
| `wave_too_large` | `wave_tab_open` com uma onda acima de 256 MiB (`MAX_TAB_WAVE`): a interface oferece a janela |
| `internal` | uma thread caiu, ou JSON inválido |
| `assistant` | o assistente: o Claude Code não foi achado, o `aurora` das ferramentas não existe, ou a sessão já terminou |
| `aurora_missing` | o programa `aurora` do servidor não foi achado ou não abriu |
| `aurora_incompatible` | o `aurora` achado fala outra versão do protocolo |
| `server_gone` | o servidor da AURORA saiu no meio de um pedido; o seguinte o abre de novo |
| `cancelled` | o pedido foi cancelado antes de começar |

Os erros da AURORA chegam com o código dele (`AuroraError::code`, API.md do
AURORA, seção 8, e os do servidor): por exemplo `invalid_name` (nome de
projeto fora da regra), `spf_missing` (o `.spf` do projeto aberto foi
apagado depois de aberto: os métodos de projeto e os fluxos recusam, e os
`fs_*` continuam, pela pasta), `cannot_move`, `cannot_replace`,
`path_exists` e `outside_project` (do `project.move`), `no_waveform` (a
onda pedida ainda não existe; `wave.open` e `wave_tab_open` devolvem o
mesmo código), `bundle_not_found` (nenhuma instalação da AURORA foi achada,
com onde procurou) e `experimental_disabled` (os métodos e os fluxos da
placa FPGA com ela desligada, e `assistant_start` com o assistente
desligado; a interface relê os recursos e tira o recurso da janela). Os do
`aurora-learn` (`learn_component_missing`), do instalador (`network`,
`no_installation`) e do `aurora-agent` (`claude_not_found`, `ended`) também
passam como estão.
