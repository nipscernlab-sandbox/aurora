//! Erros de infraestrutura do Core.
//!
//! # Duas categorias de falha
//!
//! A AURORA separa "não consegui rodar" de "rodei e deu errado":
//!
//! - **`Err(AuroraError)`**: a operação nem começou ou foi interrompida por algo
//!   fora do código do usuário. Bundle ausente, projeto mal formado, falha de
//!   I/O, processo que não subiu.
//! - **`Ok(resultado)` com `status` diferente de
//!   [`Status::Succeeded`](crate::Status::Succeeded)**: as ferramentas rodaram
//!   e recusaram a entrada. Erro de sintaxe em C±, Verilog que não elabora,
//!   simulação que termina com código diferente de 0. Os detalhes estão nos
//!   `diagnostics` do resultado.
//!
//! Uma interface trata a primeira categoria como falha do ambiente ("configure
//! a toolchain") e a segunda como feedback para o usuário ("linha 16: variável
//! não declarada"). As duas nunca se misturam.
//!
//! # Tratando erros
//!
//! ```
//! use aurora_core::{AuroraError, Toolchain};
//!
//! match Toolchain::open("/nao/existe") {
//!     Ok(_) => unreachable!(),
//!     Err(AuroraError::InvalidBundle { path, .. }) => {
//!         assert_eq!(path, "/nao/existe");
//!     }
//!     Err(other) => panic!("inesperado: {other}"),
//! }
//! ```
//!
//! Para registrar ou serializar sem depender do enum, use
//! [`AuroraError::code`], que devolve um identificador estável por variante, ou
//! [`ErrorInfo`], que leva o código, o texto e os campos da variante em JSON.

use std::fmt;

use camino::Utf8PathBuf;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Atalho para `Result<T, AuroraError>`, usado em toda a API.
pub type Result<T, E = AuroraError> = std::result::Result<T, E>;

