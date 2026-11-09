# Contribuir com a AURORA

Guia para quem vai mexer no código da AURORA pela primeira vez: como preparar
a máquina, onde mora cada coisa, como fazer as mudanças mais comuns e o que
conferir antes de mandar. O que a AURORA faz para o usuário está no
[README](README.md).

A referência da API está em [docs/API.md](docs/API.md) e na documentação do
código (`cargo doc -p aurora-core --no-deps --open`). A da linha de comando,
em [docs/CLI.md](docs/CLI.md). O bundle de ferramentas, em
[docs/BUNDLE.md](docs/BUNDLE.md).

Antes de acrescentar uma função nova ou mudar a forma de uma existente, leia
[docs/METODOLOGIA.md](docs/METODOLOGIA.md): a regra das camadas, o que é um
método do protocolo, os nomes, o que conta como mudança compatível e o
passo a passo de um método novo.

## Preparar a máquina

### Rust

Instale o Rust pelo [rustup](https://rustup.rs), com o clippy e o rustfmt:

```sh
rustup component add clippy rustfmt
```

Use o Rust estável atual, que é o que a release usa
(`dtolnay/rust-toolchain@stable`, em `.github/workflows/installers.yml`). O mínimo é
o 1.89 (`rust-version` no `Cargo.toml`, edição 2024); o código usa let chains
(`if let ... && let ...`, por exemplo em `diagnostics.rs`), que versões
anteriores à 1.88 recusam.

### Compilar e rodar os testes

```sh
cargo build
cargo test
```

Num clone recém-feito, `cargo test` passa sem nenhuma ferramenta instalada.
Os testes que precisam de YANC, Icarus, Verilator, Yosys, `dot` ou
surfer-aurora não acham o bundle, escrevem `PULADO: ...` e retornam sem
testar nada (`crates/aurora-core/tests/common/mod.rs` e `env_or_skip` em
`crates/aurora-cli/tests/cli.rs`). Um teste pulado aparece como `ok`; o aviso
só aparece com `cargo test -- --nocapture`. Um `cargo test` verde sem bundle
não diz nada sobre o fluxo das ferramentas.

Não defina a variável `CI` na sua máquina. Com ela definida, a falta do
bundle ou de uma ferramenta é falha, não teste pulado. O GitHub Actions
define `CI`, mas hoje não roda os testes do Rust.

### O bundle, para os testes que rodam ferramentas

Os testes acham o bundle pela variável `AURORA_TEST_BUNDLE`, que precisa ser
um caminho absoluto: o `cargo test` roda cada teste no diretório do crate,
então um caminho relativo aponta para o lugar errado. Há dois jeitos de ter
um bundle.

**Usar o de uma instalação da AURORA.** Instale pelo [README](README.md) e
aponte para a pasta `toolchain/` da instalação. Não compila nada.

```sh
export AURORA_TEST_BUNDLE="$HOME/.local/share/aurora/toolchain"            # Linux, macOS
```

```powershell
$env:AURORA_TEST_BUNDLE = "$env:LOCALAPPDATA\Programs\AURORA\toolchain"     # Windows
```

Confira com `aurora tools` que o identificador do bundle instalado é o mesmo
do campo `bundle` de `bundle/versions.json`; se não for, os testes rodam
contra outras versões. Sem o Verilator (numa instalação Custom sem ele), os
testes dele são pulados.

**Montar o bundle.** Precisa de Python 3.9 ou mais novo, `git`, do que
compila o YANC (`gcc`, `make`, `flex`, `bison`) e, no bundle completo, do
Node.js, para o RTL viewer; o surfer-aurora vem pré-compilado, e o Studio
vem pronto da release do aurora-ide (a tabela por sistema está em
[docs/BUNDLE.md](docs/BUNDLE.md), seção 6).

```sh
python3 scripts/bundle.py --out dist/toolchain               # completo
python3 scripts/bundle.py --out dist/toolchain --only yanc   # só o YANC: basta para os testes de build
export AURORA_TEST_BUNDLE="$PWD/dist/toolchain"
```

Com o bundle definido, há mais duas dependências:

- **No Linux e no macOS, os testes do Verilator precisam do compilador do
  sistema** (C++, `make` e Perl; [docs/BUNDLE.md](docs/BUNDLE.md), seção 4).
  Com o Verilator no bundle e sem o compilador, eles falham. No Windows o
  compilador vem no bundle.
- **`yanc_regression` precisa do fonte do YANC** no commit do bundle (o
  `commit` de `yanc` em `bundle/versions.json`), em `vendor/yanc` ou no
  caminho de `AURORA_TEST_YANC_SRC`. Sem ele, o teste é pulado, como sem o
  bundle (com `CI` definido, falha).

```sh
git clone https://github.com/nipscernlab/yanc vendor/yanc
git -C vendor/yanc checkout <commit de bundle/versions.json>
```

Depois, os testes contra a instalação:

```sh
cargo test --workspace -- --test-threads=2
```

### Variáveis dos testes

| Variável | Efeito |
|---|---|
| `AURORA_TEST_BUNDLE` | o bundle dos testes que rodam ferramentas; sem ela, eles são pulados |
| `AURORA_TEST_YANC_SRC` | o fonte do YANC para `yanc_regression` (padrão: `vendor/yanc`) |
| `AURORA_TEST_GUI=1` | com o bundle, roda o teste que abre a janela do surfer-aurora: `cargo test --test tools surfer` |
| `CI` | com qualquer valor, bundle ou ferramenta ausente é falha |

### Rodar a CLI a partir do código

O `aurora` procura o bundle ao lado do executável, e `target/debug/` não tem
um. Passe o bundle com `--toolchain` ou com a variável `AURORA_TOOLCHAIN`:

```sh
cargo run -p aurora-cli -- --toolchain "$AURORA_TEST_BUNDLE" tools
cargo run -p aurora-cli -- --toolchain "$AURORA_TEST_BUNDLE" -C examples/contador check
```

`new`, `add`, `remove`, `top`, `status` e `proc` funcionam sem
bundle.

## Mapa do código

O workspace tem seis crates. `aurora-core` é a biblioteca, com toda a
regra de negócio e os fluxos compostos.
`aurora-learn` são os exercícios do `aurora learn`, sobre o Core.
`aurora-installer` é a biblioteca de instalar, atualizar e desinstalar, que a
CLI e o `aurora serve` chamam, mais o instalador em terminal (Linux, macOS) e o
empacotador `aurora-pack`. `aurora-protocol` é o contrato do `aurora serve` e do
`aurora mcp`, sobre o Core, o `aurora-learn` e o instalador, de onde vêm os
tipos dos resultados. `aurora-agent` é o assistente de IA, uma sessão do
Claude Code com o `aurora mcp`, experimental. `aurora-cli` é o binário `aurora`,
uma casca fina sobre os outros cinco. A regra de quem pode depender de
quem está em [docs/METODOLOGIA.md](docs/METODOLOGIA.md), seção 1.

O desenho das partes, com as ferramentas e os arquivos que cada uma usa,
está no [README](README.md#como-as-partes-se-ligam).

### `crates/aurora-core/src/`

| Arquivo | O que mora lá |
|---|---|
| `lib.rs` | a lista de módulos, os `pub use` que formam a API pública, a documentação do crate (fluxo típico, garantias) e os atributos que valem para o crate todo |
| `toolchain.rs` | o bundle: `Toolchain` (`open`, `locate`, `tool`, `verify`, `sapho_library`), `Tool` e o caminho fixo de cada executável (`Tool::location`), `Platform`, o manifesto (`BundleManifest`, `BundleComponent`), `SystemCompiler` e o ambiente de cada ferramenta (`Toolchain::invocation`) |
| `process.rs` | o único módulo que cria processos: `Invocation`, `run` (lê a saída em blocos de linhas inteiras, confere cancelamento e prazo, encerra o grupo de processos no Unix e o Job Object no Windows), `Watch`, `spawn`, `RunningProcess`, `Termination`; o ambiente vazio (`INHERITED_ENV`, `GUI_ENV`); `ProcessJob` e `hide_console`, que o `aurora-agent` também usa; `open_with_system`, o abridor do sistema para o HTML do RTL viewer |
| `rtl_viewer.rs` | módulo público `aurora_core::rtl_viewer`: o modelo do HTML do RTL viewer (`template`, o componente `rtl-viewer` ou `AURORA_RTL_VIEWER`) e o HTML preenchido com o netlist (`write`), que o `aurora synth --open` abre |
| `control.rs` | `Control`, `CancelToken`, `Event` e `Stream`: o que a interface passa a cada operação para cancelá-la e acompanhar a saída ([Cancelamento e saída ao vivo](#cancelamento-e-saída-ao-vivo)) |
| `pipeline.rs` | o que todo resultado tem: `Step`, `Status`, `StepReport`, `Artifact`, `ArtifactKind`; por dentro, `PlannedStep`, `Runner` (roda os passos em sequência, emite os eventos do `Control` e para no primeiro que falha ou no cancelamento) e `ArtifactTracker` (o `fresh` dos artefatos) |
| `diagnostics.rs` | `Diagnostic` e `Severity`; `parse` traduz a saída de cada ferramenta em diagnósticos; `is_message` diz, linha a linha, se uma linha é mensagem da ferramenta (o campo `diagnostic` do `Event::Output`) |
| `error.rs` | `AuroraError`, com a mensagem de cada variante e o `code()` estável |
| `project.rs` | `Project` (`open`, `create`, `add_processor`, `configure_processor`, `buildable_processors`, `save`), `Processor`, `NewProcessor` (o fonte-modelo de um processador), `Language`, validação de nomes (`validate_project_name`, pública) |
| `spf.rs` | leitura tolerante e gravação atômica do `.spf` da AURORA, e as entradas de processador |
| `files.rs` | os arquivos Verilog do projeto no `.spf`: `add_verilog`, `remove_verilog`, `set_top`, `top_module`, `testbench_module`, `unregistered_verilog`, `add_file`, `set_top_level`, `set_testbench`; mover e renomear com o `.spf` em dia (`move_path`, `MovedPath`); os arquivos de entrada e saída da simulação (`read_data_file`, `Processor::write_input`, `read_output_values`) |
| `verilog.rs` | módulo público `aurora_core::verilog`: `classify`, `modules_in`, `read_interfaces`, `module_template`, `testbench_template` |
| `build.rs` | `build`, `build_processors` e `build_steps` (YANC 7.1): `BuildOptions`, `BuildResult`, `OnFailure`; os comandos de cada passo em `Plan::steps`, com a pasta de saída do processador |
| `yanc.rs` | a versão mínima do YANC (7.1) e a recusa de um bundle mais velho (`YancTooOld`) |
| `layout.rs` | a conversão de um projeto da estrutura do YANC 6.x (`Software/`, `Hardware/`, `Simulation/`) para a do 7.1 (`Project::convert_layout`), a mesma da AURORA |
| `source.rs` | lê as diretivas do cabeçalho do fonte (`#NUBITS`, `#pragma yanc nubits`), sem os comentários |
| `simulate.rs` | `simulate` (um processador), `simulate_project` (o testbench do projeto), `waveform_path`, `missing_inputs`; os comandos do Icarus e do Verilator em `execute`; a injeção de `$dumpvars` |
| `cocotb.rs` | módulo público `aurora_core::cocotb`: os testbenches em Python (`is_testbench`, a diretiva `# aurora-toplevel:` em `toplevel_directive`), o modelo do testbench cocotb (`testbench_template`) e o resultado dos testes (`TestReport`, do `results.xml`) |
| `synth.rs` | `check` (Icarus e, com `lint`, Verilator), `synthesize` (Yosys), `render_schematic` (`show` do Yosys e `dot`); `CheckOptions`, `DesignTarget`; os arquivos do design (`project_sources`, `processor_verilog`), que `hierarchy.rs` também usa |
| `hierarchy.rs` | `hierarchy`: a árvore de instâncias do design e de cada testbench, elaborada pelo Icarus e lida do `.vvp` (`parse_vvp`); `HierarchyOptions`, `HierarchyResult`, `Elaboration`, `ModuleInstance` |
| `stats.rs` | `SynthesisStatistics`: lê o `stat -json` do Yosys que a síntese grava (`SynthesisMetric`, `CellUsage`) |
| `history.rs` | módulo público `aurora_core::history`: o relatório de cada operação e o histórico em `.aurora8/reports/` (`record`, `list`, `load`, `latest`, `report_text`), a comparação (`compare`, `compare_reports`, `previous_comparable`) e o texto do `report.txt` (`render`) |
| `wave.rs` | `open_waveform` (surfer-aurora) e `ViewerOptions` |
| `wave_signals.rs` | a escolha dos sinais que a onda do testbench grava: a árvore elaborada pelo Icarus (`wave_signals`), a escolha em `wave/<testbench>.json` (`read_selection`, `write_selection`) e o layout salvo (`layout_file`) |
| `fpga/` | módulo público `aurora_core::fpga` ([docs/FPGA.md](docs/FPGA.md)): as placas (`board.rs`, com os JSONs de `crates/aurora-core/boards/` embutidos), o `fpga.json` e a resolução das ligações bit a bit (`config.rs`), o topo da placa gerado (`wrapper.rs`), o Quartus do sistema e a invocação dos programas dele (`quartus.rs`), o `.qsf` e o `.sdc` (`qsf.rs`), a leitura dos resumos de recursos e de tempo (`summary.rs`), a compilação para a placa (`build.rs`), o registro da compilação (`manifest.rs`, o `aurora-build.json`), a conferência dos pinos depois do Fitter (`pins.rs`), a gravação na placa pelo Quartus Programmer (`program.rs`) e `prepare`, que junta tudo para um projeto |
| `wave_layout.rs` | `wave_layout` e `prepare_wave_layout`: o layout do Surfer dos processadores SAPHO (o `.surf.ron` e os tradutores do assembly, da linha do C± e dos complexos), lido do cabeçalho da onda (VCD ou FST) e das tabelas do YANC |
| `paths.rs` | `YANC_PATH_LIMIT`, `canonicalize` sem o prefixo `\\?\` do Windows, normalização de caminhos |
| `flows/` | módulo público `aurora_core::flows`: os fluxos compostos que a CLI e o `aurora serve` chamam (o Studio, pelo `aurora serve`) (`run`, `FlowRequest`, `FlowOutcome` com `status`), a onda (`open_wave`) e a escolha de sinais (`choose_signals`) |
| `status.rs` | `Project::status` e `ProjectStatus`: o retrato do projeto, que o `aurora status`, o Studio e o `project.status` mostram |
| `delivery.rs` | módulo público `aurora_core::delivery`: a entrega do progresso em lotes, com teto de linhas (`Dispatcher`, `Notification`) |
| `locate.rs` | módulo público `aurora_core::locate`: a busca do bundle para quem não é a CLI instalada ao lado dele |
| `experimental.rs` | módulo público `aurora_core::experimental`: os recursos desligados por padrão (`Feature::Fpga`, `Feature::Assistant`), o `experimental.json` e `require` |

### `crates/aurora-protocol/src/`

| Arquivo | O que mora lá |
|---|---|
| `lib.rs` | o contrato do `aurora serve` e do `aurora mcp`: `Request`, `Response`, `PROTOCOL_VERSION`, a macro `methods!` (o nome, o tipo do resultado e as marcações de cada método, de onde saem `METHODS`, `METHOD_INFO` e `result_schema`) e o schema em duas vistas; é onde nasce um método novo ([docs/METODOLOGIA.md](docs/METODOLOGIA.md)). Os tipos dos resultados vêm do Core, do `aurora-learn` e do instalador, e nenhum deles conhece o protocolo |

### `crates/aurora-cli/src/`

| Arquivo | O que mora lá |
|---|---|
| `main.rs` | a definição da linha de comando com o `clap` (`Cli`, `Command`, `ProcCommand`, as structs `*Args`), o texto de ajuda, `main` com os códigos de saída (`EXIT_FAILED`, `EXIT_ERROR`, `EXIT_CANCELLED`), `cancel_on_signals` (Ctrl+C e SIGTERM viram cancelamento), `init_tracing` e o teste `cli_definition_is_valid` |
| `commands.rs` | uma função por comando, chamada por `run`; resolve o que a linha de comando diz (os caminhos digitados em absolutos, o processador da pasta atual) e chama o Core com o `Control`; `build`, `check`, `hierarchy`, `sim` e `synth` montam um `FlowRequest` e o entregam a `run_flow`, que chama `aurora_core::flows::run_with` (o fluxo que confere o pedido antes do bundle, compila os processadores antes de `sim` e `synth` e grava o relatório no histórico) com o `Printer`, que escreve o que o fluxo avisa; `wave` chama `flows::open_wave_with`, e `report` atende `aurora report` |
| `output.rs` | `Output`: a saída em texto (cores só em terminal, `NO_COLOR`), em JSON (`Output::json`) e em eventos (`--events`); `Output::control` monta o `Control` de cada modo, e `live_text` mostra o testbench enquanto roda; a formatação dos diagnósticos, das tabelas de `aurora report list` e `compare`, e `hint`, a dica de comando para alguns erros |
| `report.rs` | um tipo por saída do `--json` (`SimReport`, `StatusReport`, ...), o trait `Report` que `Output::json` exige, a macro `reports!` que registra cada um, e o teste que confere `docs/schema/` ([Contrato do `--json`](#contrato-do---json)) |
| `installation.rs` | a pasta da instalação de onde este `aurora` roda (`prefix`, por `aurora_installer::installation::prefix_of`) e `lib_error`, que traz um erro da biblioteca do instalador para a CLI (o cancelamento vira a saída 130); o recibo e o desinstalador são de `aurora_installer::installation` |
| `install.rs` | `aurora install`: a lista (a TUI de `aurora-installer`) ou os nomes, e de onde vêm os pedaços dos aplicativos (`--from`, ou a release, por `ReleasePayload`); quem extrai é `aurora_installer::add` |
| `update.rs` | `aurora update`: a casca sobre `aurora_installer::update`, com as tabelas, as confirmações e a barra; as versões, as releases (`aurora_installer::release`, atrás do trait `Sources`, para testar sem rede) e a troca dos arquivos são da biblioteca |
| `uninstall.rs` | `aurora uninstall`: acha a instalação ao redor do executável e roda o desinstalador que o instalador deixou (`uninstall.sh`, ou o do Inno no Windows) |
| `settings.rs` | as opções globais do bundle e do que vem do sistema (`ToolchainArgs`: `--toolchain` ou `AURORA_TOOLCHAIN`, `--compiler` ou `AURORA_COMPILER`, `--quartus` ou `AURORA_QUARTUS`); `ToolchainArgs::lookup` monta o `Lookup` (com a `QUARTUS_ROOTDIR` e a `HOME`) e `ToolchainArgs::resolve` abre o bundle por `aurora_core::locate::resolve`, onde moram o `compiler_in` e a busca do Quartus; e `absolute` |
| `fpga.rs` | `aurora fpga boards`, `aurora fpga check`, `aurora fpga build` e `aurora fpga program` (com `--list`, os cabos) |
| `serve.rs` | `aurora serve`: o protocolo por JSON-RPC 2.0 sobre stdio; `Server::handle_with` é o despacho de cada método para o Core, que o `aurora mcp` também usa; cada braço chama as mesmas funções do Core que o comando da CLI |
| `mcp/` | `aurora mcp`: as ferramentas para um agente de IA (`TOOLS`, em `mod.rs`, com o nome e a ordem; quais métodos viram ferramenta é a marcação `agent` deles no protocolo), o resumo em texto de cada resultado (`summary.rs`) e o `aurora mcp setup` (`setup.rs`) |
| `chat.rs` | `aurora chat`: a tela do assistente no terminal, sobre o `aurora-agent` |
| `experimental.rs` | `aurora experimental`: lista, liga e desliga os recursos experimentais |
| `learn.rs` | `aurora learn`: os subcomandos e a saída deles, sobre o `aurora-learn` |
| `learn_tui.rs` | o modo watch do `aurora learn`: a tela de terminal no estilo do rustlings |
| `wave_choice.rs` | `aurora wave signals`, `select` e `unselect`: a escolha dos sinais da onda |
| `update_check.rs` | a verificação automática de atualização: uma consulta por dia e o aviso de AURORA mais nova no fim do comando |
| `progress.rs` | a barra que mostra que uma ferramenta está rodando, no stderr de um terminal |
| `meter.rs` | a linha de progresso dos downloads do `aurora install` e do `aurora update` |
| `cancel.rs` | o Ctrl+C nas perguntas dos comandos que mexem na instalação: o pedido de `main` (`set`) e o `Cancelled`, que vira a saída 130 |

### `crates/aurora-learn/src/`

Os exercícios do `aurora learn` ([docs/LEARN.md](docs/LEARN.md)), sobre a API
pública do Core: o crate não roda ferramenta por conta própria: chama o
`check` e o `simulate_project` para corrigir e o `flows::open_wave` para abrir
a onda.

| Arquivo | O que mora lá |
|---|---|
| `lib.rs` | os módulos, os `pub use` e a documentação do crate |
| `track.rs` | a trilha: `Track`, `Chapter`, `Exercise`, o `exercise.json` (`Spec`, `Kind`, `Stimulus`, `Reset`), a pasta das trilhas (`tracks_dir`, `AURORA_LEARN_DIR`) |
| `text.rs` | o título, o corpo e as dicas de um `.md`, e a escolha do idioma (`prompt.en.md`) |
| `testbench.rs` | o testbench gerado: o estímulo, a comparação com a referência e o resumo `AURORA-LEARN` |
| `reference.rs` | a cópia da solução com `_ref` no nome de cada módulo |
| `workspace.rs` | a pasta de exercícios: `Workspace` (`init`, `open`, `discover`, `record`, `reset`), o estado `.aurora-learn.json` e as soluções liberadas |
| `grade.rs` | a correção: `grade`, `Grade`, `Verdict`, `Finding`; os diagnósticos que o aluno vê |
| `layout.rs` | o `.sucl` que abre a onda arrumada (`commands`, `layout_of`) |
| `dev.rs` | o `aurora learn dev check` (`check_track`) |
| `flows.rs` | os fluxos que a CLI e o `aurora serve` compartilham: corrigir (`check`, `LearnCheckOutcome`) e abrir a onda da última correção com o layout do exercício (`open_wave`) |
| `error.rs` | `LearnError`, com o `code()` que as interfaces traduzem |

### `crates/aurora-installer/src/`

| Arquivo | O que mora lá |
|---|---|
| `lib.rs` | os módulos, a forma do instalador de Linux e macOS (`install` e `payload/`) e a documentação do crate |
| `install.rs` | instalar o payload num prefixo (`Target`), o recibo `install.json` (`Receipt`) e o `uninstall.sh` |
| `installation.rs` | a instalação de onde um `aurora` roda (`prefix_of`, `receipt`, `uninstaller_exe`): a base de `aurora install`, `aurora update`, `aurora uninstall` e do `aurora serve` |
| `add.rs` | `aurora install`: os aplicativos novos numa instalação (`prepare`, `add_components`), com os pedaços do disco ou da release (`Payload`) |
| `update/` | `aurora update`: o plano e a troca só dos arquivos que mudaram (`mod.rs`: `plan`, `apply`) e a orquestração que a CLI e o `aurora serve` compartilham (`flow.rs`: `check`, `run`, `Hooks`) |
| `release.rs` | as releases na rede, pelo `curl` do sistema: `Network`, `Remote`, o trait `Sources` e `is_newer` |
| `uninstall.rs` | `aurora uninstall`: chama o desinstalador que o instalador deixou (`uninstall.sh`, ou o `unins000.exe` do Inno) |
| `files.rs` | o manifesto de arquivos (`files.json`): o SHA-256 de cada arquivo, por pedaço |
| `payload.rs` | o índice do payload (`index.json`, `Index`) e o nome de cada arquivo na release (`release_asset`) |
| `plan.rs` | os perfis de instalação e a seleção de componentes |
| `pack.rs` | o `aurora-pack`: os pedaços da TUI (`tui`), o estágio do Inno Setup (`inno`) e os arquivos da release (`release_assets`) |
| `tui.rs` | a instalação guiada no terminal (`App`), também a lista do `aurora install` (`App::for_adding`) |
| `desktop.rs` | o atalho da AURORA IDE no menu de aplicativos, no Linux e no macOS |
| `brand.rs` | as cores e a marca da AURORA no terminal |
| `error.rs` | `Error`: os erros que atravessam a fronteira, com código próprio |
| `main.rs`, `bin/aurora-pack.rs` | os executáveis `aurora-installer` (o `install` do instalador de Linux e macOS) e `aurora-pack` |

### `crates/aurora-agent/src/`

| Arquivo | O que mora lá |
|---|---|
| `lib.rs` | os módulos, `AgentError` e a documentação do crate |
| `locate.rs` | onde está o Claude Code (`find_claude`, `version`): o caminho das preferências, o `PATH` e as pastas dos instaladores |
| `session.rs` | a sessão do Claude Code em `--print` com `stream-json` (`Session`, `SessionOptions`, `Permission`), traduzida em `AgentEvent` |

### O resto do repositório

| Caminho | O que é |
|---|---|
| `crates/aurora-core/tests/` | `api_contract.rs` (garantias da API), `build.rs` (YANC, com snapshots em `snapshots/`, e a conversão da estrutura antiga), `control.rs` (cancelamento, prazo e eventos com o Icarus), `tools.rs` (simulação, síntese, esquemático, Surfer), `verilog_flow.rs` (o fluxo Verilog, mover arquivos e a hierarquia; a primeira metade roda sem ferramenta), `yanc_regression.rs` (os casos de teste do YANC), `flows.rs` (os fluxos compostos, com o bundle), `history.rs` (gravar, listar, ler e comparar relatórios), `wave_signals.rs` (a escolha dos sinais da onda e o layout salvo), `common/mod.rs` (como achar o bundle) |
| `crates/aurora-cli/tests/cli.rs` | a CLI de ponta a ponta: códigos de saída, texto, JSON, eventos e Ctrl+C |
| `crates/aurora-cli/tests/schema.rs` | a saída `--json` de cada comando, validada contra `docs/schema/` |
| `crates/aurora-cli/tests/serve.rs` | o `aurora serve` de ponta a ponta: um cliente JSON-RPC de verdade falando com o binário por stdin e stdout |
| `crates/aurora-cli/tests/learn.rs` | o `aurora learn` de ponta a ponta, com a trilha de teste e o `--json` validado |
| `crates/aurora-learn/tests/` | `grading.rs` (a pasta de exercícios e a correção, com a trilha de `fixtures/teste`), `track.rs` (as trilhas de `aurora-learn/` passam no `dev check`) |
| `aurora-learn/` | as trilhas de exercícios, que viram o componente `aurora-learn` ([docs/LEARN.md](docs/LEARN.md)) |
| `docs/schema/` | o JSON Schema de cada comando, gerado dos tipos; não edite à mão |
| `docs/PROTOCOL.md` | a referência do protocolo do `aurora serve`, gerada dos tipos do `aurora-protocol` por `crates/aurora-cli/src/protocol_doc.rs`; não edite à mão |
| `packages/rtl-viewer/` | o RTL viewer, o esquemático interativo do netlist da síntese, em TypeScript e React: o Studio compila o fonte dele (pelo submódulo do aurora-ide), e o build dele (`npm run build`) gera o bundle da extensão do VS Code e o modelo do HTML do `aurora synth --open`, que vira o componente `rtl-viewer` |
| `editors/vscode/` | a extensão do VS Code, em TypeScript, que fala com a AURORA pelo `aurora serve` ou, com uma AURORA sem ele (ou `aurora.transport` em `cli`), pela CLI com `--json`; tem versão própria e testes próprios ([README](editors/vscode/README.md)), e o teste de contrato dela quebra quando `docs/schema/` muda |
| `crates/aurora-installer/tests/` | `install.rs` (empacota um bundle pequeno e instala de verdade) e `update.rs` (duas versões de um bundle pequeno: instala a primeira e atualiza para a segunda trocando só o que mudou) |
| `assets/brand/` | a marca da AURORA (o camaleão, o nome e o ícone), que o README, a arte do instalador do Windows e o `brand.rs` do instalador usam; a AURORA IDE, no repositório [aurora-ide](https://github.com/nipscernlab-sandbox/aurora-ide), tem a mesma em `public/brand/` |
| `examples/` | os projetos `soma`, `com_erro` e `contador`, usados pelos testes |
| `bundle/versions.json` | a versão e o hash de cada pacote do bundle |
| `bundle/components.json` | o que cada componente leva de cada pacote |
| `scripts/bundle.py`, `scripts/binaries.py`, `scripts/installer_art.py` | a montagem do bundle, a leitura de dependências de binários e as imagens do assistente de Windows (`installer/windows/art/`) |
| `installer/windows/aurora.iss` | o instalador de Windows (Inno Setup) |
| `install.sh`, `install.ps1` | a instalação em um comando; servidos da `main`, valem assim que chegam nela |
| `.github/workflows/` | `release.yml` (publica a release numa tag) e o `installers.yml` que ele chama (bundle e instaladores, sem testes); `surfer-aurora.yml` (manual: o surfer-aurora pré-compilado, numa pré-release); `vscode.yml` (a extensão do VS Code e o RTL viewer, a cada push que muda `editors/vscode/`, `packages/rtl-viewer/` ou `docs/schema/`); fora esse, não há CI a cada push |
| `docs/` | API, CLI, bundle, instalação, release, metodologia, placas FPGA, `aurora learn`, as pendências de fora da AURORA e o desenho das partes (`img/`) |

### Como uma operação anda

O caminho de `aurora check`, que é o de toda operação que roda ferramentas:

1. `main.rs`: o `clap` lê os argumentos em `Cli`, e `commands::run` chama
   `check`.
2. `commands.rs`, função `check`: abre o `Project`, acha o processador da
   pasta atual, monta o `FlowRequest::Check` e chama `run_flow`, que entrega
   o pedido a `aurora_core::flows::run_with` com o jeito de achar o
   `Toolchain` (`settings.rs`), o `Control` que `main` criou
   (`Output::control`) e o `Printer`, que recebe o `Progress` do fluxo. O
   `run_with` confere o pedido e só então pede o bundle.
3. `flows/run.rs`, função `check`: monta `CheckOptions`, chama
   `aurora_core::check` sobre o que está no disco, sem compilar os
   processadores (o `sim` e o `synth` compilam antes), e grava o relatório
   no histórico.
4. `synth.rs`, função `check`: monta cada comando com
   `Toolchain::invocation` e o entrega a um `Runner`.
5. `pipeline.rs`, `Runner::run`: confere o cancelamento, avisa
   `StepStarted`, executa por `process::run` (ambiente vazio, stdin fechado,
   grupo de processos próprio), que avisa cada linha (`Event::Output`) e
   encerra o processo se o cancelamento chegar; avisa `StepFinished`,
   interpreta stdout e stderr com `diagnostics::parse`, guarda o
   `StepReport` e para no primeiro passo que falha.
6. De volta à CLI, o `Printer` escreve o texto com `Output::check` quando o
   fluxo avisa `Progress::Checked`, e `Output::json` escreve o
   `CheckReport`; `main` transforma `Ok(true)`, `Ok(false)` e `Err` em 0, 1
   e 2, ou em 130 se o Ctrl+C chegou.

## Onde mudar o quê

Cada receita diz os arquivos, as funções e o teste a rodar. Os nomes de
teste são os de hoje; procure-os com `grep -rn "fn <nome>" crates`.

### Acrescentar um método à API

Uma capacidade nova da AURORA, que um cliente vai pedir pelo `aurora serve`,
pelo `aurora mcp` ou pelo Studio, é um método do protocolo. O passo a passo,
com os arquivos e a lista de "pronto quando", está em
[docs/METODOLOGIA.md](docs/METODOLOGIA.md), seção 6. Mudar a forma de um
método existente segue a tabela de compatibilidade da seção 5 de lá.

### Acrescentar uma flag a uma ferramenta

Exemplo: uma opção nova do `iverilog` no `aurora check`.

1. Em `crates/aurora-core/src/synth.rs`, função `check`, a closure
   `iverilog` monta o comando base de todos os passos de verificação:
   `toolchain.invocation(Tool::Iverilog, project.root())`, depois `-g2012`
   se houver `.sv`, depois `.arg("-tnull").arg("-Wall")`. Acrescente a flag
   ali com `.arg("...")`. Caminho vai com `.path_arg(...)`, que o põe no
   formato do sistema.
2. As flags de outras etapas ficam em `Plan::steps` (`build.rs`, YANC), em
   `execute` e `verilator_cflags` (`simulate.rs`, Icarus e Verilator) e em
   `synthesize` e `render_schematic` (`synth.rs`, Yosys e `dot`).
3. Teste: `verilog_flow_from_scratch`, em
   `crates/aurora-core/tests/verilog_flow.rs`, confere as flags de cada passo
   de `check` com `has_arg` (`for flag in ["-tnull", "-Wall"]`). Acrescente
   a nova. Ele precisa do Icarus:

   ```sh
   cargo test -p aurora-core --test verilog_flow
   ```

4. Documente em [docs/API.md](docs/API.md), seção 5.4 (a tabela de
   comandos do `check`), e em [docs/CLI.md](docs/CLI.md), "O que `check`
   verifica".

Uma variável de ambiente nova para a ferramenta entra por
`Invocation::env` (veja `Toolchain::invocation`, em `toolchain.rs`), nunca
herdada do ambiente do usuário.

### Acrescentar um comando ou uma opção à CLI

1. `crates/aurora-cli/src/main.rs`: uma opção nova é um campo na struct de
   argumentos do comando (por exemplo `CheckArgs`), com `#[arg(long)]` e um
   comentário `///`, que vira o texto da ajuda e por isso é em inglês. Um comando novo
   é uma variante de `enum Command`.
2. `crates/aurora-cli/src/commands.rs`: o braço do `match` em `run` e a
   função do comando, que passa o valor ao Core (no `check`, um campo de
   `FlowRequest::Check`, de `aurora_core::flows`, que o fluxo passa a
   `CheckOptions`; o `FlowRequest` vai no `flow.run` do protocolo, e o
   campo novo muda o `docs/schema/`). Uma operação de mais de um passo vira
   uma função do Core, que o comando e o braço do despacho chamam: a regra
   fica num lugar só. A função devolve `Ok(false)` quando a operação rodou e
   falhou (código 1) e `Err` quando não conseguiu rodar (código 2).
3. `crates/aurora-cli/src/output.rs`: o texto, num método de `Output`. O
   JSON é um tipo de `crates/aurora-cli/src/report.rs`, escrito por
   `out.json(&relatorio)`: um comando novo ganha um tipo novo, registrado na
   macro `reports!` com o nome do arquivo de schema; uma opção que muda o
   JSON muda o tipo. Gere o schema de novo (veja
   [Contrato do `--json`](#contrato-do---json)). Com `--json`, o comando
   escreve um único objeto JSON no stdout e nada mais.
4. Se a opção pede uma regra nova (decidir, validar, escolher arquivo), a
   regra vai para o Core, e a CLI só repassa.
5. Testes:
   - `cli_definition_is_valid`, em `main.rs`, confere a definição do
     `clap` (nomes repetidos, conflitos): `cargo test -p aurora-cli --bins`;
   - um teste em `crates/aurora-cli/tests/cli.rs`, com os auxiliares de lá:
     `new_project`, `aurora_in(&root)` para rodar dentro do projeto,
     `json(&mut cmd, código)` e `stdout(&mut cmd, código)`. Se precisar de
     ferramenta, comece com `env_or_skip("AURORA_TEST_BUNDLE")` e passe o
     bundle por `AURORA_TOOLCHAIN`, como `build_ok_is_exit_code_0` faz.

   ```sh
   cargo test -p aurora-cli
   ```

6. Documente em [docs/CLI.md](docs/CLI.md): a tabela do comando e, se o JSON
   mudou, a linha do comando na seção JSON.

### Interpretar uma mensagem nova de uma ferramenta

Quando uma ferramenta escreve algo que a AURORA mostra como `unknown`, ou com
a gravidade errada:

1. `crates/aurora-core/src/diagnostics.rs`: `parse_line` escolhe o
   interpretador pela ferramenta: `parse_yanc_phrase` (`cmmcomp`,
   `asmcomp`), `parse_c_style` e `parse_tool_prefixed` (`cppcomp`),
   `parse_c_style` e `parse_icarus` (`iverilog`, `vvp`), `parse_verilator`,
   `parse_yosys`, `parse_graphviz`, `parse_quartus`. Acrescente o caso na função da
   ferramenta.
2. Mantenha as regras do módulo: o texto original fica em `raw`; linha do
   stderr que não casa vira `Severity::Unknown` e nunca é descartada; linha
   do stdout só vira diagnóstico nas ferramentas de `stdout_is_messages`.
3. Teste: um teste novo no `mod tests` do próprio `diagnostics.rs`, com a
   linha copiada da saída real da ferramenta (o `stderr` do passo, em
   `--json`). O auxiliar `one(Tool::..., "linha")` interpreta uma linha de
   stderr; `parse(Tool::..., stdout, stderr, None)` interpreta várias. Não
   precisa de bundle:

   ```sh
   cargo test -p aurora-core --lib diagnostics
   ```

4. Documente o formato novo na tabela de [docs/API.md](docs/API.md),
   seção 7.

### Mudar o modelo de módulo ou de testbench

Os arquivos Verilog que `aurora add` cria saem de `crates/aurora-core/src/verilog.rs`
(o testbench cocotb, um `.py`, sai de `testbench_template`, em `cocotb.rs`):
`module_template(nome)` e `testbench_template(testbench, dut)`, com os
auxiliares `is_clock`, `reset_polarity` e `range`. Quem os chama é
`Project::add_verilog`, em `files.rs`; o módulo testado vem de
`module_under_test`.

1. O modelo precisa continuar classificado como o que é:
   o módulo-modelo precisa ter portas (um módulo sem portas soma 3 pontos de
   testbench), e o testbench-modelo, `$dumpfile` e `$finish`.
2. Testes sem ferramenta: `templates_are_classified_as_intended` e
   `testbench_template_instantiates_every_port` em `verilog.rs`;
   `module_template_is_a_synthesizable_module`,
   `testbench_template_without_dut` e `testbench_template_drives_every_port`
   em `crates/aurora-core/tests/verilog_flow.rs`. Com Icarus e Yosys:
   `testbench_from_yosys_ports_elaborates` e `verilog_flow_from_scratch`,
   que conferem que o modelo elabora e simula.

   ```sh
   cargo test -p aurora-core --lib verilog
   cargo test -p aurora-core --test verilog_flow
   ```

3. Documente em [docs/CLI.md](docs/CLI.md), "Arquivo novo", e em
   [docs/API.md](docs/API.md), seção 4.2.

Mudar os pesos de `classify` não é esta receita: eles são os da AURORA, e
mudá-los muda a classificação dos arquivos de todos os projetos.

### Atualizar a versão de uma ferramenta do bundle

1. Em `bundle/versions.json`, dentro de `packages`, troque a versão e o
   SHA-256 (OSS CAD Suite e Graphviz, por plataforma) ou o commit (YANC e
   surfer-aurora). O SHA-256 do OSS CAD Suite está no campo `digest` da API
   de releases do GitHub; o do Graphviz, no `.sha256` ao lado do zip. No
   surfer-aurora, troque também o `prebuilt`, com as URLs e os SHA-256 do
   resumo do `gh workflow run surfer-aurora.yml`, e o `web`: o `bundle.py`
   usa o `prebuilt` que estiver lá, sem conferir o commit. No Windows, o
   Icarus e o Verilator vêm do pacote `msys`, uma release `ucrt64-vN` do
   aurora-toolchain ([docs/BUNDLE.md](docs/BUNDLE.md), seção 6).
2. Troque `bundle`, o identificador do bundle.
3. YANC: ponha o seu `vendor/yanc` no mesmo commit (ou aponte
   `AURORA_TEST_YANC_SRC` para um fonte nele), de onde o `yanc_regression` lê
   os casos de teste do próprio YANC.
4. Monte e confira:

   ```sh
   python3 scripts/bundle.py --out dist/toolchain
   cargo run -p aurora-cli -- --toolchain "$PWD/dist/toolchain" tools --verify
   AURORA_TEST_BUNDLE="$PWD/dist/toolchain" cargo test --workspace -- --test-threads=2
   ```

   Se a saída do YANC mudou, os snapshots de `crates/aurora-core/tests/build.rs`
   falham: o `insta` grava um `.snap.new`. Compare com o `.snap` em
   `crates/aurora-core/tests/snapshots/` e, se a diferença for a esperada,
   troque o `.snap` pelo novo.
5. Se o pacote mudou de estrutura, o `bundle.py` falha dizendo o caminho ou
   a biblioteca que falta. Ajuste `bundle/components.json` (`select`,
   `data`, `run`) e, se o executável mudou de lugar, `Tool::location` em
   `crates/aurora-core/src/toolchain.rs`.
6. Documente em [docs/BUNDLE.md](docs/BUNDLE.md), seção 1 (as tabelas de
   versões) e no `CHANGELOG.md`. O identificador antigo pode aparecer em
   mais lugares (hoje, no exemplo do `files.json` em
   `crates/aurora-installer/src/files.rs`):
   `grep -rn "<identificador antigo>" --exclude-dir=target --exclude-dir=vendor .`

O `installers.yml` só roda numa tag, chamado pelo `release.yml`, e não roda
teste: no macOS e no Windows, a versão nova se confirma montando o bundle e
rodando os testes nesses sistemas, à mão.

### Acrescentar um erro novo

Antes, decida se é mesmo um erro: `Err(AuroraError)` é para quando a AURORA
não consegue rodar. Se a ferramenta rodou e recusou o código do usuário, o
lugar é um diagnóstico no resultado.

1. `crates/aurora-core/src/error.rs`: a variante em `AuroraError`, com
   documentação na variante e em cada campo (o crate não compila sem ela),
   e a mensagem em `#[error("...")]`, em inglês, começando com maiúscula e
   sem nome de comando da CLI.
2. No mesmo arquivo, o braço em `AuroraError::code()`, com um código novo em
   `snake_case`, e o braço em `AuroraError::data()`, com os campos que vão em
   `ErrorInfo::data` (ou `None`). Um código existente nunca muda.
3. Se um comando resolve o erro, a dica vai na função `hint` de
   `crates/aurora-cli/src/output.rs`; no JSON ela sai em `error.hint`.
4. Testes: o código e a mensagem neutra, como em
   `new_errors_have_stable_codes_and_neutral_messages`
   (`crates/aurora-core/tests/verilog_flow.rs`); a dica, como em
   `error_hints_name_the_command` (`crates/aurora-cli/tests/cli.rs`).
5. Documente na tabela de [docs/API.md](docs/API.md), seção 8, e, se houver
   dica, em [docs/CLI.md](docs/CLI.md), "Erros e dicas".

`AuroraError` é `#[non_exhaustive]`: acrescentar uma variante não quebra
quem usa a biblioteca.

## Regras do código

Algumas regras o compilador e o clippy cobram; as outras, a revisão.

- **O Core não escreve no console.** `lib.rs` tem
  `#![deny(clippy::print_stdout, clippy::print_stderr)]`: um `println!` ou
  `eprintln!` no `aurora-core` reprova o `cargo clippy`. O log sai por
  `tracing` (`tracing::info!`, `tracing::debug!`). O Core também não chama
  `exit` e não entra em pânico em erro esperado: devolve `AuroraError`.
- **Sem `unsafe`.** `aurora-core`, `aurora-learn`, `aurora-installer`, `aurora-protocol` e `aurora-agent` têm
  `#![forbid(unsafe_code)]`. A CLI não tem o atributo, e também não usa
  `unsafe`.
- **Todo item público do Core é documentado.** `#![deny(missing_docs)]` em
  `lib.rs`. Rode o `cargo doc` com `-D warnings` antes de mandar: ele também
  reprova link quebrado na documentação. Os exemplos da documentação rodam como
  teste; marque com `no_run` o que precisa do bundle, como os de `lib.rs`.
- **Só ferramentas do bundle.** No Core, programa externo roda por
  `Toolchain::invocation` e `process::run`. Nada de
  `std::process::Command` fora de `process.rs`, nada do `PATH`, nenhuma
  variável herdada sem estar declarada. A única exceção no Core é
  `process::open_with_system`, que abre o HTML do RTL viewer no navegador
  pelo abridor do sistema: o navegador do usuário não vem no bundle. Fora
  do Core, o que não vem no bundle roda por `std::process::Command`: o
  `curl` do sistema no `aurora-installer` (`release.rs`), o Claude Code, que
  o `aurora-agent` e o `aurora mcp setup` acham no `PATH`, e o git e o `curl`
  do `aurora-git` (`run.rs`), que o `aurora serve` acha no `PATH`.
- **Regra de negócio no Core.** A CLI abre, chama e mostra. As mensagens do
  Core não citam comandos; os comandos aparecem só em `hint`, na CLI.
- **O Core nunca sobrescreve código do usuário.** Criar processador ou
  arquivo recusa se o arquivo já existe; só artefatos gerados (a pasta de
  saída de cada processador, `.aurora8/Temp/`) são reescritos. A conversão da
  estrutura antiga (`convert_layout`) move sem apagar nem sobrescrever.
- **Tipos públicos.** Os enums públicos são `#[non_exhaustive]` (hoje faltam
  `FlowRequest`, `Progress`, `WavePolicy`, `WaveSource`, `LayoutPolicy` e
  `WaveChoice`, de `flows`, `delivery::Notification`, `locate::Origin` e
  `protocol::ShutdownReason`), e as
  structs que podem ganhar campos também (`CheckOptions` é construída com
  `CheckOptions::default()` e campo a campo). Os resultados implementam
  `Serialize` e `JsonSchema`, e esse é o JSON da CLI; campo de caminho leva
  `#[schemars(with = "String")]`.
- **Toda espera por ferramenta passa pelo `Runner`.** É ele que respeita o
  `Control`: cancelamento, prazo e eventos (as exceções de hoje chamam
  `process::run` direto: as portas pelo Yosys em `verilog::read_interfaces`,
  a sonda do cocotb e os cabos do Quartus em `fpga::cables`). Uma operação
  nova que roda
  ferramenta recebe `control: &Control` como último argumento e o entrega a
  `Runner::new`.
- **Caminhos** são `camino::Utf8Path` e `Utf8PathBuf` em toda a API.
- **Português e inglês.** Comentários, documentação e `CHANGELOG.md` em
  português. O que o `aurora` e o instalador mostram é em inglês e começa com
  maiúscula: mensagens, erros (`Error:`), avisos (`Warning:`), dicas, texto
  de ajuda (os `///` que o `clap` transforma em ajuda), as telas do
  instalador e o `message` e o `hint` do JSON. A exceção é a gravidade dos
  diagnósticos (`arquivo:linha: error: ...`), em minúscula como no gcc, que
  é o que os editores reconhecem. Identificadores em inglês:
  funções, tipos, variáveis, nomes de teste, códigos de erro e campos do
  JSON (`classify`, `TESTBENCH_THRESHOLD`, `no_top_level`, `failed_step`,
  `classification_follows_the_aurora_scores`). As mensagens das asserções
  dos testes são em português. As mensagens do YANC chegam em inglês porque
  a AURORA sempre pede `--lang=en`.
- **Três plataformas.** O código compila para Linux, macOS e Windows; o que
  muda por sistema vai em `cfg(windows)` (veja `INHERITED_ENV` em
  `process.rs` e `YANC_PATH_LIMIT` em `paths.rs`). A release compila nos
  três; os testes, rode à mão.
- **Teste que precisa de ferramenta** pede o bundle com
  `common::toolchain_with(&[Tool::...])` (ou `common::toolchain()`) e
  retorna se vier `None`. Assim ele é pulado sem o bundle; com `CI`
  definido, a falta do bundle é falha.
- **Formatação** é a padrão do `rustfmt`; o repositório não tem
  `rustfmt.toml`.

## Cancelamento e saída ao vivo

Toda operação que executa ferramentas (`build`, `build_processors`,
`check`, `hierarchy`, `simulate`, `simulate_project`, `synthesize`,
`render_schematic`, `wave_signals`, `fpga::build`, `fpga::program` e os
fluxos `flows::run` e `flows::choose_signals`)
recebe um `Control` como último argumento. `Control::default()` roda até o
fim e não avisa nada; é o que os testes usam quando não testam isso.

Uma interface usa assim (o exemplo completo está na documentação de
`control.rs`):

```rust
let cancel = CancelToken::new();
let control = Control::new()
    .with_cancel(cancel.clone())                  // o botão de parar fica com o clone
    .on_event(move |event| { let _ = tx.send(event.clone()); });
// numa thread: simulate_project(&toolchain, &project, &options, &control)
```

- **Cancelar** (`cancel.cancel()`, de qualquer thread) encerra o processo
  que roda, com tudo o que ele iniciou: no Unix o grupo de processos, no
  Windows o Job Object do passo.
  Os passos seguintes não começam. A operação volta como `Ok`, com
  `Status::Cancelled` e os passos que chegaram a rodar.
- **Prazo**: o da simulação, `SimulationOptions::timeout`, e o do `dot`,
  `SchematicOptions::timeout` (60 s por padrão, `SCHEMATIC_TIMEOUT`).
  Passou dele: `Status::TimedOut`, com o que a ferramenta escreveu até ali.
- **Eventos**: `StepStarted` (com o comando), `Output` (uma linha, com
  `stream` e `diagnostic`) e `StepFinished` (com o `Termination` e a
  duração). `diagnostic` é `true` nas linhas que a AURORA reconhece como
  mensagem da ferramenta e que voltam em `diagnostics`; `false` é saída do
  programa, como o `$display` do testbench. O receptor roda na thread da
  operação: precisa ser rápido.
- **Custo**: com receptor, o `vvp` roda com `-i` (sem buffer no stdout),
  senão o `$display` só sairia no fim. Sem receptor, fica o buffer.

Na CLI, `Output::control` monta o `Control` de cada modo, e
`cancel_on_signals` liga o Ctrl+C e o SIGTERM ao `CancelToken`.

Para testar: `cancel_stops_the_process_and_what_it_started`,
`timeout_stops_the_process_and_keeps_its_output` e
`lines_arrive_while_the_process_runs` em `process.rs` rodam sem bundle;
`crates/aurora-core/tests/control.rs` e os testes de `--timeout`, `--events`
e Ctrl+C em `crates/aurora-cli/tests/cli.rs` precisam do Icarus.

```sh
cargo test -p aurora-core --lib process
cargo test -p aurora-core --test control
```

## Contrato do `--json`

O que cada comando escreve com `--json` tem um JSON Schema em
`docs/schema/<comando>.json` (`events.json` para as linhas do `--events`,
`error.json` para o código de saída 2). Os arquivos são gerados dos tipos;
não edite à mão.

- A CLI só escreve JSON por `Output::json`, que só aceita tipos de
  `crates/aurora-cli/src/report.rs` registrados na macro `reports!`.
- Os resultados do Core derivam `JsonSchema`; o comentário `///` de cada
  campo vira a descrição no schema, então ele é a documentação que o autor
  de uma extensão vai ler.
- `schema_files_match_the_types` (em `report.rs`, roda com
  `cargo test --bins`) falha quando `docs/schema/` não é o que os tipos
  geram. `crates/aurora-cli/tests/schema.rs` roda os comandos e valida a
  saída contra os arquivos.
- O `docs/PROTOCOL.md` segue a mesma regra: `protocol_doc_matches_the_schema`
  (em `protocol_doc.rs`) falha quando ele não é o que os tipos do protocolo
  geram, e o mesmo comando abaixo o grava de novo.

Quando uma mudança altera o JSON (um campo novo num resultado do Core, um
relatório novo), o teste falha e diz o que fazer:

```sh
AURORA_UPDATE_SCHEMA=1 cargo test -p aurora-cli schema
git diff docs/schema/ docs/PROTOCOL.md
```

Leia o diff: ele é a mudança que quem lê o `--json` vai ver. Renomear ou
tirar um campo quebra essas pessoas e vai no `CHANGELOG.md`; acrescentar
um, em geral, não quebra.

## Antes de mandar a mudança

Rode as verificações. Nenhuma delas roda sozinha no GitHub hoje, nem na
release:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo test --workspace --lib --bins
cargo test --workspace --doc
```

No PowerShell, o `cargo doc` é
`$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --no-deps`.

Hoje o `cargo fmt --check` acusa arquivos que já estavam fora do `rustfmt`
antes da sua mudança. Até eles serem formatados de uma vez (seção 8 da
[metodologia](docs/METODOLOGIA.md)), formate só os arquivos que você
criou ou mexeu, com `rustfmt --edition 2024 <arquivo>`, e não misture a
formatação de outros arquivos na sua mudança.

Depois confira:

- [ ] Rodou `cargo test --workspace` com `AURORA_TEST_BUNDLE` definido, se a
      mudança toca qualquer fluxo de ferramenta (build, check, sim, synth,
      wave, bundle). Sem o bundle, esses testes não testaram nada.
- [ ] O comportamento novo tem teste, e o teste falha sem a mudança.
- [ ] Nenhum `.snap.new` sobrou na árvore.
- [ ] A documentação acompanha: [docs/API.md](docs/API.md) para o Core,
      [docs/CLI.md](docs/CLI.md) para a CLI, [docs/BUNDLE.md](docs/BUNDLE.md)
      para o bundle e a documentação do código para todo item público.
- [ ] Se o usuário percebe a mudança, há uma linha no `CHANGELOG.md`, na
      seção da versão não publicada.
- [ ] Se a mudança toca o contrato (o `aurora-protocol`, os códigos de
      `error.rs` ou `docs/schema/`), ela segue
      [docs/METODOLOGIA.md](docs/METODOLOGIA.md): o diff de `docs/schema/`
      foi lido e classificado como compatível ou não.
- [ ] Você leu o diff inteiro (`git diff`) antes de mandar.

Toda mudança precisa ser explicável por quem a submete: o que ela faz, por
que é assim e como foi testada. Vale também quando um assistente de IA
escreveu o código. Quem submete responde por cada linha; se não sabe
explicar uma, ela não está pronta. Instruções locais para assistentes de IA
ficam em `AGENTS*.md` na raiz, que o `.gitignore` deixa fora do
repositório.

Hoje a CI não roda esses comandos. O `release.yml` só monta o bundle e os
instaladores numa tag, e o `vscode.yml` só testa a extensão e o RTL viewer.
Por isso as verificações acima ficam com quem submete, nos sistemas que
tiver. A CI a cada push e pull request, nas três plataformas, é a fase 1 da
proposta em [docs/METODOLOGIA.md](docs/METODOLOGIA.md), seção 8.
