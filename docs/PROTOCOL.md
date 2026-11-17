# Protocolo do `aurora serve`

<!-- Gerado por crates/aurora-cli/src/protocol_doc.rs. Não edite à mão: AURORA_UPDATE_SCHEMA=1 cargo test -p aurora-cli schema -->

Cada método do `aurora serve`, com os parâmetros e o resultado, as notificações, os erros e os tipos. O `aurora mcp` usa o mesmo contrato. Esta página sai dos tipos de `aurora_protocol` (`crates/aurora-protocol/src/lib.rs`), como os schemas [`protocol-request.json`](schema/protocol-request.json) e [`protocol-response.json`](schema/protocol-response.json), e um teste falha quando ela não confere com eles.

Como abrir o servidor, a fila por projeto, o cancelamento e o encerramento: [CLI.md](CLI.md#a-api-para-outros-programas-aurora-serve-e-aurora-mcp).

Versão do protocolo: 2 (`PROTOCOL_VERSION`, que o `initialize` devolve).

Um pedido é uma linha: `{"jsonrpc": "2.0", "id": 1, "method": "project.status", "params": {"project": "/home/eu/soma"}}`. Os parâmetros vão por nome, e os caminhos são absolutos. A resposta traz o mesmo `id` e o resultado em `result`, ou o erro em `error`.

## Métodos

| Método | Resultado | O que faz |
|---|---|---|
| [`initialize`](#metodo-initialize) | [`ServerInfo`](#tipo-serverinfo) | O primeiro pedido de uma sessão: quem é o cliente. |
| [`shutdown`](#metodo-shutdown) | `null` | Encerrar a sessão: os pedidos em andamento são cancelados. |
| [`project.create`](#metodo-project-create) | [`ProjectStatus`](#tipo-projectstatus) | Cria `<parent>/<name>/<name>.spf`. |
| [`project.status`](#metodo-project-status) | [`ProjectStatus`](#tipo-projectstatus) | O retrato do projeto. |
| [`project.add`](#metodo-project-add) | lista de [`AddedFile`](#tipo-addedfile) | Registra arquivos Verilog e VHDL, ou testbenches cocotb (criando os que não existem, pelos modelos). |
| [`project.remove`](#metodo-project-remove) | [`RemoveOutcome`](#tipo-removeoutcome) | Tira arquivos do projeto, sem apagar. |
| [`project.top`](#metodo-project-top) | [`TopOutcome`](#tipo-topoutcome) | Escolhe o topo por arquivo ou por nome de módulo; sem alvo, só consulta. |
| [`project.testbench`](#metodo-project-testbench) | [`ProjectStatus`](#tipo-projectstatus) | Escolhe o testbench da simulação do projeto. |
| [`project.order`](#metodo-project-order) | lista de [`ProjectFile`](#tipo-projectfile) | Muda a posição de um arquivo na lista dele. |
| [`project.move`](#metodo-project-move) | [`MovedPath`](#tipo-movedpath) | Move ou renomeia um arquivo ou pasta do projeto, mantendo o `.spf` em dia; com `after`, só registra um movimento já feito. |
| [`project.convert`](#metodo-project-convert) | [`LayoutConversion`](#tipo-layoutconversion) | Converte um projeto da estrutura do YANC 6.x. |
| [`project.check_name`](#metodo-project-check-name) | [`NameCheck`](#tipo-namecheck) | Um nome serve para um projeto novo? |
| [`processor.check_name`](#metodo-processor-check-name) | [`NameCheck`](#tipo-namecheck) | Um nome serve para um processador novo? |
| [`processor.add`](#metodo-processor-add) | [`Processor`](#tipo-processor) | Cria um processador. |
| [`processor.set`](#metodo-processor-set) | [`Processor`](#tipo-processor) | Muda os parâmetros de simulação de um processador. |
| [`processor.remove`](#metodo-processor-remove) | [`ProcessorRemoval`](#tipo-processorremoval) | Tira um processador do projeto. |
| [`processor.outputs`](#metodo-processor-outputs) | lista de `integer` | Os valores de uma porta de saída da última simulação do processador. |
| [`processor.defaults`](#metodo-processor-defaults) | [`NewProcessor`](#tipo-newprocessor) | Os padrões de um processador novo (os da AURORA), para um formulário começar preenchido. |
| [`flow.run`](#metodo-flow-run) | [`FlowOutcome`](#tipo-flowoutcome) | Roda um fluxo ([`FlowRequest`](#tipo-flowrequest)). |
| [`flow.command`](#metodo-flow-command) | `string` | O comando equivalente de um fluxo, como a CLI o escreveria e como ele vai para o histórico (`aurora-ide sim -p soma`), para uma interface mostrar antes de rodar. |
| [`wave.open`](#metodo-wave-open) | [`WaveOpened`](#tipo-waveopened) | Abre uma onda no surfer-aurora. |
| [`wave.signals`](#metodo-wave-signals) | [`WaveSignals`](#tipo-wavesignals) | A árvore de sinais do testbench do projeto, com a escolha gravada. |
| [`wave.choose`](#metodo-wave-choose) | [`WaveChoiceOutcome`](#tipo-wavechoiceoutcome) | Muda a escolha de sinais. |
| [`wave.reset_layout`](#metodo-wave-reset-layout) | `boolean` | Apaga o layout salvo de uma onda. |
| [`wave.layout`](#metodo-wave-layout) | [`WaveLayoutInfo`](#tipo-wavelayoutinfo) | O layout da AURORA para uma onda, para um visualizador próprio (a aba de onda do Studio): o gerado dos processadores SAPHO, o salvo no projeto da onda (que o gerado atualiza enquanto o usuário não salvar o dele) e o de um exercício do `aurora learn`. |
| [`verilog.modules`](#metodo-verilog-modules) | lista de `string` | Os módulos declarados num texto Verilog, sem os comentados, na ordem (o contorno de um editor). |
| [`verilog.testbench_names`](#metodo-verilog-testbench-names) | lista de `boolean` | Quais nomes de arquivo são de testbench pela regra da AURORA (`tb_<nome>.v`, `<nome>_tb.v`). |
| [`history.list`](#metodo-history-list) | lista de [`RunSummary`](#tipo-runsummary) | Os relatórios do projeto, do mais novo para o mais antigo. |
| [`history.show`](#metodo-history-show) | [`ReportView`](#tipo-reportview) | Um relatório (o mais novo sem `id`). |
| [`history.compare`](#metodo-history-compare) | [`RunComparison`](#tipo-runcomparison) | Compara dois relatórios. |
| [`history.plan_cleanup`](#metodo-history-plan-cleanup) | lista de `string` | Quais relatórios uma limpeza apagaria, sem apagar. |
| [`history.clean`](#metodo-history-clean) | [`CleanOutcome`](#tipo-cleanoutcome) | Apaga exatamente estes relatórios. |
| [`toolchain.info`](#metodo-toolchain-info) | [`ToolchainInfo`](#tipo-toolchaininfo) | O bundle e as ferramentas. |
| [`toolchain.verify`](#metodo-toolchain-verify) | lista de [`FileMismatch`](#tipo-filemismatch) | Confere o SHA-256 dos executáveis. |
| [`experimental.list`](#metodo-experimental-list) | [`ExperimentalInfo`](#tipo-experimentalinfo) | Os recursos experimentais e se estão ligados. |
| [`experimental.set`](#metodo-experimental-set) | [`ExperimentalInfo`](#tipo-experimentalinfo) | Liga ou desliga um recurso experimental. |
| [`fpga.boards`](#metodo-fpga-boards) | lista de [`Board`](#tipo-board) | As placas conhecidas, ou uma. |
| [`fpga.config`](#metodo-fpga-config) | [`FpgaConfig`](#tipo-fpgaconfig) \| `null` | O `fpga.json` do projeto. |
| [`fpga.config_set`](#metodo-fpga-config-set) | `null` | Grava o `fpga.json`. |
| [`fpga.connect`](#metodo-fpga-connect) | [`FpgaConfig`](#tipo-fpgaconfig) | Muda uma ligação a partir da placa: um bit de um sinal (ou o sinal inteiro) passa a ir numa porta do topo, ou fica solto; o que estava ali sai, e as outras ligações continuam. |
| [`fpga.check`](#metodo-fpga-check) | [`Prepared`](#tipo-prepared) | Confere o `fpga.json` contra a placa e o topo. |
| [`fpga.modules`](#metodo-fpga-modules) | lista de `string` | Os módulos do projeto que podem ir para a placa. |
| [`fpga.status`](#metodo-fpga-status) | [`BitstreamState`](#tipo-bitstreamstate) | O arquivo de gravação da última compilação e se serve. |
| [`fpga.cables`](#metodo-fpga-cables) | lista de `string` | Os cabos de gravação ligados. |
| [`fpga.top`](#metodo-fpga-top) | [`ModuleInterface`](#tipo-moduleinterface) | O módulo que vai para a placa e as portas dele, para montar as ligações antes de estarem certas. |
| [`vboard.layout`](#metodo-vboard-layout) | [`BoardLayout`](#tipo-boardlayout) | A placa virtual do projeto sem compilar: a placa, o topo e os periféricos, ligados ou não. |
| [`vboard.run`](#metodo-vboard-run) | [`BoardRun`](#tipo-boardrun) | Compila a placa virtual e a roda até `$/cancel` (ou o `$finish` do projeto). |
| [`vboard.input`](#metodo-vboard-input) | `null` | Uma entrada para a placa virtual de um `vboard.run` que está rodando: chaves, botões, teclas, texto, pausa, velocidade. |
| [`update.check`](#metodo-update-check) | [`UpdateReport`](#tipo-updatereport) | As versões: esta AURORA, a última release, o bundle, cada componente. |
| [`update.apply`](#metodo-update-apply) | [`UpdateReport`](#tipo-updatereport) | Atualiza a instalação. |
| [`install.add`](#metodo-install-add) | [`InstallReport`](#tipo-installreport) | Instala componentes do bundle. |
| [`learn.check`](#metodo-learn-check) | [`LearnCheckOutcome`](#tipo-learncheckoutcome) | Corrige um exercício do `aurora learn` (ou todos). |
| [`learn.tracks`](#metodo-learn-tracks) | [`LearnTracks`](#tipo-learntracks) | De onde vêm as trilhas do `aurora learn` e quais há. |
| [`learn.status`](#metodo-learn-status) | [`WorkspaceSnapshot`](#tipo-workspacesnapshot) | Abre a pasta de exercícios que contém `root` e a põe em dia com a trilha. |
| [`learn.init`](#metodo-learn-init) | [`WorkspaceSnapshot`](#tipo-workspacesnapshot) | Cria a pasta de exercícios (`aurora learn init`). |
| [`learn.set_current`](#metodo-learn-set-current) | [`WorkspaceSnapshot`](#tipo-workspacesnapshot) | Torna atual um exercício. |
| [`learn.reset`](#metodo-learn-reset) | `null` | Volta o arquivo de um exercício ao começo (`aurora learn reset`). |
| [`git.status`](#metodo-git-status) | [`GitStatus`](#tipo-gitstatus) | O git da máquina e o repositório que contém o projeto: ramo, arquivos mudados, identidade e remotos. |
| [`git.diff`](#metodo-git-diff) | `string` | O diff unificado do que mudou desde o último commit, num arquivo ou em todos. |
| [`git.read_file`](#metodo-git-read-file) | [`FileVersion`](#tipo-fileversion) | O conteúdo de um arquivo no índice, no último commit ou num commit, para comparar com o do disco. |
| [`git.commit`](#metodo-git-commit) | [`CommitResult`](#tipo-commitresult) | Faz o commit dos arquivos escolhidos, no estado em que estão no disco. |
| [`git.discard`](#metodo-git-discard) | [`DiscardResult`](#tipo-discardresult) | Descarta as mudanças de arquivos: voltam ao último commit, e os novos saem; a versão descartada vai para a lixeira do sistema. |
| [`git.init`](#metodo-git-init) | [`InitResult`](#tipo-initresult) | Cria um repositório na pasta do projeto, com um `.gitignore` para o que a AURORA gera. |
| [`git.log`](#metodo-git-log) | [`GitLog`](#tipo-gitlog) | Uma página do histórico do ramo atual. |
| [`git.commit_files`](#metodo-git-commit-files) | [`CommitFiles`](#tipo-commitfiles) | Os arquivos que um commit mudou. |
| [`git.branches`](#metodo-git-branches) | [`Branches`](#tipo-branches) | Os ramos locais e remotos. |
| [`git.switch`](#metodo-git-switch) | `string` | Troca de ramo, ou cria um. |
| [`git.fetch`](#metodo-git-fetch) | [`SyncResult`](#tipo-syncresult) | Traz do remoto os ramos e commits novos, sem mexer nos arquivos. |
| [`git.pull`](#metodo-git-pull) | [`SyncResult`](#tipo-syncresult) | Traz os commits do ramo remoto, só em avanço direto. |
| [`git.push`](#metodo-git-push) | [`SyncResult`](#tipo-syncresult) | Envia o ramo atual; sem ramo remoto, publica. |
| [`git.set_identity`](#metodo-git-set-identity) | `null` | Grava o nome e o e-mail dos commits. |
| [`git.set_remote`](#metodo-git-set-remote) | `null` | Dá um endereço a um remoto: cria o remoto ou troca o endereço de um que já existe. |
| [`git.remove_remote`](#metodo-git-remove-remote) | `null` | Tira um remoto. |
| [`github.account`](#metodo-github-account) | [`Account`](#tipo-account) \| `null` | A conta do GitHub que o git desta máquina conhece. |
| [`github.login_start`](#metodo-github-login-start) | [`DeviceCode`](#tipo-devicecode) | Começa o login no GitHub: o código que a pessoa digita em github.com/login/device. |
| [`github.login_poll`](#metodo-github-login-poll) | [`LoginPoll`](#tipo-loginpoll) | Pergunta se a pessoa já autorizou o login; com a autorização, guarda o token no gerenciador de credenciais do git. |
| [`github.logout`](#metodo-github-logout) | `boolean` | Sai do GitHub: tira o token do gerenciador de credenciais do git. |

### Sessão

<a id="metodo-initialize"></a>

#### `initialize`

O primeiro pedido de uma sessão: quem é o cliente. Resposta:
[`ServerInfo`](#tipo-serverinfo).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `client` | `string` \| `null` | não (padrão: `null`) | O cliente e a versão dele (`aurora-vscode 0.1.0`). |

Resultado: [`ServerInfo`](#tipo-serverinfo).

<a id="metodo-shutdown"></a>

#### `shutdown`

Encerrar a sessão: os pedidos em andamento são cancelados.

Sem parâmetros.

Resultado: `null`.

### `project.*`

<a id="metodo-project-create"></a>

#### `project.create`

Cria `<parent>/<name>/<name>.spf`. Resposta: o [`ProjectStatus`](#tipo-projectstatus) do
projeto novo.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `parent` | `string` | sim | A pasta de cima, que precisa existir. |
| `name` | `string` | sim | O nome do projeto. |

Resultado: [`ProjectStatus`](#tipo-projectstatus).

<a id="metodo-project-status"></a>

#### `project.status`

O retrato do projeto. Resposta: [`ProjectStatus`](#tipo-projectstatus).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O `.spf` ou a pasta do projeto. |

Resultado: [`ProjectStatus`](#tipo-projectstatus).

<a id="metodo-project-add"></a>

#### `project.add`

Registra arquivos Verilog e VHDL, ou testbenches cocotb (criando os
que não existem, pelos modelos). Resposta: um [`AddedFile`](#tipo-addedfile) por
arquivo.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `files` | lista de `string` | sim | Os arquivos. |
| `testbench` | `boolean` | não (padrão: `false`) | Registrar como testbench em vez de decidir pelo conteúdo. |

Resultado: lista de [`AddedFile`](#tipo-addedfile).

<a id="metodo-project-remove"></a>

#### `project.remove`

Tira arquivos do projeto, sem apagar. Resposta: [`RemoveOutcome`](#tipo-removeoutcome).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `files` | lista de `string` | sim | Os arquivos. |

Resultado: [`RemoveOutcome`](#tipo-removeoutcome).

<a id="metodo-project-top"></a>

#### `project.top`

Escolhe o topo por arquivo ou por nome de módulo; sem alvo, só
consulta. Resposta: [`TopOutcome`](#tipo-topoutcome).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `target` | `string` \| `null` | não (padrão: `null`) | O arquivo ou o módulo. |

Resultado: [`TopOutcome`](#tipo-topoutcome).

<a id="metodo-project-testbench"></a>

#### `project.testbench`

Escolhe o testbench da simulação do projeto. Resposta:
[`ProjectStatus`](#tipo-projectstatus).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `path` | `string` | sim | O testbench. |

Resultado: [`ProjectStatus`](#tipo-projectstatus).

<a id="metodo-project-order"></a>

#### `project.order`

Muda a posição de um arquivo na lista dele. Resposta: a lista, na
ordem nova ([`ProjectFile`](#tipo-projectfile)).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `file` | `string` | sim | O arquivo. |
| `position` | [`ListPosition`](#tipo-listposition) | sim | Para onde. |

Resultado: lista de [`ProjectFile`](#tipo-projectfile).

<a id="metodo-project-move"></a>

#### `project.move`

Move ou renomeia um arquivo ou pasta do projeto, mantendo o `.spf` em
dia; com `after`, só registra um movimento já feito. Resposta:
[`MovedPath`](#tipo-movedpath).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `from` | `string` | sim | De onde. |
| `to` | `string` | sim | Para onde (o caminho final). |
| `after` | `boolean` | não (padrão: `false`) | O movimento já aconteceu fora da AURORA. |
| `overwrite` | `boolean` | não (padrão: `false`) | Se o destino existe, ele vai para a lixeira do sistema e o movimento acontece; sem isso, `path_exists`. O que a AURORA mantém no lugar, a pasta do projeto e uma pasta que contém a origem não são substituídos (`cannot_replace`). Não vale com `after`. |

Resultado: [`MovedPath`](#tipo-movedpath).

<a id="metodo-project-convert"></a>

#### `project.convert`

Converte um projeto da estrutura do YANC 6.x. Resposta:
[`LayoutConversion`](#tipo-layoutconversion).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |

Resultado: [`LayoutConversion`](#tipo-layoutconversion).

<a id="metodo-project-check-name"></a>

#### `project.check_name`

Um nome serve para um projeto novo? Resposta: [`NameCheck`](#tipo-namecheck).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome. |

Resultado: [`NameCheck`](#tipo-namecheck).

### `processor.*`

<a id="metodo-processor-check-name"></a>

#### `processor.check_name`

Um nome serve para um processador novo? Resposta: [`NameCheck`](#tipo-namecheck).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome. |

Resultado: [`NameCheck`](#tipo-namecheck).

<a id="metodo-processor-add"></a>

#### `processor.add`

Cria um processador. Resposta: o [`Processor`](#tipo-processor).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `spec` | [`NewProcessor`](#tipo-newprocessor) | sim | O processador; só `name` e `language` são obrigatórios. |

Resultado: [`Processor`](#tipo-processor).

<a id="metodo-processor-set"></a>

#### `processor.set`

Muda os parâmetros de simulação de um processador. Resposta: o
[`Processor`](#tipo-processor).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `name` | `string` | sim | O processador. |
| `config` | [`ProcessorConfig`](#tipo-processorconfig) | sim | O que muda (`None` mantém). |

Resultado: [`Processor`](#tipo-processor).

<a id="metodo-processor-remove"></a>

#### `processor.remove`

Tira um processador do projeto. Resposta: [`ProcessorRemoval`](#tipo-processorremoval).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `name` | `string` | sim | O processador. |
| `to_trash` | `boolean` | não (padrão: `false`) | Mandar a pasta dele para a lixeira do sistema. |

Resultado: [`ProcessorRemoval`](#tipo-processorremoval).

<a id="metodo-processor-outputs"></a>

#### `processor.outputs`

Os valores de uma porta de saída da última simulação do processador.
Resposta: os valores.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `processor` | `string` | sim | O processador. |
| `port` | `integer` | sim | A porta. |

Resultado: lista de `integer`.

<a id="metodo-processor-defaults"></a>

#### `processor.defaults`

Os padrões de um processador novo (os da AURORA), para um formulário
começar preenchido. Resposta: o [`NewProcessor`](#tipo-newprocessor), com o nome vazio.

Sem parâmetros.

Resultado: [`NewProcessor`](#tipo-newprocessor).

### `flow.*`

<a id="metodo-flow-run"></a>

#### `flow.run`

Roda um fluxo ([`FlowRequest`](#tipo-flowrequest)). Enquanto roda, as notificações
(`phase`, `events`, `output_omitted`, `build`); no fim, o
[`FlowOutcome`](#tipo-flowoutcome).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `request` | [`FlowRequest`](#tipo-flowrequest) | sim | O fluxo. |
| `options` | [`FlowOptions`](#tipo-flowoptions) \| `null` | não (padrão: `null`) | As opções da interface; sem elas, o comando é o da CLI equivalente, a onda abre em janela e o RTL viewer no navegador. |
| `output` | `boolean` | não (padrão: `true`) | Receber a saída das ferramentas ao vivo (`events`). Sem isso, o `vvp` roda com buffer, que é mais rápido. |
| `step_output_limit` | `integer` \| `null` | não (padrão: `null`) | Quantos bytes do `stdout` e do `stderr` de cada passo voltam no resultado, do fim; sem isso, tudo. Com `output`, cada linha já foi pelas notificações `events`. |

Resultado: [`FlowOutcome`](#tipo-flowoutcome).

<a id="metodo-flow-command"></a>

#### `flow.command`

O comando equivalente de um fluxo, como a CLI o escreveria e como ele
vai para o histórico (`aurora-ide sim -p soma`), para uma interface
mostrar antes de rodar. Resposta: o texto.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `request` | [`FlowRequest`](#tipo-flowrequest) | sim | O fluxo. |
| `program` | `string` \| `null` | não (padrão: `null`) | O programa que começa o comando; sem ele, `aurora`. |

Resultado: `string`.

### `wave.*`

<a id="metodo-wave-open"></a>

#### `wave.open`

Abre uma onda no surfer-aurora. Resposta: [`WaveOpened`](#tipo-waveopened).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` \| `null` | não (padrão: `null`) | O projeto, para uma onda do projeto; `None` para um arquivo. |
| `request` | [`WaveRequest`](#tipo-waverequest) | sim | Qual onda e com que layout. |

Resultado: [`WaveOpened`](#tipo-waveopened).

<a id="metodo-wave-signals"></a>

#### `wave.signals`

A árvore de sinais do testbench do projeto, com a escolha gravada.
Resposta: [`WaveSignals`](#tipo-wavesignals).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |

Resultado: [`WaveSignals`](#tipo-wavesignals).

<a id="metodo-wave-choose"></a>

#### `wave.choose`

Muda a escolha de sinais. Resposta: [`WaveChoiceOutcome`](#tipo-wavechoiceoutcome).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `choice` | [`WaveChoice`](#tipo-wavechoice) | sim | A mudança. |

Resultado: [`WaveChoiceOutcome`](#tipo-wavechoiceoutcome).

<a id="metodo-wave-reset-layout"></a>

#### `wave.reset_layout`

Apaga o layout salvo de uma onda. Resposta: se havia.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `waveform` | `string` | sim | A onda. |

Resultado: `boolean`.

<a id="metodo-wave-layout"></a>

#### `wave.layout`

O layout da AURORA para uma onda, para um visualizador próprio (a aba de
onda do Studio): o gerado dos processadores SAPHO, o salvo no projeto
da onda (que o gerado atualiza enquanto o usuário não salvar o dele)
e o de um exercício do `aurora learn`. Resposta: [`WaveLayoutInfo`](#tipo-wavelayoutinfo).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `waveform` | `string` | sim | A onda. |

Resultado: [`WaveLayoutInfo`](#tipo-wavelayoutinfo).

### `verilog.*`

<a id="metodo-verilog-modules"></a>

#### `verilog.modules`

Os módulos declarados num texto Verilog, sem os comentados, na ordem
(o contorno de um editor). Resposta: os nomes.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `text` | `string` | sim | O texto. |

Resultado: lista de `string`.

<a id="metodo-verilog-testbench-names"></a>

#### `verilog.testbench_names`

Quais nomes de arquivo são de testbench pela regra da AURORA
(`tb_<nome>.v`, `<nome>_tb.v`). Resposta: um sim ou não por nome, na
ordem.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `names` | lista de `string` | sim | Os nomes, sem pasta. |

Resultado: lista de `boolean`.

### `history.*`

<a id="metodo-history-list"></a>

#### `history.list`

Os relatórios do projeto, do mais novo para o mais antigo. Resposta:
[`RunSummary`](#tipo-runsummary).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `limit` | `integer` \| `null` | não (padrão: `null`) | Quantos, no máximo. |

Resultado: lista de [`RunSummary`](#tipo-runsummary).

<a id="metodo-history-show"></a>

#### `history.show`

Um relatório (o mais novo sem `id`). Resposta: [`ReportView`](#tipo-reportview).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `id` | `string` \| `null` | não (padrão: `null`) | `run-000042` ou `42`. |

Resultado: [`ReportView`](#tipo-reportview).

<a id="metodo-history-compare"></a>

#### `history.compare`

Compara dois relatórios. Resposta: [`RunComparison`](#tipo-runcomparison).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `id` | `string` \| `null` | não (padrão: `null`) | O atual (o mais novo sem ele). |
| `against` | `string` \| `null` | não (padrão: `null`) | A referência (o anterior comparável sem ela). |

Resultado: [`RunComparison`](#tipo-runcomparison).

<a id="metodo-history-plan-cleanup"></a>

#### `history.plan_cleanup`

Quais relatórios uma limpeza apagaria, sem apagar. Resposta: os
identificadores.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `cleanup` | [`Cleanup`](#tipo-cleanup) | sim | A regra. |

Resultado: lista de `string`.

<a id="metodo-history-clean"></a>

#### `history.clean`

Apaga exatamente estes relatórios. Resposta: [`CleanOutcome`](#tipo-cleanoutcome).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `ids` | lista de `string` | sim | Os identificadores (do plano). |

Resultado: [`CleanOutcome`](#tipo-cleanoutcome).

### `toolchain.*`

<a id="metodo-toolchain-info"></a>

#### `toolchain.info`

O bundle e as ferramentas. Resposta: [`ToolchainInfo`](#tipo-toolchaininfo).

Sem parâmetros.

Resultado: [`ToolchainInfo`](#tipo-toolchaininfo).

<a id="metodo-toolchain-verify"></a>

#### `toolchain.verify`

Confere o SHA-256 dos executáveis. Resposta: os que não conferem
([`FileMismatch`](#tipo-filemismatch); vazio, todos conferem).

Sem parâmetros.

Resultado: lista de [`FileMismatch`](#tipo-filemismatch).

### `experimental.*`

<a id="metodo-experimental-list"></a>

#### `experimental.list`

Os recursos experimentais e se estão ligados. Resposta:
[`ExperimentalInfo`](#tipo-experimentalinfo).

Sem parâmetros.

Resultado: [`ExperimentalInfo`](#tipo-experimentalinfo).

<a id="metodo-experimental-set"></a>

#### `experimental.set`

Liga ou desliga um recurso experimental. Resposta:
[`ExperimentalInfo`](#tipo-experimentalinfo).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `feature` | `string` | sim | O recurso (`fpga`). |
| `enabled` | `boolean` | sim | Ligado. |

Resultado: [`ExperimentalInfo`](#tipo-experimentalinfo).

### `fpga.*`

<a id="metodo-fpga-boards"></a>

#### `fpga.boards`

As placas conhecidas, ou uma. Resposta: [`Board`](#tipo-board).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `board` | `string` \| `null` | não (padrão: `null`) | Uma placa; `None`, todas. |

Resultado: lista de [`Board`](#tipo-board).

<a id="metodo-fpga-config"></a>

#### `fpga.config`

O `fpga.json` do projeto. Resposta: o [`FpgaConfig`](#tipo-fpgaconfig), ou `null`.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |

Resultado: [`FpgaConfig`](#tipo-fpgaconfig) \| `null`.

<a id="metodo-fpga-config-set"></a>

#### `fpga.config_set`

Grava o `fpga.json`. Resposta: nada.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `config` | [`FpgaConfig`](#tipo-fpgaconfig) | sim | A configuração. |

Resultado: `null`.

<a id="metodo-fpga-connect"></a>

#### `fpga.connect`

Muda uma ligação a partir da placa: um bit de um sinal (ou o sinal
inteiro) passa a ir numa porta do topo, ou fica solto; o que estava ali
sai, e as outras ligações continuam. Grava o `fpga.json`; sem ele,
parte das portas ligadas pelo nome com a placa `board`. Resposta: o
[`FpgaConfig`](#tipo-fpgaconfig) gravado.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `board` | `string` \| `null` | não (padrão: `null`) | A placa, num projeto sem `fpga.json`. |
| `change` | [`Reconnect`](#tipo-reconnect) | sim | A mudança. |

Resultado: [`FpgaConfig`](#tipo-fpgaconfig).

<a id="metodo-fpga-check"></a>

#### `fpga.check`

Confere o `fpga.json` contra a placa e o topo. Resposta:
[`Prepared`](#tipo-prepared).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |

Resultado: [`Prepared`](#tipo-prepared).

<a id="metodo-fpga-modules"></a>

#### `fpga.modules`

Os módulos do projeto que podem ir para a placa. Resposta: os
nomes, na ordem dos arquivos.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |

Resultado: lista de `string`.

<a id="metodo-fpga-status"></a>

#### `fpga.status`

O arquivo de gravação da última compilação e se serve. Resposta:
[`BitstreamState`](#tipo-bitstreamstate).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |

Resultado: [`BitstreamState`](#tipo-bitstreamstate).

<a id="metodo-fpga-cables"></a>

#### `fpga.cables`

Os cabos de gravação ligados. Resposta: os nomes.

Sem parâmetros.

Resultado: lista de `string`.

<a id="metodo-fpga-top"></a>

#### `fpga.top`

O módulo que vai para a placa e as portas dele, para montar as
ligações antes de estarem certas. Resposta: o [`ModuleInterface`](#tipo-moduleinterface).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `top` | `string` \| `null` | não (padrão: `null`) | O módulo; `None`, o `top` do `fpga.json`, senão o do projeto. |

Resultado: [`ModuleInterface`](#tipo-moduleinterface).

### `vboard.*`

<a id="metodo-vboard-layout"></a>

#### `vboard.layout`

A placa virtual do projeto sem compilar: a placa, o topo e os
periféricos, ligados ou não. Resposta: [`BoardLayout`](#tipo-boardlayout).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `board` | `string` \| `null` | não (padrão: `null`) | A placa, num projeto sem `fpga.json`: as portas com o nome de um sinal dela ligam a ele. |

Resultado: [`BoardLayout`](#tipo-boardlayout).

<a id="metodo-vboard-run"></a>

#### `vboard.run`

Compila a placa virtual e a roda até `$/cancel` (ou o `$finish` do
projeto). Enquanto compila, `events`; rodando, `vboard_ready` com a
placa, `vboard_state` cerca de 30 vezes por segundo, `vboard_frame`
quando a imagem muda e `vboard_output` a cada linha do projeto. As
entradas vão por `vboard.input` com o id deste pedido. Resposta:
[`BoardRun`](#tipo-boardrun); com o Verilog que não compila, sem `outcome`.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `board` | `string` \| `null` | não (padrão: `null`) | A placa, num projeto sem `fpga.json` (como em `vboard.layout`). |
| `limit` | `number` \| `null` | não (padrão: `null`) | Segundos simulados por segundo de relógio, no máximo: 1 é o tempo real (o padrão), 0 é o mais rápido possível. |
| `fps` | `number` \| `null` | não (padrão: `null`) | Quadros de vídeo por segundo, no máximo (padrão 30). |

Resultado: [`BoardRun`](#tipo-boardrun).

<a id="metodo-vboard-input"></a>

#### `vboard.input`

Uma entrada para a placa virtual de um `vboard.run` que está rodando:
chaves, botões, teclas, texto, pausa, velocidade. Resposta: nada; um
periférico que a placa não tem é erro.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `run` | [`RequestId`](#tipo-requestid) | sim | O id do pedido `vboard.run`, como o cliente o mandou. |
| `input` | [`BoardInput`](#tipo-boardinput) | sim | A entrada. |

Resultado: `null`.

### `update.*`

<a id="metodo-update-check"></a>

#### `update.check`

As versões: esta AURORA, a última release, o bundle, cada componente.
Resposta: o relatório do instalador.

Sem parâmetros.

Resultado: [`UpdateReport`](#tipo-updatereport).

<a id="metodo-update-apply"></a>

#### `update.apply`

Atualiza a instalação. Enquanto roda, `install`; no fim, o
relatório do instalador. Com `updated`, o servidor encerra em
seguida: o binário dele foi trocado.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `full` | `boolean` | não (padrão: `false`) | O instalador inteiro, sem tentar por componentes. |
| `from` | `string` \| `null` | não (padrão: `null`) | Um instalador no disco, sem rede. |
| `reopen_ide` | `boolean` | não (padrão: `false`) | No Windows, quando o assistente de instalação roda: abrir a AURORA Studio quando ele terminar (o `--reopen-ide` da CLI). O Studio pede isso e fecha, para o assistente trocar os arquivos dele. |

Resultado: [`UpdateReport`](#tipo-updatereport).

### `install.*`

<a id="metodo-install-add"></a>

#### `install.add`

Instala componentes do bundle. Enquanto roda, `install`; no fim, o
relatório do instalador. Um nome que não é do bundle é recusado
antes de qualquer download.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `components` | lista de `string` | sim | Os componentes (pelo menos um). |

Resultado: [`InstallReport`](#tipo-installreport).

### `learn.*`

<a id="metodo-learn-check"></a>

#### `learn.check`

Corrige um exercício do `aurora learn` (ou todos). Resposta: o
relatório da correção do aurora-learn.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `root` | `string` | sim | A pasta de exercícios, ou uma pasta dentro dela. |
| `exercise` | `string` \| `null` | não (padrão: `null`) | O exercício; `None`, o atual. |
| `all` | `boolean` | não (padrão: `false`) | Todos os exercícios. |

Resultado: [`LearnCheckOutcome`](#tipo-learncheckoutcome).

<a id="metodo-learn-tracks"></a>

#### `learn.tracks`

De onde vêm as trilhas do `aurora learn` e quais há. Resposta: `{dir,
tracks}`.

Sem parâmetros.

Resultado: [`LearnTracks`](#tipo-learntracks).

<a id="metodo-learn-status"></a>

#### `learn.status`

Abre a pasta de exercícios que contém `root` e a põe em dia com a
trilha. Resposta: o retrato da pasta (o `WorkspaceSnapshot` do
aurora-learn).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `root` | `string` | sim | A pasta de exercícios, ou uma pasta dentro dela. |
| `lang` | `string` \| `null` | não (padrão: `null`) | O idioma dos enunciados, quando a trilha tem tradução. |

Resultado: [`WorkspaceSnapshot`](#tipo-workspacesnapshot).

<a id="metodo-learn-init"></a>

#### `learn.init`

Cria a pasta de exercícios (`aurora learn init`). Resposta: o retrato
da pasta.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `dir` | `string` | sim | A pasta, que pode existir vazia. |
| `track` | `string` \| `null` | não (padrão: `null`) | A trilha; `None`, a padrão. |
| `lang` | `string` \| `null` | não (padrão: `null`) | O idioma dos enunciados. |

Resultado: [`WorkspaceSnapshot`](#tipo-workspacesnapshot).

<a id="metodo-learn-set-current"></a>

#### `learn.set_current`

Torna atual um exercício. Resposta: o retrato da pasta.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `root` | `string` | sim | A pasta de exercícios. |
| `name` | `string` | sim | O exercício. |
| `lang` | `string` \| `null` | não (padrão: `null`) | O idioma dos enunciados. |

Resultado: [`WorkspaceSnapshot`](#tipo-workspacesnapshot).

<a id="metodo-learn-reset"></a>

#### `learn.reset`

Volta o arquivo de um exercício ao começo (`aurora learn reset`).
Resposta: nada.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `root` | `string` | sim | A pasta de exercícios. |
| `name` | `string` | sim | O exercício. |

Resultado: `null`.

### `git.*`

<a id="metodo-git-status"></a>

#### `git.status`

O git da máquina e o repositório que contém o projeto: ramo,
arquivos mudados, identidade e remotos. Fora de repositório,
`repository` vem `null`. Resposta: [`GitStatus`](#tipo-gitstatus).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |

Resultado: [`GitStatus`](#tipo-gitstatus).

<a id="metodo-git-diff"></a>

#### `git.diff`

O diff unificado do que mudou desde o último commit, num arquivo ou
em todos. Resposta: o texto.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `path` | `string` \| `null` | não (padrão: `null`) | Um arquivo, relativo ao topo do repositório, com `/`; `None`, todos. |

Resultado: `string`.

<a id="metodo-git-read-file"></a>

#### `git.read_file`

O conteúdo de um arquivo no índice, no último commit ou num commit,
para comparar com o do disco. Resposta: [`FileVersion`](#tipo-fileversion).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `revision` | [`Revision`](#tipo-revision) | sim | A versão. |
| `path` | `string` | sim | O arquivo, relativo ao topo do repositório, com `/`. |

Resultado: [`FileVersion`](#tipo-fileversion).

<a id="metodo-git-commit"></a>

#### `git.commit`

Faz o commit dos arquivos escolhidos, no estado em que estão no
disco. Resposta: [`CommitResult`](#tipo-commitresult).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `summary` | `string` | sim | A primeira linha da mensagem. |
| `description` | `string` \| `null` | não (padrão: `null`) | O resto da mensagem. |
| `paths` | lista de `string` \| `null` | não (padrão: `null`) | Os arquivos, relativos ao topo, com `/`; `None`, todos os mudados. |
| `amend` | `boolean` | não (padrão: `false`) | Refaz o último commit. |

Resultado: [`CommitResult`](#tipo-commitresult).

<a id="metodo-git-discard"></a>

#### `git.discard`

Descarta as mudanças de arquivos: voltam ao último commit, e os novos
saem; a versão descartada vai para a lixeira do sistema. Resposta:
[`DiscardResult`](#tipo-discardresult).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `paths` | lista de `string` | sim | Os arquivos, relativos ao topo, com `/`. |

Resultado: [`DiscardResult`](#tipo-discardresult).

<a id="metodo-git-init"></a>

#### `git.init`

Cria um repositório na pasta do projeto, com um `.gitignore` para o
que a AURORA gera. Resposta: [`InitResult`](#tipo-initresult).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |

Resultado: [`InitResult`](#tipo-initresult).

<a id="metodo-git-log"></a>

#### `git.log`

Uma página do histórico do ramo atual. Resposta: [`GitLog`](#tipo-gitlog).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `skip` | `integer` \| `null` | não (padrão: `null`) | Quantos commits pular, dos mais novos. |
| `limit` | `integer` \| `null` | não (padrão: `null`) | Quantos devolver (até 500; 50 sem o campo). |

Resultado: [`GitLog`](#tipo-gitlog).

<a id="metodo-git-commit-files"></a>

#### `git.commit_files`

Os arquivos que um commit mudou. Resposta: [`CommitFiles`](#tipo-commitfiles).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `oid` | `string` | sim | O identificador do commit. |

Resultado: [`CommitFiles`](#tipo-commitfiles).

<a id="metodo-git-branches"></a>

#### `git.branches`

Os ramos locais e remotos. Resposta: [`Branches`](#tipo-branches).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |

Resultado: [`Branches`](#tipo-branches).

<a id="metodo-git-switch"></a>

#### `git.switch`

Troca de ramo, ou cria um. Resposta: o ramo atual depois da troca.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `branch` | `string` | sim | Um ramo local, um remoto (`origin/x`) ou, com `create`, o novo. |
| `create` | `boolean` | não (padrão: `false`) | Cria o ramo a partir do commit atual. |

Resultado: `string`.

<a id="metodo-git-fetch"></a>

#### `git.fetch`

Traz do remoto os ramos e commits novos, sem mexer nos arquivos.
Resposta: [`SyncResult`](#tipo-syncresult).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |

Resultado: [`SyncResult`](#tipo-syncresult).

<a id="metodo-git-pull"></a>

#### `git.pull`

Traz os commits do ramo remoto, só em avanço direto. Resposta:
[`SyncResult`](#tipo-syncresult).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |

Resultado: [`SyncResult`](#tipo-syncresult).

<a id="metodo-git-push"></a>

#### `git.push`

Envia o ramo atual; sem ramo remoto, publica. Resposta:
[`SyncResult`](#tipo-syncresult).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |

Resultado: [`SyncResult`](#tipo-syncresult).

<a id="metodo-git-set-identity"></a>

#### `git.set_identity`

Grava o nome e o e-mail dos commits. Resposta: nada.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `name` | `string` | sim | O nome. |
| `email` | `string` | sim | O e-mail. |
| `global` | `boolean` | não (padrão: `false`) | Na configuração global do git, para todos os repositórios desta máquina; sem o campo, só neste. |

Resultado: `null`.

<a id="metodo-git-set-remote"></a>

#### `git.set_remote`

Dá um endereço a um remoto: cria o remoto ou troca o endereço de um
que já existe. Resposta: nada.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `name` | `string` | sim | O nome (`origin`). |
| `url` | `string` | sim | O endereço. |

Resultado: `null`.

<a id="metodo-git-remove-remote"></a>

#### `git.remove_remote`

Tira um remoto. Resposta: nada.

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `project` | `string` | sim | O projeto. |
| `name` | `string` | sim | O nome. |

Resultado: `null`.

### `github.*`

<a id="metodo-github-account"></a>

#### `github.account`

A conta do GitHub que o git desta máquina conhece. Resposta:
[`Account`](#tipo-account), ou `null`.

Sem parâmetros.

Resultado: [`Account`](#tipo-account) \| `null`.

<a id="metodo-github-login-start"></a>

#### `github.login_start`

Começa o login no GitHub: o código que a pessoa digita em
github.com/login/device. Resposta: [`DeviceCode`](#tipo-devicecode).

Sem parâmetros.

Resultado: [`DeviceCode`](#tipo-devicecode).

<a id="metodo-github-login-poll"></a>

#### `github.login_poll`

Pergunta se a pessoa já autorizou o login; com a autorização, guarda o
token no gerenciador de credenciais do git. Resposta: [`LoginPoll`](#tipo-loginpoll).

| Parâmetro | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `device_code` | `string` | sim | O `device_code` de `github.login_start`. |

Resultado: [`LoginPoll`](#tipo-loginpoll).

<a id="metodo-github-logout"></a>

#### `github.logout`

Sai do GitHub: tira o token do gerenciador de credenciais do git.
Resposta: se havia um.

Sem parâmetros.

Resultado: `boolean`.

## Notificações do servidor

Enquanto um pedido roda, o servidor manda notificações dele, sempre antes da resposta, com o `id` do pedido em `params.id`: `{"jsonrpc": "2.0", "method": "phase", "params": {"id": 2, "phase": "build"}}`.

### `phase`

Uma fase começou.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `phase` | [`Phase`](#tipo-phase) | sim | Qual. |

### `events`

Eventos do core, na ordem em que aconteceram.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `events` | lista de [`Event`](#tipo-event) | sim | Os eventos (`step_started`, `output`, `step_finished`). |

### `output_omitted`

Linhas de saída que ficaram de fora (`Throttle`), no lugar onde
estariam: antes da próxima linha que passou ou do fim do passo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `step` | [`Step`](#tipo-step) | sim | O passo que as escreveu. |
| `tool` | [`Tool`](#tipo-tool) | sim | A ferramenta. |
| `lines` | `integer` | sim | Quantas. |

### `build`

Um processador terminou de compilar.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `result` | [`BuildResult`](#tipo-buildresult) | sim | O resultado. |

### `vboard_ready`

A placa virtual subiu (`crate::virtual_board::run`): a placa, o topo
e os periféricos.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `layout` | [`BoardLayout`](#tipo-boardlayout) | sim | A placa virtual. |

### `vboard_state`

O estado da placa virtual: LEDs, segmentos e UART.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `state` | [`BoardState`](#tipo-boardstate) | sim | O estado. |

### `vboard_frame`

Um quadro de vídeo novo da placa virtual.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `frame` | [`Frame`](#tipo-frame) | sim | O quadro, em PNG. |

### `vboard_output`

As linhas que o projeto escreveu na placa virtual (`$display`), em
lotes de no máximo 100, um por estado; as que passam disso são só
contadas.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `lines` | lista de `string` | sim | As linhas, na ordem. |
| `omitted` | `integer` | sim | Quantas ficaram de fora deste lote, depois de `lines`. |

### `install`

Progresso de uma instalação ou atualização da AURORA (o evento do
instalador, serializado por ele; o core não conhece o tipo).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `event` | qualquer JSON | sim | O evento, em JSON. |
| `line` | `string` \| `null` | não | A linha do console para o evento, com os nomes dos componentes; ausente no que é só barra de progresso. |

### `shutdown`

O servidor vai encerrar: depois do `shutdown`, quando a entrada fecha, num sinal ou depois de um `update.apply` que trocou o binário. Vem sem `id`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `reason` | [`ShutdownReason`](#tipo-shutdownreason) | sim | Por quê. |

## Notificação do cliente

### `$/cancel`

Cancela um pedido: `{"jsonrpc": "2.0", "method": "$/cancel", "params": {"id": 2}}`. Um pedido que ainda não começou responde o erro `-32001`; um fluxo em andamento para e responde o resultado com `status: "cancelled"`.

## Erros

| `error.code` | Quando |
|---|---|
| `-32700` | a linha não é JSON |
| `-32600` | o JSON não é um pedido JSON-RPC 2.0 |
| `-32601` | método desconhecido |
| `-32602` | parâmetros que o método não aceita |
| `-32000` | a AURORA não conseguiu atender: o `ErrorInfo` vai em `error.data` |
| `-32001` | cancelado com `$/cancel` antes de começar |
| `-32002` | pedido antes do `initialize` |
| `-32003` | pedido depois do `shutdown` |

No `-32000`, `error.data` é um [`ErrorInfo`](#tipo-errorinfo): os valores de `code` estão em [API.md, seção 8](API.md#8-erros).

## Tipos

Em ordem alfabética. Num tipo que o cliente manda, obrigatório é o campo que ele precisa mandar; num que ele recebe, o campo que sempre vem, mesmo que `null`.

<a id="tipo-account"></a>

### `Account`

A conta do GitHub conectada.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `login` | `string` | sim | O nome de usuário (`octocat`). |
| `id` | `integer` | sim | O identificador numérico. |
| `name` | `string` \| `null` | sim | O nome, se a pessoa preencheu. |
| `avatar_url` | `string` | sim | A foto. |
| `html_url` | `string` | sim | A página do perfil. |
| `commit_email` | `string` | sim | O e-mail para os commits que o GitHub liga à conta sem expor o e-mail real (`123+octocat@users.noreply.github.com`). |
| `saved` | `boolean` | sim | O gerenciador de credenciais do git guardou o token: o login vale depois de fechar a AURORA, e o `git push` do terminal também o usa. Falso quando o git desta máquina não tem gerenciador de credenciais. |

<a id="tipo-addedfile"></a>

### `AddedFile`

O que `Project::add_hdl` fez com um arquivo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `path` | `string` | sim | Caminho absoluto. |
| `role` | [`FileRole`](#tipo-filerole) | sim | Em qual lista ficou. |
| `language` | [`SourceLanguage`](#tipo-sourcelanguage) \| `null` | sim | A linguagem, pela extensão. |
| `created` | `boolean` | sim | O arquivo não existia e foi criado a partir do modelo. |
| `selected` | `boolean` | sim | Virou o módulo de topo (era o primeiro sintetizável) ou o testbench escolhido. |

<a id="tipo-artifact"></a>

### `Artifact`

Um arquivo que a operação deveria produzir.

`fresh` é calculado comparando o horário de modificação antes e depois da
operação. Um `.v` que sobrou de um build anterior aparece com
`fresh = false`, e não como sucesso. Artefatos obrigatórios aparecem
sempre; intermediários, só se existirem.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `kind` | [`ArtifactKind`](#tipo-artifactkind) | sim | O papel do arquivo. |
| `path` | `string` | sim | Caminho absoluto. |
| `required` | `boolean` | sim | Sem ele a operação não está completa. |
| `fresh` | `boolean` | sim | Existe e foi escrito por esta operação (não é sobra de uma anterior). O executável do Verilator, que o `make` só refaz quando o modelo muda, conta como escrito quando a compilação terminou bem: ele está em dia com a entrada. |

<a id="tipo-artifactkind"></a>

### `ArtifactKind`

O papel de um arquivo produzido por uma operação. Em JSON, em
`snake_case` (`"data_memory"`).

| Valor | Descrição |
|---|---|
| `"assembly"` | `<nome>/<nome>/<nome>.asm`, na pasta de saída do processador. |
| `"verilog"` | `<nome>/<nome>/<nome>.v`, o processador sintetizável. |
| `"data_memory"` | `<nome>/<nome>/<nome>_data.mif` |
| `"instruction_memory"` | `<nome>/<nome>/<nome>_inst.mif` |
| `"testbench"` | `<nome>/<nome>/<nome>_tb.v`: o testbench que o `asmcomp` gera, que roda na pasta de saída. |
| `"preprocessed_source"` | `<temp>/pp.cpp`, saída do `cpppp` do YANC até a 6.x, nos relatórios antigos. |
| `"program_counter_map"` | `<nome>/<nome>/pc_<nome>_mem.txt`, liga endereço de instrução a linha do fonte. |
| `"source_translation"` | `<nome>/<nome>/src_lines.txt`: o texto de cada linha do fonte. |
| `"opcode_translation"` | `<nome>/<nome>/trad_opcode.txt` |
| `"compiler_log"` | `<nome>/<nome>/prog_log.txt`: as variáveis, o número de instruções e a pasta do fonte. |
| `"pre_assembler_log"` | `<nome>/<nome>/app_log.txt` |
| `"icarus_image"` | O `.vvp` do Icarus. |
| `"verilated_model"` | O executável que o Verilator gerou. |
| `"waveform"` | Onda da simulação (VCD ou FST). |
| `"simulation_output"` | `<nome>/<nome>/output_<n>.txt`, escrito pelo testbench. |
| `"netlist"` | Netlist JSON do Yosys. |
| `"schematic_graph"` | Grafo do esquemático (`.dot`), gerado pelo Yosys. |
| `"schematic"` | Esquemático SVG, desenhado pelo `dot`. |
| `"synthesis_statistics"` | As estatísticas do `stat -json` do Yosys (`stat.json`). |
| `"board_top"` | O topo da placa que a AURORA gera (`aurora_board_top.v`). |
| `"quartus_project"` | As configurações do projeto do Quartus (`.qsf`): a FPGA, os fontes e os pinos. |
| `"sram_object"` | O arquivo de gravação do Quartus (`.sof`, SRAM Object File), que o `quartus_pgm` grava. |
| `"raw_binary"` | O bitstream cru, sem compressão (`.rbf`, Raw Binary File), que o openFPGALoader grava. |
| `"serial_vector_format"` | A gravação como vetores JTAG (`.svf`, Serial Vector Format), que o openFPGALoader também grava. |

<a id="tipo-auroraversions"></a>

### `AuroraVersions`

As versões da AURORA em `aurora update`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `installed` | `string` | sim | A versão deste `aurora`. |
| `latest` | `string` | sim | A versão da última release no GitHub. |
| `newer` | `boolean` | sim | `latest` é mais nova que `installed`. |

<a id="tipo-availability"></a>

### `Availability`

Quanto de uma parte um relatório tem.

| Valor | Descrição |
|---|---|
| `"available"` | Tudo. |
| `"partial"` | Uma parte (uma simulação que não compilou não tem tempo de execução). |
| `"unavailable"` | Nada. |

<a id="tipo-bitsource"></a>

### `BitSource`

De onde vem um bit de entrada.

Uma das formas:

**`kind: "pin"`**

Um bit de um sinal da placa, invertido ou não.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `signal` | `string` | sim | O sinal. |
| `bit` | `integer` | sim | O bit. |
| `invert` | `boolean` | sim | Invertido (`!`). |

**`kind: "constant"`**

Um valor fixo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `value` | `boolean` | sim | O valor. |

<a id="tipo-bitstreamstate"></a>

### `BitstreamState`

O arquivo de gravação da placa do `fpga.json`: se existe e, se existe,
por que não descreve o projeto de agora (`bitstream_state`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `board` | `string` | sim | A placa (o `id`). |
| `bitstream` | `string` \| `null` | sim | O `.sof`, se a placa já foi compilada. |
| `built_at_ms` | `integer` \| `null` | sim | Quando o `.sof` foi gravado, em milissegundos desde 1970. |
| `reasons` | lista de `string` | sim | Por que o `.sof` não serve para gravar, uma razão por item: vazio com `bitstream`, ele descreve o projeto de agora. |

<a id="tipo-board"></a>

### `Board`

Uma placa.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `id` | `string` | sim | O identificador, usado no `fpga.json` e na CLI (`de2-115`). |
| `name` | `string` | sim | O nome para mostrar (`Terasic DE2-115`). |
| `source` | `string` | sim | De onde vieram os pinos: o manual, a versão e as tabelas. |
| `device` | [`Device`](#tipo-device) | sim | O FPGA. |
| `jtag` | [`Jtag`](#tipo-jtag) | sim | A gravação pela USB da placa. |
| `signals` | lista de [`BoardSignal`](#tipo-boardsignal) | sim | Os sinais da placa ligados ao FPGA. |
| `peripherals` | lista de [`Peripheral`](#tipo-peripheral) | não | O que cada sinal é na placa (chaves, LEDs, displays, tela), para a placa virtual desenhá-los e simulá-los. Um sinal fora daqui também vai para a placa; só não aparece na placa virtual. |

<a id="tipo-boardbuild"></a>

### `BoardBuild`

O resultado de `build`: a placa e o executável, ou os erros do
Verilator e do compilador C++.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `status` | [`Status`](#tipo-status) | sim | Como terminou. `failed` é Verilog (ou ligação) que não compila. |
| `failed_step` | [`Step`](#tipo-step) \| `null` | sim | O passo que falhou, quando for o caso (`verilate`). |
| `steps` | lista de [`StepReport`](#tipo-stepreport) | sim | Os passos rodados, com o comando e a saída. |
| `diagnostics` | lista de [`Diagnostic`](#tipo-diagnostic) | sim | As mensagens do YANC, do Verilator e do compilador. |
| `processors` | lista de [`BuildResult`](#tipo-buildresult) | sim | Os processadores SAPHO compilados antes da placa, os desatualizados (`crate::Processor::is_out_of_date`); vazio quando todos estavam em dia. |
| `layout` | [`BoardLayout`](#tipo-boardlayout) \| `null` | sim | A placa virtual do projeto; `None` quando um processador não compilou, antes de o topo poder ser lido. |
| `executable` | `string` \| `null` | sim | O executável, se a compilação deu certo. |
| `duration_ms` | `integer` | sim | Quanto levou, em milissegundos. |

<a id="tipo-boardinput"></a>

### `BoardInput`

Uma entrada para a placa virtual em execução.

Uma das formas:

**`kind: "set"`**

O valor de chaves (bit 1 = chave para cima) ou dos botões apertados
(bit 1 = apertado; a polaridade da placa é aplicada no pino).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `device` | `string` | sim | O periférico (`SW`, `KEY`). |
| `value` | `integer` | sim | Os bits, do bit 0 em diante. |

**`kind: "key"`**

Uma tecla do teclado do computador, apertada ou solta, para o teclado
PS/2. O código é o `KeyboardEvent.code` dos navegadores (`KeyA`,
`Enter`, `ArrowUp`, `IntlRo`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `device` | `string` | sim | O periférico PS/2. |
| `code` | `string` | sim | O código da tecla. |
| `pressed` | `boolean` | sim | Apertada (`true`) ou solta (`false`). |

**`kind: "text"`**

Texto: no teclado PS/2, cada caractere vira as teclas que o digitam
num teclado americano (letras, números, espaço, Enter e a pontuação
comum); na UART, os bytes em UTF-8.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `device` | `string` | sim | O teclado PS/2 ou a UART. |
| `text` | `string` | sim | O texto. |

**`kind: "bytes"`**

Bytes crus: códigos de tecla do PS/2 ou bytes para o RX da UART.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `device` | `string` | sim | O teclado PS/2 ou a UART. |
| `bytes` | lista de `integer` | sim | Os bytes. |

**`kind: "pause"`**

Para a simulação, com o estado atual na tela.

Sem campos.

**`kind: "resume"`**

Continua depois de `BoardInput::Pause` ou `BoardInput::Step`.

Sem campos.

**`kind: "step"`**

Roda esse número de flancos de relógio e para.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `edges` | `integer` | sim | Quantos flancos (dois por ciclo de um relógio). |

**`kind: "speed"`**

Limita a velocidade: 1 é o tempo real, 0 é o mais rápido possível.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `limit` | `number` | sim | Segundos simulados por segundo de relógio; 0 sem limite. |

<a id="tipo-boardlayout"></a>

### `BoardLayout`

A placa virtual de um projeto: a placa, o topo e cada periférico, ligado
ao projeto ou não, com as larguras. É o que uma interface desenha.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `board` | `string` | sim | A placa (`de2-115`). |
| `board_name` | `string` | sim | O nome da placa (`Terasic DE2-115`). |
| `top` | `string` | sim | O módulo do projeto que vai na placa. |
| `peripherals` | lista de [`LayoutPeripheral`](#tipo-layoutperipheral) | sim | Os periféricos, na ordem do JSON da placa. |
| `clocks` | lista de [`LayoutClock`](#tipo-layoutclock) | sim | Os relógios da placa que chegam ao projeto. |
| `notes` | lista de `string` | sim | Os ajustes das ligações (larguras completadas ou cortadas, entradas soltas), como no fluxo da placa. |

<a id="tipo-boardrun"></a>

### `BoardRun`

A placa virtual compilada e rodada: a compilação e, se ela deu certo,
como a execução terminou.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `build` | [`BoardBuild`](#tipo-boardbuild) | sim | A compilação. |
| `outcome` | [`SessionOutcome`](#tipo-sessionoutcome) \| `null` | sim | Como a execução terminou; `None` se a compilação falhou. |

<a id="tipo-boardsignal"></a>

### `BoardSignal`

Um sinal da placa: um fio (`CLOCK_50`) ou um barramento (`SW`), com um
pino por bit.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome, como no manual (`KEY`, `LEDR`). |
| `direction` | [`SignalDirection`](#tipo-signaldirection) | sim | A direção. |
| `pins` | lista de `string` | sim | Os pinos, do bit 0 em diante (`PIN_Y2`). |
| `io_standard` | [`IoStandard`](#tipo-iostandard) | sim | O padrão de I/O, como no `.qsf` (`3.3-V LVTTL`): um para o sinal inteiro ou um por bit, quando os bits ficam em bancos de tensões diferentes. |
| `active_low` | `boolean` | sim (padrão: `false`) | Ativo em nível baixo: o botão apertado lê 0, o segmento acende em 0. Num sinal de saída, os bits que nada liga ficam no nível inativo. |
| `clock_mhz` | `number` \| `null` | não | A frequência, se for um clock. |
| `description` | `string` | sim | O que é, como no manual. |

<a id="tipo-boardstate"></a>

### `BoardState`

O estado da placa virtual, cerca de 30 vezes por segundo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `time_ps` | `integer` | sim | O tempo simulado, em picossegundos. |
| `speed` | `number` | sim | Segundos simulados por segundo de relógio desde o estado anterior: 1 é o tempo real. |
| `paused` | `boolean` | sim | A simulação está parada (`BoardInput::Pause`). |
| `lights` | mapa de `string` para lista de `integer` | sim | Quanto cada LED e cada segmento ficou aceso desde o estado anterior, de 0 a 255, por sinal, do bit 0 em diante: o PWM aparece como brilho, e o display multiplexado como persistência. |
| `uart` | mapa de `string` para lista de `integer` | sim | Os bytes que o projeto transmitiu em cada UART desde o estado anterior. |
| `uart_baud` | mapa de `string` para `integer` | não | A velocidade de cada UART que não está na da placa: o projeto transmitia em outra, e a placa passou a ler na padrão mais próxima do bit medido (os primeiros bytes se perdem). Vazio na velocidade da placa. |

<a id="tipo-branchstate"></a>

### `BranchState`

O ramo atual e a relação com o remoto.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` \| `null` | sim | O nome; `None` com o HEAD fora de ramo. |
| `commit` | `string` \| `null` | sim | O commit atual; `None` num repositório ainda sem commit. |
| `upstream` | `string` \| `null` | sim | O ramo remoto que ele acompanha (`origin/main`); `None` se ainda não foi publicado. |
| `ahead` | `integer` | sim | Commits locais que o remoto não tem. |
| `behind` | `integer` | sim | Commits do remoto que o local não tem (pelo último fetch). |
| `remote` | `string` \| `null` | sim | O remoto para onde o ramo vai ao publicar: o configurado no ramo, o `origin` ou o único que houver. |

<a id="tipo-branches"></a>

### `Branches`

`git.branches`: os ramos.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `current` | `string` \| `null` | sim | O atual; `None` com o HEAD fora de ramo. |
| `local` | lista de [`LocalBranch`](#tipo-localbranch) | sim | Os locais, do usado mais recentemente para o mais antigo. |
| `remote` | lista de [`RemoteBranch`](#tipo-remotebranch) | sim | Os remotos sem ramo local com o mesmo nome. |

<a id="tipo-buildresult"></a>

### `BuildResult`

O resultado de `build`. Ver o módulo `pipeline` para os campos comuns a
toda operação.

Em JSON (resumido):

```json
{ "processor": "soma", "language": "cmm", "status": "succeeded",
  "failed_step": null, "frequency_mhz": 100, "clocks": 2000,
  "steps": [ { "step": "compile", "tool": "cmmcomp", "...": "..." } ],
  "diagnostics": [],
  "artifacts": [ { "kind": "verilog", "path": "/p/soma/soma/soma.v",
                   "required": true, "fresh": true } ] }
```

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `processor` | `string` | sim | O processador compilado. |
| `language` | [`Language`](#tipo-language) | sim | A linguagem do fonte, que decidiu o pipeline. |
| `status` | [`Status`](#tipo-status) | sim | Como o build terminou. |
| `failed_step` | [`Step`](#tipo-step) \| `null` | sim | O passo que falhou ou quebrou, quando for o caso. |
| `frequency_mhz` | `integer` | sim | A frequência usada (do `.spf` ou de `BuildOptions`). |
| `clocks` | `integer` | sim | Os clocks usados (do `.spf` ou de `BuildOptions`). |
| `steps` | lista de [`StepReport`](#tipo-stepreport) | sim | Um relatório por compilador executado: `compile` e `assemble` (`build_steps`). |
| `diagnostics` | lista de [`Diagnostic`](#tipo-diagnostic) | sim | Todas as mensagens de todos os passos, na ordem em que saíram. Hoje os compiladores param no primeiro erro, mas a API já é uma lista. |
| `artifacts` | lista de [`Artifact`](#tipo-artifact) | sim | Obrigatórios, na pasta de saída: `<nome>.asm`, `<nome>.v`, `<nome>_data.mif`, `<nome>_inst.mif` e o testbench `<nome>_tb.v`. Intermediários (`pc_<nome>_mem.txt`, `src_lines.txt`, `trad_opcode.txt`, `prog_log.txt`, `app_log.txt`), quando existem. |
| `duration_ms` | `integer` | sim | Quanto o build levou, do começo ao fim, em milissegundos. |

<a id="tipo-bundlecomponent"></a>

### `BundleComponent`

Um componente instalado (`components/<nome>.json`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | Nome (ver o módulo `component`); é também o nome do arquivo. |
| `version` | `string` | sim | Versão exata: data da release do OSS CAD Suite, tag do YANC, do surfer-aurora, do Graphviz. |
| `dir` | `string` | sim | Diretório do componente, relativo ao bundle. Componentes do OSS CAD Suite dividem o mesmo. |
| `source` | `string` | sim | De onde veio: URL do pacote, ou `git+<repositório>@<commit>` quando o empacotamento compilou do fonte. |
| `sha256` | `string` \| `null` | sim (padrão: `null`) | SHA-256 do pacote de origem, quando foi baixado pronto. |
| `files` | mapa de `string` para `string` | sim (padrão: `{}`) | SHA-256 dos executáveis do componente que a AURORA roda, por caminho relativo ao bundle, conferido por `Toolchain::verify`. |

<a id="tipo-bundleversions"></a>

### `BundleVersions`

As versões do bundle em `aurora update`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `installed` | `string` \| `null` | sim | O bundle instalado; `null` sem bundle. |
| `latest` | `string` \| `null` | sim | O bundle da última release; `null` se o `bundle/versions.json` dela não pôde ser lido. |
| `newer` | `boolean` | sim | `latest` é mais novo que `installed`. |

<a id="tipo-cable"></a>

### `Cable`

O cabo de gravação embutido na placa.

| Valor | Descrição |
|---|---|
| `"usb-blaster"` | USB-Blaster (FT245 e um CPLD). |
| `"usb-blasterII"` | USB-Blaster II (EZ-USB FX2). O openFPGALoader só o usa com o firmware `blaster_6810.hex`, que vem com o Quartus. |

<a id="tipo-cellcomparison"></a>

### `CellComparison`

Um tipo de célula, comparado.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `cell_type` | `string` | sim | O tipo (`$add`). |
| `usage` | [`MetricComparison`](#tipo-metriccomparison) | sim | A comparação. Um tipo que só existe de um lado conta como 0 do outro, com `added` ou `removed`. |

<a id="tipo-cellusage"></a>

### `CellUsage`

Quantas células de um tipo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `cell_type` | `string` | sim | O tipo, como o Yosys o nomeia (`$add`, `$dff`). |
| `count` | `integer` | sim | Quantas. |

<a id="tipo-change"></a>

### `Change`

O que aconteceu com um número de um relatório para o outro.

| Valor | Descrição |
|---|---|
| `"unchanged"` | Igual. |
| `"increased"` | Maior. |
| `"decreased"` | Menor. |
| `"added"` | Só existe no atual (um tipo de célula novo). |
| `"removed"` | Só existe na referência (um tipo de célula que sumiu). |
| `"not_comparable"` | Falta de um dos lados. |

<a id="tipo-change2"></a>

### `Change2`

O tipo de mudança de um arquivo.

| Valor | Descrição |
|---|---|
| `"added"` | Novo, já conhecido do git (preparado). |
| `"untracked"` | Novo, que o git ainda não acompanha. |
| `"modified"` | Modificado. |
| `"deleted"` | Apagado. |
| `"renamed"` | Renomeado. |
| `"copied"` | Copiado. |
| `"type_changed"` | Mudou de tipo (arquivo virou link, por exemplo). |
| `"conflicted"` | Com conflito de merge. |

<a id="tipo-changedfile"></a>

### `ChangedFile`

Um arquivo com mudança.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `path` | `string` | sim | O caminho relativo ao topo do repositório, com `/`. |
| `original` | `string` \| `null` | sim | O caminho anterior, num arquivo renomeado. |
| `change` | [`Change2`](#tipo-change2) | sim | A mudança em relação ao último commit, juntando o que já foi preparado (o índice) e o que não foi. |
| `staged` | `boolean` | sim | Tem mudança preparada no índice (por um `git add` no terminal). |
| `unstaged` | `boolean` | sim | Tem mudança ainda não preparada. |
| `in_project` | `boolean` | sim | Está dentro da pasta do projeto. |

<a id="tipo-chaptersnapshot"></a>

### `ChapterSnapshot`

Um capítulo de [`WorkspaceSnapshot`](#tipo-workspacesnapshot).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `id` | `string` | sim | A pasta do capítulo (`01_primeiros_passos`). |
| `title` | `string` | sim | O título. |
| `intro` | `string` | sim | A introdução, em markdown. |
| `exercises` | lista de [`ExerciseSnapshot`](#tipo-exercisesnapshot) | sim | Os exercícios, na ordem. |

<a id="tipo-checkresult"></a>

### `CheckResult`

O resultado de `check`. Não há artefatos: a verificação só diz se o
Verilog elabora, e o que os compiladores acharam.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `targets` | lista de `string` | sim | Os módulos elaborados como raiz: os do projeto (ou os do arquivo pedido) e cada testbench. |
| `status` | [`Status`](#tipo-status) | sim | `Succeeded` se tudo elabora sem erro. |
| `failed_step` | [`Step`](#tipo-step) \| `null` | sim | O primeiro passo que falhou. |
| `steps` | lista de [`StepReport`](#tipo-stepreport) | sim | Um `iverilog -t null` para o projeto, um por testbench e, com `lint`, o `verilator --lint-only`. |
| `diagnostics` | lista de [`Diagnostic`](#tipo-diagnostic) | sim | Os erros e avisos, com arquivo e linha. |
| `duration_ms` | `integer` | sim | Quanto a verificação levou, do começo ao fim, em milissegundos. |

<a id="tipo-cleanoutcome"></a>

### `CleanOutcome`

`history.clean`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `removed` | lista de `string` | sim | Os apagados, do mais antigo para o mais novo. |
| `kept` | `integer` | sim | Quantos ficaram. |

<a id="tipo-cleanup"></a>

### `Cleanup`

Quais relatórios `plan_cleanup` escolhe para apagar. Em JSON:
`{"kind": "all"}`, `{"kind": "keep_latest", "value": 5}`,
`{"kind": "reports", "value": ["run-000042", "7"]}`.

Uma das formas:

**`kind: "all"`**

Todos.

Sem campos.

**`kind: "keep_latest"`**

Todos menos os `n` mais novos.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `value` | `integer` | sim |  |

**`kind: "reports"`**

Estes, como o usuário escreveu (`run-000042` ou `42`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `value` | lista de `string` | sim |  |

<a id="tipo-clock"></a>

### `Clock`

Uma porta ligada a um clock da placa: vai para o `.sdc`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `signal` | `string` | sim | O sinal da placa. |
| `port` | `string` | sim | A porta do topo. |
| `mhz` | `number` | sim | A frequência. |

<a id="tipo-clocktiming"></a>

### `ClockTiming`

O tempo de um clock, no pior dos cantos de operação (tensão e
temperatura) que o Timing Analyzer analisa.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `clock` | `string` | sim | O clock: o sinal da placa, pelo nome do `create_clock`, ou o que o Quartus achou que é clock (um registrador que dirige um `always`). |
| `target_mhz` | `number` \| `null` | sim | A frequência do oscilador da placa, em MHz: a que o `.sdc` pede. `None` num clock que não é da placa. |
| `fmax_mhz` | `number` \| `null` | sim | A maior frequência em que o design funciona com esse clock (a `Restricted Fmax`), em MHz. |
| `setup_slack_ns` | `number` \| `null` | sim | A folga de setup, em ns. Negativa: o design não alcança a frequência do clock. |
| `hold_slack_ns` | `number` \| `null` | sim | A folga de hold, em ns. |

<a id="tipo-commitfile"></a>

### `CommitFile`

Um arquivo mudado por um commit.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `path` | `string` | sim | O caminho, relativo ao topo, com `/`. |
| `original` | `string` \| `null` | sim | O caminho anterior, num arquivo renomeado. |
| `change` | [`Change2`](#tipo-change2) | sim | A mudança. |

<a id="tipo-commitfiles"></a>

### `CommitFiles`

`git.commit_files`: os arquivos que um commit mudou.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `oid` | `string` | sim | O commit. |
| `parent` | `string` \| `null` | sim | O pai com que ele é comparado (o primeiro, num merge); `None` no primeiro commit. |
| `files` | lista de [`CommitFile`](#tipo-commitfile) | sim | Os arquivos. |

<a id="tipo-commitresult"></a>

### `CommitResult`

`git.commit`: o commit criado.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `oid` | `string` | sim | O identificador completo. |
| `short` | `string` | sim | O identificador curto. |
| `summary` | `string` | sim | A primeira linha da mensagem. |
| `branch` | `string` \| `null` | sim | O ramo; `None` com o HEAD fora de ramo. |

<a id="tipo-commitsummary"></a>

### `CommitSummary`

Um commit do histórico.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `oid` | `string` | sim | O identificador completo. |
| `short` | `string` | sim | O identificador curto. |
| `parents` | lista de `string` | sim | Os pais (dois num merge, nenhum no primeiro commit). |
| `author` | `string` | sim | O autor. |
| `email` | `string` | sim | O e-mail do autor. |
| `time` | `integer` | sim | A data do autor, em segundos desde 1970 (UTC). |
| `summary` | `string` | sim | A primeira linha da mensagem. |
| `body` | `string` | sim | O resto da mensagem. |
| `refs` | lista de `string` | sim | Os ramos e tags que apontam para ele (`origin/main`, `tag: v1.0`). |
| `local` | `boolean` | sim | Ainda não está em nenhum ramo remoto: só existe nesta máquina. |

<a id="tipo-componentversions"></a>

### `ComponentVersions`

Um componente instalado em `aurora update`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O componente (`yanc`, `icarus`, `verilator`, `yosys`, `graphviz`, `surfer-aurora`). |
| `package` | `string` \| `null` | sim | O pacote de `bundle/versions.json` de onde ele sai (`oss-cad-suite`, `yanc`, `surfer-aurora`, `graphviz`); `null` para um componente que esta AURORA não conhece. |
| `installed` | `string` | sim | A versão instalada. |
| `release` | `string` \| `null` | sim | A versão no bundle da última release; `null` se não pôde ser lida. |
| `release_newer` | `boolean` | sim | `release` é mais nova que `installed`: chega atualizando a AURORA. |
| `upstream` | `string` \| `null` | sim | A última versão upstream; `null` se a consulta falhou. |
| `upstream_newer` | `boolean` | sim | `upstream` é mais nova que `installed` e que `release`: só chega num bundle novo, numa release nova da AURORA. Sem `release`, `false` quando há AURORA mais nova, cujo bundle pode já trazê-la. |

<a id="tipo-connection"></a>

### `Connection`

Uma ligação como texto, para mostrar.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `target` | `string` | sim | A entrada do topo (`in[15:0]`) ou o sinal de saída da placa (`LEDR[17:0]`). |
| `source` | `string` | sim | O que a dirige (`~KEY[0]`, `{10'b0, out[7:0]}`). |

<a id="tipo-convertedpath"></a>

### `ConvertedPath`

Um arquivo ou pasta que a conversão moveu.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `processor` | `string` | sim | De que processador. |
| `from` | `string` | sim | Onde estava, absoluto. |
| `to` | `string` | sim | Onde ficou, absoluto. |
| `generated` | `boolean` | sim | É algo que o YANC gera (foi para a pasta de saída). |

<a id="tipo-counterexample"></a>

### `Counterexample`

A primeira amostra errada: o que entrou e o que saiu, para o aluno
refazer o caso de cabeça. Vem das linhas `first` do resumo; um `tb.v`
escrito à mão que não as escreve fica sem.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `time_ns` | `integer` | sim | O instante da amostra, em ns. |
| `cycle` | `integer` \| `null` | sim | No sequencial, o ciclo (a partir de 0). |
| `inputs` | lista de [`SignalValue`](#tipo-signalvalue) | sim | As entradas nessa amostra, sem o clock. |
| `outputs` | lista de [`OutputValue`](#tipo-outputvalue) | sim | Cada saída nessa amostra: a esperada e a do aluno. |

<a id="tipo-device"></a>

### `Device`

O FPGA de uma placa.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `family` | `string` | sim | A família, como o Quartus a escreve no `.qsf` (`Cyclone IV E`). |
| `part` | `string` | sim | O modelo, como no `.qsf` (`EP4CE115F29C7`). |

<a id="tipo-devicecode"></a>

### `DeviceCode`

O código do fluxo de dispositivo, de `login_start`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `device_code` | `string` | sim | O código que identifica este login em `login_poll`. Não é para mostrar. |
| `user_code` | `string` | sim | O código que a pessoa digita no GitHub (`WDJB-MJHT`). |
| `verification_uri` | `string` | sim | Onde ela digita (`https://github.com/login/device`). |
| `expires_in` | `integer` | sim | Em quantos segundos o código vence. |
| `interval` | `integer` | sim | Quantos segundos esperar entre duas chamadas de `login_poll`. |

<a id="tipo-diagnostic"></a>

### `Diagnostic`

Uma mensagem de uma ferramenta, com os campos que deu para extrair.

`file`, `line` e `column` são opcionais porque o `cmmcomp` não informa
arquivo nem coluna, e várias mensagens não informam nem a linha. Quando a
ferramenta não cita o arquivo mas a AURORA sabe qual é (o `cmmcomp` compila
um fonte só), `file` vem preenchido pela AURORA.

Em JSON:

```json
{ "tool": "cmmcomp", "severity": "error",
  "message": "c'mon dude, declare the variable 'total' properly!",
  "file": "/p/conta/conta.cmm", "line": 15, "column": null,
  "raw": "Error on line 15: c'mon dude, declare the variable 'total' properly!" }
```

No fluxo C, o `cppcomp` cita o arquivo e a linha do fonte, ou do
cabeçalho incluído onde está o erro.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `tool` | [`Tool`](#tipo-tool) | sim | A ferramenta que escreveu a mensagem. |
| `severity` | [`Severity`](#tipo-severity) | sim | A gravidade. |
| `message` | `string` | sim | A mensagem, sem o prefixo de gravidade e de localização. Vem no idioma da ferramenta (inglês: a AURORA sempre pede inglês ao YANC). |
| `file` | `string` \| `null` | sim | O arquivo a que a mensagem se refere, como a ferramenta o escreveu (normalmente absoluto, porque a AURORA passa caminhos absolutos). |
| `line` | `integer` \| `null` | sim | Linha, a partir de 1. |
| `column` | `integer` \| `null` | sim | Coluna, a partir de 1 (Verilator e compilador C++). |
| `raw` | `string` | sim | O texto exatamente como a ferramenta o escreveu (várias linhas quando a ferramenta continua a mensagem em linhas indentadas). |

<a id="tipo-discardresult"></a>

### `DiscardResult`

`git.discard`: o que foi descartado.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `discarded` | lista de `string` | sim | Os arquivos que voltaram ao último commit ou saíram, relativos ao topo. |
| `trashed` | lista de `string` | sim | As versões descartadas que foram para a lixeira do sistema, de onde voltam: o caminho absoluto de cada uma, antes de ir. |

<a id="tipo-elaboration"></a>

### `Elaboration`

Uma chamada do `iverilog`, ou do GHDL no VHDL, e a árvore que saiu dela.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `testbench` | `string` \| `null` | sim | O testbench elaborado; `None` no design. |
| `processor` | `string` \| `null` | sim | O processador, quando o testbench é o que o build dele gerou. |
| `status` | [`Status`](#tipo-status) | sim | Como esta elaboração terminou. Uma que falha não impede as outras. |
| `roots` | lista de [`ModuleInstance`](#tipo-moduleinstance) | sim | Os módulos que nenhum outro instancia, com o que eles instanciam. Vazio quando a elaboração falhou. |
| `diagnostics` | lista de [`Diagnostic`](#tipo-diagnostic) | sim | Os erros e avisos desta elaboração (também em `HierarchyResult::diagnostics`). |

<a id="tipo-endreason"></a>

### `EndReason`

Por que a placa virtual terminou.

| Valor | Descrição |
|---|---|
| `"stopped"` | Quem roda pediu para parar (cancelamento). |
| `"finished"` | O projeto chamou `$finish`. |
| `"crashed"` | O processo terminou sem avisar: quebrou. |

<a id="tipo-environment"></a>

### `Environment`

A máquina onde a operação rodou. O que o sistema não informa sem rodar
outro programa fica `null`: a AURORA só lê arquivos do sistema (no Linux,
`/proc` e `/etc/os-release`) e o `uname`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `hostname` | `string` \| `null` | sim | O nome da máquina. |
| `os` | `string` | sim | `linux`, `macos`, `windows`. |
| `os_name` | `string` \| `null` | sim | O nome da distribuição (Linux). |
| `kernel` | `string` \| `null` | sim | A versão do kernel (Linux e macOS). |
| `arch` | `string` | sim | `x86_64`, `aarch64`. |
| `cpu_model` | `string` \| `null` | sim | O modelo do processador (Linux). |
| `cpus` | `integer` \| `null` | sim | Processadores lógicos disponíveis. |
| `memory_bytes` | `integer` \| `null` | sim | A memória total, em bytes (Linux). |
| `fingerprint` | `string` | sim | Hash do sistema, da arquitetura, do processador, do número de processadores e do nome da máquina: muda quando a máquina muda. |

<a id="tipo-errorinfo"></a>

### `ErrorInfo`

Um `AuroraError` pronto para atravessar uma fronteira: o JSON da CLI, a
mensagem do Studio para a interface, a resposta do `aurora serve`. É o que
o código, o texto e os campos da variante viram quando o enum não vai.

```json
{ "code": "processor_not_found",
  "message": "No processor 'x' in the project (available: soma, filtro)",
  "data": { "name": "x", "available": ["soma", "filtro"] } }
```

A dica de cada interface (o comando `aurora` que resolve, o botão do
Studio) não mora aqui: a mensagem do Core é neutra, e quem mostra decide
pelo `code`. Um erro que não é do Core (o `busy` do Studio, o `cli` da
CLI) usa o mesmo tipo com `ErrorInfo::new` e um código próprio.

```rust
use aurora_core::{ErrorInfo, AuroraError};

let info = ErrorInfo::from(AuroraError::ProcessorExists("soma".into()));
assert_eq!(info.code, "processor_exists");
assert_eq!(info.data.unwrap()["name"], "soma");
```

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `code` | `string` | sim | O identificador estável: `AuroraError::code`, ou um código de quem chamou (`busy`, `cli`), sempre em `snake_case`. |
| `message` | `string` | sim | O texto do erro, em inglês, pronto para mostrar. |
| `hint` | `string` \| `null` | não | O que fazer, quando quem mostra sabe: o comando `aurora` que resolve, na CLI. O core não o preenche; a mensagem dele é neutra. Omitido no JSON quando não há. |
| `data` | qualquer JSON | não | Os campos estruturados da variante, quando ela os tem (`AuroraError::data`): `available`, `files`, `name`, `value`, `reason`, `path`, `component`. Omitido no JSON quando não há. |

<a id="tipo-event"></a>

### `Event`

O que aconteceu durante uma operação, avisado na hora em que acontece.

Em JSON, com o tipo no campo `event`:

```json
{ "event": "step_started", "step": "simulate", "tool": "vvp", "command": { "...": "..." } }
{ "event": "output", "step": "simulate", "tool": "vvp", "stream": "stdout",
  "line": "t=10 y=1", "diagnostic": false }
{ "event": "step_finished", "step": "simulate", "tool": "vvp",
  "termination": { "kind": "exited", "value": 0 }, "duration_ms": 12 }
```

Os mesmos dados chegam depois no resultado da operação
([`StepReport`](#tipo-stepreport)); os eventos só os adiantam.

Uma das formas:

**`event: "step_started"`**

Um passo começou: o processo vai ser iniciado.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `step` | [`Step`](#tipo-step) | sim | Qual passo. |
| `tool` | [`Tool`](#tipo-tool) | sim | A ferramenta. |
| `command` | [`Invocation`](#tipo-invocation) | sim | O que vai ser executado. |

**`event: "output"`**

Uma linha que o processo escreveu, sem o fim de linha. Linhas de
stdout e de stderr chegam na ordem em que a AURORA as leu, que pode não
ser exatamente a ordem em que o processo as escreveu.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `step` | [`Step`](#tipo-step) | sim | O passo que escreveu. |
| `tool` | [`Tool`](#tipo-tool) | sim | A ferramenta que escreveu. |
| `stream` | [`Stream`](#tipo-stream) | sim | stdout ou stderr. |
| `line` | `string` | sim | A linha, decodificada como UTF-8 (bytes inválidos viram `U+FFFD`). |
| `diagnostic` | `boolean` | sim | A linha é uma mensagem da ferramenta, que volta interpretada em `diagnostics` no resultado: tudo do stderr, todo o stdout dos compiladores, e no stdout de uma simulação as linhas do próprio simulador (`$finish called at`, `VCD info:`, o `ERROR:` de um `$error`). `false` é saída do programa, como o `$display` do testbench. Decidido linha a linha: a continuação indentada de uma mensagem conta como `false`. |

**`event: "step_finished"`**

O processo do passo terminou.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `step` | [`Step`](#tipo-step) | sim | Qual passo. |
| `tool` | [`Tool`](#tipo-tool) | sim | A ferramenta. |
| `termination` | [`Termination`](#tipo-termination) | sim | Como terminou. |
| `duration_ms` | `integer` | sim | Duração, em milissegundos. |

<a id="tipo-exercisesnapshot"></a>

### `ExerciseSnapshot`

Um exercício de [`WorkspaceSnapshot`](#tipo-workspacesnapshot), com o que o aluno já fez.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome. |
| `title` | `string` | sim | O título. |
| `prompt` | `string` | sim | O enunciado, em markdown. |
| `hints` | lista de `string` | sim | As dicas, em markdown. |
| `module` | `string` | sim | O módulo que o aluno escreve. |
| `file` | `string` | sim | O arquivo que o aluno edita. |
| `spf` | `string` | sim | O `.spf` do projeto do exercício. |
| `waveform` | `string` | sim | A onda da última correção. |
| `solved` | `boolean` | sim | Resolvido. |
| `solution` | `string` \| `null` | sim | A solução liberada, depois de resolvido. |

<a id="tipo-experimentalinfo"></a>

### `ExperimentalInfo`

`experimental.list` e `experimental.set`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `path` | `string` \| `null` | sim | O `experimental.json`; `None` sem pasta de configuração. |
| `components` | lista de [`FeatureState`](#tipo-featurestate) | sim | Cada recurso e se está ligado. |

<a id="tipo-feature"></a>

### `Feature`

Um recurso experimental. Em JSON e na CLI, pelo identificador (`fpga`,
`assistant`, `git`).

| Valor | Descrição |
|---|---|
| `"fpga"` | A placa FPGA: as ligações do `fpga.json`, a compilação pelo Quartus Prime e a gravação na placa (`aurora fpga`, a aba Placa FPGA do Studio). |
| `"assistant"` | O assistente de IA: o `aurora mcp` (as ferramentas da AURORA para um agente), o `aurora chat` e a vista Assistente do Studio, sobre o Claude Code instalado na máquina (o crate `aurora-agent`). |
| `"git"` | O controle de versão: os métodos `git.*` e `github.*` e as vistas Controle de versão e Perfil do Studio, sobre o git instalado na máquina (o crate `aurora-git`). |

<a id="tipo-featurestate"></a>

### `FeatureState`

Um recurso e se está ligado.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `feature` | [`Feature`](#tipo-feature) | sim | O recurso. |
| `enabled` | `boolean` | sim | Ligado. |
| `description` | `string` | sim | O que ele faz, em inglês (`Feature::description`). |

<a id="tipo-filemismatch"></a>

### `FileMismatch`

Um executável do bundle cujo hash não bate com o manifesto.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `component` | `string` | sim | O componente que lista o arquivo. |
| `path` | `string` | sim | Caminho relativo ao bundle. |
| `expected` | `string` | sim | O hash do manifesto. |
| `actual` | `string` \| `null` | sim | O hash do arquivo, ou `None` se ele não existe. |

<a id="tipo-filerole"></a>

### `FileRole`

Em qual lista do `.spf` um arquivo fica. Em JSON: `"synthesizable"` ou
`"testbench"`.

`Project::add_verilog` decide o papel pelo conteúdo, como a AURORA
(`classify`); `Project::add_file` recebe o
papel de quem chama. Um arquivo fica numa lista só.

| Valor | Descrição |
|---|---|
| `"synthesizable"` | Entra na síntese e na simulação. |
| `"testbench"` | Só entra na simulação. |

<a id="tipo-fileversion"></a>

### `FileVersion`

`git.read_file`: o conteúdo de um arquivo numa versão.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `exists` | `boolean` | sim | O arquivo existe nessa versão. |
| `size` | `integer` | sim | O tamanho, em bytes. |
| `binary` | `boolean` | sim | O conteúdo é binário (tem byte nulo no começo). |
| `too_large` | `boolean` | sim | Passou de `MAX_FILE_SIZE` e não foi lido. |
| `text` | `string` \| `null` | sim | O texto, quando existe, não é binário e não é grande demais. |

<a id="tipo-finding"></a>

### `Finding`

O que a correção percebeu sobre a causa do erro.

Uma das formas:

**`kind: "interface_changed"`**

O erro de compilação é no testbench: o nome do módulo ou o de uma
porta mudou.

Sem campos.

**`kind: "undriven_output"`**

Todas as amostras erradas desta saída estão em X ou Z: ela não é
atribuída, ou depende de algo que não é.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `output` | `string` | sim | A saída. |

**`kind: "reset_only"`**

Só erra com o reset ativo: o reset não faz o que o enunciado pede
(síncrono ou assíncrono, o valor depois dele).

Sem campos.

**`kind: "latch_inferred"`**

A síntese achou um latch: o sinal guarda o valor quando nenhum ramo
do `always` o atribui. Num circuito combinacional, faltou um `else`,
um `default` ou um valor antes do `if`. Pode aparecer com o exercício
resolvido: a simulação passa, e o circuito não é o que parece.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `signal` | `string` | sim | O sinal (`modulo.sinal` quando é de um submódulo). |
| `line` | `integer` \| `null` | sim | A linha do `always` que gerou o latch, no arquivo do aluno. |

<a id="tipo-flowoptions"></a>

### `FlowOptions`

O que é da interface, não do fluxo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `command` | `string` | sim | O texto do comando para o histórico e para o console (`aurora sim -p soma`, `aurora-ide sim -p soma`). A CLI passa a linha digitada; um cliente sem linha de comando monta com `FlowRequest::command_line`. |
| `wave` | [`WavePolicy`](#tipo-wavepolicy) | não (padrão: `"window"`) | Como abrir a onda quando o pedido tem `open_wave`. |
| `viewer_opener` | `string` \| `null` | não (padrão: `null`) | Quem abre o RTL viewer do `synth --open`: `None` abre no navegador do sistema; `Some(nome)` só grava o HTML, para quem chamou abrir (o `AURORA_OPENER=vscode` do terminal da extensão do VS Code). |

<a id="tipo-flowoutcome"></a>

### `FlowOutcome`

O resultado de um fluxo. Os campos de uma fase que não rodou ficam `None`
ou vazios.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `flow` | `string` | sim | O fluxo (`FlowRequest::name`). |
| `command` | `string` | sim | O comando equivalente (`FlowOptions::command`). |
| `status` | [`Status`](#tipo-status) | sim | Como terminou: o pior entre as fases que rodaram. `Cancelled` se o cancelamento foi pedido; `TimedOut` se a simulação estourou o prazo; `Failed` se alguma fase falhou (inclusive um build de `Build`, que compila todos); senão `Succeeded`. |
| `succeeded` | `boolean` | sim | `status == Succeeded`. |
| `builds` | lista de [`BuildResult`](#tipo-buildresult) | sim | Os builds feitos antes (ou o próprio build). |
| `registered` | lista de [`AddedFile`](#tipo-addedfile) | sim | O que esses builds geraram e entrou no `.spf` agora. |
| `check` | [`CheckResult`](#tipo-checkresult) \| `null` | sim | O resultado da verificação. |
| `lint_missing` | `string` \| `null` | sim | Com `lint`, o componente do Verilator que falta quando nenhum passo dele rodou: para a interface sugerir a instalação. |
| `hierarchy` | [`HierarchyResult`](#tipo-hierarchyresult) \| `null` | sim | A hierarquia. |
| `not_built` | lista de `string` | sim | Na hierarquia sem processador: os processadores com fonte e sem build, que ficaram de fora. |
| `simulation` | [`SimulationResult`](#tipo-simulationresult) \| `null` | sim | O resultado da simulação. |
| `outputs` | lista de [`FlowPortValues`](#tipo-flowportvalues) | sim | Os valores de cada porta de saída do processador simulado (só com processador e sucesso). |
| `synthesis` | [`SynthesisResult`](#tipo-synthesisresult) \| `null` | sim | O resultado da síntese. |
| `schematic` | [`SchematicResult`](#tipo-schematicresult) \| `null` | sim | O resultado do esquemático. |
| `schematic_error` | [`ErrorInfo`](#tipo-errorinfo) \| `null` | sim | Por que o esquemático não foi desenhado depois da síntese (o módulo pedido não está no netlist, `module_not_found`); a síntese vale. |
| `rtl_viewer` | `string` \| `null` | sim | O HTML do RTL viewer, gravado ao lado do netlist. |
| `rtl_viewer_error` | [`ErrorInfo`](#tipo-errorinfo) \| `null` | sim | Por que o RTL viewer não saiu (a síntese vale). |
| `wave` | [`WaveOpened`](#tipo-waveopened) \| `null` | sim | A onda aberta no surfer-aurora. |
| `wave_error` | [`ErrorInfo`](#tipo-errorinfo) \| `null` | sim | Por que a onda não abriu (a simulação vale). |
| `report` | `string` \| `null` | sim | O relatório gravado no histórico (`run-000042`). |
| `report_error` | `string` \| `null` | sim | Por que o relatório não foi gravado (o fluxo não falha por isso). |
| `fpga` | [`FpgaBuildResult`](#tipo-fpgabuildresult) \| `null` | sim | A compilação para a placa. |
| `fpga_program` | [`FpgaProgramResult`](#tipo-fpgaprogramresult) \| `null` | sim | A gravação na placa. |
| `learn` | qualquer JSON | sim | A correção de um exercício (`learn`): o `Grade` do aurora-learn, em JSON (o core não conhece o tipo). |

<a id="tipo-flowportvalues"></a>

### `FlowPortValues`

Os valores de uma porta de saída de um processador simulado.

No schema chama-se `FlowPortValues`: o `PortValues` do relatório `sim`
da CLI tem o erro em texto, e os dois convivem no mesmo conjunto de
tipos gerados.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `port` | `integer` | sim | O número da porta. |
| `path` | `string` | sim | `<proc>/<proc>/output_<porta>.txt`, na pasta de saída. |
| `values` | lista de `integer` | sim | Os valores, na ordem em que saíram. Vazio quando a porta não pôde ser lida. |
| `error` | [`ErrorInfo`](#tipo-errorinfo) \| `null` | sim | Por que a porta não pôde ser lida: uma linha que não é inteiro, como o `x` que o simulador escreve numa divisão por zero. `None` quando leu. |

<a id="tipo-flowrequest"></a>

### `FlowRequest`

O que o cliente pede. Os alvos são sempre explícitos: a CLI resolve o
processador da pasta atual antes de montar o pedido.

Uma das formas:

**`flow: "build"`**

Compila os processadores pedidos (vazio: todos), mesmo que um falhe,
para mostrar todos os erros. Registra no `.spf` o que gerou e grava
relatório.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `processors` | lista de `string` | não (padrão: `[]`) | Os processadores; vazio é todos os do projeto. |

**`flow: "check"`**

Verifica o Verilog que está no disco, sem compilar os processadores.
Grava relatório.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `file` | `string` \| `null` | não (padrão: `null`) | Só os módulos deste arquivo como raiz. |
| `processor` | `string` \| `null` | não (padrão: `null`) | Só este processador: o Verilog dele e o testbench que o YANC gerou, no lugar do projeto. |
| `lint` | `boolean` | não (padrão: `false`) | Acrescenta o `verilator --lint-only`. |

**`flow: "hierarchy"`**

A hierarquia do design e de cada testbench, elaborada pelo Icarus,
do que está no disco. Não compila nem grava relatório.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `processor` | `string` \| `null` | não (padrão: `null`) | Só este processador. |

**`flow: "simulate"`**

Compila os processadores que faltam (parando no primeiro que falhar)
e simula o processador ou o projeto. Grava relatório; abre a onda se
pedido.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `processor` | `string` \| `null` | não (padrão: `null`) | Simula este processador com o testbench do YANC; `None`, a simulação do projeto. |
| `testbench` | `string` \| `null` | não (padrão: `null`) | Troca o testbench da simulação do projeto antes de simular (grava o `.spf`). |
| `simulator` | [`Simulator`](#tipo-simulator) | sim | Icarus ou Verilator. Na simulação rápida, só vale para um testbench cocotb. |
| `fast` | `boolean` | não (padrão: `false`) | A simulação rápida: sem onda, com o testbench Verilog no Verilator. |
| `timeout_s` | `integer` \| `null` | não (padrão: `null`) | Prazo do passo `simulate`, em segundos. |
| `open_wave` | `boolean` | não (padrão: `false`) | Abrir a onda se a simulação a entregar, conforme `FlowOptions::wave`. A simulação rápida não tem onda. |

**`flow: "synthesize"`**

Compila os processadores que faltam e sintetiza o processador ou o
topo do projeto com o Yosys. Grava relatório.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `processor` | `string` \| `null` | não (padrão: `null`) | Sintetiza só este processador; `None`, o topo do projeto. |
| `schematic` | `boolean` | não (padrão: `false`) | Desenha o esquemático (Yosys `show` e Graphviz) depois. |
| `module` | `string` \| `null` | não (padrão: `null`) | O módulo do esquemático; `None`, o topo da síntese. |
| `open_viewer` | `boolean` | não (padrão: `false`) | Grava o HTML do RTL viewer ao lado do netlist e o abre conforme `FlowOptions::viewer_opener`. |

**`flow: "schematic"`**

Desenha outro módulo de um netlist que já existe, sem sintetizar de
novo. Sem relatório.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `netlist` | `string` | sim | O `hierarchy.json` da síntese. |
| `module` | `string` | sim | O módulo. |
| `bus_widths` | `boolean` | não (padrão: `true`) | Escrever a largura dos barramentos nas arestas. |

**`flow: "fpga_build"`**

Compila os processadores e depois o projeto para a placa do
`fpga.json`, pelo Quartus. Recurso experimental; sem relatório.

Sem campos.

**`flow: "fpga_program"`**

Grava na placa o `.sof` da última compilação. Recurso experimental;
sem relatório.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `cable` | `string` \| `null` | não (padrão: `null`) | O cabo; `None`, o primeiro que o Quartus lista. |

**`flow: "learn"`**

Corrige um exercício do `aurora learn` e grava se ele está resolvido;
a correção vai em `FlowOutcome::learn`. Quem roda é o servidor do
AURORA, que linka o aurora-learn: o `run` do core responde
`unsupported`. Sem relatório.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `root` | `string` | sim | A pasta de exercícios (ou uma pasta dentro dela). |
| `exercise` | `string` | sim | O exercício. |

<a id="tipo-fpgabuildresult"></a>

### `FpgaBuildResult`

O resultado de `build`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `board` | `string` | sim | A placa (o `id`: `de2-115`). |
| `top` | `string` | sim | O topo do projeto, que o topo da placa instancia. |
| `dir` | `string` | sim | A pasta do projeto do Quartus (`build_dir`). |
| `quartus` | [`Quartus`](#tipo-quartus) | sim | O Quartus que compilou. |
| `status` | [`Status`](#tipo-status) | sim | Como a compilação terminou. Um design que não alcança o clock compila do mesmo jeito: quem diz é `FpgaBuildResult::timing`. |
| `failed_step` | [`Step`](#tipo-step) \| `null` | sim | O passo que falhou. |
| `steps` | lista de [`StepReport`](#tipo-stepreport) | sim | Até quatro passos, todos do Quartus: `synthesize` (`quartus_map`), `fit`, `bitstream` (`quartus_asm`) e `timing` (`quartus_sta`). |
| `diagnostics` | lista de [`Diagnostic`](#tipo-diagnostic) | sim | Os erros e avisos do Quartus, sem os `Info` e sem repetição (o aviso de tempo vem uma vez por canto analisado). |
| `artifacts` | lista de [`Artifact`](#tipo-artifact) | sim | O topo da placa, o `.qsf` e os arquivos de gravação: o `.sof`, obrigatório, e o `.rbf` e o `.svf`, para o openFPGALoader. |
| `bitstream` | `string` \| `null` | sim | O `.sof`, quando a compilação terminou. |
| `resources` | lista de [`ResourceUsage`](#tipo-resourceusage) | sim | O que o design usa da FPGA, do resumo do Fitter, quando ele terminou. |
| `timing` | [`TimingSummary`](#tipo-timingsummary) \| `null` | sim | A Fmax e as folgas de cada clock, quando a análise de tempo terminou. |
| `notes` | lista de `string` | sim | Os ajustes das ligações do `fpga.json` (larguras completadas, entradas soltas), como em `aurora fpga check`. |
| `duration_ms` | `integer` | sim | Quanto a compilação levou, do começo ao fim, em milissegundos. |

<a id="tipo-fpgaconfig"></a>

### `FpgaConfig`

O que o `fpga.json` guarda.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `board` | `string` | sim | A placa (`super::board()`). |
| `top` | `string` \| `null` | não | O módulo que vai para a placa. Sem ele, o topo do projeto. |
| `connect` | mapa de `string` para [`LinkTargets`](#tipo-linktargets) | sim; opcional no pedido (padrão: `{}`) | As ligações, na ordem do arquivo. Uma porta com vários sinais (uma saída que vai a mais de um lugar da placa) tem uma `Link` para cada. |

<a id="tipo-fpgaprogramresult"></a>

### `FpgaProgramResult`

O resultado de `program`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `board` | `string` | sim | A placa (o `id`). |
| `bitstream` | `string` | sim | O `.sof` gravado. |
| `cable` | `string` | sim | O cabo usado, como o `quartus_pgm -l` o escreve (`USB-Blaster [USB-0]`). |
| `position` | `integer` | sim | A posição da FPGA na cadeia JTAG (o `@n`). |
| `quartus` | [`Quartus`](#tipo-quartus) | sim | O Quartus que gravou. |
| `status` | [`Status`](#tipo-status) | sim | Como a gravação terminou. |
| `failed_step` | [`Step`](#tipo-step) \| `null` | sim | `program` quando falhou. |
| `steps` | lista de [`StepReport`](#tipo-stepreport) | sim | Um passo: `program` (`quartus_pgm`). |
| `diagnostics` | lista de [`Diagnostic`](#tipo-diagnostic) | sim | Os erros e avisos do Programmer. |
| `duration_ms` | `integer` | sim | Quanto a gravação levou, do começo ao fim, em milissegundos. |

<a id="tipo-frame"></a>

### `Frame`

Um quadro de vídeo remontado pela placa virtual. Só sai quando a imagem
muda.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `device` | `string` | sim | O periférico de vídeo. |
| `width` | `integer` | sim | A largura visível, em pixels. |
| `height` | `integer` | sim | A altura visível, em linhas. |
| `png` | `string` | sim | A imagem em PNG, em base64. |

<a id="tipo-generatedfiles"></a>

### `GeneratedFiles`

Os arquivos que o build de um processador gera, pelo papel. Só entram os
que existem.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `verilog` | `string` \| `null` | sim | `<nome>.v`: o processador em Verilog, um módulo do design. |
| `testbench` | `string` \| `null` | sim | `<nome>_tb.v`: o testbench que o `asmcomp` gera. |
| `assembly` | `string` \| `null` | sim | `<nome>.asm`: o assembly que o compilador gera. |
| `memories` | lista de `string` | sim | `*.mif`: as memórias de dados e de instruções. |
| `intermediates` | lista de `string` | sim | As tabelas e os logs do YANC (`prog_log.txt`, `src_lines.txt`, `trad_opcode.txt`, `app_log.txt`, `pc_<nome>_mem.txt`); as saídas da simulação (`output_<n>.txt`, `toma_<n>.txt`) ficam de fora. |

<a id="tipo-githubrepository"></a>

### `GitHubRepository`

Um repositório do GitHub.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `owner` | `string` | sim | O dono (usuário ou organização). |
| `name` | `string` | sim | O nome. |

<a id="tipo-gitlog"></a>

### `GitLog`

`git.log`: uma página do histórico do ramo atual.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `commits` | lista de [`CommitSummary`](#tipo-commitsummary) | sim | Os commits, do mais novo para o mais velho. |
| `more` | `boolean` | sim | Há mais commits depois desta página. |

<a id="tipo-gitstatus"></a>

### `GitStatus`

`git.status`: o git da máquina e o repositório do projeto.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `version` | `string` | sim | A versão do git (`2.54.0.windows.1`). |
| `repository` | [`RepositoryStatus`](#tipo-repositorystatus) \| `null` | sim | O repositório que contém o projeto; `None` fora de repositório. |

<a id="tipo-grade"></a>

### `Grade`

O resultado da correção.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `exercise` | `string` | sim | O exercício. |
| `verdict` | [`Verdict`](#tipo-verdict) | sim | Como terminou. |
| `samples` | `integer` | sim | Quantas amostras o testbench comparou. |
| `mismatched` | `integer` | sim | Em quantas amostras alguma saída errou. |
| `outputs` | lista de [`OutputCheck`](#tipo-outputcheck) | sim | Cada saída, na ordem das portas. |
| `reset_mismatches` | `integer` \| `null` | sim | Com reset: quantas amostras erradas foram com ele ativo. |
| `counterexample` | [`Counterexample`](#tipo-counterexample) \| `null` | sim | A primeira amostra errada, com as entradas e as saídas. |
| `findings` | lista de [`Finding`](#tipo-finding) | sim | O que se percebeu sobre a causa do erro, e o latch que a síntese achou (este, também com o exercício resolvido). |
| `diagnostics` | lista de [`Diagnostic`](#tipo-diagnostic) | sim | Os erros e avisos dos compiladores sobre o código do aluno, com os avisos do lint do Verilator quando ele está instalado. |
| `output` | lista de `string` | sim | O que o testbench escreveu, fora o resumo (um `tb.v` escrito à mão pode explicar o erro aqui). |
| `waveform` | `string` \| `null` | sim | A onda da simulação. |
| `layout` | `string` \| `null` | sim | O arquivo de comandos do Surfer que mostra a onda com as entradas, as saídas lado a lado com as da referência e o primeiro erro marcado. |
| `duration_ms` | `integer` | sim | Quanto a correção levou, em milissegundos. |

<a id="tipo-hierarchyresult"></a>

### `HierarchyResult`

O resultado de `hierarchy`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `design` | [`Elaboration`](#tipo-elaboration) \| `null` | sim | O design, sem testbench: os módulos que nenhum outro instancia (em geral o topo), com tudo o que eles instanciam. `None` num projeto sem Verilog de design. |
| `testbenches` | lista de [`Elaboration`](#tipo-elaboration) | sim | Cada testbench elaborado com o design: os registrados, na ordem do `.spf`, e depois o testbench gerado de cada processador compilado. |
| `status` | [`Status`](#tipo-status) | sim | `Succeeded` se todas as elaborações deram certo; senão, como terminou a primeira que não deu. |
| `failed_step` | [`Step`](#tipo-step) \| `null` | sim | `elaborate` quando uma elaboração falhou ou foi cancelada. |
| `steps` | lista de [`StepReport`](#tipo-stepreport) | sim | Um `iverilog` para o design e um por testbench, na ordem. |
| `diagnostics` | lista de [`Diagnostic`](#tipo-diagnostic) | sim | Os erros e avisos de todas as elaborações, com arquivo e linha. |
| `artifacts` | lista de [`Artifact`](#tipo-artifact) | sim | Os `.vvp` de cada elaboração. |
| `duration_ms` | `integer` | sim | Quanto as elaborações levaram juntas, em milissegundos. |

<a id="tipo-identity"></a>

### `Identity`

Nome e e-mail de quem faz o commit.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome. |
| `email` | `string` | sim | O e-mail. |

<a id="tipo-initresult"></a>

### `InitResult`

`git.init`: o repositório criado.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `top` | `string` | sim | A pasta do repositório (a do projeto). |
| `branch` | `string` | sim | O ramo inicial. |
| `ignored` | lista de `string` | sim | As linhas acrescentadas ao `.gitignore`. |

<a id="tipo-installreport"></a>

### `InstallReport`

`aurora install` (`docs/schema/install.json`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `prefix` | `string` | sim | A pasta da instalação. |
| `components` | lista de `string` | sim | Os aplicativos do bundle instalados agora. |
| `added` | lista de `string` | sim | Os que entraram nesta chamada (com os que eles exigem). |

<a id="tipo-invocation"></a>

### `Invocation`

Uma execução de programa, totalmente especificada: o registro exato do
que a AURORA rodou, presente em todo [`StepReport`](#tipo-stepreport).

Só o Core monta invocações; para os clientes o tipo é somente leitura.
Reproduzir um passo à mão é rodar `program` com `args`, dentro de `cwd`,
com um ambiente vazio mais `env` e as variáveis de `inherit`.

Em JSON:

```json
{ "program": "/opt/yanc/bin/cmmcomp",
  "args": ["-i", "soma.cmm", "-n", "soma", "-p", "/p/soma", "..."],
  "cwd": "/p/soma",
  "env": [],
  "inherit": [] }
```

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `program` | `string` | sim | Caminho absoluto do executável. Nunca é procurado no `PATH`. |
| `args` | lista de `string` | sim | Argumentos, na ordem. Caminhos já estão no formato que a ferramenta aceita: o nativo do sistema, menos os do `iverilog` no Windows, que vão com `/`. |
| `cwd` | `string` | sim | Diretório de trabalho. Faz parte do contrato de várias ferramentas (o testbench do YANC lê e grava os arquivos dele relativo a ele, e grava a onda nele), por isso nunca é herdado. |
| `env` | lista de (`string`, `string`) | sim | Variáveis definidas pela AURORA, somadas às de `INHERITED_ENV`. |
| `inherit` | lista de `string` | sim | Nomes de variáveis copiadas do ambiente da AURORA, quando existirem. Só os nomes aparecem no relatório. |

<a id="tipo-iostandard"></a>

### `IoStandard`

O padrão de I/O de um sinal.

Uma das formas:

**`string`**

O mesmo para todos os bits.

**lista de `string`**

Um por bit, do bit 0 em diante.

<a id="tipo-issuekind"></a>

### `IssueKind`

O tipo de um [`ProjectIssue`](#tipo-projectissue). Em JSON, `snake_case`.

| Valor | Descrição |
|---|---|
| `"rescued_path"` | Um absoluto de outra máquina que não existe aqui foi achado pela cauda dentro da raiz; o `.spf` sai consertado na próxima gravação. |
| `"selection_not_registered"` | `topLevelFile` ou `testbenchFile` aponta para um arquivo que não está na lista do papel, e é ignorado. |
| `"testbench_as_top"` | `topLevelFile` tem nome de testbench (`tb_x.v`, `x_tb.v`), que não pode ser topo, e é ignorado. |
| `"invalid_processor_name"` | Um processador do `.spf` com nome que o YANC não compila (`x-y`, `void`); o build dele recusa com `invalid_name`. |
| `"duplicate_file"` | Um arquivo repetido numa lista, ou nas duas: conta uma vez, e a próxima gravação tira as entradas que sobram. |
| `"nested_project"` | O projeto está dentro da pasta de outro (uma pasta acima tem `.spf`): os arquivos dele aparecem também no outro. |
| `"legacy_layout"` | Um processador ainda tem a estrutura de pastas de antes do YANC 7.1 (`Software/`, `Hardware/`, `Simulation/`): o build dele recusa com `legacy_layout` até a conversão (`Project::convert_layout`). |

<a id="tipo-jtag"></a>

### `Jtag`

Como gravar a placa pela USB dela.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `cable` | [`Cable`](#tipo-cable) | sim | O cabo. |
| `position` | `integer` | sim | A posição do FPGA na cadeia JTAG, a partir de 1. Na DE10-Nano o FPGA vem depois do processador ARM (HPS). |
| `openfpgaloader` | `string` \| `null` | não | O nome da placa no openFPGALoader (`-b`), se ele a conhece. |

<a id="tipo-language"></a>

### `Language`

Linguagem do programa de um processador. Em JSON: `"cmm"` ou `"cpp"`.

| Valor | Descrição |
|---|---|
| `"cmm"` | C±, compilado pelo `cmmcomp`. |
| `"cpp"` | C, compilado pelo `cppcomp`. A AURORA e o YANC chamam de "cpp" e usam a extensão `.cpp`. |

<a id="tipo-layoutclock"></a>

### `LayoutClock`

Um relógio da placa que chega ao projeto.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `signal` | `string` | sim | O sinal (`CLOCK_50`). |
| `mhz` | `number` | sim | A frequência, em MHz. |

<a id="tipo-layoutconversion"></a>

### `LayoutConversion`

O que `Project::convert_layout` fez.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `processors` | lista de `string` | sim | Os processadores que tinham a estrutura antiga, na ordem do `.spf`. |
| `moved` | lista de [`ConvertedPath`](#tipo-convertedpath) | sim | O que mudou de lugar. |
| `skipped` | lista de [`SkippedPath`](#tipo-skippedpath) | sim | O que ficou onde estava, com o motivo. |
| `name_lines` | lista de [`RemovedNameLine`](#tipo-removednameline) | sim | Os fontes que perderam a linha do nome (`#PRNAME`, `#pragma yanc prname`). |
| `invalid_names` | lista de `string` | sim | Os processadores cujo nome não serve mais: na 7.1 ele vira o módulo Verilog, e o YANC recusa um fonte com esse nome. |
| `remaining_dirs` | lista de `string` | sim | As subpastas antigas que ficaram com algo que não foi movido (o que o sistema deixa sozinho numa pasta, como o `.DS_Store`, não conta). |

<a id="tipo-layoutperipheral"></a>

### `LayoutPeripheral`

Um periférico na placa virtual.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `id` | `string` | sim | O identificador nas entradas e no estado: o nome do primeiro sinal do periférico (`SW`, `HEX7`, `VGA_R`). |
| `peripheral` | [`Peripheral`](#tipo-peripheral) | sim | O tipo e os sinais, como no JSON da placa. |
| `connected` | `boolean` | sim | Algum sinal do periférico chega ao projeto. Um periférico solto aparece apagado: o projeto não o usa. |
| `signals` | lista de [`LayoutSignal`](#tipo-layoutsignal) | sim | Os sinais, com largura e polaridade. |

<a id="tipo-layoutpolicy"></a>

### `LayoutPolicy`

O que fazer com o layout dos processadores SAPHO ao abrir a onda.

Uma das formas:

**`kind: "generated"`**

O layout gerado (ou o salvo no projeto, se o usuário salvou).

Sem campos.

**`kind: "none"`**

Nenhum: a onda crua.

Sem campos.

**`kind: "reset"`**

Apaga o layout salvo no projeto e abre com o gerado.

Sem campos.

**`kind: "file"`**

Um arquivo de estado ou de comandos do Surfer (o layout de um
exercício do aurora-learn).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `path` | `string` | sim | O arquivo (`.surf.ron` ou `.sucl`). |

<a id="tipo-layoutsignal"></a>

### `LayoutSignal`

Um sinal de um periférico.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome (`LEDR`). |
| `width` | `integer` | sim | A largura em bits. |
| `active_low` | `boolean` | sim | Ativo em nível baixo na placa (os segmentos e os botões da Terasic): o estado da placa virtual já vem convertido para aceso e apertado. |
| `connected` | `boolean` | sim | O sinal chega ao projeto. |
| `ports` | lista de `string` \| `null` | sim | A porta do topo em cada bit, do bit 0 em diante (`b[3]`, `!rst`, `a`); `None` no bit solto. Numa entrada da placa lida por mais de uma porta, as portas separadas por vírgula. |

<a id="tipo-learncheckoutcome"></a>

### `LearnCheckOutcome`

O resultado de uma rodada de correção (e do `learn.check` do protocolo).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `results` | lista de [`Grade`](#tipo-grade) | sim | A correção de cada exercício pedido, na ordem; termina no primeiro cancelado. |
| `solved` | `integer` | sim | Quantos exercícios da trilha estão resolvidos depois da rodada. |
| `total` | `integer` | sim | Quantos exercícios a trilha tem. |

<a id="tipo-learntracks"></a>

### `LearnTracks`

De onde vêm as trilhas e quais há (o resultado do `learn.tracks` do
protocolo).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `dir` | `string` | sim | A pasta das trilhas. |
| `tracks` | lista de `string` | sim | As trilhas dela. |

<a id="tipo-ledcolor"></a>

### `LedColor`

A cor de um LED.

| Valor | Descrição |
|---|---|
| `"red"` | Vermelho. |
| `"green"` | Verde. |
| `"blue"` | Azul. |
| `"yellow"` | Amarelo. |

<a id="tipo-linktargets"></a>

### `LinkTargets`

O valor de uma ligação no `fpga.json`: um sinal da placa (`"LEDR[7:0]"`,
`"!KEY[0]"`, `"0"`), ou uma lista deles para uma saída que vai a vários.
Só descreve o formato; o [`FpgaConfig`](#tipo-fpgaconfig) guarda as ligações em `Link`.

Uma das formas:

**`string`**

Um sinal.

**lista de `string`**

Vários sinais, para uma saída do topo.

<a id="tipo-listposition"></a>

### `ListPosition`

Para onde `Project::reorder_file` leva um arquivo na lista dele. Em
JSON: `{"kind": "first"}`, `{"kind": "before", "path": "rtl/b.v"}`.

Uma das formas:

**`kind: "first"`**

O primeiro da lista.

Sem campos.

**`kind: "last"`**

O último.

Sem campos.

**`kind: "before"`**

Logo antes deste arquivo, da mesma lista.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `path` | `string` | sim |  |

**`kind: "after"`**

Logo depois deste arquivo, da mesma lista.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `path` | `string` | sim |  |

<a id="tipo-localbranch"></a>

### `LocalBranch`

Um ramo local.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome. |
| `upstream` | `string` \| `null` | sim | O ramo remoto que ele acompanha. |
| `ahead` | `integer` | sim | Commits que o remoto não tem. |
| `behind` | `integer` | sim | Commits do remoto que ele não tem. |
| `gone` | `boolean` | sim | O ramo remoto foi apagado. |
| `time` | `integer` | sim | A data do último commit, em segundos desde 1970. |

<a id="tipo-loginpoll"></a>

### `LoginPoll`

A resposta de `login_poll`.

Uma das formas:

**`state: "pending"`**

Ainda não autorizou: pergunte de novo depois de `interval` segundos.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `interval` | `integer` | sim | O intervalo, em segundos (o GitHub o aumenta se a pergunta vier cedo demais). |

**`state: "done"`**

Entrou.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `account` | [`Account`](#tipo-account) | sim | A conta. |

**`state: "expired"`**

O código venceu antes da autorização.

Sem campos.

**`state: "denied"`**

A pessoa recusou no GitHub.

Sem campos.

<a id="tipo-mappingtranslator"></a>

### `MappingTranslator`

Um tradutor de valor do Surfer: um arquivo `Name = <nome>`, `Bits = <n>`
e uma linha `<valor> <texto>` por valor.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome, que é também o nome do arquivo e o `format` dos sinais. |
| `content` | `string` | sim | O conteúdo do arquivo. |

<a id="tipo-metriccomparison"></a>

### `MetricComparison`

Um número comparado.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `baseline` | `integer` \| `null` | sim | Na referência. |
| `current` | `integer` \| `null` | sim | No atual. |
| `change` | [`Change`](#tipo-change) | sim | O que mudou. |
| `delta` | `integer` \| `null` | sim | Atual menos referência. |
| `percent` | `number` \| `null` | sim | A variação em porcentagem da referência. `null` quando não se compara ou quando a referência é zero e o atual não (não há porcentagem de zero; o texto mostra `new`). |

<a id="tipo-moduleinstance"></a>

### `ModuleInstance`

Uma instância de módulo na hierarquia elaborada.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome da instância (`dut`). Numa raiz, o nome do módulo. Dentro de blocos `generate`, `begin` ou `fork`, com o nome deles na frente, separado por `.` (`op_add.my_add`): os blocos não viram nós. |
| `module` | `string` | sim | O módulo instanciado; no VHDL, a entidade. |
| `architecture` | `string` \| `null` | sim | No VHDL, a arquitetura que o GHDL elaborou (`rtl`); `None` no Verilog. |
| `file` | `string` \| `null` | sim | O arquivo que define o módulo, absoluto. |
| `line` | `integer` \| `null` | sim | A linha da definição (`module ...`). |
| `instance_file` | `string` \| `null` | sim | O arquivo da instância, no módulo de cima. `None` numa raiz. |
| `instance_line` | `integer` \| `null` | sim | A linha da instância. |
| `library` | `boolean` | sim | O módulo vem da biblioteca SAPHO (`Toolchain::sapho_library`). |
| `children` | lista de [`ModuleInstance`](#tipo-moduleinstance) | sim | As instâncias de dentro, na ordem do fonte. |

<a id="tipo-moduleinterface"></a>

### `ModuleInterface`

Um módulo e as portas dele, na ordem da declaração.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | Nome do módulo. |
| `file` | `string` | sim | O arquivo que o declara. |
| `ports` | lista de [`Port`](#tipo-port) | sim | As portas. |

<a id="tipo-movedpath"></a>

### `MovedPath`

O que `Project::move_path` fez.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `from` | `string` | sim | Onde estava, absoluto. |
| `to` | `string` | sim | Onde ficou, absoluto. |
| `files` | lista de [`ProjectFile`](#tipo-projectfile) | sim | Os arquivos registrados no `.spf` que mudaram de lugar, já com o caminho novo: o próprio arquivo, ou os que estavam dentro da pasta. Continuam com o mesmo papel e a mesma marca de topo ou de testbench escolhido. |

<a id="tipo-namecheck"></a>

### `NameCheck`

`project.check_name` e `processor.check_name`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `ok` | `boolean` | sim | O nome serve. |
| `error` | [`ErrorInfo`](#tipo-errorinfo) \| `null` | sim | Por que não serve (`invalid_name`). |

<a id="tipo-newprocessor"></a>

### `NewProcessor`

Parâmetros do cabeçalho de um processador novo.

Os padrões são os da AURORA (`js/project/processor_defaults.ts`): o float
estreito do SAPHO, 23 = 16 + 6 + 1 bits. No fluxo C só as portas vão para
o fonte; o resto fica com o que o `cppcomp` foi compilado para assumir.

Em JSON, só `name` e `language` são obrigatórios: um campo ausente fica
com o padrão de `NewProcessor::new`, e, sem `nubits`, com `nbmant` ou
`nbexpo` dados, `nubits` vira `nbmant + nbexpo + 1`
(`NewProcessor::nubits_for`), a única largura que o `asmcomp` aceita.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | Nome do processador: letras, dígitos e `_`, começando por letra ou `_`. |
| `language` | [`Language`](#tipo-language) | sim | Linguagem do fonte-modelo. |
| `nubits` | `integer` | sim; opcional no pedido (padrão: `23`) | `#NUBITS`: largura da palavra de dados, em bits. Só C±. |
| `nbmant` | `integer` | sim; opcional no pedido (padrão: `16`) | `#NBMANT`: bits de mantissa do ponto flutuante do SAPHO (não é IEEE 754). Só C±. |
| `nbexpo` | `integer` | sim; opcional no pedido (padrão: `6`) | `#NBEXPO`: bits de expoente do ponto flutuante. Só C±. |
| `nugain` | `integer` | sim; opcional no pedido (padrão: `128`) | `#NUGAIN`: divisor de `norm(x)`. Precisa ser potência de dois, senão o `cmmcomp` recusa (um divisor genérico viraria o caminho crítico da ULA). Só C±; o fonte só traz a diretiva quando difere de 128, o padrão do YANC. |
| `ndstac` | `integer` \| `null` | sim; opcional no pedido (padrão: `null`) | `#NDSTAC`: profundidade da pilha de dados. Só C±. `None`, o padrão: sem a diretiva, e o `asmcomp` calcula a profundidade pelo programa. |
| `sdepth` | `integer` \| `null` | sim; opcional no pedido (padrão: `null`) | `#SDEPTH`: profundidade da pilha de instruções (chamadas). Só C±. `None`, o padrão: sem a diretiva, calculada pelo programa. |
| `input_ports` | `integer` | sim; opcional no pedido (padrão: `1`) | `#NUIOIN` / `#pragma yanc nuioin`: número de portas de entrada. |
| `output_ports` | `integer` | sim; opcional no pedido (padrão: `1`) | `#NUIOOU` / `#pragma yanc nuioou`: número de portas de saída. |

<a id="tipo-operation"></a>

### `Operation`

Uma operação pela metade no repositório.

| Valor | Descrição |
|---|---|
| `"merge"` | Um merge. |
| `"rebase"` | Um rebase. |
| `"cherry_pick"` | Um cherry-pick. |
| `"revert"` | Um revert. |

<a id="tipo-origin"></a>

### `Origin`

Como o bundle foi achado.

| Valor | Descrição |
|---|---|
| `"settings"` | A pasta declarada numa preferência. |
| `"environment"` | A pasta declarada no ambiente (`AURORA_TOOLCHAIN`). |
| `"bundled"` | O bundle em que o programa está instalado. |
| `"installation"` | Uma pasta padrão do instalador. |
| `"path"` | O `aurora` do `PATH`. |
| `"beside"` | Ao lado do executável. |

<a id="tipo-outputcheck"></a>

### `OutputCheck`

O resultado de uma saída do módulo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome da porta. |
| `mismatches` | `integer` | sim | Em quantas amostras ela diferiu da referência. |
| `first_ns` | `integer` \| `null` | sim | O instante do primeiro erro, em ns. |
| `unknown` | `integer` | sim | Das amostras erradas, quantas tinham bit em X ou Z. |

<a id="tipo-outputvalue"></a>

### `OutputValue`

Uma saída no contraexemplo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | A porta. |
| `expected_bits` | `string` | sim | Os bits que a referência deu. |
| `actual_bits` | `string` | sim | Os bits que o módulo do aluno deu. |
| `expected` | `string` | sim | O esperado, para ler. |
| `actual` | `string` | sim | O do aluno, para ler. |
| `wrong` | `boolean` | sim | Se esta saída errou nessa amostra. |

<a id="tipo-peripheral"></a>

### `Peripheral`

Um periférico da placa: o tipo e os sinais dele. A placa virtual simula
cada um nos pinos, como a placa de verdade os liga ao FPGA.

Uma das formas:

**`kind: "switches"`**

Chaves deslizantes: o bit fica no valor da chave.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `signal` | `string` | sim | O sinal de entrada. |

**`kind: "buttons"`**

Botões: o bit muda enquanto o botão está apertado (vai a 0 num sinal
ativo em baixo, como os KEY das placas da Terasic).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `signal` | `string` | sim | O sinal de entrada. |

**`kind: "leds"`**

LEDs, um por bit.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `signal` | `string` | sim | O sinal de saída. |
| `color` | [`LedColor`](#tipo-ledcolor) | sim | A cor dos LEDs. |

**`kind: "seven_segment"`**

Displays de 7 segmentos, um sinal por dígito, da esquerda para a
direita. Bit 0 é o segmento a, bit 6 o segmento g; um oitavo bit é o
ponto decimal.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `digits` | lista de `string` | sim | Os dígitos, da esquerda para a direita. |

**`kind: "vga"`**

Saída de vídeo VGA (ou o vídeo paralelo de um transmissor HDMI): a
placa virtual remonta a imagem pelos sinais de sincronismo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `red` | `string` | sim | O canal vermelho. |
| `green` | `string` | sim | O canal verde. |
| `blue` | `string` | sim | O canal azul. |
| `hsync` | `string` | sim | O sincronismo horizontal. |
| `vsync` | `string` | sim | O sincronismo vertical. |
| `blank` | `string` \| `null` | não | O vídeo ativo (`VGA_BLANK_N`, `DE`), se a placa tem. |
| `clock` | `string` \| `null` | não | O relógio de pixel que o projeto entrega ao conversor (`VGA_CLK`), se a placa tem. |

**`kind: "ps2"`**

Teclado PS/2: o projeto recebe os quadros do teclado nos dois fios.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `clock` | `string` | sim | O relógio do teclado (entrada do FPGA). |
| `data` | `string` | sim | O dado do teclado (entrada do FPGA). |

**`kind: "uart"`**

Porta serial (UART).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `tx` | `string` | sim | O que o projeto transmite (saída do FPGA). |
| `rx` | `string` \| `null` | não | O que o projeto recebe (entrada do FPGA), se a placa liga. |
| `baud` | `integer` | sim | A velocidade padrão, em bits por segundo. |

<a id="tipo-phase"></a>

### `Phase`

A fase de um fluxo que começou. Os nomes são os que o Studio mostra;
`Learn` é a correção de um exercício do aurora-learn, que o Studio nomeia
com o mesmo tipo.

| Valor | Descrição |
|---|---|
| `"build"` | Compilando processadores (YANC). |
| `"check"` | Verificando o Verilog. |
| `"hierarchy"` | Elaborando a hierarquia. |
| `"simulate"` | Simulando. |
| `"synthesize"` | Sintetizando. |
| `"schematic"` | Desenhando o esquemático. |
| `"rtl_viewer"` | Gravando e abrindo o RTL viewer. |
| `"wave"` | Abrindo a onda. |
| `"learn"` | Corrigindo um exercício do `aurora learn`. |
| `"fpga"` | Compilando para a placa (Quartus). |
| `"program"` | Gravando na placa (Quartus Programmer). |

<a id="tipo-port"></a>

### `Port`

Uma porta de um módulo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | Nome. |
| `direction` | [`PortDirection`](#tipo-portdirection) | sim | Direção. |
| `width` | `integer` | sim | Largura em bits, com os parâmetros nos valores padrão. |
| `signed` | `boolean` | sim | `signed`. |

<a id="tipo-portbit"></a>

### `PortBit`

Um bit de uma saída do topo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `port` | `string` | sim | A porta. |
| `bit` | `integer` | sim | O bit. |
| `invert` | `boolean` | sim | Invertido (`!`). |

<a id="tipo-portchoice"></a>

### `PortChoice`

A porta do topo de uma [`Reconnect`](#tipo-reconnect).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `port` | `string` | sim | A porta. |
| `bit` | `integer` \| `null` | não (padrão: `null`) | O bit da porta que vai no bit do sinal (no sinal inteiro, o que vai no bit 0 dele); `None`, o bit 0. |
| `invert` | `boolean` | não (padrão: `false`) | Invertido (`!`): o botão apertado chega como 1, o LED acende com 0. |

<a id="tipo-portdirection"></a>

### `PortDirection`

Direção de uma porta.

| Valor | Descrição |
|---|---|
| `"input"` | `input`. |
| `"output"` | `output`. |
| `"inout"` | `inout`. |

<a id="tipo-portdrive"></a>

### `PortDrive`

A origem de cada bit de uma entrada do topo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `port` | `string` | sim | A porta. |
| `bits` | lista de [`BitSource`](#tipo-bitsource) | sim | Um item por bit, do bit 0 em diante. |

<a id="tipo-portfile"></a>

### `PortFile`

Um arquivo de porta de entrada ou de saída de um processador.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `port` | `integer` | sim | A porta (o `n` de `in(n)` e `out(n)`). |
| `path` | `string` | sim | O arquivo. |

<a id="tipo-prepared"></a>

### `Prepared`

O `fpga.json` conferido: a placa, as ligações bit a bit e o topo da
placa gerado.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `config` | `string` | sim | O `fpga.json`. |
| `resolved` | [`Resolved`](#tipo-resolved) | sim | As ligações conferidas. |
| `board_top` | `string` | sim | O Verilog do topo da placa (`wrapper::generate`). |
| `connections` | lista de [`Connection`](#tipo-connection) | sim | Cada ligação como texto (`wrapper::connections`): cada entrada do topo e cada sinal de saída da placa usado, com o que o dirige. |

<a id="tipo-processor"></a>

### `Processor`

Um processador do projeto, com todos os caminhos já absolutos.

Os valores de simulação (`frequency_mhz`, `clocks`, `show_arrays`) vêm do
`.spf` (`clk`, `numClocks`, `showArrays`) ou dos padrões da AURORA
(`DEFAULT_FREQUENCY_MHZ`, `DEFAULT_CLOCKS`, `false`). Para mudá-los
só num build, use `BuildOptions`.

A linguagem é resolvida como na AURORA: `language` declarada no `.spf`;
senão a extensão de `sourceFile`/`cmmFile`; senão o arquivo que existir na
pasta do processador (`<nome>.cmm` antes de `<nome>.cpp`); senão C±.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome: a pasta, o nome do fonte, o módulo Verilog e os arquivos gerados. |
| `language` | [`Language`](#tipo-language) | sim | A linguagem do fonte. |
| `dir` | `string` | sim | `<raiz>/<nome>`: o fonte e os arquivos que a pessoa escreve (os que inicializam arrays, `input_<n>.txt`, `cade_<n>.txt`, os cabeçalhos do C). |
| `source` | `string` | sim | O programa-fonte, `<dir>/<nome>.cmm` ou `.cpp`. |
| `output_dir` | `string` | sim | `<dir>/<nome>`, a pasta de saída do YANC (`-o`): o `.asm`, o Verilog, as memórias, o testbench, as tabelas da onda e as saídas da simulação, que roda nela. |
| `temp_dir` | `string` | sim | `<raiz>/.aurora8/Temp/<nome>`: o que só a AURORA usa (o `.vvp`, o modelo do Verilator, a cópia instrumentada do testbench, a trava de execução). |
| `frequency_mhz` | `integer` | sim | Frequência de operação em MHz (`-f` do `asmcomp`). |
| `clocks` | `integer` | sim | Clocks a simular (`-c` do `asmcomp`, vai para o testbench). |
| `show_arrays` | `boolean` | sim | Exporta cada elemento de array como um sinal da onda (`--wave-arrays` do `cmmcomp`). |

<a id="tipo-processorconfig"></a>

### `ProcessorConfig`

Parâmetros de simulação de um processador, para
`Project::configure_processor`. `None` em um campo mantém o valor
atual.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `frequency_mhz` | `integer` \| `null` | não | Frequência de operação em MHz (`clk` no `.spf`). |
| `clocks` | `integer` \| `null` | não | Clocks que o testbench gerado simula (`numClocks` no `.spf`). |
| `show_arrays` | `boolean` \| `null` | não | Exportar arrays para a simulação (`showArrays` no `.spf`). |

<a id="tipo-processorremoval"></a>

### `ProcessorRemoval`

`processor.remove`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `removed` | [`RemovedProcessor`](#tipo-removedprocessor) | sim | O que saiu: o nome, a pasta e os arquivos que saíram das listas. |
| `trashed` | `boolean` | sim | A pasta foi para a lixeira do sistema. |

<a id="tipo-processorstatus"></a>

### `ProcessorStatus`

Um processador, com o que uma interface precisa saber dele além do
[`Processor`](#tipo-processor).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome: a pasta, o nome do fonte, o módulo Verilog e os arquivos gerados. |
| `language` | [`Language`](#tipo-language) | sim | A linguagem do fonte. |
| `dir` | `string` | sim | `<raiz>/<nome>`: o fonte e os arquivos que a pessoa escreve (os que inicializam arrays, `input_<n>.txt`, `cade_<n>.txt`, os cabeçalhos do C). |
| `source` | `string` | sim | O programa-fonte, `<dir>/<nome>.cmm` ou `.cpp`. |
| `output_dir` | `string` | sim | `<dir>/<nome>`, a pasta de saída do YANC (`-o`): o `.asm`, o Verilog, as memórias, o testbench, as tabelas da onda e as saídas da simulação, que roda nela. |
| `temp_dir` | `string` | sim | `<raiz>/.aurora8/Temp/<nome>`: o que só a AURORA usa (o `.vvp`, o modelo do Verilator, a cópia instrumentada do testbench, a trava de execução). |
| `frequency_mhz` | `integer` | sim | Frequência de operação em MHz (`-f` do `asmcomp`). |
| `clocks` | `integer` | sim | Clocks a simular (`-c` do `asmcomp`, vai para o testbench). |
| `show_arrays` | `boolean` | sim | Exporta cada elemento de array como um sinal da onda (`--wave-arrays` do `cmmcomp`). |
| `built` | `boolean` | sim | O Verilog e o testbench gerados existem (`Processor::is_built`). |
| `inputs` | lista de [`PortFile`](#tipo-portfile) | sim | Os arquivos de entrada (`<proc>/input_<n>.txt`, ao lado do fonte) que existem, por porta. |
| `outputs` | lista de [`PortFile`](#tipo-portfile) | sim | Os arquivos de saída (`<proc>/<proc>/output_<n>.txt`, na pasta de saída) que existem, por porta. |
| `missing_inputs` | lista de `string` | sim | As entradas que o testbench lê e que não existem; só depois do build, porque é o testbench gerado que diz quais são. |
| `waveform` | `string` \| `null` | sim | A onda da simulação do processador, se já existe. |
| `generated` | [`GeneratedFiles`](#tipo-generatedfiles) | sim | O que o build gerou e existe no disco. |

<a id="tipo-projectfile"></a>

### `ProjectFile`

Um arquivo registrado no projeto.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `role` | [`FileRole`](#tipo-filerole) | sim | Em qual lista ele está. |
| `path` | `string` | sim | Caminho absoluto. |
| `top_level` | `boolean` | sim | Marcado como topo (sintetizável) ou como o testbench escolhido. |
| `language` | [`SourceLanguage`](#tipo-sourcelanguage) \| `null` | sim | A linguagem, pela extensão; `None` para um arquivo de outra extensão que o `.spf` traga. |

<a id="tipo-projectissue"></a>

### `ProjectIssue`

Um aviso de `Project::issues`: algo estranho no `.spf` que não impede
de abrir.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `kind` | [`IssueKind`](#tipo-issuekind) | sim | O tipo, para a interface traduzir. |
| `path` | `string` \| `null` | sim | O arquivo ou a pasta envolvida, absoluta. |
| `detail` | `string` \| `null` | sim | O complemento do tipo: o caminho como estava gravado (`rescued_path`), o campo do `.spf` (`selection_not_registered`, `testbench_as_top`) ou o nome do processador (`invalid_processor_name`, `legacy_layout`). |
| `message` | `string` | sim | O aviso inteiro, em inglês, pronto para mostrar. |

<a id="tipo-projectstatus"></a>

### `ProjectStatus`

O projeto inteiro, como uma interface o mostra. Só lê o disco.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome do projeto (o do `.spf`). |
| `spf` | `string` | sim | O `.spf`. |
| `root` | `string` | sim | A pasta do projeto. |
| `synthesizable` | lista de [`ProjectFile`](#tipo-projectfile) | sim | Os módulos registrados (sintetizáveis), na ordem do `.spf`. |
| `testbenches` | lista de [`ProjectFile`](#tipo-projectfile) | sim | Os testbenches registrados, na ordem do `.spf`. |
| `top_level` | `string` \| `null` | sim | O arquivo de topo, se escolhido. |
| `top_module` | `string` \| `null` | sim | O módulo de topo, quando dá para saber: o `topLevelModule` do `.spf`, o único módulo do arquivo de topo ou o que tem o nome dele. |
| `top_module_error` | [`ErrorInfo`](#tipo-errorinfo) \| `null` | sim | Por que o módulo de topo não dá para saber (arquivo com vários módulos, nenhum com o nome dele). |
| `selected_testbench` | `string` \| `null` | sim | O testbench que a simulação do projeto roda. |
| `testbench_module` | `string` \| `null` | sim | O módulo desse testbench. |
| `wave_selection` | lista de `string` | sim | A escolha de sinais da onda desse testbench (`wave/<testbench>.json`); vazia, a onda grava todos os sinais. |
| `unregistered` | lista de `string` | sim | Os `.v` e `.sv` da pasta que não estão registrados, e os `.py` com `@cocotb.test`. |
| `processors` | lista de [`ProcessorStatus`](#tipo-processorstatus) | sim | Os processadores SAPHO, na ordem do `.spf`. |
| `waveform` | `string` \| `null` | sim | A onda da simulação do projeto, se já existe no disco. |
| `top_candidates` | lista de `string` | sim | Os Verilog conhecidos que podem ser o topo: os registrados, o gerado de cada processador e os de fora do `.spf`, menos os com nome de testbench e os testbenches cocotb (a regra de `Project::set_top_level`). |
| `issues` | lista de [`ProjectIssue`](#tipo-projectissue) | sim | O que está estranho no `.spf` sem impedir de abrir (`Project::issues`). |

<a id="tipo-quartus"></a>

### `Quartus`

Uma instalação do Quartus Prime.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `root` | `string` | sim | A pasta `quartus` da instalação (`C:\intelFPGA_lite\22.1std\quartus`). |
| `bin` | `string` | sim | A pasta dos programas: `bin64` no Windows, `bin` no Linux. |
| `version` | `string` \| `null` | não | A versão, pelo nome da pasta da instalação (`22.1std`). |

<a id="tipo-reconnect"></a>

### `Reconnect`

Uma mudança de ligação feita na placa (`FpgaConfig::reconnect`): o sinal
da placa, um bit dele ou ele inteiro, e a porta do topo que passa a ir
nele.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `signal` | `string` | sim | O sinal da placa (`SW`, `HEX0`). |
| `bit` | `integer` \| `null` | não (padrão: `null`) | O bit do sinal; `None`, o sinal inteiro, do bit 0 em diante. |
| `port` | [`PortChoice`](#tipo-portchoice) \| `null` | não (padrão: `null`) | A porta que vai nele; `None` deixa o bit (ou o sinal) solto. |

<a id="tipo-remote"></a>

### `Remote`

Um remoto.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome (`origin`). |
| `url` | `string` | sim | O endereço, sem usuário nem senha embutidos. |
| `github` | [`GitHubRepository`](#tipo-githubrepository) \| `null` | sim | O repositório do GitHub, quando o remoto é do GitHub. |

<a id="tipo-remotebranch"></a>

### `RemoteBranch`

Um ramo remoto.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome completo (`origin/feature`). |
| `remote` | `string` | sim | O remoto (`origin`). |
| `branch` | `string` | sim | O ramo no remoto (`feature`). |
| `time` | `integer` | sim | A data do último commit, em segundos desde 1970. |

<a id="tipo-removeoutcome"></a>

### `RemoveOutcome`

`project.remove`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `removed` | lista de `string` | sim | Tirados do projeto (continuam no disco). |
| `not_registered` | lista de `string` | sim | Pedidos, mas que não estavam no projeto. |

<a id="tipo-removednameline"></a>

### `RemovedNameLine`

Um fonte que perdeu a linha do nome.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `processor` | `string` | sim | De que processador. |
| `file` | `string` | sim | O fonte, já no lugar novo. |
| `old_name` | `string` | sim | O nome que a linha dava, que era o do módulo Verilog até a 6.x. Se difere do processador, o topo que instanciava o módulo pelo nome antigo precisa do novo. |

<a id="tipo-removedprocessor"></a>

### `RemovedProcessor`

O que `Project::remove_processor` tirou do projeto.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome do processador. |
| `dir` | `string` | sim | A pasta dele (`<raiz>/<nome>`), que continua no disco. |
| `unregistered` | lista de [`ProjectFile`](#tipo-projectfile) | sim | Os arquivos de dentro da pasta que estavam registrados e saíram das listas: o papel de cada um e se era o topo ou o testbench escolhido. |

<a id="tipo-reportview"></a>

### `ReportView`

`history.show`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `id` | `string` | sim | O identificador (`run-000042`). |
| `path` | `string` | sim | O `report.txt`. |
| `record` | [`RunRecord`](#tipo-runrecord) | sim | O que a comparação usa. |
| `text` | `string` | sim | O `report.txt`, sem mudança. |

<a id="tipo-repositorystatus"></a>

### `RepositoryStatus`

O estado de um repositório.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `top` | `string` | sim | A pasta de topo do repositório, com `/`. |
| `git_dir` | `string` | sim | A pasta do git (`.git`, ou a de um worktree), com `/`: é ela que muda num commit ou numa troca de ramo feitos fora da AURORA. |
| `prefix` | `string` | sim | A pasta do projeto relativa ao topo, com `/` e sem barra no fim; vazia quando o projeto é o topo. |
| `branch` | [`BranchState`](#tipo-branchstate) | sim | O ramo atual. |
| `files` | lista de [`ChangedFile`](#tipo-changedfile) | sim | Os arquivos com mudança em relação ao último commit, primeiro os de dentro da pasta do projeto. |
| `truncated` | `boolean` | sim | A lista parou em `MAX_FILES`: há mais arquivos mudados do que os listados (falta um `.gitignore`, em geral). |
| `identity` | [`Identity`](#tipo-identity) \| `null` | sim | Quem assina os commits, pela configuração do git; `None` sem nome ou sem e-mail. |
| `remotes` | lista de [`Remote`](#tipo-remote) | sim | Os remotos. |
| `operation` | [`Operation`](#tipo-operation) \| `null` | sim | Uma operação pela metade (um merge com conflito, por exemplo). |

<a id="tipo-requestid"></a>

### `RequestId`

O id de um pedido JSON-RPC, como o cliente o mandou: número ou texto.
Só para o schema de `vboard.input`.

Uma das formas:

**`integer`**

Um número.

**`string`**

Um texto.

<a id="tipo-resolved"></a>

### `Resolved`

As ligações conferidas, bit a bit: o que o topo da placa precisa para ser
gerado.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `board` | [`Board`](#tipo-board) | sim | A placa. |
| `top` | `string` | sim | O módulo do topo. |
| `ports` | lista de [`TopPort`](#tipo-topport) | sim | As portas do topo, na ordem da declaração. |
| `inputs` | lista de [`PortDrive`](#tipo-portdrive) | sim | As entradas do topo, com a origem de cada bit, do bit 0 em diante. |
| `board_outputs` | lista de [`SignalDrive`](#tipo-signaldrive) | sim | Os sinais de saída da placa usados, com a porta que dirige cada bit. |
| `board_inputs` | lista de `string` | sim | Os sinais de entrada da placa usados, na ordem em que aparecem. |
| `clocks` | lista de [`Clock`](#tipo-clock) | sim | As portas ligadas a um clock da placa. |
| `notes` | lista de `string` | sim | Os ajustes feitos: larguras completadas ou cortadas, entradas soltas. |

<a id="tipo-resourceusage"></a>

### `ResourceUsage`

Um recurso da FPGA no resumo do Fitter.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome, como o Quartus escreve (`Total logic elements`, `Total pins`, `Logic utilization (in ALMs)`). |
| `used` | `integer` | sim | Quanto o design usa. |
| `available` | `integer` \| `null` | sim | Quanto a FPGA tem, quando o Quartus diz. |
| `detail` | `boolean` | sim | Detalhe do recurso de cima (`Dedicated logic registers`, dentro de `Total logic elements`). |

<a id="tipo-revision"></a>

### `Revision`

Uma versão de um arquivo, para `git.read_file`.

Uma das formas:

**`kind: "index"`**

O que está preparado (o índice).

Sem campos.

**`kind: "head"`**

O último commit do ramo atual.

Sem campos.

**`kind: "commit"`**

Um commit, pelo identificador.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `oid` | `string` | sim |  |

<a id="tipo-runcomparison"></a>

### `RunComparison`

Dois relatórios comparados: o atual contra a referência.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `current_id` | `string` | sim | O relatório atual. |
| `current` | [`RunMetadata`](#tipo-runmetadata) | sim | O contexto dele. |
| `baseline_id` | `string` | sim | A referência. |
| `baseline` | [`RunMetadata`](#tipo-runmetadata) | sim | O contexto dela. |
| `synthesis` | [`SynthesisComparison`](#tipo-synthesiscomparison) \| `null` | sim | A síntese comparada; `null` se os dois não têm estatísticas do mesmo topo. |
| `simulation` | [`TimingComparison`](#tipo-timingcomparison) \| `null` | sim | A simulação comparada; `null` se os dois não têm tempos do mesmo simulador e testbench. |
| `warnings` | lista de `string` | sim | O que difere no contexto e pode explicar uma mudança (fontes, versões, máquina), em inglês, sem ponto final. |

<a id="tipo-runmetadata"></a>

### `RunMetadata`

O contexto de uma operação guardada.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `timestamp` | `string` | sim | Quando terminou, em UTC (`2026-10-03T21:04:05Z`). |
| `command` | `string` | sim | O comando. |
| `status` | [`Status`](#tipo-status) | sim | Como terminou. |
| `duration_ms` | `integer` | sim | Quanto levou, do começo ao fim, em milissegundos. |
| `aurora_version` | `string` | sim | A versão que gravou. `lace_version` nos relatórios gravados pelo Lace, que a `.lace` levada para a `.aurora8` traz. |
| `project` | `string` | sim | O que identifica o projeto: um hash da raiz dele. Relatórios de projetos diferentes não se comparam. |
| `project_name` | `string` | sim | O nome do projeto. |
| `bundle` | `string` | sim | O bundle de ferramentas (`2026.09.29`) e a plataforma. |
| `platform` | `string` | sim | `linux-x64`, `darwin-arm64`, `windows-x64`. |
| `top` | `string` \| `null` | sim | O módulo de topo: o da síntese, ou o testbench simulado, ou o topo do projeto. |
| `synthesis` | [`SynthesisContext`](#tipo-synthesiscontext) \| `null` | sim | O contexto da síntese, quando houve. |
| `simulation` | [`SimulationContext`](#tipo-simulationcontext) \| `null` | sim | O contexto da simulação, quando houve. |
| `environment` | [`Environment`](#tipo-environment) | sim | A máquina. |

<a id="tipo-runrecord"></a>

### `RunRecord`

Um relatório guardado: o que a comparação e a lista usam. O texto fica ao
lado, em `report.txt` (`report_text`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `schema` | `integer` | sim | O formato (`RECORD_SCHEMA`). |
| `id` | `string` | sim | O identificador (`run-000042`). |
| `metadata` | [`RunMetadata`](#tipo-runmetadata) | sim | O contexto da operação. |
| `synthesis` | [`SynthesisStatistics`](#tipo-synthesisstatistics) \| `null` | sim | As estatísticas do Yosys, quando houve síntese e o Yosys as gravou. |
| `simulation` | [`SimulationTimings`](#tipo-simulationtimings) \| `null` | sim | Os tempos da simulação, quando houve simulação. |

<a id="tipo-runsummary"></a>

### `RunSummary`

Uma linha de `list`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `id` | `string` | sim | O identificador. |
| `timestamp` | `string` \| `null` | sim | Quando terminou; `null` num relatório ilegível. |
| `command` | `string` \| `null` | sim | O comando. |
| `status` | [`Status`](#tipo-status) \| `null` | sim | Como terminou. |
| `top` | `string` \| `null` | sim | O módulo de topo. |
| `synthesis` | `boolean` | sim | Tem estatísticas de síntese. |
| `simulation` | [`Availability`](#tipo-availability) | sim | Quanto dos tempos de simulação tem. |
| `readable` | `boolean` | sim | O `record.json` pôde ser lido. |

<a id="tipo-savedlayout"></a>

### `SavedLayout`

O layout salvo no projeto para a onda de um testbench:
`wave/<testbench>.surf.ron` (`saved_layout`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `path` | `string` | sim | O arquivo, onde o Surfer salva o estado (Ctrl+S). |
| `state` | `string` | sim | O estado que vai para o Surfer: o que o usuário salvou, ou o gerado. |
| `customized` | `boolean` | sim | O usuário salvou o estado depois da última vez que a AURORA o gerou: a AURORA o deixa como está. |

<a id="tipo-savedlayoutinfo"></a>

### `SavedLayoutInfo`

O layout salvo no projeto para a onda aberta.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `path` | `string` | sim | O arquivo. |
| `customized` | `boolean` | sim | O usuário salvou depois que a AURORA o gerou: a AURORA não o refaz. |

<a id="tipo-schematicresult"></a>

### `SchematicResult`

O resultado de `render_schematic`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `module` | `string` | sim | O módulo desenhado. |
| `status` | [`Status`](#tipo-status) | sim | Como terminou. Uma mensagem de erro do `dot` conta como falha mesmo que ele saia com código 0. |
| `failed_step` | [`Step`](#tipo-step) \| `null` | sim | `graph` (Yosys) ou `render` (`dot`) quando falhou. |
| `steps` | lista de [`StepReport`](#tipo-stepreport) | sim | Dois passos: `graph` (`yosys show`) e `render` (`dot`). |
| `diagnostics` | lista de [`Diagnostic`](#tipo-diagnostic) | sim | Mensagens do Yosys e do Graphviz. |
| `artifacts` | lista de [`Artifact`](#tipo-artifact) | sim | O grafo (`.dot`) e o SVG. |
| `svg` | `string` \| `null` | sim | O SVG, quando foi gerado. Fica ao lado do netlist, com o nome do módulo (caracteres fora de `[A-Za-z0-9_.-]` viram `_`). |
| `duration_ms` | `integer` | sim | Quanto o desenho levou, do começo ao fim, em milissegundos. |

<a id="tipo-scopekind"></a>

### `ScopeKind`

O tipo de um escopo. Em JSON, `"module"`, `"generate"` ou `"block"`.

| Valor | Descrição |
|---|---|
| `"module"` | Uma instância de módulo. |
| `"generate"` | Um bloco `generate`. |
| `"block"` | Um bloco `begin` ou `fork` com nome. |

<a id="tipo-serverinfo"></a>

### `ServerInfo`

O que `initialize` devolve.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `aurora` | `string` | sim | A versão da AURORA. |
| `protocol` | `integer` | sim | A versão do protocolo (`PROTOCOL_VERSION`). |
| `prefix` | `string` \| `null` | sim | A pasta da instalação da AURORA, quando o servidor é o `aurora` instalado. |
| `experimental` | mapa de `string` para `boolean` | sim | Cada recurso experimental e se está ligado. |
| `methods` | lista de `string` | sim | Os métodos que este servidor atende. |

<a id="tipo-sessionoutcome"></a>

### `SessionOutcome`

Como a placa virtual terminou.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `reason` | [`EndReason`](#tipo-endreason) | sim | Por que terminou. |
| `time_ps` | `integer` | sim | O tempo simulado no fim, em picossegundos. |
| `termination` | [`Termination`](#tipo-termination) | sim | Como o processo terminou. |
| `duration_ms` | `integer` | sim | Quanto rodou, em milissegundos. |
| `tail` | lista de `string` | sim | As últimas linhas do stderr, quando o processo quebrou. |

<a id="tipo-severity"></a>

### `Severity`

Gravidade de um [`Diagnostic`](#tipo-diagnostic). A ordem (`Error < Warning < Info <
Unknown`) serve para ordenar do mais grave ao menos grave.

Em JSON: `"error"`, `"warning"`, `"info"`, `"unknown"`.

| Valor | Descrição |
|---|---|
| `"error"` | A ferramenta recusou a entrada. |
| `"warning"` | A ferramenta aceitou, mas avisou (`Heads up on line`, `%Warning-`). |
| `"info"` | Mensagem informativa (`Info:`, `Sweet:`, resumos de contagem). |
| `"unknown"` | Linha que o parser não reconheceu. |

<a id="tipo-shutdownreason"></a>

### `ShutdownReason`

Como um pedido terminou, do ponto de vista de quem o atende.

| Valor | Descrição |
|---|---|
| `"requested"` | O cliente pediu (`shutdown`) ou fechou a entrada. |
| `"signal"` | O processo do servidor recebeu um sinal. |
| `"updated"` | A instalação foi atualizada e o binário do servidor é outro: o cliente o abre de novo. |

<a id="tipo-signaldirection"></a>

### `SignalDirection`

A direção de um sinal da placa, do ponto de vista do FPGA.

| Valor | Descrição |
|---|---|
| `"input"` | Entra no FPGA: clock, botão, chave. |
| `"output"` | Sai do FPGA: LED, display. |

<a id="tipo-signaldrive"></a>

### `SignalDrive`

O que dirige cada bit de um sinal de saída da placa.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `signal` | `string` | sim | O sinal. |
| `active_low` | `boolean` | sim | Ativo em nível baixo: os bits sem ligação ficam em 1. |
| `bits` | lista de [`PortBit`](#tipo-portbit) \| `null` | sim | Um item por bit, do bit 0 em diante; `None` fica no nível inativo. |

<a id="tipo-signalkind"></a>

### `SignalKind`

O tipo de um sinal. Em JSON, `"reg"`, `"wire"`, `"integer"` ou `"real"`.

| Valor | Descrição |
|---|---|
| `"reg"` | Uma variável (`reg`, `logic`, `bit`). |
| `"wire"` | Uma rede (`wire`). |
| `"integer"` | `integer`. |
| `"real"` | `real`. |

<a id="tipo-signalscope"></a>

### `SignalScope`

Um escopo da árvore: uma instância de módulo, ou um bloco `generate` ou
`begin` com nome.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome (`dut`, `g[0]`). |
| `path` | `string` | sim | O caminho na hierarquia (`tb.dut`), como vai na escolha. |
| `kind` | [`ScopeKind`](#tipo-scopekind) | sim | Instância de módulo, bloco `generate` ou bloco com nome. |
| `module` | `string` \| `null` | sim | O módulo, numa instância. |
| `processor` | `boolean` | sim | É um processador SAPHO (tem `valr2` e `linetabs`): os sinais dele vão para a onda com qualquer escolha. |
| `signals` | lista de [`WaveSignal`](#tipo-wavesignal) | sim | Os sinais, na ordem do Icarus. |
| `scopes` | lista de [`SignalScope`](#tipo-signalscope) | sim | Os escopos de dentro. |

<a id="tipo-signalvalue"></a>

### `SignalValue`

O valor de uma entrada no contraexemplo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | A porta. |
| `bits` | `string` | sim | Os bits, do mais significativo ao menos, com `x` e `z`. |
| `text` | `string` | sim | O valor para ler: `1`, `4'b0101 (5)`, `16'h00ff (255)`. |

<a id="tipo-simulationcontext"></a>

### `SimulationContext`

O que decide se duas simulações se comparam, e o que vira aviso.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `simulator` | [`Simulator`](#tipo-simulator) | sim | O simulador. |
| `testbench` | `string` | sim | O testbench (o módulo de topo simulado). |
| `component_version` | `string` \| `null` | sim | A versão do componente do simulador no bundle. |
| `sources` | `string` | sim | Hash dos fontes, com os testbenches, e dos programas dos processadores. |
| `inputs` | `string` | sim | Hash das entradas dos processadores (`<proc>/input_*.txt`, ao lado do fonte). |
| `waveform` | `boolean` | sim | A simulação gravou onda. |

<a id="tipo-simulationresult"></a>

### `SimulationResult`

O resultado de `simulate` e de `simulate_project`.

Uma simulação que roda até o `$finish` e termina com código 0 é
`Succeeded`, mesmo que o circuito esteja errado: conferir o comportamento
é olhar `outputs` e a onda. O stdout do testbench (`$display`) está no
`stdout` do passo `simulate`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `top` | `string` | sim | O módulo de topo simulado (o testbench). |
| `simulator` | [`Simulator`](#tipo-simulator) | sim | O simulador usado. Na simulação rápida de um testbench Verilog, o Verilator, qualquer que fosse o pedido. |
| `fast` | `boolean` | sim | A simulação rápida (`SimulationOptions::fast`): rodou sem gravar onda, e `waveform` é `null`. |
| `status` | [`Status`](#tipo-status) | sim | Como a simulação terminou. |
| `failed_step` | [`Step`](#tipo-step) \| `null` | sim | O passo que falhou, quando for o caso: `elaborate` / `verilate` (Verilog que não compila) ou `simulate` (a simulação em si). |
| `steps` | lista de [`StepReport`](#tipo-stepreport) | sim | Icarus: `elaborate` e `simulate`. Verilator: `verilate` e `simulate`. |
| `diagnostics` | lista de [`Diagnostic`](#tipo-diagnostic) | sim | Mensagens de todos os passos. No passo `simulate`, só as linhas que o parser reconhece: o resto do stdout é saída do testbench. |
| `artifacts` | lista de [`Artifact`](#tipo-artifact) | sim | O `.vvp` ou o executável do Verilator, a onda, e cada `output_<n>.txt` escrito. |
| `waveform` | [`Waveform`](#tipo-waveform) \| `null` | sim | A onda gerada, se a simulação chegou ao fim; nunca na simulação rápida. |
| `outputs` | lista de `string` | sim | `<proc>/<proc>/output_<n>.txt` escritos nesta simulação. |
| `missing_inputs` | lista de `string` | sim | Arquivos que o testbench tenta ler e que não existem. Não impedem a simulação (o testbench gerado pelo `asmcomp` confere o `$fopen` antes de ler), mas a porta correspondente não recebe dado nenhum. |
| `tests` | [`TestReport`](#tipo-testreport) \| `null` | sim | Os testes de um testbench cocotb (`.py`), lidos do `results.xml`; `null` num testbench Verilog, ou se o cocotb não chegou a gravar o resultado. |
| `duration_ms` | `integer` | sim | Quanto a simulação levou, do começo ao fim (preparar, compilar e rodar), em milissegundos. |

<a id="tipo-simulationtimings"></a>

### `SimulationTimings`

Os tempos de uma simulação, em milissegundos de relógio. O tempo simulado
é outra grandeza: o quanto o tempo andou dentro do modelo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `succeeded` | `boolean` | sim | A simulação terminou com sucesso. |
| `compile_ms` | `integer` \| `null` | sim | O passo que compila o Verilog (`iverilog` ou `verilator --binary`). |
| `execution_ms` | `integer` \| `null` | sim | O passo que roda o testbench (`vvp` ou o modelo do Verilator). |
| `total_ms` | `integer` \| `null` | sim | A simulação inteira, medida à parte: com a preparação da AURORA. |
| `simulated_fs` | `integer` \| `null` | sim | O tempo simulado no `$finish`, em femtossegundos, quando o simulador o informa (o `vvp` informa; o Verilator, não). |

<a id="tipo-simulator"></a>

### `Simulator`

Qual simulador usar. Em JSON: `"icarus"`, `"verilator"` ou `"ghdl"`.

Um testbench VHDL roda sempre no GHDL, o simulador de VHDL: num pedido
com `icarus` ou `verilator`, que valem para o Verilog, ele vai para o
GHDL, e o resultado diz `ghdl`. O GHDL com um testbench Verilog é o erro
`AuroraError::SimulatorLanguage`.

| Valor | Descrição |
|---|---|
| `"icarus"` | Icarus Verilog (`iverilog` + `vvp`): compila em segundos, simula devagar. O padrão. |
| `"verilator"` | Verilator: compila o modelo para C++ (dezenas de segundos na primeira vez; o `obj_dir` é reaproveitado depois), simula rápido. |
| `"ghdl"` | GHDL: o simulador de VHDL. Analisa os arquivos VHDL, elabora o testbench e o roda, gravando a onda em GHW. |

<a id="tipo-skippedpath"></a>

### `SkippedPath`

Um arquivo ou pasta que a conversão deixou onde estava.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `processor` | `string` | sim | De que processador. |
| `from` | `string` | sim | Onde está, absoluto. |
| `to` | `string` | sim | Para onde iria. |
| `reason` | `string` | sim | Por quê: o destino já existe, ou o erro do sistema. |

<a id="tipo-sourcelanguage"></a>

### `SourceLanguage`

A linguagem de um arquivo do projeto. Em JSON: `"verilog"`, `"vhdl"` ou
`"python"`.

| Valor | Descrição |
|---|---|
| `"verilog"` | Verilog ou SystemVerilog: `.v` e `.sv`. Simula no Icarus ou no Verilator; é o que a síntese, a hierarquia pelo Icarus e as placas leem. |
| `"vhdl"` | VHDL: `.vhd` e `.vhdl`, em qualquer caixa. Simula no GHDL. |
| `"python"` | Python: um testbench do cocotb (`.py`). |

<a id="tipo-status"></a>

### `Status`

Como uma operação terminou. Em JSON, em `snake_case` (`"succeeded"`).

| Valor | Descrição |
|---|---|
| `"succeeded"` | Todos os passos terminaram com código 0 e os artefatos foram gerados. |
| `"failed"` | Uma ferramenta recusou a entrada (código de saída diferente de 0). |
| `"crashed"` | Uma ferramenta morreu (sinal no Unix, exceção no Windows). É defeito da ferramenta, não do código do usuário. |
| `"incomplete"` | As ferramentas terminaram bem, mas faltou algum artefato obrigatório. |
| `"cancelled"` | O cancelamento foi pedido (`CancelToken`): o passo que rodava foi encerrado e os seguintes não rodaram. |
| `"timed_out"` | Um passo passou do prazo e a AURORA o encerrou. Numa simulação, é o testbench que não chega ao `$finish`. |

<a id="tipo-step"></a>

### `Step`

Um passo de uma operação. Em JSON, em `snake_case` (`"pre_assemble"`).

| Valor | Descrição |
|---|---|
| `"preprocess"` | `cpppp`, do YANC até a 6.x, nos relatórios antigos (o `cppcomp` da 7.1 pré-processa). |
| `"compile"` | `cmmcomp` ou `cppcomp`: fonte para assembly. |
| `"pre_assemble"` | `appcomp`, do YANC até a 6.x, nos relatórios antigos (o `asmcomp` da 7.1 faz as duas passadas). |
| `"assemble"` | `asmcomp`: assembly para Verilog e memórias. |
| `"check_syntax"` | `iverilog -t null`: só confere se o Verilog elabora. |
| `"lint"` | `verilator --lint-only`: a verificação mais rigorosa (larguras, sinais sem uso), com `aurora check --lint`. |
| `"analyze"` | `ghdl -i`: lê os arquivos VHDL para a biblioteca de trabalho; os erros de sintaxe aparecem aqui. |
| `"elaborate"` | `iverilog`: Verilog para o `.vvp` do Icarus. No VHDL, `ghdl -m`: a análise de cada unidade na ordem das dependências e a elaboração. |
| `"verilate"` | `verilator --binary`: Verilog para um executável C++. |
| `"simulate"` | `vvp`, o executável do Verilator ou o GHDL rodando o testbench. |
| `"synthesize"` | `yosys`: Verilog para netlist JSON. Numa placa, o `quartus_map` (Analysis & Synthesis do Quartus). |
| `"graph"` | `yosys show`: netlist para o grafo do esquemático (`.dot`). |
| `"render"` | `dot`: grafo do esquemático para SVG. |
| `"fit"` | `quartus_fit`: posiciona e roteia o design na FPGA da placa (Fitter). |
| `"bitstream"` | `quartus_asm`: os arquivos de gravação (`.sof`, `.rbf`, `.svf`), no Assembler do Quartus. |
| `"timing"` | `quartus_sta`: a análise de tempo (Timing Analyzer), com a folga e a Fmax de cada clock. |
| `"program"` | `quartus_pgm`: grava o `.sof` na FPGA da placa pelo cabo JTAG. |

<a id="tipo-stepreport"></a>

### `StepReport`

O que aconteceu num passo: o comando exato, como terminou e tudo o que
escreveu. É o registro de auditoria da operação; os `diagnostics` do
resultado são a versão interpretada de `stdout` e `stderr`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `step` | [`Step`](#tipo-step) | sim | Qual passo. |
| `tool` | [`Tool`](#tipo-tool) | sim | A ferramenta executada. Para o executável que o Verilator gera, é `Tool::Verilator`. |
| `command` | [`Invocation`](#tipo-invocation) | sim | O que foi executado, com CWD e ambiente. |
| `termination` | [`Termination`](#tipo-termination) | sim | Como o processo terminou. |
| `stdout` | `string` | sim | Tudo que o processo escreveu no stdout, decodificado como UTF-8 (bytes inválidos viram `U+FFFD`). Numa simulação, inclui o `$display` do testbench. |
| `stderr` | `string` | sim | Tudo que o processo escreveu no stderr. |
| `duration_ms` | `integer` | sim | Duração, do início do processo até o fim, em milissegundos. |

<a id="tipo-stream"></a>

### `Stream`

De qual saída do processo veio uma linha. Em JSON: `"stdout"` ou
`"stderr"`.

| Valor | Descrição |
|---|---|
| `"stdout"` | Saída padrão. Numa simulação, é onde sai o `$display` do testbench. |
| `"stderr"` | Saída de erro. |

<a id="tipo-syncresult"></a>

### `SyncResult`

`git.fetch`, `git.pull` e `git.push`: o que foi feito com o remoto.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `remote` | `string` | sim | O remoto. |
| `branch` | `string` \| `null` | sim | O ramo. |
| `upstream` | `string` \| `null` | sim | O ramo remoto que ele acompanha, depois da operação. |
| `published` | `boolean` | sim | O ramo foi publicado agora (não existia no remoto). |

<a id="tipo-synthesiscomparison"></a>

### `SynthesisComparison`

As estatísticas de síntese, comparadas.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `metrics` | lista de [`SynthesisMetricComparison`](#tipo-synthesismetriccomparison) | sim | As contagens do resumo, na ordem do relatório. |
| `cell_types` | lista de [`CellComparison`](#tipo-cellcomparison) | sim | Os tipos de célula, da maior mudança para a menor e, no empate, pelo nome. |
| `added_cell_types` | `integer` | sim | Tipos que só existem no atual. |
| `removed_cell_types` | `integer` | sim | Tipos que só existem na referência. |
| `increased` | `integer` | sim | Contagens do resumo que aumentaram. |
| `decreased` | `integer` | sim | Que diminuíram. |
| `unchanged` | `integer` | sim | Iguais. |
| `not_comparable` | `integer` | sim | Sem um dos lados. |

<a id="tipo-synthesiscontext"></a>

### `SynthesisContext`

O que decide se duas sínteses se comparam, e o que vira aviso.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `top` | `string` | sim | O módulo de topo. |
| `component_version` | `string` \| `null` | sim | A versão do componente yosys do bundle. |
| `sources` | `string` | sim | Hash dos fontes sintetizáveis e dos programas dos processadores. |

<a id="tipo-synthesismetric"></a>

### `SynthesisMetric`

Uma contagem do resumo do projeto.

| Valor | Descrição |
|---|---|
| `"modules"` | Módulos no projeto, contando o de topo. |
| `"wires"` | Fios. |
| `"wire_bits"` | Bits de fio. |
| `"public_wires"` | Fios com nome do usuário. |
| `"public_wire_bits"` | Bits de fio com nome do usuário. |
| `"memories"` | Memórias. |
| `"memory_bits"` | Bits de memória. |
| `"processes"` | Processos que sobraram depois do `proc`. |
| `"cells"` | Células, sem contar as instâncias de submódulo. |

<a id="tipo-synthesismetriccomparison"></a>

### `SynthesisMetricComparison`

Uma contagem do resumo, comparada.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `metric` | [`SynthesisMetric`](#tipo-synthesismetric) | sim | Qual. |
| `comparison` | [`MetricComparison`](#tipo-metriccomparison) | sim | A comparação. |

<a id="tipo-synthesisresult"></a>

### `SynthesisResult`

O resultado de `synthesize`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `top` | `string` | sim | O módulo de topo sintetizado. |
| `status` | [`Status`](#tipo-status) | sim | Como a síntese terminou. |
| `failed_step` | [`Step`](#tipo-step) \| `null` | sim | `synthesize` quando falhou. |
| `steps` | lista de [`StepReport`](#tipo-stepreport) | sim | Um passo: o `yosys`. O log completo fica em `yosys.log`, ao lado do netlist. |
| `diagnostics` | lista de [`Diagnostic`](#tipo-diagnostic) | sim | Os erros e avisos do Yosys, com arquivo e linha quando ele informa. |
| `artifacts` | lista de [`Artifact`](#tipo-artifact) | sim | O netlist (`hierarchy.json`). |
| `netlist` | `string` \| `null` | sim | `hierarchy.json`, quando a síntese terminou. |
| `modules` | lista de `string` | sim | Os módulos do netlist, na ordem do JSON, para escolher o que desenhar. |
| `statistics` | [`SynthesisStatistics`](#tipo-synthesisstatistics) \| `null` | sim | O que o `stat` do Yosys contou no netlist, quando a síntese terminou e o Yosys gravou o `stat.json`. |
| `duration_ms` | `integer` | sim | Quanto a síntese levou, do começo ao fim, em milissegundos. |

<a id="tipo-synthesisstatistics"></a>

### `SynthesisStatistics`

O que o `stat` do Yosys contou. Uma contagem que ele não informou fica
`null`, e é diferente de zero.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `tool` | `string` | sim | O Yosys que contou, como ele se identifica (`Yosys 0.69+156 (...)`). |
| `top` | `string` | sim | O módulo de topo. |
| `modules` | `integer` \| `null` | sim | Módulos. |
| `wires` | `integer` \| `null` | sim | Fios. |
| `wire_bits` | `integer` \| `null` | sim | Bits de fio. |
| `public_wires` | `integer` \| `null` | sim | Fios com nome do usuário. |
| `public_wire_bits` | `integer` \| `null` | sim | Bits de fio com nome do usuário. |
| `memories` | `integer` \| `null` | sim | Memórias. |
| `memory_bits` | `integer` \| `null` | sim | Bits de memória. |
| `processes` | `integer` \| `null` | sim | Processos. |
| `cells` | `integer` \| `null` | sim | Células, sem as instâncias de submódulo. |
| `cell_types` | lista de [`CellUsage`](#tipo-cellusage) | sim | As células por tipo, da mais usada para a menos usada e, no empate, pelo nome. |

<a id="tipo-systemcompiler"></a>

### `SystemCompiler`

O compilador C++, o `make` e o Perl que o Verilator usa.

No Windows vêm do bundle, no componente verilator (`msys/ucrt64/bin` e
`msys/usr/bin`), e `SystemCompiler::bundled` é verdadeiro. No Linux e no
macOS vêm do sistema, a exceção à regra do bundle, procurados em locais
fixos (`SystemCompiler::detect`), nunca no `PATH`: `/usr/bin` no Linux;
`/usr/bin` com as Command Line Tools do Xcode no macOS. Ou num diretório
declarado (`SystemCompiler::in_dir`, e `SystemCompiler::in_msys2`
para um MSYS2 no Windows), que substitui o padrão.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `perl` | `string` | sim | O Perl que executa o script `verilator`. |
| `make` | `string` | sim | O `make` que o Verilator chama. |
| `cxx` | `string` | sim | O compilador C++ que o `make` chama. |
| `path` | lista de `string` | sim | Os diretórios postos no `PATH` do Verilator para ele achar `make` e o compilador. |
| `bundled` | `boolean` | sim | Veio do bundle (Windows), e não do sistema. |

<a id="tipo-termination"></a>

### `Termination`

Como um processo terminou.

Em JSON: `{"kind": "exited", "value": 1}`, `{"kind": "signaled",
"value": 11}`, `{"kind": "exception", "value": 3221225477}`,
`{"kind": "cancelled"}`, `{"kind": "timed_out"}`, `{"kind": "unknown"}`.

Um código de saída e um sinal são coisas diferentes: `msg_internal` do
`cppcomp` chama `abort()`, e um `asmcomp` que perde um `fopen` morre por
segfault. Nenhum dos dois é um "erro de compilação" do usuário.

Uma das formas:

**`kind: "exited"`**

Terminou com um código de saída.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `value` | `integer` | sim |  |

**`kind: "signaled"`**

Morto por sinal (só Unix).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `value` | `integer` | sim |  |

**`kind: "exception"`**

Terminou com um código NTSTATUS de exceção (só Windows), por exemplo
`0xC0000005`, violação de acesso.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `value` | `integer` | sim |  |

**`kind: "cancelled"`**

A AURORA encerrou o processo porque a operação foi cancelada
(`CancelToken::cancel`).

Sem campos.

**`kind: "timed_out"`**

A AURORA encerrou o processo porque ele passou do prazo (por exemplo,
`SimulationOptions::timeout`).

Sem campos.

**`kind: "unknown"`**

O sistema não informou nem código nem sinal.

Sem campos.

<a id="tipo-testcase"></a>

### `TestCase`

Um teste (`@cocotb.test()`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | `<módulo>.<teste>`, como o cocotb mostra (`test_somador.basic_test`). |
| `status` | [`TestStatus`](#tipo-teststatus) | sim | Como terminou. |
| `message` | `string` \| `null` | sim | Por que falhou (a mensagem da exceção) ou não rodou; `null` quando passou. |
| `file` | `string` \| `null` | sim | O `.py` do teste. |
| `line` | `integer` \| `null` | sim | Numa falha, a linha mais funda do traceback dentro do `.py`; senão, a linha do teste. |

<a id="tipo-testreport"></a>

### `TestReport`

O resultado dos testes de um testbench cocotb, lido do `results.xml`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `results` | `string` | sim | O `results.xml` que o cocotb gravou. |
| `cases` | lista de [`TestCase`](#tipo-testcase) | sim | Os testes, na ordem em que rodaram. |
| `passed` | `integer` | sim | Quantos passaram. |
| `failed` | `integer` | sim | Quantos falharam. |
| `skipped` | `integer` | sim | Quantos não rodaram (`@cocotb.test(skip=True)`). |

<a id="tipo-teststatus"></a>

### `TestStatus`

Como um teste terminou. Em JSON: `"passed"`, `"failed"` ou `"skipped"`.

| Valor | Descrição |
|---|---|
| `"passed"` | Passou. |
| `"failed"` | Falhou: um `assert` ou uma exceção. |
| `"skipped"` | Não rodou. |

<a id="tipo-timingcomparison"></a>

### `TimingComparison`

Os tempos de simulação, comparados.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `compile` | [`MetricComparison`](#tipo-metriccomparison) | sim | Compilação, em milissegundos. |
| `execution` | [`MetricComparison`](#tipo-metriccomparison) | sim | Execução, em milissegundos. |
| `total` | [`MetricComparison`](#tipo-metriccomparison) | sim | A simulação inteira, em milissegundos. |
| `simulated` | [`MetricComparison`](#tipo-metriccomparison) | sim | O tempo simulado, em femtossegundos. |
| `environment_changed` | `boolean` | sim | As duas rodaram em máquinas diferentes. |

<a id="tipo-timingsummary"></a>

### `TimingSummary`

O tempo do design.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `clocks` | lista de [`ClockTiming`](#tipo-clocktiming) | sim | Um por clock, na ordem em que o relatório os cita. |
| `met` | `boolean` | sim | Nenhuma folga de setup ou de hold é negativa. |

<a id="tipo-tool"></a>

### `Tool`

As ferramentas que a AURORA sabe orquestrar.

Em JSON e em `Display`, cada uma aparece pelo nome do
programa (`Tool::binary_name`).

| Valor | Descrição |
|---|---|
| `"cmmcomp"` | Compilador C± do YANC: `.cmm` para `.asm`. |
| `"appcomp"` | Pré-assembler do YANC até a 6.x, que contava instruções e variáveis do `.asm`. Na 7.1 é a primeira passada do `Tool::Asmcomp`; fica aqui para os relatórios de builds antigos. |
| `"asmcomp"` | Assembler do YANC: `.asm` para o Verilog do processador, as memórias `.mif` e o testbench. |
| `"cpppp"` | Pré-processador do fluxo C do YANC até a 6.x (`#include`, `#define`). Na 7.1 faz parte do `Tool::Cppcomp`; fica aqui para os relatórios de builds antigos. |
| `"cppcomp"` | Compilador C do YANC: `.cpp` para `.asm`, com o pré-processador. |
| `"iverilog"` | Compilador do Icarus Verilog. |
| `"vvp"` | Simulador do Icarus: executa o `.vvp` que o `iverilog` gera. |
| `"verilator"` | Verilator: compila o Verilog para um executável C++ e o roda como simulação, com o compilador de [`SystemCompiler`](#tipo-systemcompiler). |
| `"ghdl"` | GHDL: analisa, elabora e simula VHDL. |
| `"yosys"` | Yosys: sintetiza Verilog para netlist e gera o grafo do esquemático. |
| `"dot"` | `dot` do Graphviz: desenha o grafo do esquemático em SVG. |
| `"surfer"` | O visualizador de ondas surfer-aurora. Aplicação gráfica: abre e fica aberta. |
| `"openfpgaloader"` | O openFPGALoader: grava um bitstream (`.rbf`, `.svf`) na placa pelo cabo USB dela. |
| `"quartus"` | O Quartus Prime do sistema, que compila para as placas Intel (`crate::fpga`). Cada passo roda um programa de linha de comando dele (`quartus_map`, `quartus_fit`, `quartus_asm`, `quartus_sta`); não vem no bundle (`crate::fpga::Quartus`). |
| `"perl"` | O Perl que executa o script `verilator`, parte do [`SystemCompiler`](#tipo-systemcompiler): do sistema no Linux e no macOS, do bundle no Windows. |

<a id="tipo-toolstatus"></a>

### `ToolStatus`

Uma ferramenta do bundle: onde está, ou por que não está.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome do executável (`iverilog`, `cmmcomp`). |
| `component` | `string` \| `null` | sim | O componente que a traz; `None` para o Perl do sistema e o Quartus. |
| `path` | `string` \| `null` | sim | O executável, quando disponível. |
| `system` | `boolean` | sim | Vem do sistema, não do bundle: o Perl do Verilator no Linux e no macOS (no Windows ele vem no bundle) e o Quartus. |
| `error` | [`ErrorInfo`](#tipo-errorinfo) \| `null` | sim | Por que não está disponível. |

<a id="tipo-toolchaininfo"></a>

### `ToolchainInfo`

O que uma tela de ferramentas mostra: o `aurora tools` em dados. Nunca
falha: um bundle que não abriu vira `found: false` com o erro
(`describe_error`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `found` | `boolean` | sim | O bundle abriu. |
| `error` | [`ErrorInfo`](#tipo-errorinfo) \| `null` | sim | Por que não abriu. |
| `origin` | [`Origin`](#tipo-origin) \| `null` | sim | Como foi achado. |
| `root` | `string` \| `null` | sim | A pasta do bundle. |
| `bundle` | `string` \| `null` | sim | A versão do bundle. |
| `platform` | `string` \| `null` | sim | A plataforma do bundle (`linux-x64`, `darwin-arm64`, `windows-x64`). |
| `components` | lista de [`BundleComponent`](#tipo-bundlecomponent) | sim | Os componentes instalados. |
| `not_installed` | lista de `string` | sim | Os componentes que a AURORA conhece e que não estão instalados. |
| `tools` | lista de [`ToolStatus`](#tipo-toolstatus) | sim | Cada ferramenta que a AURORA roda. |
| `system_compiler` | [`SystemCompiler`](#tipo-systemcompiler) \| `null` | sim | O compilador do Verilator: do sistema no Linux e no macOS, do bundle no Windows (`bundled`). |
| `compiler_error` | [`ErrorInfo`](#tipo-errorinfo) \| `null` | sim | O compilador declarado não serve. |
| `quartus` | [`Quartus`](#tipo-quartus) \| `null` | sim | O Quartus Prime do sistema, se encontrado. |
| `quartus_error` | [`ErrorInfo`](#tipo-errorinfo) \| `null` | sim | A pasta declarada do Quartus não serve. |
| `aurora_cli` | `string` \| `null` | sim | A CLI `aurora` da instalação. |
| `surfer_web` | `string` \| `null` | sim | O cliente web do Surfer (`surfer-aurora/web`), para a onda numa aba; `None` num bundle que não o traz. |

<a id="tipo-topoutcome"></a>

### `TopOutcome`

`project.top`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `top_level` | `string` \| `null` | sim | O arquivo de topo, se escolhido. |
| `module` | `string` \| `null` | sim | O módulo de topo. |
| `module_error` | [`ErrorInfo`](#tipo-errorinfo) \| `null` | sim | Por que não dá para saber o módulo de um arquivo de topo escolhido. |

<a id="tipo-topport"></a>

### `TopPort`

Uma porta do topo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome. |
| `input` | `boolean` | sim | `true` para entrada, `false` para saída. |
| `width` | `integer` | sim | A largura. |

<a id="tipo-updateaction"></a>

### `UpdateAction`

O que `aurora update` fez.

| Valor | Descrição |
|---|---|
| `"checked"` | `--check`: só comparou as versões. |
| `"up_to_date"` | Não há AURORA mais nova publicado: nada mudou. |
| `"updated"` | A instalação foi trocada (por componentes, ou pelo instalador no Linux e no macOS). O `aurora` em uso é o antigo: quem chamou termina e reinicia. |
| `"wizard_opened"` | O assistente de instalação do Windows foi aberto e termina depois que quem chamou sai. |

<a id="tipo-updatechanges"></a>

### `UpdateChanges`

O que a atualização por componentes trocou.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `components` | lista de `string` | sim | Os componentes com arquivos que entraram ou saíram, na ordem do bundle; `aurora` é a própria AURORA e o cabeçalho do bundle. |
| `download_bytes` | `integer` | sim | Bytes baixados. |
| `files` | `integer` | sim | Arquivos que entraram (novos ou trocados). |
| `removed` | `integer` | sim | Arquivos que saíram. |

<a id="tipo-updatemethod"></a>

### `UpdateMethod`

Como `aurora update` atualizou.

| Valor | Descrição |
|---|---|
| `"components"` | Só os arquivos que mudaram, dos pedaços que a release publica. |
| `"installer"` | O instalador inteiro da versão nova. |

<a id="tipo-updatereport"></a>

### `UpdateReport`

O relatório do `aurora update` (`docs/schema/update.json`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `aurora` | [`AuroraVersions`](#tipo-auroraversions) | sim | A AURORA: este e o da última release. |
| `bundle` | [`BundleVersions`](#tipo-bundleversions) | sim | O bundle de ferramentas: o instalado e o da última release. |
| `components` | lista de [`ComponentVersions`](#tipo-componentversions) | sim | Cada componente instalado, na ordem do manifesto do bundle. Vazio sem bundle. |
| `prefix` | `string` \| `null` | sim | A pasta da instalação; `null` se este `aurora` não foi instalado pelo instalador (um build em `target/`, por exemplo). |
| `action` | [`UpdateAction`](#tipo-updateaction) | sim | O que o comando fez. |
| `method` | [`UpdateMethod`](#tipo-updatemethod) \| `null` | sim | Como atualizou; `null` quando não atualizou. |
| `changes` | [`UpdateChanges`](#tipo-updatechanges) \| `null` | sim | O que a atualização por componentes trocou, ou vai trocar com `--check --from`; `null` quando ela não rodou. |

<a id="tipo-verdict"></a>

### `Verdict`

Como terminou a correção.

| Valor | Descrição |
|---|---|
| `"solved"` | Compilou e nenhuma saída errou. |
| `"compile_error"` | O Verilog não compila: os erros estão em `diagnostics`. |
| `"mismatch"` | Simulou, e alguma saída diferiu da referência. |
| `"incomplete"` | A simulação terminou sem o resumo do testbench, ou parou com erro. |
| `"timed_out"` | A simulação passou do prazo: um laço sem fim, ou um laço combinacional que não estabiliza. |
| `"cancelled"` | A correção foi cancelada. |

<a id="tipo-wavechoice"></a>

### `WaveChoice`

Uma mudança na escolha de sinais da onda do testbench do projeto
(`choose_signals`).

Uma das formas:

**`kind: "all"`**

Todos os sinais de novo: a escolha fica vazia e o arquivo sai.

Sem campos.

**`kind: "add"`**

Acrescenta sinais e escopos (caminhos como `tb.dut.q`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `paths` | lista de `string` | sim |  |

**`kind: "remove"`**

Tira sinais e escopos.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `paths` | lista de `string` | sim |  |

**`kind: "set"`**

Troca a escolha inteira por estes sinais e escopos; vazia, como
`All`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `paths` | lista de `string` | sim |  |

<a id="tipo-wavechoiceoutcome"></a>

### `WaveChoiceOutcome`

A escolha de sinais depois da mudança.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `testbench` | `string` | sim | O módulo do testbench do projeto. |
| `file` | `string` \| `null` | sim | O arquivo da escolha (`wave/<testbench>.json`); `None` quando ela ficou vazia e o arquivo saiu: a onda grava todos os sinais. |
| `selection` | lista de `string` | sim | A escolha; vazia, todos os sinais. |

<a id="tipo-wavelayout"></a>

### `WaveLayout`

O layout de uma onda, em memória.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `state` | `string` | sim | O conteúdo do `.surf.ron`. |
| `mappings` | lista de [`MappingTranslator`](#tipo-mappingtranslator) | sim | Os tradutores de valor que o estado usa, pelo nome. |
| `processors` | lista de [`WaveProcessor`](#tipo-waveprocessor) | sim | Os processadores achados na onda, na ordem do cabeçalho. |
| `testbench` | `string` \| `null` | sim | O testbench da onda (o escopo de cima dela): o nome dos arquivos dele em `wave/`, a escolha de sinais e o layout salvo. |
| `selection` | lista de `string` | sim | A escolha de sinais que o layout seguiu (`wave/<testbench>.json`); vazia sem escolha. |

<a id="tipo-wavelayoutinfo"></a>

### `WaveLayoutInfo`

`wave.layout`: o que um visualizador próprio precisa para mostrar a
onda com o layout da AURORA.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `layout` | [`WaveLayout`](#tipo-wavelayout) \| `null` | sim | O layout dos processadores SAPHO, gerado da onda; `None` sem processador. |
| `saved` | [`SavedLayout`](#tipo-savedlayout) \| `null` | sim | O layout salvo no projeto da onda (`wave/<testbench>.surf.ron`), onde o Surfer salva (Ctrl+S): o do usuário, se ele salvou depois da última geração, senão o gerado agora; `None` numa onda fora de projeto. |
| `layout_error` | `string` \| `null` | sim | Por que o layout não foi gerado (a onda abre sem ele). |
| `exercise_layout` | `string` \| `null` | sim | Os comandos do Surfer de um exercício do `aurora learn` (o `.aurora-learn/wave.sucl` ao lado da onda), quando a onda é de um. |

<a id="tipo-waveopened"></a>

### `WaveOpened`

A onda aberta no surfer-aurora, que continua aberto depois.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `waveform` | `string` | sim | O arquivo da onda. |
| `pid` | `integer` | sim | O PID do surfer-aurora. |
| `log` | `string` | sim | Onde vai o stdout e o stderr do surfer-aurora. |
| `layout` | `string` \| `null` | sim | O estado do Surfer com que a onda abriu (`.surf.ron` ou `.sucl`): o layout salvo no projeto, o gerado, ou o de um exercício; `None` sem layout. |
| `saved_layout` | [`SavedLayoutInfo`](#tipo-savedlayoutinfo) \| `null` | sim | O layout salvo no projeto (`wave/<testbench>.surf.ron`), onde o Surfer salva (Ctrl+S); `None` numa onda fora de projeto ou sem layout gerado. |
| `processors` | lista de [`WaveProcessor`](#tipo-waveprocessor) | sim | Os processadores do layout gerado. |
| `selection` | lista de `string` | sim | A escolha de sinais que o layout gerado seguiu (`wave/<testbench>.json`); vazia sem escolha ou sem layout gerado. |
| `outdated` | lista de `string` | sim | Os processadores compilados de novo depois desta simulação: o PC e a linha aparecem como números. |
| `layout_error` | `string` \| `null` | sim | O layout gerado falhou e a onda abriu sem ele: o motivo. |
| `reset_deleted` | `string` \| `null` | sim | Com `LayoutPolicy::Reset`: o testbench cujo layout salvo foi apagado antes de abrir. |

<a id="tipo-wavepolicy"></a>

### `WavePolicy`

Como um fluxo abre a onda que a simulação entregou.

| Valor | Descrição |
|---|---|
| `"none"` | Não abre: o cliente mostra a onda por conta própria (a aba do Studio). O caminho fica em `SimulationResult::waveform`. |
| `"window"` | Abre no surfer-aurora, em janela, com o layout dos processadores SAPHO (`open_wave`). |

<a id="tipo-waveprocessor"></a>

### `WaveProcessor`

Um processador SAPHO achado na onda.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `instance` | `string` | sim | O escopo da instância na onda (`soma_tb.proc`). |
| `processor` | `string` | sim | O processador (o nome da pasta no projeto e do fonte). |
| `variables` | `integer` | sim | Quantas variáveis do programa estão na onda (cada elemento de array conta). |
| `assembly` | `boolean` | sim | O PC aparece como instrução de assembly (havia `trad_opcode.txt`). |
| `source` | `boolean` | sim | A linha do fonte aparece como texto (havia `src_lines.txt`). |
| `outdated` | `boolean` | sim | As tabelas são mais novas que a onda: o processador foi compilado de novo depois da simulação, e o assembly e a linha do C± sairiam deslocados. O PC e a linha aparecem como números até simular de novo; as interfaces avisam. |

<a id="tipo-waverequest"></a>

### `WaveRequest`

O pedido de `open_wave`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `source` | [`WaveSource`](#tipo-wavesource) | sim | Qual onda. |
| `layout` | [`LayoutPolicy`](#tipo-layoutpolicy) | não (padrão: `{"kind":"generated"}`) | O layout. |

<a id="tipo-wavesignal"></a>

### `WaveSignal`

Um sinal da árvore.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `name` | `string` | sim | O nome. |
| `path` | `string` | sim | O caminho na hierarquia (`tb.dut.q`). |
| `width` | `integer` | sim | Bits (64 num `real`). |
| `kind` | [`SignalKind`](#tipo-signalkind) | sim | `reg`, `wire`, `integer` ou `real`. |
| `direction` | [`PortDirection`](#tipo-portdirection) \| `null` | sim | A direção, quando o sinal é uma porta do módulo. |

<a id="tipo-wavesignals"></a>

### `WaveSignals`

A árvore de sinais do testbench do projeto, para escolher o que a onda
grava (`wave_signals`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `testbench` | `string` | sim | O testbench elaborado (o da simulação do projeto). |
| `module` | `string` | sim | O módulo do testbench: a raiz da árvore e o nome dos arquivos em `wave/`. |
| `root` | [`SignalScope`](#tipo-signalscope) \| `null` | sim | A árvore, a partir do módulo do testbench. `None` quando a elaboração falhou (os erros em `diagnostics`). |
| `selection` | lista de `string` | sim | A escolha gravada; vazia: a simulação grava todos os sinais. |
| `selection_file` | `string` | sim | Onde a escolha fica (`wave/<módulo>.json`), exista ou não. |
| `unknown` | lista de `string` | sim | Itens da escolha que a árvore não tem (um sinal renomeado, um módulo que saiu): a simulação os deixa de fora. |
| `status` | [`Status`](#tipo-status) | sim | Como a elaboração terminou. |
| `steps` | lista de [`StepReport`](#tipo-stepreport) | sim | A elaboração (`iverilog`). |
| `diagnostics` | lista de [`Diagnostic`](#tipo-diagnostic) | sim | Os erros e avisos da elaboração. |
| `duration_ms` | `integer` | sim | Quanto levou, em milissegundos. |

<a id="tipo-wavesource"></a>

### `WaveSource`

Qual onda abrir.

Uma das formas:

**`kind: "file"`**

Um arquivo VCD, FST ou GHW.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `path` | `string` | sim | O arquivo. |

**`kind: "project"`**

A onda da simulação de um processador do projeto, ou, sem
processador, a da simulação do projeto
(`waveform_path`).

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `processor` | `string` \| `null` | não (padrão: `null`) | O processador; `None`, o projeto. |

<a id="tipo-waveform"></a>

### `Waveform`

Uma onda gerada por simulação, pronta para
`open_waveform`.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `path` | `string` | sim | Caminho absoluto, com o nome que o `$dumpfile` do testbench deu. |
| `format` | [`WaveformFormat`](#tipo-waveformformat) | sim | O formato do conteúdo. |

<a id="tipo-waveformformat"></a>

### `WaveformFormat`

O formato do conteúdo de uma onda: FST no Icarus, VCD no Verilator, GHW
no GHDL. A extensão é a do formato, menos quando o testbench nomeia a onda
com uma expressão que a AURORA não resolve: aí o Icarus grava VCD no nome
que o testbench der.

| Valor | Descrição |
|---|---|
| `"vcd"` | Value Change Dump, texto. |
| `"fst"` | Fast Signal Trace, binário e comprimido. |
| `"ghw"` | O formato do GHDL, com os tipos do VHDL (os estados de um tipo enumerado aparecem pelo nome). |

<a id="tipo-workspacesnapshot"></a>

### `WorkspaceSnapshot`

A pasta de exercícios como uma interface a mostra: a trilha, os
capítulos com os enunciados e o que o aluno já fez
(`Workspace::snapshot`). É o resultado do `learn.status` do protocolo.

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `root` | `string` | sim | A pasta de exercícios. |
| `track` | `string` | sim | A trilha (`verilog`). |
| `title` | `string` | sim | O título da trilha. |
| `welcome` | `string` | sim | As boas-vindas, em markdown. |
| `farewell` | `string` \| `null` | sim | A mensagem do fim, em markdown. |
| `chapters` | lista de [`ChapterSnapshot`](#tipo-chaptersnapshot) | sim | Os capítulos, na ordem. |
| `current` | `string` | sim | O exercício atual. |
| `solved` | `integer` | sim | Quantos resolvidos. |
| `total` | `integer` | sim | Quantos exercícios. |