/// Tudo que impede uma operação da AURORA de rodar.
///
/// O enum é `#[non_exhaustive]`: novas variantes podem surgir sem quebrar a
/// API, então todo `match` precisa de um braço `_`. A mensagem de
/// [`Display`](std::fmt::Display) é em inglês e pronta para mostrar ao
/// usuário.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AuroraError {
    /// Não há bundle instalado junto com a AURORA.
    ///
    /// Vem de [`Toolchain::locate`](crate::Toolchain::locate). A AURORA só
    /// executa ferramentas do bundle; sem ele, nada que dependa de ferramenta
    /// roda.
    #[error("Tool bundle not found (searched: {})", searched.iter().map(|p| p.as_str()).collect::<Vec<_>>().join(", "))]
    BundleNotFound {
        /// Os diretórios em que a AURORA procurou um `bundle.json`.
        searched: Vec<Utf8PathBuf>,
    },

    /// O diretório não é um bundle válido para esta AURORA: sem `bundle.json`,
    /// JSON inválido, formato desconhecido, montado para outra plataforma, ou
    /// com componente ausente ou apontando para fora do bundle.
    ///
    /// Vem de [`Toolchain::open`](crate::Toolchain::open).
    #[error("Invalid bundle at {path}: {reason}")]
    InvalidBundle {
        /// O diretório do bundle.
        path: Utf8PathBuf,
        /// O que está errado.
        reason: String,
    },

    /// A AURORA está rodando numa plataforma para a qual não há bundle (só
    /// Linux x64, macOS arm64 e Windows x64).
    #[error(
        "Unsupported platform: {os} {arch} (bundles exist for linux-x64, darwin-arm64 and windows-x64)"
    )]
    UnsupportedPlatform {
        /// `std::env::consts::OS`.
        os: String,
        /// `std::env::consts::ARCH`.
        arch: String,
    },

    /// O componente de que a ferramenta faz parte não foi instalado: o usuário
    /// não o escolheu no instalador, ou é um bundle de desenvolvimento só com
    /// o YANC.
    ///
    /// Vem de [`Toolchain::tool`](crate::Toolchain::tool). O texto não diz
    /// como instalar: cada interface tem o seu caminho (`aurora install`, o
    /// botão Instalar do Studio).
    #[error("Component {0} is not installed")]
    ComponentMissing(String),

    /// Um arquivo que o bundle deveria ter não está lá: o executável de uma
    /// ferramenta, ou um diretório do YANC (`HDL`, `Macros`, `Header`).
    ///
    /// Vem de [`Toolchain::tool`](crate::Toolchain::tool) e dos métodos de
    /// diretório do YANC. Indica bundle corrompido ou incompleto: reinstale.
    #[error("Incomplete bundle: {what} not found at {path}")]
    ToolchainIncomplete {
        /// O que faltou, em texto (por exemplo "yosys executable").
        what: String,
        /// Onde se esperava encontrar.
        path: Utf8PathBuf,
    },

    /// No Linux e no macOS, o Verilator precisa de compilador C++, `make` e
    /// Perl do sistema, e eles não foram encontrados nos locais padrão (ver
    /// [`SystemCompiler`](crate::SystemCompiler)). No Windows os três vêm com
    /// o componente verilator, e a falta deles é
    /// [`AuroraError::ToolchainIncomplete`].
    #[error(
        "Verilator needs a system C++ compiler, make and Perl, and they were not found \
         (Linux: g++, make and perl in /usr/bin, from sudo apt install build-essential perl,          sudo dnf install gcc-c++ make perl or sudo pacman -S --needed base-devel;          macOS: Xcode Command Line Tools)"
    )]
    SystemCompilerMissing,

    /// A compilação para uma placa Intel precisa do Quartus Prime, que não
    /// vem no bundle e não foi achado ([`crate::fpga::Quartus`]). Ele roda
    /// no Windows e no Linux.
    #[error(
        "Intel FPGA boards need Quartus Prime, which does not come in the bundle, and it was not found"
    )]
    QuartusMissing,

    /// Gravar na placa sem a compilação para ela: falta o `.sof`
    /// ([`crate::fpga::program()`]).
    #[error("No bitstream for the board: {0} does not exist")]
    NoBitstream(Utf8PathBuf),

    /// Nenhum cabo de gravação respondeu ao `quartus_pgm -l`: a placa
    /// desligada ou desconectada, a porta USB errada ou o driver do
    /// USB-Blaster ausente.
    #[error("No programming cable found: the Quartus Programmer sees no USB-Blaster")]
    NoCable,

    /// O `.sof` não descreve o projeto de agora: um fonte, uma memória, o
    /// `fpga.json` ou a placa mudou depois da compilação, o `.sof` foi
    /// trocado, ou a compilação não terminou. A placa receberia outro design.
    #[error("The bitstream does not match the project: {}", reasons.join("; "))]
    StaleBitstream {
        /// Cada diferença, em texto.
        reasons: Vec<String>,
    },

    /// O projeto no disco não está como deveria: diretório ausente, arquivo
    /// onde deveria haver diretório, arquivo registrado que sumiu, arquivo que
    /// a AURORA se recusa a sobrescrever.
    #[error("Invalid project at {path}: {reason}")]
    InvalidProject {
        /// O caminho com problema.
        path: Utf8PathBuf,
        /// O que está errado, em texto.
        reason: String,
    },

    /// Já existe um `.spf` onde [`Project::create`](crate::Project::create)
    /// ia criar o projeto. Nada foi alterado.
    #[error("Project already exists: {0}")]
    ProjectExists(Utf8PathBuf),

    /// Um caminho que precisa estar dentro da pasta do projeto está fora
    /// dela. Vem de [`Project::move_path`](crate::Project::move_path).
    #[error("{0} is outside the project folder")]
    OutsideProject(Utf8PathBuf),

    /// [`Project::move_path`](crate::Project::move_path) recusou o movimento:
    /// o `.spf`, a pasta `.aurora8`, a pasta de um processador, a pasta de saída
    /// dele e o fonte ficam onde o Core os procura; uma pasta não vai para dentro dela mesma; nada vai
    /// para dentro de `.aurora8`. Nada foi alterado.
    #[error("Cannot move {path}: {reason}")]
    CannotMove {
        /// O que seria movido.
        path: Utf8PathBuf,
        /// Por que não.
        reason: String,
    },

    /// O destino já existe. Nada foi alterado: a AURORA não sobrescreve.
    #[error("{0} already exists")]
    PathExists(Utf8PathBuf),

    /// [`Project::add_processor`](crate::Project::add_processor) recebeu um
    /// nome que já está no projeto. Nada foi alterado.
    #[error("Processor '{0}' already exists in the project")]
    ProcessorExists(String),

    /// O projeto não tem o processador pedido.
    ///
    /// Vem de [`Project::require_processor`](crate::Project::require_processor)
    /// e das operações que recebem um nome de processador.
    #[error("No processor '{name}' in the project (available: {})", available.join(", "))]
    ProcessorNotFound {
        /// O nome pedido.
        name: String,
        /// Os processadores que o projeto tem, na ordem do `.spf`.
        available: Vec<String>,
    },

    /// Nome de projeto, de processador ou de módulo de topo que não pode ser
    /// usado.
    ///
    /// Processador e módulo de topo precisam ser identificadores
    /// (`[A-Za-z_][A-Za-z0-9_]*`), porque viram nome de arquivo e de módulo
    /// Verilog. Projeto precisa ser um nome de pasta válido no Windows e no
    /// Linux.
    #[error("Invalid name '{name}': {reason}")]
    InvalidName {
        /// O nome recusado.
        name: String,
        /// Por quê.
        reason: String,
    },

    /// O `.spf` não pôde ser lido: JSON quebrado mesmo depois da leitura
    /// tolerante, falta a seção `structure`, ou um campo tem tipo ou valor
    /// inválido (por exemplo `clk: 12.5`).
    #[error("Invalid project file {path}: {reason}")]
    InvalidProjectFile {
        /// O `.spf`.
        path: Utf8PathBuf,
        /// O que está errado.
        reason: String,
    },

    /// O programa-fonte de um processador não pode ser compilado como está:
    /// não existe, ou não se chama `<processador>.<ext>` (o YANC 7.1 dá ao
    /// processador o nome do arquivo).
    ///
    /// Vem de [`build`](crate::build), antes de rodar qualquer compilador.
    #[error("Invalid source {path}: {reason}")]
    InvalidSource {
        /// O arquivo-fonte.
        path: Utf8PathBuf,
        /// O que está errado.
        reason: String,
    },

    /// Um processador ainda tem a estrutura de pastas de antes do YANC 7.1
    /// (`Software/`, `Hardware/`, `Simulation/`), que não compila mais.
    ///
    /// Vem de [`build`](crate::build), antes de rodar qualquer compilador. O
    /// conserto é [`Project::convert_layout`](crate::Project::convert_layout);
    /// o texto não diz como chamá-lo, cada interface tem o seu caminho.
    #[error(
        "Processor {processor} still has the folder layout of YANC 6.x (Software/, Hardware/, Simulation/ in {dir}); the project needs to be converted to the layout of YANC 7.1"
    )]
    LegacyLayout {
        /// O processador.
        processor: String,
        /// A pasta dele.
        dir: Utf8PathBuf,
    },

    /// O YANC do bundle é anterior à 7.1, que mudou a linha de comando e as
    /// pastas: a AURORA não sabe mais usá-lo.
    ///
    /// Vem de [`build`](crate::build), antes de rodar qualquer compilador.
    #[error("The bundle has YANC {found}, and AURORA needs YANC {minimum} or newer")]
    YancTooOld {
        /// A versão do componente no manifesto do bundle.
        found: String,
        /// A menor que a AURORA usa.
        minimum: String,
    },

    /// A operação precisa de artefatos que [`build`](crate::build) ainda não
    /// gerou para este processador (testbench, Verilog).
    #[error("Processor '{processor}' has not been built: {missing} is missing")]
    NotBuilt {
        /// O processador.
        processor: String,
        /// O primeiro artefato que faltou.
        missing: Utf8PathBuf,
    },

    /// [`simulate_project`](crate::simulate_project) foi chamado num projeto
    /// sem testbench registrado. Crie ou registre um com
    /// [`Project::add_verilog`](crate::Project::add_verilog).
    #[error("Project {0} has no testbench")]
    NoTestbench(Utf8PathBuf),

    /// A onda que se quer abrir ainda não existe no disco: a simulação do
    /// projeto, ou a do processador, não rodou (ou não chegou ao fim), ou o
    /// arquivo pedido não existe. Vem de [`flows::open_wave`](crate::flows).
    /// O texto não diz como simular: cada interface tem o seu caminho (`aurora
    /// sim [-p X]`, o botão Simular do Studio).
    #[error("Waveform {path} does not exist yet{}; simulate first", processor.as_deref().map(|p| format!(" (processor {p})")).unwrap_or_default())]
    NoWaveform {
        /// Onde a onda ficaria.
        path: Utf8PathBuf,
        /// O processador, quando a onda é a da simulação dele; `None` na do
        /// projeto.
        processor: Option<String>,
    },

    /// A escolha de sinais da onda ([`flows::choose_signals`](crate::flows))
    /// recebeu um caminho que a árvore do testbench elaborado não tem.
    #[error("{path} is not a signal or scope of {root}{}", if similar.is_empty() { String::new() } else { format!("; did you mean {}?", similar.join(", ")) })]
    SignalNotFound {
        /// O caminho pedido (`tb.dut.q`).
        path: String,
        /// A raiz da árvore (o módulo do testbench).
        root: String,
        /// Caminhos da árvore com o mesmo nome final, até cinco, para a
        /// interface sugerir.
        similar: Vec<String>,
    },

    /// A escolha de sinais recebeu, para tirar, um caminho que não está na
    /// escolha gravada do testbench nem no design.
    #[error("{path} is not in the signal choice of {testbench}")]
    SignalNotChosen {
        /// O caminho pedido.
        path: String,
        /// O módulo do testbench cuja escolha é essa.
        testbench: String,
    },

    /// A escolha de sinais precisa da árvore do testbench, e a elaboração
    /// dele pelo Icarus falhou. `reason` é a primeira mensagem de erro da
    /// elaboração, ou vazio se ela não disse nada.
    #[error("The project testbench did not elaborate{}", if reason.is_empty() { String::new() } else { format!(": {reason}") })]
    TestbenchNotElaborated {
        /// O primeiro erro da elaboração, ou vazio.
        reason: String,
    },

    /// O testbench cocotb (`.py`) não diz qual módulo testa (a linha
    /// `# aurora-toplevel: <módulo>`) e o projeto não tem módulo de topo
    /// para o lugar dele.
    #[error(
        "The cocotb testbench {0} does not say which module it tests: add a `# aurora-toplevel: <module>` line to it, or choose the project top module"
    )]
    NoCocotbToplevel(Utf8PathBuf),

    /// O simulador pedido não simula a linguagem do testbench: o GHDL com um
    /// testbench Verilog ou de um processador, ou um testbench cocotb no
    /// GHDL, que a AURORA ainda não roda. Um testbench VHDL não chega aqui:
    /// ele vai sempre para o GHDL.
    #[error("{simulator} does not simulate this testbench ({language}): {testbench}")]
    SimulatorLanguage {
        /// O simulador pedido (`ghdl`).
        simulator: String,
        /// A linguagem do testbench (`Verilog`, `Python`).
        language: String,
        /// O testbench.
        testbench: Utf8PathBuf,
    },

    /// O Python do componente `cocotb` não carregou o cocotb (a sonda que diz
    /// onde estão a VPI e a biblioteca do Python falhou), ou o cocotb dele
    /// não traz a biblioteca do simulador pedido.
    #[error("cocotb could not be loaded by {python}: {reason}")]
    CocotbUnavailable {
        /// O comando da sonda.
        python: String,
        /// O fim do que a sonda escreveu, ou o motivo.
        reason: String,
    },

    /// [`synthesize`](crate::synthesize) com
    /// [`DesignTarget::TopLevel`](crate::DesignTarget::TopLevel) num projeto
    /// sem módulo de topo. Escolha um com
    /// [`Project::set_top`](crate::Project::set_top).
    #[error("Project {0} has no top module")]
    NoTopLevel(Utf8PathBuf),

    /// [`check`](crate::check) num projeto sem nenhum arquivo Verilog
    /// registrado e sem processadores.
    #[error("Project {0} has no Verilog files added")]
    EmptyProject(Utf8PathBuf),

    /// Um módulo pedido pelo nome não está em nenhum arquivo registrado do
    /// projeto ([`Project::set_top`](crate::Project::set_top)), ou o arquivo de
    /// topo não declara um módulo que dê para usar.
    #[error("Module {name} not found{}", if available.is_empty() { String::new() } else { format!(" (project modules: {})", available.join(", ")) })]
    ModuleNotFound {
        /// O nome pedido (ou o arquivo, quando ele não declara módulo).
        name: String,
        /// Os módulos dos arquivos sintetizáveis registrados.
        available: Vec<String>,
    },

    /// O módulo pedido pelo nome está declarado em mais de um arquivo
    /// registrado ([`Project::set_top`](crate::Project::set_top)). Escolha
    /// pelo arquivo. Nada foi alterado.
    #[error("Module {name} is declared in more than one file: {}", files.iter().map(|p| p.as_str()).collect::<Vec<_>>().join(", "))]
    AmbiguousModule {
        /// O nome pedido.
        name: String,
        /// Os arquivos que o declaram, na ordem do `.spf`.
        files: Vec<Utf8PathBuf>,
    },

    /// Um parâmetro de processador fora do que o YANC compila: `#NUBITS`
    /// diferente de `#NBMANT + #NBEXPO + 1`, `#NUGAIN` que não é potência de
    /// dois, pilha vazia, clocks ou frequência fora da faixa. Vem de
    /// [`Project::add_processor`](crate::Project::add_processor) e de
    /// [`Project::configure_processor`](crate::Project::configure_processor);
    /// nada foi alterado.
    #[error("Invalid {name} ({value}): {reason}")]
    InvalidParameter {
        /// O parâmetro, como aparece no fonte ou no `.spf` (`#NUBITS`,
        /// `numClocks`).
        name: String,
        /// O valor recusado.
        value: String,
        /// A regra.
        reason: String,
    },

    /// Outra operação da AURORA (build ou simulação) está usando o mesmo
    /// processador, ou o projeto: duas ao mesmo tempo regravariam os mesmos
    /// arquivos e quebrariam as duas. Vem de [`build`](crate::build),
    /// [`simulate`](crate::simulate) e
    /// [`simulate_project`](crate::simulate_project); nada foi alterado.
    #[error(
        "Another AURORA operation (a build, a simulation, a synthesis, maybe in another window) is using the files of {name}; try again when it finishes"
    )]
    OperationInProgress {
        /// O processador ou o projeto.
        name: String,
    },

    /// O netlist JSON do Yosys não pôde ser lido ou não tem o módulo pedido.
    ///
    /// Vem de [`render_schematic`](crate::render_schematic).
    #[error("Invalid netlist {path}: {reason}")]
    InvalidNetlist {
        /// O arquivo de netlist.
        path: Utf8PathBuf,
        /// O que está errado.
        reason: String,
    },

    /// Um arquivo de dados da simulação (`input_<n>.txt`, `output_<n>.txt`)
    /// tem uma linha que não é um inteiro decimal.
    ///
    /// Vem de [`read_data_file`](crate::read_data_file) e de
    /// [`Processor::read_output_values`](crate::Processor::read_output_values).
    #[error("{path}:{line}: {reason}")]
    InvalidDataFile {
        /// O arquivo.
        path: Utf8PathBuf,
        /// A linha, a partir de 1.
        line: u32,
        /// O que está errado.
        reason: String,
    },

    /// Nenhuma pasta, da pedida até a raiz do sistema, tem um `.spf`
    /// ([`Project::discover`](crate::Project::discover)).
    #[error("Na AURORA project here: no .spf file in {0} or in any folder above it")]
    ProjectNotFound(Utf8PathBuf),

    /// O projeto ainda não tem relatório guardado: nenhuma operação foi
    /// registrada no histórico ([`history`](crate::history)).
    #[error("Project {0} has no stored reports yet")]
    NoReports(Utf8PathBuf),

    /// Não há relatório com esse identificador no histórico do projeto.
    #[error("Report {0} not found")]
    ReportNotFound(String),

    /// Um relatório guardado não pôde ser lido: o `record.json` sumiu, não é
    /// JSON, ou é de um formato que esta AURORA não entende.
    #[error("Invalid report {path}: {reason}")]
    InvalidReport {
        /// O arquivo.
        path: Utf8PathBuf,
        /// O que está errado.
        reason: String,
    },

    /// Os relatórios pedidos não se comparam: de projetos diferentes, sem
    /// estatísticas de síntese nem tempos de simulação, ou sem um anterior
    /// compatível.
    #[error("{0}")]
    NotComparable(String),

    /// Nenhuma placa com esse identificador ([`crate::fpga::board()`]).
    #[error("No board {name} (known boards: {})", available.join(", "))]
    BoardNotFound {
        /// O que foi pedido.
        name: String,
        /// As placas conhecidas.
        available: Vec<String>,
    },

    /// O JSON embutido de uma placa é inválido. Os testes impedem isso; se
    /// acontecer, é bug.
    #[error("Invalid board {id}: {reason}")]
    InvalidBoard {
        /// A placa.
        id: String,
        /// O que está errado.
        reason: String,
    },

    /// O projeto não tem `fpga.json` ([`crate::fpga::config`]): falta dizer a
    /// placa e as ligações.
    #[error("The project has no fpga.json ({0})")]
    NoFpgaConfig(Utf8PathBuf),

    /// O `fpga.json` não é válido: JSON malformado, ou ligações que não
    /// batem com a placa e o topo (uma por linha).
    #[error("Invalid {path}: {reason}")]
    InvalidFpgaConfig {
        /// O arquivo.
        path: Utf8PathBuf,
        /// O que está errado.
        reason: String,
    },

    /// A placa virtual ainda não foi compilada para o projeto
    /// ([`crate::virtual_board::build`]): falta o executável dela.
    #[error("The virtual board has not been built: {0} is missing")]
    VirtualBoardNotBuilt(Utf8PathBuf),

    /// Uma entrada para a placa virtual cita um periférico que ela não tem
    /// ligado (ou que não recebe esse tipo de entrada).
    #[error("The virtual board has no {device} that takes this input{}", if available.is_empty() { String::new() } else { format!(" (available: {})", available.join(", ")) })]
    VirtualBoardDevice {
        /// O periférico pedido.
        device: String,
        /// Os periféricos ligados que recebem esse tipo de entrada.
        available: Vec<String>,
    },

    /// Um valor para chaves ou botões que não cabe neles: um bit além da
    /// largura do periférico.
    #[error("{device} has {width} bits; {value:#x} does not fit (the largest is {:#x})", if *width >= 64 { u64::MAX } else { (1u64 << width) - 1 })]
    VirtualBoardValue {
        /// O periférico.
        device: String,
        /// A largura dele, em bits.
        width: u32,
        /// O valor pedido.
        value: u64,
    },

    /// Um caminho não é UTF-8. A API serializa caminhos para JSON e usa
    /// [`camino`](https://docs.rs/camino) em toda a superfície, então recusa
    /// esses caminhos na entrada. O texto é a representação com perdas.
    #[error("Path is not UTF-8: {0}")]
    NonUtf8Path(String),

    /// Caminho longo demais para os buffers fixos dos compiladores YANC (ver
    /// [`YANC_PATH_LIMIT`](crate::YANC_PATH_LIMIT)). Mova o projeto para um
    /// diretório mais curto.
    #[error("Path too long for YANC ({len} bytes, limit {limit}): {path}")]
    PathTooLong {
        /// O caminho recusado.
        path: Utf8PathBuf,
        /// O tamanho dele, em bytes.
        len: usize,
        /// O limite.
        limit: usize,
    },

    /// Um subprocesso não chegou a iniciar: executável ausente, sem permissão
    /// de execução, CWD inexistente.
    ///
    /// Um processo que inicia e falha não é este erro; ele aparece como
    /// [`Termination`](crate::Termination) no relatório do passo.
    #[error("Could not run {program}: {source}")]
    Spawn {
        /// O programa que se tentou executar.
        program: Utf8PathBuf,
        /// O erro do sistema operacional.
        #[source]
        source: std::io::Error,
    },

    /// Um processo de longa duração (o Surfer) terminou logo depois de
    /// iniciar. Vem de
    /// [`RunningProcess::ensure_started`](crate::RunningProcess::ensure_started).
    #[error("{program} exited right after starting ({termination:?}); end of log {log}:\n{tail}")]
    ProcessExitedEarly {
        /// O programa.
        program: Utf8PathBuf,
        /// Como terminou.
        termination: crate::Termination,
        /// O arquivo de log completo.
        log: Utf8PathBuf,
        /// As últimas linhas do log.
        tail: String,
    },

    /// Um recurso experimental que o usuário não ligou
    /// ([`crate::experimental`]): a placa FPGA, por exemplo. Vem de
    /// [`experimental::require`](crate::experimental::require), que a CLI e o
    /// Studio chamam antes de oferecer o recurso. O texto não diz como ligar:
    /// cada interface tem o seu caminho (`aurora experimental enable`, as
    /// Preferências do Studio).
    #[error("The experimental component {feature} ({}) is disabled", feature.name())]
    ExperimentalDisabled {
        /// O recurso.
        feature: crate::experimental::Feature,
    },

    /// O pedido não é atendido aqui: a correção de um exercício
    /// ([`FlowRequest::Learn`](crate::flows::FlowRequest::Learn)) chega ao
    /// [`flows::run`](crate::flows::run), mas quem a roda é o servidor do
    /// AURORA, que linka o aurora-learn.
    #[error("{0}")]
    Unsupported(String),

    /// Falha de I/O da própria AURORA: criar diretório, ler ou gravar arquivo.
    #[error("{context}: {path}: {source}")]
    Io {
        /// O que a AURORA estava fazendo, em texto ("Creating directory").
        context: &'static str,
        /// O caminho envolvido.
        path: Utf8PathBuf,
        /// O erro do sistema operacional.
        #[source]
        source: std::io::Error,
    },
}

impl AuroraError {
    /// Identificador estável da variante, em `snake_case`, para JSON, logs e
    /// uma futura fronteira FFI. Nunca muda para uma variante existente.
    ///
    /// ```
    /// # use aurora_core::AuroraError;
    /// let err = AuroraError::ProcessorExists("soma".into());
    /// assert_eq!(err.code(), "processor_exists");
    /// ```
    pub fn code(&self) -> &'static str {
        match self {
            AuroraError::BundleNotFound { .. } => "bundle_not_found",
            AuroraError::InvalidBundle { .. } => "invalid_bundle",
            AuroraError::UnsupportedPlatform { .. } => "unsupported_platform",
            AuroraError::ComponentMissing(_) => "component_missing",
            AuroraError::ToolchainIncomplete { .. } => "toolchain_incomplete",
            AuroraError::SystemCompilerMissing => "system_compiler_missing",
            AuroraError::QuartusMissing => "quartus_missing",
            AuroraError::NoBitstream(_) => "no_bitstream",
            AuroraError::NoCable => "no_cable",
            AuroraError::StaleBitstream { .. } => "stale_bitstream",
            AuroraError::InvalidProject { .. } => "invalid_project",
            AuroraError::ProjectExists(_) => "project_exists",
            AuroraError::OutsideProject(_) => "outside_project",
            AuroraError::CannotMove { .. } => "cannot_move",
            AuroraError::PathExists(_) => "path_exists",
            AuroraError::ProcessorExists(_) => "processor_exists",
            AuroraError::ProcessorNotFound { .. } => "processor_not_found",
            AuroraError::InvalidName { .. } => "invalid_name",
            AuroraError::InvalidProjectFile { .. } => "invalid_project_file",
            AuroraError::InvalidSource { .. } => "invalid_source",
            AuroraError::LegacyLayout { .. } => "legacy_layout",
            AuroraError::YancTooOld { .. } => "yanc_too_old",
            AuroraError::NotBuilt { .. } => "not_built",
            AuroraError::NoTestbench(_) => "no_testbench",
            AuroraError::NoWaveform { .. } => "no_waveform",
            AuroraError::SignalNotFound { .. } => "signal_not_found",
            AuroraError::SignalNotChosen { .. } => "signal_not_chosen",
            AuroraError::TestbenchNotElaborated { .. } => "testbench_not_elaborated",
            AuroraError::NoCocotbToplevel(_) => "no_cocotb_toplevel",
            AuroraError::CocotbUnavailable { .. } => "cocotb_unavailable",
            AuroraError::SimulatorLanguage { .. } => "simulator_language",
            AuroraError::NoTopLevel(_) => "no_top_level",
            AuroraError::EmptyProject(_) => "empty_project",
            AuroraError::ModuleNotFound { .. } => "module_not_found",
            AuroraError::AmbiguousModule { .. } => "ambiguous_module",
            AuroraError::InvalidParameter { .. } => "invalid_parameter",
            AuroraError::OperationInProgress { .. } => "operation_in_progress",
            AuroraError::InvalidNetlist { .. } => "invalid_netlist",
            AuroraError::InvalidDataFile { .. } => "invalid_data_file",
            AuroraError::NonUtf8Path(_) => "non_utf8_path",
            AuroraError::PathTooLong { .. } => "path_too_long",
            AuroraError::Spawn { .. } => "spawn",
            AuroraError::ProcessExitedEarly { .. } => "process_exited_early",
            AuroraError::Io { .. } => "io",
            AuroraError::ProjectNotFound(_) => "project_not_found",
            AuroraError::NoReports(_) => "no_reports",
            AuroraError::ReportNotFound(_) => "report_not_found",
            AuroraError::InvalidReport { .. } => "invalid_report",
            AuroraError::NotComparable(_) => "not_comparable",
            AuroraError::BoardNotFound { .. } => "board_not_found",
            AuroraError::InvalidBoard { .. } => "invalid_board",
            AuroraError::NoFpgaConfig(_) => "no_fpga_config",
            AuroraError::InvalidFpgaConfig { .. } => "invalid_fpga_config",
            AuroraError::VirtualBoardNotBuilt(_) => "virtual_board_not_built",
            AuroraError::VirtualBoardDevice { .. } => "virtual_board_device",
            AuroraError::VirtualBoardValue { .. } => "virtual_board_value",
            AuroraError::ExperimentalDisabled { .. } => "experimental_disabled",
            AuroraError::Unsupported(_) => "unsupported",
        }
    }

    /// Fecho que transforma um `std::io::Error` em [`AuroraError::Io`], para
    /// usar com `map_err`.
    pub(crate) fn io(
        context: &'static str,
        path: impl Into<Utf8PathBuf>,
    ) -> impl FnOnce(std::io::Error) -> AuroraError {
        let path = path.into();
        move |source| AuroraError::Io {
            context,
            path,
            source,
        }
    }

    /// Os campos estruturados da variante, como objeto JSON, para
    /// [`ErrorInfo::data`]: o que uma interface usa além do texto (a lista
    /// `available` para sugerir, o `path` para abrir, o `name` e o `reason`
    /// de um parâmetro). `None` nas variantes sem campo que valha a pena
    /// (um `io::Error` fica só no texto).
    pub fn data(&self) -> Option<Value> {
        let data = match self {
            AuroraError::BundleNotFound { searched } => json!({ "searched": searched }),
            AuroraError::InvalidBundle { path, reason } => {
                json!({ "path": path, "reason": reason })
            }
            AuroraError::UnsupportedPlatform { os, arch } => json!({ "os": os, "arch": arch }),
            AuroraError::ComponentMissing(component) => json!({ "component": component }),
            AuroraError::ToolchainIncomplete { what, path } => {
                json!({ "what": what, "path": path })
            }
            AuroraError::SystemCompilerMissing
            | AuroraError::QuartusMissing
            | AuroraError::NoCable
            | AuroraError::Unsupported(_) => {
                return None;
            }
            AuroraError::NoBitstream(path) => json!({ "path": path }),
            AuroraError::StaleBitstream { reasons } => json!({ "reasons": reasons }),
            AuroraError::InvalidProject { path, reason } => {
                json!({ "path": path, "reason": reason })
            }
            AuroraError::ProjectExists(path)
            | AuroraError::OutsideProject(path)
            | AuroraError::PathExists(path)
            | AuroraError::NoTestbench(path)
            | AuroraError::NoCocotbToplevel(path)
            | AuroraError::NoTopLevel(path)
            | AuroraError::EmptyProject(path)
            | AuroraError::ProjectNotFound(path)
            | AuroraError::NoReports(path)
            | AuroraError::NoFpgaConfig(path) => json!({ "path": path }),
            AuroraError::CannotMove { path, reason } => json!({ "path": path, "reason": reason }),
            AuroraError::ProcessorExists(name) | AuroraError::OperationInProgress { name } => {
                json!({ "name": name })
            }
            AuroraError::ProcessorNotFound { name, available }
            | AuroraError::ModuleNotFound { name, available }
            | AuroraError::BoardNotFound { name, available } => {
                json!({ "name": name, "available": available })
            }
            AuroraError::InvalidName { name, reason } => json!({ "name": name, "reason": reason }),
            AuroraError::InvalidProjectFile { path, reason }
            | AuroraError::InvalidSource { path, reason }
            | AuroraError::InvalidNetlist { path, reason }
            | AuroraError::InvalidReport { path, reason }
            | AuroraError::InvalidFpgaConfig { path, reason } => {
                json!({ "path": path, "reason": reason })
            }
            AuroraError::LegacyLayout { processor, dir } => {
                json!({ "processor": processor, "dir": dir })
            }
            AuroraError::YancTooOld { found, minimum } => {
                json!({ "found": found, "minimum": minimum })
            }
            AuroraError::NotBuilt { processor, missing } => {
                json!({ "processor": processor, "missing": missing })
            }
            AuroraError::NoWaveform { path, processor } => {
                json!({ "path": path, "processor": processor })
            }
            AuroraError::SignalNotFound {
                path,
                root,
                similar,
            } => json!({ "path": path, "root": root, "similar": similar }),
            AuroraError::SignalNotChosen { path, testbench } => {
                json!({ "path": path, "testbench": testbench })
            }
            AuroraError::TestbenchNotElaborated { reason } => json!({ "reason": reason }),
            AuroraError::SimulatorLanguage {
                simulator,
                language,
                testbench,
            } => json!({ "simulator": simulator, "language": language, "testbench": testbench }),
            AuroraError::CocotbUnavailable { python, reason } => {
                json!({ "python": python, "reason": reason })
            }
            AuroraError::AmbiguousModule { name, files } => json!({ "name": name, "files": files }),
            AuroraError::InvalidParameter {
                name,
                value,
                reason,
            } => json!({ "name": name, "value": value, "reason": reason }),
            AuroraError::InvalidDataFile { path, line, reason } => {
                json!({ "path": path, "line": line, "reason": reason })
            }
            AuroraError::ReportNotFound(report) => json!({ "report": report }),
            AuroraError::NotComparable(_) => return None,
            AuroraError::InvalidBoard { id, reason } => json!({ "id": id, "reason": reason }),
            AuroraError::NonUtf8Path(path) => json!({ "path": path }),
            AuroraError::PathTooLong { path, len, limit } => {
                json!({ "path": path, "len": len, "limit": limit })
            }
            AuroraError::Spawn { program, .. } => json!({ "program": program }),
            AuroraError::ProcessExitedEarly {
                program,
                termination,
                log,
                tail,
            } => {
                json!({ "program": program, "termination": termination, "log": log, "tail": tail })
            }
            AuroraError::VirtualBoardNotBuilt(path) => json!({ "path": path }),
            AuroraError::VirtualBoardDevice { device, available } => {
                json!({ "device": device, "available": available })
            }
            AuroraError::VirtualBoardValue {
                device,
                width,
                value,
            } => json!({ "device": device, "width": width, "value": value }),
            AuroraError::ExperimentalDisabled { feature } => json!({ "feature": feature }),
            AuroraError::Io { context, path, .. } => json!({ "context": context, "path": path }),
        };
        Some(data)
    }
}

/// Um [`AuroraError`] pronto para atravessar uma fronteira: o JSON da CLI, a
/// mensagem do Studio para a interface, a resposta do `aurora serve`. É o que
/// o código, o texto e os campos da variante viram quando o enum não vai.
///
/// ```json
/// { "code": "processor_not_found",
///   "message": "No processor 'x' in the project (available: soma, filtro)",
///   "data": { "name": "x", "available": ["soma", "filtro"] } }
/// ```
///
/// A dica de cada interface (o comando `aurora` que resolve, o botão do
/// Studio) não mora aqui: a mensagem do Core é neutra, e quem mostra decide
/// pelo `code`. Um erro que não é do Core (o `busy` do Studio, o `cli` da
/// CLI) usa o mesmo tipo com [`ErrorInfo::new`] e um código próprio.
///
/// ```
/// use aurora_core::{ErrorInfo, AuroraError};
///
/// let info = ErrorInfo::from(AuroraError::ProcessorExists("soma".into()));
/// assert_eq!(info.code, "processor_exists");
/// assert_eq!(info.data.unwrap()["name"], "soma");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorInfo {
    /// O identificador estável: [`AuroraError::code`], ou um código de quem
    /// chamou (`busy`, `cli`), sempre em `snake_case`.
    pub code: String,
    /// O texto do erro, em inglês, pronto para mostrar.
    pub message: String,
    /// O que fazer, quando quem mostra sabe: o comando `aurora` que resolve,
    /// na CLI. O core não o preenche; a mensagem dele é neutra. Omitido no
    /// JSON quando não há.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Os campos estruturados da variante, quando ela os tem
    /// ([`AuroraError::data`]): `available`, `files`, `name`, `value`,
    /// `reason`, `path`, `component`. Omitido no JSON quando não há.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ErrorInfo {
    /// Um erro com código e mensagem, sem dados: para o que não é um
    /// [`AuroraError`].
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        ErrorInfo {
            code: code.into(),
            message: message.into(),
            hint: None,
            data: None,
        }
    }

    /// O mesmo erro com a dica de quem mostra.
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl fmt::Display for ErrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ErrorInfo {}

impl From<&AuroraError> for ErrorInfo {
    fn from(error: &AuroraError) -> Self {
        ErrorInfo {
            code: error.code().to_owned(),
            message: error.to_string(),
            hint: None,
            data: error.data(),
        }
    }
}

impl From<AuroraError> for ErrorInfo {
    fn from(error: AuroraError) -> Self {
        ErrorInfo::from(&error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_info_carries_code_text_and_the_structured_fields() {
        let error = AuroraError::ProcessorNotFound {
            name: "x".into(),
            available: vec!["soma".into(), "filtro".into()],
        };
        let info = ErrorInfo::from(&error);
        assert_eq!(info.code, "processor_not_found");
        assert_eq!(info.message, error.to_string());
        let data = info.data.clone().expect("os campos da variante");
        assert_eq!(data["name"], "x");
        assert_eq!(data["available"], json!(["soma", "filtro"]));
        // O JSON tem a forma que a CLI e o Studio já mandam, mais o data.
        let value = serde_json::to_value(&info).unwrap();
        assert_eq!(value["code"], "processor_not_found");
        assert!(value["data"]["available"].is_array());
        // E volta do JSON igual.
        let back: ErrorInfo = serde_json::from_value(value).unwrap();
        assert_eq!(back, info);
    }

    #[test]
    fn error_info_without_fields_omits_data() {
        let info = ErrorInfo::from(AuroraError::SystemCompilerMissing);
        assert_eq!(info.code, "system_compiler_missing");
        assert_eq!(info.data, None);
        let value = serde_json::to_value(&info).unwrap();
        assert!(value.get("data").is_none(), "{value}");
        let plain = ErrorInfo::new("busy", "Another operation is running");
        assert_eq!(plain.to_string(), "Another operation is running");
        let back: ErrorInfo =
            serde_json::from_str(r#"{"code":"busy","message":"x"}"#).expect("data é opcional");
        assert_eq!(back.data, None);
        assert_eq!(back.hint, None);
        let hinted = plain.with_hint("Try again");
        let value = serde_json::to_value(&hinted).unwrap();
        assert_eq!(value["hint"], "Try again");
        assert!(value.get("data").is_none());
    }

    #[test]
    fn the_new_variants_have_codes_and_neutral_messages() {
        let wave = AuroraError::NoWaveform {
            path: "/p/soma/soma/soma_tb.fst".into(),
            processor: Some("soma".into()),
        };
        assert_eq!(wave.code(), "no_waveform");
        assert_eq!(
            wave.to_string(),
            "Waveform /p/soma/soma/soma_tb.fst does not exist yet (processor soma); simulate first"
        );
        assert_eq!(ErrorInfo::from(&wave).data.unwrap()["processor"], "soma");

        let signal = AuroraError::SignalNotFound {
            path: "tb.q".into(),
            root: "tb".into(),
            similar: vec!["tb.dut.q".into()],
        };
        assert_eq!(signal.code(), "signal_not_found");
        assert_eq!(
            signal.to_string(),
            "tb.q is not a signal or scope of tb; did you mean tb.dut.q?"
        );
        let chosen = AuroraError::SignalNotChosen {
            path: "tb.q".into(),
            testbench: "tb".into(),
        };
        assert_eq!(chosen.code(), "signal_not_chosen");
        let elaborated = AuroraError::TestbenchNotElaborated {
            reason: String::new(),
        };
        assert_eq!(elaborated.code(), "testbench_not_elaborated");
        assert_eq!(
            elaborated.to_string(),
            "The project testbench did not elaborate"
        );
        // Nenhuma mensagem cita um comando da CLI: a dica é de cada interface.
        for error in [&wave, &signal, &chosen, &elaborated] {
            assert!(!error.to_string().contains("aurora "), "{error}");
        }
    }
}
