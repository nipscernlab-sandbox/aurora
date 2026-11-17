//! O projeto SAPHO no disco, no mesmo formato da AURORA.
//!
//! ```text
//! <raiz>/
//!   <projeto>.spf              # JSON, ver o módulo spf
//!   <processador>/             # o que a pessoa escreve:
//!     <processador>.cmm | .cpp #   o fonte, que dá o nome ao processador
//!     input_N.txt, cade_K.txt  #   as entradas da simulação
//!     ...                      #   os arquivos de arrays, os cabeçalhos do C
//!     <processador>/           # a pasta de saída: tudo o que o YANC gera
//!       <processador>.asm, .v, _data.mif, _inst.mif, _tb.v
//!       prog_log.txt, src_lines.txt, trad_opcode.txt, app_log.txt,
//!       pc_<processador>_mem.txt, output_N.txt, toma_K.txt, a onda
//!   .aurora8/Temp/<processador>/  # o que só a AURORA usa (o .vvp, o modelo do
//!                              # Verilator, a cópia instrumentada do testbench)
//! ```
//!
//! É a estrutura do YANC 7.1, a de quem o usa no terminal (`cmmcomp
//! soma.cmm` grava em `./soma/`), e a da AURORA desde a 7.1. Até a 6.x cada
//! processador tinha `Software/`, `Hardware/` e `Simulation/`; um projeto
//! assim não compila mais e é convertido por [`Project::convert_layout`].
//!
//! A raiz do projeto é sempre o diretório do `.spf`; o `basePath` gravado no
//! arquivo é ignorado na leitura, como a AURORA faz ao abrir um projeto
//! copiado de outra máquina.
//!
//! # Compatibilidade com a AURORA
//!
//! Um projeto criado pela AURORA abre na AURORA e vice-versa. A AURORA lê o
//! `.spf` com a mesma tolerância (comentários, vírgula sobrando, BOM), grava
//! de forma atômica com indentação de 2 espaços, e preserva os campos que não
//! entende (`commandOverrides`, `metadata.lastOpened`, ...) e a ordem das
//! chaves. Ao contrário da AURORA, abrir um projeto não o regrava.
//!
//! # Exemplo
//!
//! ```
//! use aurora_core::{Language, NewProcessor, Project};
//! # let tmp = tempfile::tempdir()?;
//! # let dir = camino::Utf8Path::from_path(tmp.path()).unwrap();
//!
//! let mut project = Project::create(dir, "demo")?;
//! let soma = project.add_processor(&NewProcessor::new("soma", Language::Cmm))?;
//! assert!(soma.source.ends_with("soma/soma.cmm"));
//! assert!(soma.output_dir.ends_with("soma/soma"));
//!
//! let reopened = Project::open(dir.join("demo"))?;
//! assert_eq!(reopened.processors().len(), 1);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::{AuroraError, Result};
use crate::files::{FileRole, ProjectFile};
use crate::{paths, spf};

/// Frequência padrão quando o `.spf` não traz `clk` (a da AURORA).
pub const DEFAULT_FREQUENCY_MHZ: u32 = 100;
/// Número de clocks padrão quando o `.spf` não traz `numClocks` (o da AURORA).
pub const DEFAULT_CLOCKS: u32 = 2000;

/// O `#NUGAIN` que o YANC assume sem a diretiva.
const DEFAULT_NUGAIN: u32 = 128;

/// Maior `numClocks`: o testbench gerado conta os clocks num inteiro de 32
/// bits com sinal, e o `asmcomp` recusa acima disso.
pub const MAX_CLOCKS: u32 = i32::MAX as u32;

/// Maior frequência, em MHz. O testbench gerado usa `timescale 1ns/1ps`:
/// acima de 500000 MHz o meio período fica abaixo de 1 ps, vira 0, e o
/// Icarus recusa o `always` sem atraso.
pub const MAX_FREQUENCY_MHZ: u32 = 500_000;

/// Mais portas de entrada ou de saída que um processador novo pode ter. É
/// um teto da AURORA, folgado para os projetos do laboratório: o `asmcomp`
/// estoura um buffer e gera testbench corrompido muito antes do limite do
/// `int` (com 100000 portas, 904 erros de sintaxe).
pub const MAX_PORTS: u32 = 256;

/// Maior nome de projeto. O nome aparece duas vezes em cada caminho
/// (`<nome>/<nome>.spf`), e o `.spf` é gravado por um temporário com sufixo:
/// com 248 caracteres, o nome do temporário passava do limite de 255 do
/// sistema de arquivos.
pub const MAX_PROJECT_NAME: usize = 64;

/// Maior nome de processador. O `cmmcomp` guarda o nome em buffers de 100 a
/// 128 bytes e morre com sinal 11 acima de 97 caracteres; a AURORA deixa
/// folga.
pub const MAX_PROCESSOR_NAME: usize = 64;

/// A pasta do projeto que é só nossa: intermediários (`Temp/`), relatórios,
/// projetos de placa e a trava do `.spf`. O nome tem o 8 porque a `.aurora`
/// é da AURORA 7, que pode abrir o mesmo projeto; até a renomeação, era a
/// `.lace`.
pub const PROJECT_DIR: &str = ".aurora8";

/// A pasta que o Lace usava no lugar da [`PROJECT_DIR`]. Nunca a `.aurora`,
/// que é da AURORA 7 e não se toca.
const LACE_PROJECT_DIR: &str = ".lace";

/// Leva a `.lace` de um projeto do Lace para a [`PROJECT_DIR`], quando ela
/// ainda não existe, com o que tinha dentro (relatórios, placas). É melhor
/// esforço: se o renomear falhar (no Windows, um arquivo aberto lá dentro),
/// a `.lace` fica, e a pasta nova nasce vazia no primeiro uso.
pub(crate) fn migrate_project_dir(root: &Utf8Path) {
    let old = root.join(LACE_PROJECT_DIR);
    let new = root.join(PROJECT_DIR);
    if old.is_dir() && !new.exists() {
        let _ = std::fs::rename(&old, &new);
    }
}

/// Linguagem do programa de um processador. Em JSON: `"cmm"` ou `"cpp"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Language {
    /// C±, compilado pelo `cmmcomp`.
    Cmm,
    /// C, compilado pelo `cppcomp`. A AURORA e o YANC chamam de "cpp" e
    /// usam a extensão `.cpp`.
    Cpp,
}

impl Language {
    /// A extensão do fonte, sem ponto: `"cmm"` ou `"cpp"`.
    pub fn extension(self) -> &'static str {
        match self {
            Language::Cmm => "cmm",
            Language::Cpp => "cpp",
        }
    }

    pub(crate) fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "cmm" => Some(Language::Cmm),
            "cpp" => Some(Language::Cpp),
            _ => None,
        }
    }
}

/// Um processador do projeto, com todos os caminhos já absolutos.
///
/// Os valores de simulação (`frequency_mhz`, `clocks`, `show_arrays`) vêm do
/// `.spf` (`clk`, `numClocks`, `showArrays`) ou dos padrões da AURORA
/// ([`DEFAULT_FREQUENCY_MHZ`], [`DEFAULT_CLOCKS`], `false`). Para mudá-los
/// só num build, use [`BuildOptions`](crate::BuildOptions).
///
/// A linguagem é resolvida como na AURORA: `language` declarada no `.spf`;
/// senão a extensão de `sourceFile`/`cmmFile`; senão o arquivo que existir na
/// pasta do processador (`<nome>.cmm` antes de `<nome>.cpp`); senão C±.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct Processor {
    /// O nome: a pasta, o nome do fonte, o módulo Verilog e os arquivos
    /// gerados.
    pub name: String,
    /// A linguagem do fonte.
    pub language: Language,
    /// `<raiz>/<nome>`: o fonte e os arquivos que a pessoa escreve (os que
    /// inicializam arrays, `input_<n>.txt`, `cade_<n>.txt`, os cabeçalhos
    /// do C).
    #[schemars(with = "String")]
    pub dir: Utf8PathBuf,
    /// O programa-fonte, `<dir>/<nome>.cmm` ou `.cpp`.
    #[schemars(with = "String")]
    pub source: Utf8PathBuf,
    /// `<dir>/<nome>`, a pasta de saída do YANC (`-o`): o `.asm`, o Verilog,
    /// as memórias, o testbench, as tabelas da onda e as saídas da
    /// simulação, que roda nela.
    #[schemars(with = "String")]
    pub output_dir: Utf8PathBuf,
    /// `<raiz>/.aurora8/Temp/<nome>`: o que só a AURORA usa (o `.vvp`, o modelo do
    /// Verilator, a cópia instrumentada do testbench, a trava de execução).
    #[schemars(with = "String")]
    pub temp_dir: Utf8PathBuf,
    /// Frequência de operação em MHz (`-f` do `asmcomp`).
    pub frequency_mhz: u32,
    /// Clocks a simular (`-c` do `asmcomp`, vai para o testbench).
    pub clocks: u32,
    /// Exporta cada elemento de array como um sinal da onda
    /// (`--wave-arrays` do `cmmcomp`).
    pub show_arrays: bool,
}

impl Processor {
    /// `<saída>/<nome>.asm`, o que o compilador gera e o `asmcomp` lê.
    pub fn assembly_path(&self) -> Utf8PathBuf {
        self.output_dir.join(format!("{}.asm", self.name))
    }

    /// `<saída>/<nome>.v`, o processador sintetizável.
    pub fn verilog_path(&self) -> Utf8PathBuf {
        self.output_dir.join(format!("{}.v", self.name))
    }

    /// `<saída>/<nome>_tb.v`, o testbench que o `asmcomp` gera. Ele cita os
    /// arquivos pelo caminho relativo à pasta de saída, e só roda com ela
    /// como diretório de trabalho.
    pub fn testbench_path(&self) -> Utf8PathBuf {
        self.output_dir.join(format!("{}_tb.v", self.name))
    }

    /// Os arquivos que o Verilog gerado lê pelo nome, relativo ao diretório
    /// de trabalho de quem o roda: as memórias (`<nome>_data.mif`,
    /// `<nome>_inst.mif`) e a tabela do PC (`pc_<nome>_mem.txt`). Uma
    /// simulação, síntese ou compilação para a placa fora da pasta de saída
    /// precisa deles copiados para o diretório dela.
    pub fn files_read_by_verilog(&self) -> [Utf8PathBuf; 3] {
        let n = &self.name;
        [
            self.output_dir.join(format!("{n}_data.mif")),
            self.output_dir.join(format!("{n}_inst.mif")),
            self.output_dir.join(format!("pc_{n}_mem.txt")),
        ]
    }

    /// Já foi compilado: o Verilog e o testbench gerados existem no disco.
    /// Não diz se estão atualizados em relação ao fonte.
    pub fn is_built(&self) -> bool {
        self.verilog_path().is_file() && self.testbench_path().is_file()
    }

    /// As pastas da estrutura de antes do YANC 7.1 (`Software/`,
    /// `Hardware/`, `Simulation/`) que este processador ainda tem, com algo
    /// que a conversão moveria. Vazio na estrutura atual. O nome é o da 6.x,
    /// com as maiúsculas: no Windows e no macOS, uma `simulation/` da pessoa
    /// também abriria como `Simulation/`. O que o sistema deixa sozinho numa
    /// pasta (um `.DS_Store`) não conta. Ver [`Project::convert_layout`].
    pub fn legacy_dirs(&self) -> Vec<Utf8PathBuf> {
        let Ok(entries) = self.dir.read_dir_utf8() else {
            return Vec::new();
        };
        let dirs: Vec<String> = entries
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.file_name().to_owned())
            .collect();
        crate::layout::LEGACY_DIRS
            .iter()
            .filter(|d| dirs.iter().any(|name| name == *d))
            .map(|d| self.dir.join(d))
            .filter(|d| crate::layout::has_content(d))
            .collect()
    }

    /// O processador só compila depois da conversão: uma pasta antiga tem o
    /// que mover ([`legacy_dirs`](Self::legacy_dirs)), e o fonte não está no
    /// lugar da 7.1. Com o fonte no lugar, o que sobrou numa pasta antiga (um
    /// arquivo que a conversão deixou porque o destino já existia) não impede
    /// o build.
    pub fn needs_conversion(&self) -> bool {
        !self.source.is_file() && !self.legacy_dirs().is_empty()
    }

    /// O Verilog gerado não acompanha o fonte: falta, ou algum arquivo de
    /// [`Processor::dir`] (o fonte, as tabelas que inicializam arrays, os
    /// cabeçalhos do C) é mais novo que ele. As subpastas ficam de fora: a
    /// de saída é a do próprio YANC.
    pub fn is_out_of_date(&self) -> bool {
        let modified = |m: std::fs::Metadata| m.modified().ok();
        let Some(built) = std::fs::metadata(self.verilog_path())
            .ok()
            .and_then(modified)
        else {
            return true;
        };
        let Ok(entries) = self.dir.read_dir_utf8() else {
            return false;
        };
        entries.flatten().any(|entry| {
            entry
                .metadata()
                .ok()
                .filter(std::fs::Metadata::is_file)
                .and_then(modified)
                .is_some_and(|time| time > built)
        })
    }

    /// Cria a pasta do processador, a de saída e a temporária, se faltarem.
    /// O YANC 7.1 cria a de saída sozinho, mas a AURORA confere antes que
    /// nenhuma delas é um arquivo.
    pub fn ensure_dirs(&self) -> Result<()> {
        for dir in [
            self.dir.clone(),
            self.output_dir.clone(),
            self.temp_dir.clone(),
        ] {
            if dir.exists() && !dir.is_dir() {
                return Err(AuroraError::InvalidProject {
                    path: dir,
                    reason: "Exists but is not a directory".into(),
                });
            }
            std::fs::create_dir_all(&dir).map_err(AuroraError::io("Creating directory", &dir))?;
        }
        Ok(())
    }
}

/// Parâmetros do cabeçalho de um processador novo.
///
/// Os padrões são os da AURORA (`js/project/processor_defaults.ts`): o float
/// estreito do SAPHO, 23 = 16 + 6 + 1 bits. No fluxo C só as portas vão para
/// o fonte; o resto fica com o que o `cppcomp` foi compilado para assumir.
///
/// Em JSON, só `name` e `language` são obrigatórios: um campo ausente fica
/// com o padrão de [`NewProcessor::new`], e, sem `nubits`, com `nbmant` ou
/// `nbexpo` dados, `nubits` vira `nbmant + nbexpo + 1`
/// ([`NewProcessor::nubits_for`]), a única largura que o `asmcomp` aceita.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct NewProcessor {
    /// Nome do processador: letras, dígitos e `_`, começando por letra ou `_`.
    pub name: String,
    /// Linguagem do fonte-modelo.
    pub language: Language,
    /// `#NUBITS`: largura da palavra de dados, em bits. Só C±.
    #[schemars(default = "default_nubits")]
    pub nubits: u32,
    /// `#NBMANT`: bits de mantissa do ponto flutuante do SAPHO (não é IEEE
    /// 754). Só C±.
    #[schemars(default = "default_nbmant")]
    pub nbmant: u32,
    /// `#NBEXPO`: bits de expoente do ponto flutuante. Só C±.
    #[schemars(default = "default_nbexpo")]
    pub nbexpo: u32,
    /// `#NUGAIN`: divisor de `norm(x)`. Precisa ser potência de dois, senão o
    /// `cmmcomp` recusa (um divisor genérico viraria o caminho crítico da
    /// ULA). Só C±; o fonte só traz a diretiva quando difere de 128, o
    /// padrão do YANC.
    #[schemars(default = "default_nugain")]
    pub nugain: u32,
    /// `#NDSTAC`: profundidade da pilha de dados. Só C±. `None`, o padrão:
    /// sem a diretiva, e o `asmcomp` calcula a profundidade pelo programa.
    #[schemars(default)]
    pub ndstac: Option<u32>,
    /// `#SDEPTH`: profundidade da pilha de instruções (chamadas). Só C±.
    /// `None`, o padrão: sem a diretiva, calculada pelo programa.
    #[schemars(default)]
    pub sdepth: Option<u32>,
    /// `#NUIOIN` / `#pragma yanc nuioin`: número de portas de entrada.
    #[schemars(default = "default_port")]
    pub input_ports: u32,
    /// `#NUIOOU` / `#pragma yanc nuioou`: número de portas de saída.
    #[schemars(default = "default_port")]
    pub output_ports: u32,
}

fn default_nubits() -> u32 {
    23
}
fn default_nbmant() -> u32 {
    16
}
fn default_nbexpo() -> u32 {
    6
}
fn default_nugain() -> u32 {
    DEFAULT_NUGAIN
}
fn default_port() -> u32 {
    1
}

/// Os campos de um [`NewProcessor`] como chegam de quem pede (o JSON do
/// protocolo, as opções do `aurora proc add`): só o nome e a linguagem são
/// obrigatórios. Um campo ausente fica com o padrão de [`NewProcessor::new`],
/// e, sem `nubits`, com `nbmant` ou `nbexpo` dados, `nubits` vira
/// `nbmant + nbexpo + 1` ([`NewProcessor::nubits_for`]): a regra mora no
/// `From` daqui, e quem monta um processador novo passa por ele.
#[derive(Debug, Clone, Deserialize)]
pub struct NewProcessorFields {
    /// Nome do processador.
    pub name: String,
    /// Linguagem do fonte-modelo.
    pub language: Language,
    /// `#NUBITS`.
    #[serde(default)]
    pub nubits: Option<u32>,
    /// `#NBMANT`.
    #[serde(default)]
    pub nbmant: Option<u32>,
    /// `#NBEXPO`.
    #[serde(default)]
    pub nbexpo: Option<u32>,
    /// `#NUGAIN`.
    #[serde(default)]
    pub nugain: Option<u32>,
    /// `#NDSTAC`; ausente, o `asmcomp` calcula pelo programa.
    #[serde(default)]
    pub ndstac: Option<u32>,
    /// `#SDEPTH`; ausente, calculada pelo programa.
    #[serde(default)]
    pub sdepth: Option<u32>,
    /// Portas de entrada.
    #[serde(default)]
    pub input_ports: Option<u32>,
    /// Portas de saída.
    #[serde(default)]
    pub output_ports: Option<u32>,
}

impl NewProcessorFields {
    /// Só o nome e a linguagem; o resto com o padrão.
    pub fn new(name: impl Into<String>, language: Language) -> Self {
        NewProcessorFields {
            name: name.into(),
            language,
            nubits: None,
            nbmant: None,
            nbexpo: None,
            nugain: None,
            ndstac: None,
            sdepth: None,
            input_ports: None,
            output_ports: None,
        }
    }
}

impl From<NewProcessorFields> for NewProcessor {
    fn from(fields: NewProcessorFields) -> Self {
        let mut spec = NewProcessor::new(fields.name, fields.language);
        if let Some(n) = fields.nbmant {
            spec.nbmant = n;
        }
        if let Some(n) = fields.nbexpo {
            spec.nbexpo = n;
        }
        spec.nubits = match fields.nubits {
            Some(n) => n,
            None if fields.nbmant.is_some() || fields.nbexpo.is_some() => {
                NewProcessor::nubits_for(spec.nbmant, spec.nbexpo)
            }
            None => spec.nubits,
        };
        if let Some(n) = fields.nugain {
            spec.nugain = n;
        }
        spec.ndstac = fields.ndstac;
        spec.sdepth = fields.sdepth;
        if let Some(n) = fields.input_ports {
            spec.input_ports = n;
        }
        if let Some(n) = fields.output_ports {
            spec.output_ports = n;
        }
        spec
    }
}

impl<'de> Deserialize<'de> for NewProcessor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        NewProcessorFields::deserialize(deserializer).map(NewProcessor::from)
    }
}

impl NewProcessor {
    /// Um processador com os padrões da AURORA: 23 bits de palavra, mantissa
    /// 16, expoente 6, ganho 128, as pilhas que o YANC calcular pelo
    /// programa, uma porta de entrada e uma de saída.
    ///
    /// ```
    /// use aurora_core::{Language, NewProcessor};
    ///
    /// let mut spec = NewProcessor::new("fft", Language::Cmm);
    /// spec.nubits = 32;
    /// spec.input_ports = 2;
    /// ```
    pub fn new(name: impl Into<String>, language: Language) -> Self {
        NewProcessor {
            name: name.into(),
            language,
            nubits: default_nubits(),
            nbmant: default_nbmant(),
            nbexpo: default_nbexpo(),
            nugain: default_nugain(),
            ndstac: None,
            sdepth: None,
            input_ports: default_port(),
            output_ports: default_port(),
        }
    }

    /// A largura da palavra que o `asmcomp` aceita para uma mantissa e um
    /// expoente: `nbmant + nbexpo + 1` (o bit de sinal), saturando. É o que
    /// uma interface usa quando o usuário dá a mantissa ou o expoente sem
    /// dar a largura.
    ///
    /// ```
    /// assert_eq!(aurora_core::NewProcessor::nubits_for(16, 6), 23);
    /// ```
    pub fn nubits_for(nbmant: u32, nbexpo: u32) -> u32 {
        nbmant.saturating_add(nbexpo).saturating_add(1)
    }

    /// Confere os parâmetros contra o que o YANC compila, a regra de
    /// [`Project::add_processor`]:
    ///
    /// - portas de entrada e de saída: de 0 a [`MAX_PORTS`];
    /// - só em C±: `#NBEXPO` de 2 a 8 (acima de 8 o `cmmcomp` converte
    ///   constantes com o `float` da máquina e perde a faixa), `#NBMANT` de
    ///   pelo menos 2, `#NUBITS` até 32 e igual a `#NBMANT + #NBEXPO + 1`
    ///   (o `asmcomp` exige), `#NUGAIN` potência de dois, `#NDSTAC` e
    ///   `#SDEPTH`, quando dados, de pelo menos 1 (pilha vazia passa no build
    ///   e quebra a simulação).
    ///
    /// ```
    /// use aurora_core::{Language, NewProcessor};
    ///
    /// let mut spec = NewProcessor::new("fft", Language::Cmm);
    /// assert!(spec.validate().is_ok());
    /// spec.nubits = 32;
    /// assert!(spec.validate().is_err(), "32 != 16 + 6 + 1");
    /// ```
    ///
    /// # Erros
    ///
    /// [`AuroraError::InvalidParameter`] no primeiro parâmetro recusado.
    pub fn validate(&self) -> Result<()> {
        let bad = |name: &str, value: u32, reason: String| {
            Err(AuroraError::InvalidParameter {
                name: name.to_owned(),
                value: value.to_string(),
                reason,
            })
        };
        for (name, value) in [
            ("#NUIOIN", self.input_ports),
            ("#NUIOOU", self.output_ports),
        ] {
            if value > MAX_PORTS {
                return bad(name, value, format!("at most {MAX_PORTS} ports"));
            }
        }
        if self.language == Language::Cpp {
            return Ok(());
        }
        if !(2..=8).contains(&self.nbexpo) {
            return bad(
                "#NBEXPO",
                self.nbexpo,
                "must be between 2 and 8 bits".into(),
            );
        }
        if self.nbmant < 2 {
            return bad("#NBMANT", self.nbmant, "must be at least 2 bits".into());
        }
        let width = self.nbmant.saturating_add(self.nbexpo).saturating_add(1);
        if width > 32 {
            return bad(
                "#NBMANT",
                self.nbmant,
                format!(
                    "#NBMANT + #NBEXPO + 1 is {width}, and the word (#NUBITS) has at most 32 bits"
                ),
            );
        }
        if self.nubits != width {
            return bad(
                "#NUBITS",
                self.nubits,
                format!(
                    "must be #NBMANT + #NBEXPO + 1 = {} + {} + 1 = {width}",
                    self.nbmant, self.nbexpo
                ),
            );
        }
        if !self.nugain.is_power_of_two() {
            return bad(
                "#NUGAIN",
                self.nugain,
                "must be a power of two (1, 2, 4, 8, ...)".into(),
            );
        }
        for (name, value) in [("#NDSTAC", self.ndstac), ("#SDEPTH", self.sdepth)] {
            if value == Some(0) {
                return bad(name, 0, "a stack needs at least 1 position".into());
            }
        }
        Ok(())
    }

    /// O fonte-modelo. Sem `#PRNAME` nem `#pragma yanc prname`: no YANC 7.1
    /// o processador se chama como o arquivo, e as duas diretivas são erro.
    /// As diretivas que têm padrão no YANC só entram com outro valor: as
    /// pilhas quando escolhidas, o `#NUGAIN` quando difere de 128.
    fn source_text(&self) -> String {
        match self.language {
            // O `cmmcomp` recusa função de corpo vazio (só comentário também
            // conta como vazio): o modelo traz uma instrução. `out(0, 0)` só
            // compila se houver porta de saída.
            Language::Cmm => {
                let optional = |directive: &str, value: Option<u32>| {
                    value.map_or_else(String::new, |v| format!("{directive} {v}\n"))
                };
                format!(
                    "#NUBITS {}\n{}{}#NUIOIN {}\n#NUIOOU {}\n#NBMANT {}\n#NBEXPO {}\n{}\n\
                     void main()\n{{\n{}}}\n",
                    self.nubits,
                    optional("#NDSTAC", self.ndstac),
                    optional("#SDEPTH", self.sdepth),
                    self.input_ports,
                    self.output_ports,
                    self.nbmant,
                    self.nbexpo,
                    optional(
                        "#NUGAIN",
                        Some(self.nugain).filter(|&g| g != DEFAULT_NUGAIN)
                    ),
                    if self.output_ports > 0 {
                        "    // out(porta, valor) escreve numa porta de saída\n    out(0, 0);\n"
                    } else {
                        "    int x = 0;\n"
                    },
                )
            }
            Language::Cpp => format!(
                "#pragma yanc nuioin {}\n#pragma yanc nuioou {}\n\nvoid main(void)\n{{\n}}\n",
                self.input_ports, self.output_ports,
            ),
        }
    }
}

/// Um projeto aberto: o `.spf` em memória e seus processadores.
///
/// Os métodos que alteram o projeto (`add_processor`, `add_file`,
/// `set_top_level`, ...) gravam o `.spf` na hora; não há "salvar". Cada um
/// trava `.aurora8/spf.lock`, relê o `.spf` do disco, aplica a mudança e grava:
/// duas gravações ao mesmo tempo (a AURORA IDE e a CLI no terminal, dois
/// `aurora add`) entram uma depois da outra, e nenhuma perde a da outra. Para
/// ler o que outro programa mudou sem gravar, abra de novo: o `Project` só
/// relê ao gravar.
#[derive(Debug, Clone, Serialize)]
pub struct Project {
    name: String,
    pub(crate) root: Utf8PathBuf,
    spf_path: Utf8PathBuf,
    processors: Vec<Processor>,
    /// O `.spf` inteiro, para regravar sem perder o que a AURORA não entende.
    #[serde(skip)]
    pub(crate) document: Value,
    /// A trava de gravação, de [`Project::begin_write`] até o `save`.
    #[serde(skip)]
    write_lock: Option<std::sync::Arc<std::fs::File>>,
}

impl Project {
    /// Abre um projeto a partir do `.spf` ou do diretório que o contém
    /// (`<dir>/<nome-do-dir>.spf`, ou o único `.spf` do diretório).
    ///
    /// Não regrava o `.spf`, ao contrário da AURORA 7. A única escrita é a
    /// da pasta `.lace` de um projeto do Lace, que vira a `.aurora8`, com o
    /// que tinha dentro. Os diretórios dos processadores não
    /// precisam existir para abrir; eles são criados no primeiro
    /// [`build`](crate::build).
    ///
    /// # Erros
    ///
    /// - [`AuroraError::InvalidProject`] se o caminho não existir, ou se o
    ///   diretório não tiver `.spf` ou tiver mais de um;
    /// - [`AuroraError::InvalidProjectFile`] se o `.spf` não for JSON mesmo com
    ///   a leitura tolerante, se não tiver a seção `structure`, ou se um
    ///   processador tiver `language` desconhecida ou `clk`/`numClocks` que
    ///   não seja inteiro positivo;
    /// - [`AuroraError::Io`] se o arquivo não puder ser lido.
    pub fn open(path: impl AsRef<Utf8Path>) -> Result<Self> {
        // No Windows, o `.spf` que outra gravação troca neste instante
        // (`spf::write`, por rename) pode sumir ou negar acesso por alguns
        // milissegundos: o Studio e a CLI abrindo e gravando o mesmo projeto.
        let mut attempt = 0;
        loop {
            match Self::open_once(path.as_ref()) {
                Err(error) if cfg!(windows) && attempt < 20 && is_transient(&error) => {
                    attempt += 1;
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                other => return other,
            }
        }
    }

    fn open_once(path: &Utf8Path) -> Result<Self> {
        let spf_path = locate_spf(path)?;
        // A pasta canonicalizada, e não o arquivo: no Windows, canonicalizar
        // o `.spf` bem na troca devolve o caminho do arquivo apagado
        // (`C:\$Extend\$Deleted\...`).
        let name = spf_path.file_name().unwrap_or_default().to_owned();
        let parent = match spf_path.parent() {
            Some(p) if !p.as_str().is_empty() => p.to_owned(),
            _ => Utf8PathBuf::from("."),
        };
        let root = paths::canonicalize(&parent)?;
        migrate_project_dir(&root);
        let spf_path = root.join(name);
        let text = std::fs::read_to_string(&spf_path)
            .map_err(AuroraError::io("Reading project", &spf_path))?;
        let document = spf::parse(&spf_path, &text)?;
        Self::from_document(spf_path, document)
    }

    /// Acha o projeto que contém `start`: o `.spf` de `start` ou do primeiro
    /// diretório acima dele que tenha um, como o git acha o repositório.
    /// Assim um comando funciona de qualquer pasta do projeto: da pasta de um
    /// processador, da pasta de saída dele, de uma pasta de fontes.
    ///
    /// `start` pode ser o próprio `.spf`, que é aberto direto, ou um arquivo
    /// do projeto, e aí a busca começa na pasta dele. Em cada pasta vale a
    /// regra de [`Project::open`]: `<pasta>/<nome-da-pasta>.spf`, ou o único
    /// `.spf` dela.
    ///
    /// # Erros
    ///
    /// - [`AuroraError::ProjectNotFound`] se nenhuma pasta, de `start` até a
    ///   raiz do sistema, tem `.spf`;
    /// - [`AuroraError::InvalidProject`] se `start` não existe, ou se a
    ///   primeira pasta com `.spf` tem mais de um;
    /// - os de [`Project::open`] para o `.spf` achado.
    pub fn discover(start: impl AsRef<Utf8Path>) -> Result<Self> {
        let start = start.as_ref();
        if start.is_file() && start.extension() == Some("spf") {
            return Self::open(start);
        }
        if !start.exists() {
            return Err(AuroraError::InvalidProject {
                path: start.to_owned(),
                reason: "Does not exist".into(),
            });
        }
        let start = paths::canonicalize(start)?;
        let first = if start.is_file() {
            start.parent().unwrap_or(&start).to_owned()
        } else {
            start
        };
        for dir in first.ancestors() {
            if let Some(spf) = spf_in(dir)? {
                return Self::open(spf);
            }
        }
        Err(AuroraError::ProjectNotFound(first))
    }

    /// O processador cuja pasta contém `path` (a pasta dele, com a de saída
    /// dentro, ou a temporária dele em `.aurora8/Temp/`). `None` fora de todas.
    pub fn processor_at(&self, path: impl AsRef<Utf8Path>) -> Option<&Processor> {
        let path = path.as_ref();
        let path = paths::canonicalize(path).unwrap_or_else(|_| path.to_owned());
        // Pelo caminho real também: a pasta do processador pode ser um link.
        self.processors.iter().find(|p| {
            [&p.dir, &p.temp_dir]
                .into_iter()
                .any(|dir| path.starts_with(dir) || path.starts_with(paths::real_path(dir)))
        })
    }

    /// O que está estranho no `.spf` mas não impede de abrir: um caminho de
    /// outra máquina achado pela cauda, um `topLevelFile` ou `testbenchFile`
    /// fora da lista (ignorado), um topo com nome de testbench (ignorado), um
    /// processador com nome que o YANC não compila, um processador na
    /// estrutura de pastas de antes do YANC 7.1. As interfaces mostram
    /// como aviso ao abrir (`aurora status`, painel de problemas do Studio).
    ///
    /// Só lê; o que dá para consertar sozinho (o caminho achado) é consertado
    /// na próxima gravação.
    pub fn issues(&self) -> Vec<ProjectIssue> {
        let mut issues = self.file_issues();
        for processor in &self.processors {
            let legacy = processor.legacy_dirs();
            if !legacy.is_empty() {
                let message = if processor.needs_conversion() {
                    format!(
                        "Processor '{}' still has the folder layout of YANC 6.x (Software/, Hardware/, Simulation/) and does not build until the project is converted",
                        processor.name
                    )
                } else {
                    let names: Vec<&str> = legacy.iter().filter_map(|d| d.file_name()).collect();
                    format!(
                        "Processor '{}' still has files in folders of the YANC 6.x layout ({}); it builds from {}, and converting the project moves what can still be moved",
                        processor.name,
                        names.join(", "),
                        processor.source.file_name().unwrap_or_default()
                    )
                };
                issues.push(ProjectIssue::new(
                    IssueKind::LegacyLayout,
                    Some(processor.dir.clone()),
                    Some(processor.name.clone()),
                    message,
                ));
            }
            if let Err(AuroraError::InvalidName { reason, .. }) =
                validate_processor_name(&processor.name)
            {
                issues.push(ProjectIssue::new(
                    IssueKind::InvalidProcessorName,
                    Some(processor.dir.clone()),
                    Some(processor.name.clone()),
                    format!("Processor '{}' cannot be built: {reason}", processor.name),
                ));
            }
        }
        // Outro projeto numa pasta acima: o `status` dele lista os arquivos
        // deste como não registrados, e um arquivo pode acabar nos dois.
        if let Some(outer) = self
            .root
            .ancestors()
            .skip(1)
            .find_map(|dir| spf_in(dir).ok().flatten())
        {
            issues.push(ProjectIssue::new(
                IssueKind::NestedProject,
                Some(outer.clone()),
                None,
                format!("This project is inside the folder of another one ({outer}); files here also show up there"),
            ));
        }
        issues
    }

    fn from_document(spf_path: Utf8PathBuf, document: Value) -> Result<Self> {
        let root = spf_path
            .parent()
            .expect("a canonicalized file has a parent directory")
            .to_owned();
        let name = spf_path.file_stem().unwrap_or_default().to_owned();

        let entries = spf::processors(&spf_path, &document)?;
        let mut seen: Vec<&str> = Vec::new();
        for (i, entry) in entries.iter().enumerate() {
            let name = entry.name.as_str();
            let reason = if name.trim().is_empty() {
                Some("the name is empty".to_owned())
            } else if !is_single_folder_name(name) {
                Some(format!("'{name}' is not a folder name inside the project"))
            } else if seen.iter().any(|s| s.eq_ignore_ascii_case(name)) {
                Some(format!("'{name}' appears more than once"))
            } else {
                None
            };
            if let Some(reason) = reason {
                return Err(spf::invalid(
                    &spf_path,
                    format!("structure.processors[{i}]: {reason}"),
                ));
            }
            seen.push(name);
        }
        let processors = entries
            .into_iter()
            .map(|entry| processor_from_entry(&spf_path, &root, entry))
            .collect::<Result<Vec<_>>>()?;

        Ok(Project {
            name,
            root,
            spf_path,
            processors,
            document,
            write_lock: None,
        })
    }

    /// Cria `<pai>/<nome>/<nome>.spf`, sem processadores, no formato da
    /// AURORA (`metadata` e `structure` completos). Cria `<pai>/<nome>` se
    /// preciso; `<pai>` precisa existir.
    ///
    /// # Erros
    ///
    /// - [`AuroraError::InvalidName`] se o nome não passar em
    ///   [`validate_project_name`] (espaço, acento, caractere especial,
    ///   começo que não é letra, nome reservado do Windows);
    /// - [`AuroraError::ProjectExists`] se o `.spf` já existir (nada é
    ///   alterado);
    /// - [`AuroraError::Io`] se `<pai>` não existir ou não puder ser escrito.
    pub fn create(parent: impl AsRef<Utf8Path>, name: &str) -> Result<Self> {
        validate_project_name(name)?;
        let parent = paths::canonicalize(parent.as_ref())?;
        let root = parent.join(name);
        let spf_path = root.join(format!("{name}.spf"));
        // `Ok1` ao lado de `ok1`: duas pastas no Linux, a mesma no Windows e
        // no macOS. Antes de conferir o `.spf`: no macOS e no Windows,
        // `Ok1/Ok1.spf` acharia o de `ok1`, e o erro diria outra coisa em
        // cada sistema.
        if let Ok(entries) = parent.read_dir_utf8()
            && let Some(other) = entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_owned())
                .find(|other| other != name && other.eq_ignore_ascii_case(name))
        {
            return Err(AuroraError::InvalidName {
                name: name.to_owned(),
                reason: format!(
                    "'{other}' already exists here, and Windows and macOS do not tell the two names apart"
                ),
            });
        }
        if spf_path.exists() {
            return Err(AuroraError::ProjectExists(spf_path));
        }
        let created = !root.exists();
        std::fs::create_dir_all(&root).map_err(AuroraError::io("Creating project", &root))?;

        let now = iso8601_now();
        let document = json!({
            "metadata": {
                "projectName": name,
                "createdAt": now,
                "lastModified": now,
                "computerName": computer_name(),
                "appVersion": concat!("aurora ", env!("CARGO_PKG_VERSION")),
                "projectPath": root,
            },
            "structure": {
                "basePath": root,
                "processors": [],
                "folders": [],
                "topLevelFile": "",
                "testbenchFile": "",
                "synthesizableFiles": [],
                "testbenchFiles": [],
            },
        });
        if let Err(error) = spf::write(&spf_path, &document) {
            // Nada fica para trás: a pasta só sai se foi criada agora e
            // continua vazia.
            if created {
                let _ = std::fs::remove_dir(&root);
            }
            return Err(error);
        }
        tracing::info!(%spf_path, "Project created");
        Self::from_document(spf_path, document)
    }

    /// Cria um processador: a pasta `<raiz>/<nome>/`, o fonte
    /// `<nome>.cmm` (ou `.cpp`) dentro dela, a partir do modelo, e a entrada
    /// no `.spf` (`{"name"}` em C±, `{"name", "language": "cpp"}` em C).
    /// Grava o `.spf`. A pasta de saída, `<nome>/<nome>/`, nasce no primeiro
    /// [`build`](crate::build), como a AURORA faz.
    ///
    /// O fonte-modelo em C± tem as diretivas do cabeçalho (`#NUBITS`,
    /// `#NBMANT`, ...; as pilhas e o `#NUGAIN` só com valor escolhido) e um
    /// `main`; em C, os pragmas das portas. Não tem `#PRNAME`: o processador
    /// se chama como o arquivo.
    ///
    /// # Erros
    ///
    /// - [`AuroraError::InvalidName`] se o nome não passar em
    ///   [`validate_new_processor_name`]: não é um identificador (a AURORA
    ///   aceita `-`, mas o lexer do `cmmcomp` não), passa de
    ///   [`MAX_PROCESSOR_NAME`] caracteres, é palavra do C± ou do Verilog,
    ///   módulo da biblioteca SAPHO, ou nome de uma pasta da AURORA;
    /// - [`AuroraError::InvalidParameter`] se um parâmetro não passar em
    ///   [`NewProcessor::validate`];
    /// - [`AuroraError::ProcessorExists`] se o nome já estiver no projeto,
    ///   inclusive com outra caixa (`Soma` e `soma` colidem no Windows e no
    ///   macOS);
    /// - [`AuroraError::InvalidProject`] se o fonte já existir no disco: o
    ///   AURORA nunca sobrescreve código do usuário.
    pub fn add_processor(&mut self, spec: &NewProcessor) -> Result<&Processor> {
        self.begin_write()?;
        validate_new_processor_name(&spec.name)?;
        spec.validate()?;
        // Windows e macOS não distinguem maiúsculas em nome de pasta: `Soma`
        // e `soma` iriam para o mesmo diretório.
        if let Some(existing) = self
            .processors
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(&spec.name))
        {
            return Err(AuroraError::ProcessorExists(existing.name.clone()));
        }

        let processor = self.processor_paths(&spec.name, spec.language, None);
        if processor.source.exists() {
            return Err(AuroraError::InvalidProject {
                path: processor.source,
                reason: "The source already exists; AURORA does not overwrite code".into(),
            });
        }
        if processor.dir.exists() && !processor.dir.is_dir() {
            return Err(AuroraError::InvalidProject {
                path: processor.dir,
                reason: "Exists but is not a directory".into(),
            });
        }
        std::fs::create_dir_all(&processor.dir)
            .map_err(AuroraError::io("Creating directory", &processor.dir))?;
        std::fs::write(&processor.source, spec.source_text())
            .map_err(AuroraError::io("Creating source", &processor.source))?;

        let mut entry = json!({ "name": spec.name });
        if spec.language == Language::Cpp {
            entry["language"] = "cpp".into();
        }
        let structure = &mut self.document["structure"];
        if !structure.get("processors").is_some_and(Value::is_array) {
            structure["processors"] = json!([]);
        }
        structure["processors"]
            .as_array_mut()
            .expect("acabou de virar array")
            .push(entry);
        self.save()?;

        tracing::info!(name = %spec.name, "Processor created");
        self.processors.push(processor);
        Ok(self.processors.last().expect("acabou de entrar"))
    }

    /// Regrava o `.spf`, alinhando `basePath` e `projectPath` à raiz atual e
    /// atualizando `lastModified`, como o escritor da AURORA.
    /// Antes de mudar o projeto: trava `.aurora8/spf.lock` (espera quem estiver
    /// gravando, de outro processo ou de outra thread) e relê o `.spf` do
    /// disco, para a mudança cair sobre a versão mais nova. A trava fica até
    /// o próximo `save`. Chamado no começo de cada método que grava, antes de
    /// ler o estado.
    pub(crate) fn begin_write(&mut self) -> Result<()> {
        if self.write_lock.is_some() {
            return Ok(());
        }
        let dir = self.root.join(".aurora8");
        std::fs::create_dir_all(&dir).map_err(AuroraError::io("Creating directory", &dir))?;
        let path = dir.join("spf.lock");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(AuroraError::io("Locking project", &path))?;
        file.lock()
            .map_err(AuroraError::io("Locking project", &path))?;
        let fresh = Self::open(&self.spf_path)?;
        self.document = fresh.document;
        self.processors = fresh.processors;
        self.write_lock = Some(std::sync::Arc::new(file));
        Ok(())
    }

    pub(crate) fn save(&mut self) -> Result<()> {
        let result = self.write_document();
        // Solta a trava de `begin_write`, gravando ou não.
        self.write_lock = None;
        result
    }

    /// Solta a trava de `begin_write` sem gravar: a mudança não aconteceu.
    pub(crate) fn release_write(&mut self) {
        self.write_lock = None;
    }

    fn write_document(&mut self) -> Result<()> {
        self.repair_rescued();
        self.repair_duplicates();
        let root = Value::from(self.root.as_str());
        if let Some(meta) = self
            .document
            .get_mut("metadata")
            .and_then(Value::as_object_mut)
        {
            meta.insert("projectPath".into(), root.clone());
            meta.insert("lastModified".into(), iso8601_now().into());
        }
        self.document["structure"]["basePath"] = root;
        spf::write(&self.spf_path, &self.document)
    }

    /// O nome do projeto: o nome do `.spf` sem a extensão.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// A raiz: o diretório do `.spf`, canonicalizado.
    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    /// O `.spf`, canonicalizado.
    pub fn spf_path(&self) -> &Utf8Path {
        &self.spf_path
    }

    /// Intermediários do projeto inteiro (`<raiz>/.aurora8/Temp`): o `.vvp` e o
    /// `obj_dir` da simulação do projeto, a síntese, os esquemáticos.
    pub fn temp_dir(&self) -> Utf8PathBuf {
        self.root.join(".aurora8").join("Temp")
    }

    /// Os processadores, na ordem do `.spf`.
    pub fn processors(&self) -> &[Processor] {
        &self.processors
    }

    /// Os processadores que têm o fonte no disco, na ordem do `.spf`: os que o
    /// botão Wave da AURORA compila antes de simular o projeto. Um processador
    /// registrado sem fonte fica de fora em vez de fazer o build falhar. Um
    /// na estrutura antiga entra ([`Processor::needs_conversion`]): o build
    /// dele diz que falta a conversão ([`AuroraError::LegacyLayout`]), em vez de
    /// a simulação seguir com o Verilog velho de `Hardware/`.
    pub fn buildable_processors(&self) -> Vec<&Processor> {
        self.processors
            .iter()
            .filter(|p| p.source.is_file() || p.needs_conversion())
            .collect()
    }

    /// O processador cujo build gera `path`: o Verilog (`<nome>/<nome>/<nome>.v`)
    /// ou o testbench (`<nome>/<nome>/<nome>_tb.v`). Um desses registrado e
    /// ausente quer dizer processador ainda não compilado.
    pub(crate) fn generated_by(&self, path: &Utf8Path) -> Option<&Processor> {
        self.processors.iter().find(|p| {
            paths::same_file(path, &p.verilog_path()) || paths::same_file(path, &p.testbench_path())
        })
    }

    /// O processador com esse nome, se houver.
    pub fn processor(&self, name: &str) -> Option<&Processor> {
        self.processors.iter().find(|p| p.name == name)
    }

    /// Como [`Project::processor`], mas com um erro que lista os disponíveis.
    pub fn require_processor(&self, name: &str) -> Result<&Processor> {
        self.processor(name)
            .ok_or_else(|| AuroraError::ProcessorNotFound {
                name: name.to_owned(),
                available: self.processors.iter().map(|p| p.name.clone()).collect(),
            })
    }

    /// Muda os parâmetros de simulação de um processador e grava no `.spf`
    /// (`clk`, `numClocks`, `showArrays`, os mesmos campos que o painel de
    /// configuração da AURORA escreve). Campos `None` ficam como estão.
    ///
    /// Os novos valores valem a partir do próximo [`build`](crate::build): o
    /// testbench gerado embute frequência e clocks.
    ///
    /// ```
    /// use aurora_core::{Language, NewProcessor, Project, ProcessorConfig};
    /// # let tmp = tempfile::tempdir()?;
    /// # let dir = camino::Utf8Path::from_path(tmp.path()).unwrap();
    /// let mut project = Project::create(dir, "p")?;
    /// project.add_processor(&NewProcessor::new("soma", Language::Cmm))?;
    ///
    /// let mut config = ProcessorConfig::default();
    /// config.frequency_mhz = Some(50);
    /// let soma = project.configure_processor("soma", &config)?;
    /// assert_eq!((soma.frequency_mhz, soma.clocks), (50, 2000));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Erros
    ///
    /// - [`AuroraError::ProcessorNotFound`] se o processador não existir;
    /// - [`AuroraError::InvalidParameter`] se `frequency_mhz` não estiver entre
    ///   1 e [`MAX_FREQUENCY_MHZ`], ou `clocks` entre 1 e [`MAX_CLOCKS`].
    ///   Nada é gravado.
    pub fn configure_processor(
        &mut self,
        name: &str,
        config: &ProcessorConfig,
    ) -> Result<&Processor> {
        self.begin_write()?;
        let index = self
            .processors
            .iter()
            .position(|p| p.name == name)
            .ok_or_else(|| AuroraError::ProcessorNotFound {
                name: name.to_owned(),
                available: self.processors.iter().map(|p| p.name.clone()).collect(),
            })?;
        for (field, value) in [
            ("frequency", config.frequency_mhz),
            ("clocks", config.clocks),
        ] {
            if let Some(value) = value
                && let Some(reason) = simulation_range(field, value)
            {
                return Err(AuroraError::InvalidParameter {
                    name: field.to_owned(),
                    value: value.to_string(),
                    reason: reason.to_owned(),
                });
            }
        }

        let list = self.document["structure"]["processors"]
            .as_array_mut()
            .expect("o processador veio desta lista");
        let entry = list
            .iter_mut()
            .find(|e| e.as_str() == Some(name) || e["name"].as_str() == Some(name))
            .expect("o processador veio desta lista");
        // Entrada no formato antigo (só o nome) vira objeto.
        if entry.is_string() {
            *entry = json!({ "name": name });
        }
        if let Some(clk) = config.frequency_mhz {
            entry["clk"] = clk.into();
        }
        if let Some(clocks) = config.clocks {
            entry["numClocks"] = clocks.into();
        }
        if let Some(show) = config.show_arrays {
            entry["showArrays"] = show.into();
        }
        self.save()?;

        let processor = &mut self.processors[index];
        processor.frequency_mhz = config.frequency_mhz.unwrap_or(processor.frequency_mhz);
        processor.clocks = config.clocks.unwrap_or(processor.clocks);
        processor.show_arrays = config.show_arrays.unwrap_or(processor.show_arrays);
        Ok(&self.processors[index])
    }

    /// Tira um processador do projeto e grava o `.spf`: sai a entrada dele em
    /// `structure.processors`, saem das listas os arquivos registrados de
    /// dentro da pasta dele (o Verilog e o testbench que o build gerou), com
    /// a escolha de topo ou de testbench que apontava para eles, e sai o que
    /// só a AURORA guardava dele (`.aurora8/Temp/<nome>`).
    ///
    /// A pasta do processador, com o fonte e as entradas, fica no disco, como
    /// um arquivo em [`Project::remove_verilog`]. Quem quiser apagá-la usa
    /// [`RemovedProcessor::dir`]: a CLI e o Studio a mandam para a lixeira,
    /// se pedido. Enquanto ela fica, os `.v` gerados lá dentro aparecem em
    /// [`Project::unregistered_verilog`], como qualquer Verilog solto.
    ///
    /// ```
    /// use aurora_core::{Language, NewProcessor, Project};
    /// # let tmp = tempfile::tempdir()?;
    /// # let dir = camino::Utf8Path::from_path(tmp.path()).unwrap();
    /// let mut project = Project::create(dir, "p")?;
    /// project.add_processor(&NewProcessor::new("soma", Language::Cmm))?;
    ///
    /// let removed = project.remove_processor("soma")?;
    /// assert!(project.processors().is_empty());
    /// assert!(removed.dir.join("soma.cmm").is_file());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Erros
    ///
    /// - [`AuroraError::ProcessorNotFound`] se o processador não existir; nada
    ///   muda;
    /// - [`AuroraError::Io`] se o `.spf` não puder ser gravado. A pasta
    ///   temporária só sai depois de o `.spf` ser gravado, e um erro ao
    ///   apagá-la fica no log, sem desfazer a remoção.
    pub fn remove_processor(&mut self, name: &str) -> Result<RemovedProcessor> {
        self.begin_write()?;
        let Some(index) = self.processors.iter().position(|p| p.name == name) else {
            let available = self.processors.iter().map(|p| p.name.clone()).collect();
            self.release_write();
            return Err(AuroraError::ProcessorNotFound {
                name: name.to_owned(),
                available,
            });
        };
        let processor = self.processors[index].clone();

        // O `.spf` guarda o caminho real: com a pasta sendo um link, os
        // arquivos dela estão registrados pelo destino dele.
        let real_dir = paths::real_path(&processor.dir);
        let unregistered: Vec<ProjectFile> = [FileRole::Synthesizable, FileRole::Testbench]
            .into_iter()
            .flat_map(|role| self.files(role))
            .filter(|file| {
                file.path.starts_with(&processor.dir) || file.path.starts_with(&real_dir)
            })
            .collect();
        for file in &unregistered {
            self.unlist(file.role, &file.path);
        }
        // A entrada: o nome (o formato antigo) ou o objeto.
        if let Some(list) = self.document["structure"]["processors"].as_array_mut() {
            list.retain(|entry| {
                entry.as_str() != Some(name) && entry["name"].as_str() != Some(name)
            });
        }
        self.save()?;
        self.processors.remove(index);

        if processor.temp_dir.is_dir()
            && let Err(error) = std::fs::remove_dir_all(&processor.temp_dir)
        {
            tracing::warn!(dir = %processor.temp_dir, %error, "Could not delete the temporary folder of a removed processor");
        }
        tracing::info!(name, "Processor removed");
        Ok(RemovedProcessor {
            name: processor.name,
            dir: processor.dir,
            unregistered,
        })
    }

    fn processor_paths(&self, name: &str, language: Language, file: Option<&str>) -> Processor {
        processor_paths(&self.root, name, language, file)
    }
}

/// O que [`Project::remove_processor`] tirou do projeto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct RemovedProcessor {
    /// O nome do processador.
    pub name: String,
    /// A pasta dele (`<raiz>/<nome>`), que continua no disco.
    #[schemars(with = "String")]
    pub dir: Utf8PathBuf,
    /// Os arquivos de dentro da pasta que estavam registrados e saíram das
    /// listas: o papel de cada um e se era o topo ou o testbench escolhido.
    pub unregistered: Vec<ProjectFile>,
}

/// Parâmetros de simulação de um processador, para
/// [`Project::configure_processor`]. `None` em um campo mantém o valor
/// atual.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct ProcessorConfig {
    /// Frequência de operação em MHz (`clk` no `.spf`).
    pub frequency_mhz: Option<u32>,
    /// Clocks que o testbench gerado simula (`numClocks` no `.spf`).
    pub clocks: Option<u32>,
    /// Exportar arrays para a simulação (`showArrays` no `.spf`).
    pub show_arrays: Option<bool>,
}

fn processor_paths(
    root: &Utf8Path,
    name: &str,
    language: Language,
    file: Option<&str>,
) -> Processor {
    let dir = root.join(name);
    let file = file.map_or_else(|| format!("{name}.{}", language.extension()), str::to_owned);
    Processor {
        name: name.to_owned(),
        language,
        source: dir.join(file),
        output_dir: dir.join(name),
        temp_dir: root.join(".aurora8").join("Temp").join(name),
        dir,
        frequency_mhz: DEFAULT_FREQUENCY_MHZ,
        clocks: DEFAULT_CLOCKS,
        show_arrays: false,
    }
}

/// Resolve a linguagem como a AURORA (`processor_source.ts` e
/// `processor_dispatch.ts`): `language` declarada; senão a extensão de
/// `sourceFile`/`cmmFile`; senão procura `<nome>.cmm` e depois `<nome>.cpp` na
/// pasta do processador (e, num projeto da estrutura de antes do YANC 7.1,
/// em `Software/`); senão C±.
fn processor_from_entry(
    spf_path: &Utf8Path,
    root: &Utf8Path,
    entry: spf::RawEntry,
) -> Result<Processor> {
    let name = entry.name;
    // Só o nome do arquivo importa: o fonte fica sempre na pasta do
    // processador, então um diretório no sourceFile não teria efeito.
    let file = entry
        .source_file
        .or(entry.cmm_file)
        .filter(|f| !f.trim().is_empty())
        .map(|f| {
            Utf8Path::new(&f.replace('\\', "/"))
                .file_name()
                .unwrap_or_default()
                .to_owned()
        });

    let declared = match entry.language.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(lang) => Some(Language::from_extension(lang).ok_or_else(|| {
            spf::invalid(
                spf_path,
                format!("Processor '{name}': unknown language '{lang}'"),
            )
        })?),
    };
    let from_file = file
        .as_deref()
        .and_then(|f| Utf8Path::new(f).extension())
        .and_then(Language::from_extension);
    let dir = root.join(&name);
    let probed = || {
        [Language::Cmm, Language::Cpp].into_iter().find(|l| {
            let file = format!("{name}.{}", l.extension());
            dir.join(&file).is_file() || dir.join("Software").join(&file).is_file()
        })
    };
    let language = declared
        .or(from_file)
        .or_else(probed)
        .unwrap_or(Language::Cmm);

    let mut processor = processor_paths(root, &name, language, file.as_deref());
    let in_range = |field: &str, kind: &str, value: u32| match simulation_range(kind, value) {
        Some(reason) => Err(spf::invalid(
            spf_path,
            format!("Processor '{name}': {field} {value}: {reason}"),
        )),
        None => Ok(value),
    };
    if let Some(clk) = spf::positive_int(spf_path, &name, "clk", entry.clk.as_ref())? {
        processor.frequency_mhz = in_range("clk", "frequency", clk)?;
    }
    if let Some(n) = spf::positive_int(spf_path, &name, "numClocks", entry.num_clocks.as_ref())? {
        processor.clocks = in_range("numClocks", "clocks", n)?;
    }
    if let Some(b) = spf::boolean(entry.show_arrays.as_ref()) {
        processor.show_arrays = b;
    }
    Ok(processor)
}

/// Por que um valor de simulação está fora da faixa, ou `None` se serve.
/// `kind` é `"frequency"` (MHz) ou `"clocks"`.
fn simulation_range(kind: &str, value: u32) -> Option<&'static str> {
    match kind {
        _ if value == 0 => Some("must be at least 1"),
        "frequency" if value > MAX_FREQUENCY_MHZ => Some(
            "the generated testbench works in picoseconds (timescale 1ns/1ps): above 500000 MHz the half period rounds to 0",
        ),
        "clocks" if value > MAX_CLOCKS => {
            Some("the generated testbench counts clocks in a 32-bit integer: at most 2147483647")
        }
        _ => None,
    }
}

/// Um nome que vira uma pasta direto dentro da raiz: sem barra, sem `:`, e
/// diferente de `.` e `..`. Um nome vazio tomaria a raiz como pasta do
/// processador; `../x` sairia do projeto.
fn is_single_folder_name(name: &str) -> bool {
    !name.contains(['/', '\\', ':']) && name != "." && name != ".."
}

/// Um erro que a troca do `.spf` por outra gravação causa no Windows, e que
/// some na tentativa seguinte (ver [`Project::open`]).
fn is_transient(error: &AuroraError) -> bool {
    match error {
        AuroraError::Io { source, .. } => matches!(
            source.kind(),
            std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
        ),
        AuroraError::InvalidProject { .. } => true,
        _ => false,
    }
}

fn locate_spf(path: &Utf8Path) -> Result<Utf8PathBuf> {
    if path.is_file() {
        return Ok(path.to_owned());
    }
    if !path.is_dir() {
        return Err(AuroraError::InvalidProject {
            path: path.to_owned(),
            reason: "Does not exist".into(),
        });
    }
    // Canonicaliza antes para que "." tenha nome de diretório.
    let path = &paths::canonicalize(path)?;
    spf_in(path)?.ok_or_else(|| AuroraError::InvalidProject {
        path: path.to_owned(),
        reason: "No .spf file in the directory".into(),
    })
}

/// O `.spf` de uma pasta: `<pasta>/<nome-da-pasta>.spf`, ou o único `.spf`
/// dela. `None` sem nenhum, ou se a pasta não pode ser listada.
pub(crate) fn spf_in(dir: &Utf8Path) -> Result<Option<Utf8PathBuf>> {
    if let Some(stem) = dir.file_name() {
        let candidate = dir.join(format!("{stem}.spf"));
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
    }
    let Ok(entries) = dir.read_dir_utf8() else {
        return Ok(None);
    };
    let mut found: Vec<Utf8PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension() == Some("spf") && p.is_file())
        .collect();
    match found.len() {
        0 => Ok(None),
        1 => Ok(Some(found.remove(0))),
        n => Err(AuroraError::InvalidProject {
            path: dir.to_owned(),
            reason: format!("{n} .spf files in the directory; name the one to open"),
        }),
    }
}

/// O nome do processador vira nome de arquivo, nome de módulo Verilog e, até
/// o YANC 6.x, `#PRNAME`, então precisa ser um identificador válido para os
/// três. A AURORA
/// aceita `-`, mas o lexer do cmmcomp não (`{LETRA}({LETRA}|[0-9])*`).
/// Um identificador Verilog simples (letras, dígitos e `_`, começando por
/// letra ou `_`). `context` diz por que o nome precisa ser assim.
pub(crate) fn validate_identifier(name: &str, context: &str) -> Result<()> {
    let mut chars = name.chars();
    let valid = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if valid {
        Ok(())
    } else {
        Err(AuroraError::InvalidName {
            name: name.to_owned(),
            reason: format!(
                "{context}, so use only letters, digits and '_', starting with a letter or '_'"
            ),
        })
    }
}

/// Confere um nome de processador, a regra de
/// [`Project::add_processor`]. O nome vira a pasta, o fonte, os arquivos e
/// o módulo Verilog `<nome>` (e `<nome>_tb`), então precisa servir para
/// todos. A regra é a do YANC 7.1 (`Compilers/common/proc_name.c`), que
/// recusa o fonte com outro nome, mais o que a AURORA sabe que quebra depois:
///
/// - identificador: letras sem acento, dígitos e `_`, sem dígito no começo
///   (a AURORA de antes aceitava `-`), e sem `__` no começo (o YANC guarda
///   esses nomes para os dele);
/// - fora das palavras reservadas do Verilog e do SystemVerilog (`module`,
///   `wire`, `logic`, ...), a lista do YANC;
/// - fora dos módulos da biblioteca SAPHO (`core`, `processor`, `ula`, ...):
///   o processador instanciaria a si mesmo;
/// - fora dos nomes reservados do Windows, porque vira pasta;
/// - até [`MAX_PROCESSOR_NAME`] caracteres.
///
/// Palavras do C± (`in`, `out`, `sqrt`) servem: na 7.1 o nome não vai para
/// o fonte.
///
/// Uma interface pode chamar enquanto o usuário digita.
///
/// ```
/// use aurora_core::validate_processor_name;
///
/// assert!(validate_processor_name("filtro_fir").is_ok());
/// for bad in ["proc-1", "2fast", "__x", "void", "module", "core", "con"] {
///     assert!(validate_processor_name(bad).is_err(), "{bad}");
/// }
/// for good in ["out", "sqrt", "main"] {
///     assert!(validate_processor_name(good).is_ok(), "{good}");
/// }
/// ```
///
/// # Erros
///
/// [`AuroraError::InvalidName`], com o motivo em `reason`.
pub fn validate_processor_name(name: &str) -> Result<()> {
    let mut chars = name.chars();
    let valid = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    let invalid = |reason: String| AuroraError::InvalidName {
        name: name.to_owned(),
        reason,
    };
    if !valid {
        return Err(invalid(
            "Use only letters, digits and '_', starting with a letter or '_'".into(),
        ));
    }
    if name.starts_with("__") {
        return Err(invalid(
            "YANC keeps the names starting with '__' for its own, and refuses a source named like that".into(),
        ));
    }
    if name.len() > MAX_PROCESSOR_NAME {
        return Err(invalid(format!(
            "At most {MAX_PROCESSOR_NAME} characters (it has {}): the YANC compilers keep the name in fixed buffers",
            name.len()
        )));
    }
    if is_windows_reserved(name) {
        return Err(invalid(WINDOWS_RESERVED_REASON.into()));
    }
    if VERILOG_WORDS.contains(&name) {
        return Err(invalid(
            "It is a Verilog or SystemVerilog keyword, which YANC refuses as the name of the processor module".into(),
        ));
    }
    if SAPHO_MODULES.contains(&name) {
        return Err(invalid(
            "It is a module of the SAPHO library, and the processor would instantiate itself"
                .into(),
        ));
    }
    Ok(())
}

/// As pastas da própria AURORA com nome que um processador poderia ter:
/// `.aurora8/Temp/<nome>` (o lint, a síntese, a hierarquia, a árvore de sinais,
/// os layouts do Surfer, o cocotb) e `wave/` na raiz, com as escolhas de
/// sinais. Um processador assim dividiria a pasta com uma operação da AURORA:
/// a hierarquia apaga `.aurora8/Temp/hierarchy`, que seria a pasta temporária
/// do processador `hierarchy`, com a onda dele; remover o processador
/// `synth` apagava a síntese do projeto; e a pasta do processador `wave` é a
/// das escolhas de sinais, que remover o processador com a pasta mandaria
/// para a lixeira.
const AURORA_FOLDERS: [&str; 6] = ["synth", "hierarchy", "lint", "wave", "surfer", "cocotb"];

/// Confere o nome de um processador novo: a regra do YANC
/// ([`validate_processor_name`]) e, sem diferença de maiúsculas (no Windows
/// e no macOS a pasta é a mesma), fora os nomes das pastas da AURORA (`synth`,
/// `hierarchy`, `lint`, `wave`, `surfer`, `cocotb`). Um processador que já
/// existe com um desses nomes continua abrindo e compilando.
///
/// Uma interface pode chamar enquanto o usuário digita.
///
/// ```
/// use aurora_core::{validate_new_processor_name, validate_processor_name};
///
/// assert!(validate_new_processor_name("filtro_fir").is_ok());
/// assert!(validate_new_processor_name("Wave").is_err());
/// assert!(validate_processor_name("wave").is_ok());
/// ```
///
/// # Erros
///
/// [`AuroraError::InvalidName`], com o motivo em `reason`.
pub fn validate_new_processor_name(name: &str) -> Result<()> {
    validate_processor_name(name)?;
    if AURORA_FOLDERS.iter().any(|f| f.eq_ignore_ascii_case(name)) {
        return Err(AuroraError::InvalidName {
            name: name.to_owned(),
            reason: "AURORA keeps a folder with this name for its own work (.aurora8/Temp/<name>, or wave/ for the signal choices), and the processor would share it".into(),
        });
    }
    Ok(())
}

/// As palavras reservadas do Verilog (IEEE 1364-2005) e do SystemVerilog
/// (IEEE 1800-2017) que o YANC 7.1 recusa como nome de processador: cópia
/// de `Compilers/common/proc_name.c`, na mesma ordem. Um módulo pode ser lido
/// pelo Icarus, pelo Verilator ou pelo Quartus em qualquer um dos modos.
const VERILOG_WORDS: &[&str] = &[
    "accept_on",
    "alias",
    "always",
    "always_comb",
    "always_ff",
    "always_latch",
    "and",
    "assert",
    "assign",
    "assume",
    "automatic",
    "before",
    "begin",
    "bind",
    "bins",
    "binsof",
    "bit",
    "break",
    "buf",
    "bufif0",
    "bufif1",
    "byte",
    "case",
    "casex",
    "casez",
    "cell",
    "chandle",
    "checker",
    "class",
    "clocking",
    "cmos",
    "config",
    "const",
    "constraint",
    "context",
    "continue",
    "cover",
    "covergroup",
    "coverpoint",
    "cross",
    "deassign",
    "default",
    "defparam",
    "design",
    "disable",
    "dist",
    "do",
    "edge",
    "else",
    "end",
    "endcase",
    "endchecker",
    "endclass",
    "endclocking",
    "endconfig",
    "endfunction",
    "endgenerate",
    "endgroup",
    "endinterface",
    "endmodule",
    "endpackage",
    "endprimitive",
    "endprogram",
    "endproperty",
    "endsequence",
    "endspecify",
    "endtable",
    "endtask",
    "enum",
    "event",
    "eventually",
    "expect",
    "export",
    "extends",
    "extern",
    "final",
    "first_match",
    "for",
    "force",
    "foreach",
    "forever",
    "fork",
    "forkjoin",
    "function",
    "generate",
    "genvar",
    "global",
    "highz0",
    "highz1",
    "if",
    "iff",
    "ifnone",
    "ignore_bins",
    "illegal_bins",
    "implements",
    "implies",
    "import",
    "incdir",
    "include",
    "initial",
    "inout",
    "input",
    "inside",
    "instance",
    "int",
    "integer",
    "interconnect",
    "interface",
    "intersect",
    "join",
    "join_any",
    "join_none",
    "large",
    "let",
    "liblist",
    "library",
    "local",
    "localparam",
    "logic",
    "longint",
    "macromodule",
    "matches",
    "medium",
    "modport",
    "module",
    "nand",
    "negedge",
    "nettype",
    "new",
    "nexttime",
    "nmos",
    "nor",
    "noshowcancelled",
    "not",
    "notif0",
    "notif1",
    "null",
    "or",
    "output",
    "package",
    "packed",
    "parameter",
    "pmos",
    "posedge",
    "primitive",
    "priority",
    "program",
    "property",
    "protected",
    "pull0",
    "pull1",
    "pulldown",
    "pullup",
    "pulsestyle_ondetect",
    "pulsestyle_onevent",
    "pure",
    "rand",
    "randc",
    "randcase",
    "randsequence",
    "rcmos",
    "real",
    "realtime",
    "ref",
    "reg",
    "reject_on",
    "release",
    "repeat",
    "restrict",
    "return",
    "rnmos",
    "rpmos",
    "rtran",
    "rtranif0",
    "rtranif1",
    "s_always",
    "s_eventually",
    "s_nexttime",
    "s_until",
    "s_until_with",
    "scalared",
    "sequence",
    "shortint",
    "shortreal",
    "showcancelled",
    "signed",
    "small",
    "soft",
    "solve",
    "specify",
    "specparam",
    "static",
    "string",
    "strong",
    "strong0",
    "strong1",
    "struct",
    "super",
    "supply0",
    "supply1",
    "sync_accept_on",
    "sync_reject_on",
    "table",
    "tagged",
    "task",
    "this",
    "throughout",
    "time",
    "timeprecision",
    "timeunit",
    "tran",
    "tranif0",
    "tranif1",
    "tri",
    "tri0",
    "tri1",
    "triand",
    "trior",
    "trireg",
    "type",
    "typedef",
    "union",
    "unique",
    "unique0",
    "unsigned",
    "until",
    "until_with",
    "untyped",
    "use",
    "uwire",
    "var",
    "vectored",
    "virtual",
    "void",
    "wait",
    "wait_order",
    "wand",
    "weak",
    "weak0",
    "weak1",
    "while",
    "wildcard",
    "wire",
    "with",
    "within",
    "wor",
    "xnor",
    "xor",
];

/// Os módulos da biblioteca SAPHO do YANC (`SAPHO/*.v`), compilados junto
/// com cada processador.
const SAPHO_MODULES: &[&str] = &[
    "addr_dec",
    "core",
    "div_array",
    "instr_dec",
    "instr_fetch",
    "io_ctrl",
    "mem_ctrl",
    "mem_data",
    "mem_instr",
    "myDCFIFO",
    "myFIFO",
    "myKeyPS2",
    "myVGAText",
    "norm_mux",
    "pc",
    "prefetch",
    "processor",
    "rel_addr",
    "stack",
    "ula",
    "ula_abs",
    "ula_add",
    "ula_and",
    "ula_denorm",
    "ula_equ",
    "ula_f2i",
    "ula_fabs",
    "ula_fadd",
    "ula_fcmp",
    "ula_fdiv",
    "ula_fmlt",
    "ula_fneg",
    "ula_fpst",
    "ula_frot",
    "ula_fsgn",
    "ula_gre",
    "ula_i2f",
    "ula_idiv",
    "ula_in1_ctrl",
    "ula_in2_ctrl",
    "ula_inv",
    "ula_lan",
    "ula_les",
    "ula_lin",
    "ula_lor",
    "ula_lzc",
    "ula_mlt",
    "ula_mux",
    "ula_neg",
    "ula_norm",
    "ula_nrm",
    "ula_or",
    "ula_pst",
    "ula_scl",
    "ula_sgn",
    "ula_shift",
    "ula_xor",
    "ula_xpo",
];

/// Um aviso de [`Project::issues`]: algo estranho no `.spf` que não impede
/// de abrir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct ProjectIssue {
    /// O tipo, para a interface traduzir.
    pub kind: IssueKind,
    /// O arquivo ou a pasta envolvida, absoluta.
    #[schemars(with = "Option<String>")]
    pub path: Option<Utf8PathBuf>,
    /// O complemento do tipo: o caminho como estava gravado
    /// (`rescued_path`), o campo do `.spf` (`selection_not_registered`,
    /// `testbench_as_top`) ou o nome do processador
    /// (`invalid_processor_name`, `legacy_layout`).
    pub detail: Option<String>,
    /// O aviso inteiro, em inglês, pronto para mostrar.
    pub message: String,
}

impl ProjectIssue {
    pub(crate) fn new(
        kind: IssueKind,
        path: Option<Utf8PathBuf>,
        detail: Option<String>,
        message: String,
    ) -> Self {
        ProjectIssue {
            kind,
            path,
            detail,
            message,
        }
    }
}

/// O tipo de um [`ProjectIssue`]. Em JSON, `snake_case`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum IssueKind {
    /// Um absoluto de outra máquina que não existe aqui foi achado pela
    /// cauda dentro da raiz; o `.spf` sai consertado na próxima gravação.
    RescuedPath,
    /// `topLevelFile` ou `testbenchFile` aponta para um arquivo que não está
    /// na lista do papel, e é ignorado.
    SelectionNotRegistered,
    /// `topLevelFile` tem nome de testbench (`tb_x.v`, `x_tb.v`), que não
    /// pode ser topo, e é ignorado.
    TestbenchAsTop,
    /// Um processador do `.spf` com nome que o YANC não compila (`x-y`,
    /// `void`); o build dele recusa com `invalid_name`.
    InvalidProcessorName,
    /// Um arquivo repetido numa lista, ou nas duas: conta uma vez, e a
    /// próxima gravação tira as entradas que sobram.
    DuplicateFile,
    /// O projeto está dentro da pasta de outro (uma pasta acima tem `.spf`):
    /// os arquivos dele aparecem também no outro.
    NestedProject,
    /// Um processador ainda tem a estrutura de pastas de antes do YANC 7.1
    /// (`Software/`, `Hardware/`, `Simulation/`): o build dele recusa com
    /// `legacy_layout` até a conversão ([`Project::convert_layout`]).
    LegacyLayout,
}

const WINDOWS_RESERVED_REASON: &str = "Reserved name on Windows (CON, PRN, AUX, NUL, COM0-9, LPT0-9): the folder could not be created there";

/// Nomes que o Windows não aceita como arquivo ou pasta, com qualquer
/// extensão e em qualquer caixa. Recusados em todo sistema, para que um
/// projeto criado no Linux abra no Windows.
fn is_windows_reserved(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or(name)
        .trim_end()
        .to_ascii_uppercase();
    match stem.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" => true,
        _ => {
            let (prefix, digit) = stem.split_at(stem.len().min(3));
            matches!(prefix, "COM" | "LPT")
                && digit.len() == 1
                && digit.as_bytes()[0].is_ascii_digit()
        }
    }
}

/// Confere o nome de um projeto novo, a regra de [`Project::create`]:
/// letras sem acento (`A-Z`, `a-z`), dígitos, `_` e `-`, começando por letra,
/// até [`MAX_PROJECT_NAME`] caracteres, e fora dos nomes reservados do
/// Windows (`CON`, `COM1`...).
///
/// O nome vira a pasta, o `.spf` e parte do caminho de tudo que as
/// ferramentas recebem; espaço e acento num caminho exigem que cada script e
/// cada ferramenta citem o argumento direito, e a AURORA prefere recusar o nome
/// (decisão de 2026-10-04). Projetos que já existem com outros nomes, como
/// os da AURORA, continuam abrindo: a regra só vale para criar.
///
/// Uma interface pode chamar esta função enquanto o usuário digita, para
/// avisar antes de tentar criar.
///
/// ```
/// use aurora_core::validate_project_name;
///
/// assert!(validate_project_name("filtro_fir-2").is_ok());
/// assert!(validate_project_name("Projeto Teste").is_err());
/// assert!(validate_project_name("ação").is_err());
/// assert!(validate_project_name("2fir").is_err());
/// ```
///
/// # Erros
///
/// [`AuroraError::InvalidName`], com o motivo em `reason`.
pub fn validate_project_name(name: &str) -> Result<()> {
    let reason = if name.is_empty() {
        Some("Empty")
    } else if name.chars().any(char::is_whitespace) {
        Some("Contains a space; use '_' or '-' instead")
    } else if !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        Some("Must start with a letter (A-Z, a-z)")
    } else if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Some("Use only letters without accents, digits, '_' and '-'")
    } else if is_windows_reserved(name) {
        Some(WINDOWS_RESERVED_REASON)
    } else if name.len() > MAX_PROJECT_NAME {
        Some(
            "Too long: at most 64 characters (the name repeats in the folder, the .spf and every path the tools receive)",
        )
    } else {
        None
    };
    match reason {
        Some(reason) => Err(AuroraError::InvalidName {
            name: name.to_owned(),
            reason: reason.into(),
        }),
        None => Ok(()),
    }
}

fn computer_name() -> String {
    ["COMPUTERNAME", "HOSTNAME"]
        .iter()
        .find_map(|k| std::env::var(k).ok())
        .unwrap_or_default()
}

/// Data e hora UTC no formato do `Date.toISOString()` do JavaScript.
fn iso8601_now() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
        now.subsec_millis()
    )
}

/// `time` em UTC, até os segundos (`2026-10-03T21:04:05Z`), para o
/// relatório (`history.rs`).
pub(crate) fn utc_seconds(time: SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Dias desde 1970-01-01 para (ano, mês, dia), algoritmo de Howard Hinnant.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> (tempfile::TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = paths::canonicalize(Utf8Path::from_path(dir.path()).unwrap()).unwrap();
        (dir, path)
    }

    #[test]
    fn a_processor_is_out_of_date_until_built_after_its_files() {
        use std::time::{Duration, SystemTime};
        let (_guard, root) = tmp();
        let mut project = Project::create(&root, "p").unwrap();
        let soma = project
            .add_processor(&NewProcessor::new("soma", Language::Cmm))
            .unwrap();
        // Sem o Verilog gerado.
        assert!(soma.is_out_of_date());
        let at = |path: &Utf8Path, time: SystemTime| {
            std::fs::File::options()
                .write(true)
                .open(path)
                .unwrap()
                .set_modified(time)
                .unwrap();
        };
        let now = SystemTime::now();
        std::fs::create_dir_all(&soma.output_dir).unwrap();
        std::fs::write(soma.verilog_path(), "module soma; endmodule\n").unwrap();
        at(&soma.source, now - Duration::from_secs(60));
        at(&soma.verilog_path(), now - Duration::from_secs(30));
        assert!(!soma.is_out_of_date());
        // Uma tabela de array ao lado do fonte, mexida depois do build.
        let table = soma.dir.join("tabela.txt");
        std::fs::write(&table, "1\n").unwrap();
        at(&table, now);
        assert!(soma.is_out_of_date());
        // A pasta de saída é do YANC: o que muda nela não conta.
        at(&table, now - Duration::from_secs(60));
        std::fs::write(soma.output_dir.join("soma.asm"), "").unwrap();
        assert!(!soma.is_out_of_date());
    }

    /// A `.lace` de um projeto do Lace vira a `.aurora8` ao abrir, com o que
    /// tinha dentro; com as duas, nenhuma muda. A `.aurora`, da AURORA 7,
    /// fica onde está.
    #[test]
    fn opening_moves_the_lace_folder_to_aurora8() {
        let (_guard, dir) = tmp();
        let project = Project::create(&dir, "demo").unwrap();
        let root = project.root().to_owned();
        let _ = std::fs::remove_dir_all(root.join(PROJECT_DIR));
        let report = root.join(".lace/reports/run-000001/report.json");
        std::fs::create_dir_all(report.parent().unwrap()).unwrap();
        std::fs::write(&report, "{}").unwrap();
        let memory = root.join(".aurora/memory/notas.md");
        std::fs::create_dir_all(memory.parent().unwrap()).unwrap();
        std::fs::write(&memory, "da AURORA 7").unwrap();

        Project::open(project.spf_path()).unwrap();
        assert!(!root.join(".lace").exists());
        assert!(
            root.join(".aurora8/reports/run-000001/report.json")
                .is_file()
        );
        assert!(memory.is_file(), "a .aurora da AURORA 7 não muda");

        std::fs::create_dir_all(root.join(".lace/fpga")).unwrap();
        Project::open(project.spf_path()).unwrap();
        assert!(root.join(".lace/fpga").is_dir());
        assert!(root.join(".aurora8/reports").is_dir());
    }

    /// Num projeto só da AURORA 7, abrir não mexe na `.aurora` dela.
    #[test]
    fn opening_leaves_the_aurora_7_folder_alone() {
        let (_guard, dir) = tmp();
        let project = Project::create(&dir, "demo").unwrap();
        let root = project.root().to_owned();
        let _ = std::fs::remove_dir_all(root.join(PROJECT_DIR));
        let temp = root.join(".aurora/Temp/obj_dir_tb/Vtb.mk");
        std::fs::create_dir_all(temp.parent().unwrap()).unwrap();
        std::fs::write(&temp, "").unwrap();

        Project::open(project.spf_path()).unwrap();
        assert!(temp.is_file());
        assert!(!root.join(PROJECT_DIR).exists());
    }

    #[test]
    fn discover_climbs_to_the_project_from_any_inner_folder() {
        let (_guard, dir) = tmp();
        let mut project = Project::create(&dir, "demo").unwrap();
        let soma = project
            .add_processor(&NewProcessor::new("soma", Language::Cmm))
            .unwrap()
            .clone();
        soma.ensure_dirs().unwrap();
        let root = project.root().to_owned();
        for inner in [
            root.clone(),
            soma.dir.clone(),
            soma.output_dir.clone(),
            soma.temp_dir.clone(),
            soma.source.clone(),
            project.spf_path().to_owned(),
        ] {
            let found = Project::discover(&inner).unwrap();
            assert_eq!(found.spf_path(), project.spf_path(), "de {inner}");
        }

        // A pasta diz o processador; fora das dele, nenhum.
        let found = Project::discover(&soma.output_dir).unwrap();
        for inner in [&soma.dir, &soma.output_dir, &soma.temp_dir] {
            assert_eq!(found.processor_at(inner).unwrap().name, "soma", "{inner}");
        }
        assert!(found.processor_at(&root).is_none());
        assert!(found.processor_at(root.join(".aurora8")).is_none());

        // Fora de qualquer projeto.
        let (_outside_guard, outside) = tmp();
        assert!(matches!(
            Project::discover(&outside),
            Err(AuroraError::ProjectNotFound(p)) if p == outside
        ));
        assert!(matches!(
            Project::discover(outside.join("nao-existe")),
            Err(AuroraError::InvalidProject { .. })
        ));
    }

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_724), (2026, 9, 28));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
    }

    #[test]
    fn create_add_and_reopen() {
        let (_guard, dir) = tmp();
        let mut project = Project::create(&dir, "meu_projeto").unwrap();
        project
            .add_processor(&NewProcessor::new("soma", Language::Cmm))
            .unwrap();
        project
            .add_processor(&NewProcessor::new("filtro", Language::Cpp))
            .unwrap();

        let root = dir.join("meu_projeto");
        // Só a pasta do processador, com o fonte: a de saída nasce no build.
        let names: Vec<_> = std::fs::read_dir(root.join("soma"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["soma.cmm"]);
        let src = std::fs::read_to_string(root.join("soma/soma.cmm")).unwrap();
        // Sem #PRNAME, e sem as diretivas que têm padrão no YANC.
        assert_eq!(
            src,
            "#NUBITS 23\n#NUIOIN 1\n#NUIOOU 1\n#NBMANT 16\n#NBEXPO 6\n\nvoid main()\n{\n    \
             // out(porta, valor) escreve numa porta de saída\n    out(0, 0);\n}\n"
        );
        let cpp = std::fs::read_to_string(root.join("filtro/filtro.cpp")).unwrap();
        assert!(cpp.starts_with("#pragma yanc nuioin 1\n"), "{cpp}");

        let reopened = Project::open(&root).unwrap();
        assert_eq!(reopened.name(), "meu_projeto");
        let names: Vec<_> = reopened
            .processors()
            .iter()
            .map(|p| (p.name.as_str(), p.language))
            .collect();
        assert_eq!(names, [("soma", Language::Cmm), ("filtro", Language::Cpp)]);
        let soma = reopened.processor("soma").unwrap();
        assert_eq!(soma.frequency_mhz, DEFAULT_FREQUENCY_MHZ);
        assert_eq!(soma.temp_dir, root.join(".aurora8/Temp/soma"));
    }

    #[test]
    fn refuses_duplicates_and_bad_names() {
        let (_guard, dir) = tmp();
        let mut project = Project::create(&dir, "p").unwrap();
        project
            .add_processor(&NewProcessor::new("a", Language::Cmm))
            .unwrap();
        assert!(matches!(
            project.add_processor(&NewProcessor::new("a", Language::Cmm)),
            Err(AuroraError::ProcessorExists(_))
        ));
        assert!(matches!(
            project.add_processor(&NewProcessor::new("proc-1", Language::Cmm)),
            Err(AuroraError::InvalidName { .. })
        ));
        assert!(matches!(
            Project::create(&dir, "p"),
            Err(AuroraError::ProjectExists(_))
        ));
        assert!(matches!(
            Project::create(&dir, "a/b"),
            Err(AuroraError::InvalidName { .. })
        ));
    }

    #[test]
    fn project_names_without_spaces_accents_or_symbols() {
        for good in ["p", "contador", "Filtro_FIR", "soma-2", "a1_b-c"] {
            assert!(
                validate_project_name(good).is_ok(),
                "deveria aceitar {good}"
            );
        }
        for bad in [
            "",
            "Projeto Teste",
            "ação",
            "café",
            "2fir",
            "_x",
            "-x",
            "a.b",
            "a@b",
            "a/b",
            "tab\tname",
            "con",
        ] {
            assert!(
                matches!(
                    validate_project_name(bad),
                    Err(AuroraError::InvalidName { .. })
                ),
                "deveria recusar {bad:?}"
            );
        }
        // A regra é a de criar; nada foi gravado.
        let (_guard, dir) = tmp();
        assert!(Project::create(&dir, "Projeto Teste").is_err());
        assert!(!dir.join("Projeto Teste").exists());
    }

    #[test]
    fn names_that_break_on_windows_or_macos_are_refused() {
        let (_guard, dir) = tmp();
        for name in ["con", "Aux", "NUL", "com1", "LPT9"] {
            assert!(Project::create(&dir, name).is_err(), "projeto {name}");
        }
        assert!(Project::create(&dir, "con.v2").is_err());
        assert!(Project::create(&dir, "console").is_ok());
        assert!(Project::create(&dir, "com10").is_ok());

        let mut project = Project::create(&dir, "p").unwrap();
        assert!(matches!(
            project.add_processor(&NewProcessor::new("aux", Language::Cmm)),
            Err(AuroraError::InvalidName { .. })
        ));
        project
            .add_processor(&NewProcessor::new("Soma", Language::Cmm))
            .unwrap();
        assert!(matches!(
            project.add_processor(&NewProcessor::new("soma", Language::Cmm)),
            Err(AuroraError::ProcessorExists(name)) if name == "Soma"
        ));
    }

    #[test]
    fn preserves_unknown_fields_on_save() {
        let (_guard, dir) = tmp();
        let spf = dir.join("x.spf");
        std::fs::write(
            &spf,
            r#"{"metadata": {"projectName": "x"}, "structure": {"basePath": "C:\\velho", "processors": [], "commandOverrides": {"asm": {"x": 1}}}}"#,
        )
        .unwrap();
        let mut project = Project::open(&spf).unwrap();
        project
            .add_processor(&NewProcessor::new("p", Language::Cmm))
            .unwrap();
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&spf).unwrap()).unwrap();
        assert_eq!(doc["structure"]["commandOverrides"]["asm"]["x"], 1);
        assert_eq!(doc["structure"]["basePath"], dir.as_str());
    }

    #[test]
    fn configure_processor_persists_and_converts_legacy_entry() {
        let (_guard, dir) = tmp();
        std::fs::write(
            dir.join("x.spf"),
            r#"{"structure": {"processors": ["velho", {"name": "novo", "language": "cpp", "extra": 1}]}}"#,
        )
        .unwrap();
        let mut project = Project::open(dir.join("x.spf")).unwrap();
        let mut config = ProcessorConfig {
            frequency_mhz: Some(25),
            show_arrays: Some(true),
            ..ProcessorConfig::default()
        };
        project.configure_processor("velho", &config).unwrap();
        config.clocks = Some(99);
        project.configure_processor("novo", &config).unwrap();

        let reopened = Project::open(dir.join("x.spf")).unwrap();
        let velho = reopened.processor("velho").unwrap();
        assert_eq!(
            (velho.frequency_mhz, velho.clocks, velho.show_arrays),
            (25, DEFAULT_CLOCKS, true)
        );
        assert_eq!(reopened.processor("novo").unwrap().clocks, 99);
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("x.spf")).unwrap()).unwrap();
        assert_eq!(
            doc["structure"]["processors"][0],
            json!({"name": "velho", "clk": 25, "showArrays": true})
        );
        assert_eq!(doc["structure"]["processors"][1]["extra"], 1);

        config.clocks = Some(0);
        assert!(project.configure_processor("novo", &config).is_err());
        assert!(matches!(
            project.configure_processor("nada", &ProcessorConfig::default()),
            Err(AuroraError::ProcessorNotFound { .. })
        ));
    }

    #[test]
    fn removing_a_processor_takes_out_its_entry_and_files_and_keeps_the_folder() {
        let (_guard, dir) = tmp();
        let mut project = Project::create(&dir, "demo").unwrap();
        let soma = project
            .add_processor(&NewProcessor::new("soma", Language::Cmm))
            .unwrap()
            .clone();
        project
            .add_processor(&NewProcessor::new("filtro", Language::Cpp))
            .unwrap();
        // O que um build deixaria: o Verilog e o testbench gerados,
        // registrados e escolhidos, e a pasta temporária.
        soma.ensure_dirs().unwrap();
        std::fs::write(soma.verilog_path(), "module soma(input clk);\nendmodule\n").unwrap();
        std::fs::write(
            soma.testbench_path(),
            "module soma_tb;\nsoma u(.clk(1'b0));\nendmodule\n",
        )
        .unwrap();
        std::fs::write(soma.temp_dir.join("soma.vvp"), "").unwrap();
        let root = project.root().to_owned();
        std::fs::write(root.join("top.v"), "module top;\nendmodule\n").unwrap();
        project
            .add_file(FileRole::Synthesizable, root.join("top.v"), None)
            .unwrap();
        project
            .add_file(FileRole::Synthesizable, soma.verilog_path(), None)
            .unwrap();
        project
            .add_file(FileRole::Testbench, soma.testbench_path(), None)
            .unwrap();
        project.set_top_level(soma.verilog_path()).unwrap();
        project.set_testbench(soma.testbench_path()).unwrap();

        let removed = project.remove_processor("soma").unwrap();
        assert_eq!((removed.name.as_str(), &removed.dir), ("soma", &soma.dir));
        let gone: Vec<_> = removed
            .unregistered
            .iter()
            .map(|f| (f.role, f.path.clone(), f.top_level))
            .collect();
        assert_eq!(
            gone,
            [
                (FileRole::Synthesizable, soma.verilog_path(), true),
                (FileRole::Testbench, soma.testbench_path(), true),
            ]
        );

        let reopened = Project::open(project.spf_path()).unwrap();
        for p in [&project, &reopened] {
            let names: Vec<_> = p.processors().iter().map(|p| p.name.as_str()).collect();
            assert_eq!(names, ["filtro"]);
        }
        let synthesizable: Vec<_> = reopened
            .files(FileRole::Synthesizable)
            .into_iter()
            .map(|f| f.path)
            .collect();
        assert_eq!(synthesizable, [root.join("top.v")]);
        assert!(reopened.files(FileRole::Testbench).is_empty());
        assert!(reopened.top_level().is_none());
        assert!(reopened.testbench().is_none());
        // A pasta fica, com o fonte e o que o build gerou; a temporária sai,
        // e o Verilog gerado passa a ser um Verilog solto do projeto.
        assert!(soma.source.is_file() && soma.verilog_path().is_file());
        assert!(!soma.temp_dir.exists());
        assert!(reopened.unregistered_hdl().contains(&soma.verilog_path()));
    }

    #[test]
    fn removing_an_unknown_or_old_style_processor() {
        let (_guard, dir) = tmp();
        let spf = dir.join("x.spf");
        std::fs::write(
            &spf,
            r#"{"structure": {"processors": ["velho", {"name": "novo", "extra": 1}]}}"#,
        )
        .unwrap();
        let mut project = Project::open(&spf).unwrap();
        let before = std::fs::read_to_string(&spf).unwrap();
        assert!(matches!(
            project.remove_processor("Velho"),
            Err(AuroraError::ProcessorNotFound { available, .. }) if available == ["velho", "novo"]
        ));
        assert_eq!(std::fs::read_to_string(&spf).unwrap(), before);

        // Sem a pasta no disco, e no formato antigo (só o nome), também sai;
        // a trava da recusa acima foi solta.
        project.remove_processor("velho").unwrap();
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&spf).unwrap()).unwrap();
        assert_eq!(
            doc["structure"]["processors"],
            json!([{"name": "novo", "extra": 1}])
        );
        project.remove_processor("novo").unwrap();
        assert!(Project::open(&spf).unwrap().processors().is_empty());
    }

    #[test]
    fn language_resolution_order() {
        let (_guard, dir) = tmp();
        // Um projeto da estrutura do YANC 6.x também tem a linguagem
        // descoberta pelo fonte em `Software/`, para a conversão.
        std::fs::create_dir_all(dir.join("sonda/Software")).unwrap();
        std::fs::write(dir.join("sonda/Software/sonda.cpp"), "").unwrap();
        std::fs::create_dir_all(dir.join("nova")).unwrap();
        std::fs::write(dir.join("nova/nova.cpp"), "").unwrap();
        std::fs::write(
            dir.join("x.spf"),
            r#"{"structure": {"processors": [
                "sonda",
                {"name": "ext", "sourceFile": "C:\\a\\ext.cpp"},
                {"name": "decl", "language": "cpp", "clk": 50, "numClocks": 10, "showArrays": true},
                {"name": "padrao"},
                "nova"
            ]}}"#,
        )
        .unwrap();
        let project = Project::open(dir.join("x.spf")).unwrap();
        let lang = |n| project.processor(n).unwrap().language;
        assert_eq!(lang("sonda"), Language::Cpp);
        assert_eq!(lang("nova"), Language::Cpp);
        assert_eq!(lang("ext"), Language::Cpp);
        assert_eq!(
            project.processor("ext").unwrap().source,
            dir.join("ext/ext.cpp")
        );
        assert_eq!(lang("padrao"), Language::Cmm);
        let decl = project.processor("decl").unwrap();
        assert_eq!(
            (
                decl.language,
                decl.frequency_mhz,
                decl.clocks,
                decl.show_arrays
            ),
            (Language::Cpp, 50, 10, true)
        );
    }

    #[test]
    fn broken_processor_entries_are_refused_on_open() {
        let (_guard, dir) = tmp();
        let spf = dir.join("x.spf");
        for (list, expected) in [
            (r#"[{"name": ""}]"#, "empty"),
            (r#"["../evil"]"#, "not a folder name"),
            (r#"["soma", "Soma"]"#, "more than once"),
        ] {
            std::fs::write(
                &spf,
                format!(r#"{{"structure": {{"processors": {list}}}}}"#),
            )
            .unwrap();
            match Project::open(&spf) {
                Err(AuroraError::InvalidProjectFile { reason, .. }) => {
                    assert!(reason.contains(expected), "{list}: {reason}")
                }
                other => panic!("{list}: {other:?}"),
            }
        }
        // Nome que não compila abre, com aviso; o build recusa.
        std::fs::write(
            &spf,
            r#"{"structure": {"processors": ["x-y", "void", "ok"]}}"#,
        )
        .unwrap();
        let project = Project::open(&spf).unwrap();
        let names: Vec<_> = project
            .issues()
            .into_iter()
            .map(|i| i.detail.unwrap())
            .collect();
        assert_eq!(names, ["x-y", "void"]);
    }

    #[test]
    fn processor_names_that_never_compile_are_refused() {
        for bad in [
            "void",
            "int",
            "module",
            "wire",
            "logic",
            "always_ff",
            "core",
            "processor",
            "ula",
            "__x",
            "2fast",
        ] {
            assert!(validate_processor_name(bad).is_err(), "{bad}");
        }
        // Palavras do C± que não são do Verilog: no YANC 7.1 o nome não vai
        // para o fonte (conferido compilando e simulando `out`, `in` e `sqrt`).
        for good in ["out", "in", "sqrt", "main"] {
            assert!(validate_processor_name(good).is_ok(), "{good}");
        }
        assert!(validate_processor_name(&"a".repeat(64)).is_ok());
        assert!(validate_processor_name(&"a".repeat(65)).is_err());
        assert!(validate_processor_name("soma_2").is_ok());
    }

    #[test]
    fn processor_parameters_are_checked_before_creating() {
        let (_guard, dir) = tmp();
        let mut project = Project::create(&dir, "p").unwrap();
        let base = NewProcessor::new("a", Language::Cmm);
        type Change = fn(&mut NewProcessor);
        let cases: [(&str, Change); 8] = [
            ("#NUBITS", |s| s.nubits = 0),
            ("#NUBITS", |s| s.nubits = 200),
            ("#NUGAIN", |s| s.nugain = 3),
            ("#NBMANT", |s| {
                s.nbmant = 30;
                s.nubits = 37;
            }),
            ("#NBEXPO", |s| s.nbexpo = 9),
            ("#NDSTAC", |s| s.ndstac = Some(0)),
            ("#SDEPTH", |s| s.sdepth = Some(0)),
            ("#NUIOIN", |s| s.input_ports = 100_000),
        ];
        for (field, change) in cases {
            let mut spec = base.clone();
            change(&mut spec);
            match project.add_processor(&spec) {
                Err(AuroraError::InvalidParameter { name, .. }) => assert_eq!(name, field),
                other => panic!("{field}: {other:?}"),
            }
        }
        assert!(!dir.join("p/a").exists(), "nada foi criado");
        let mut c = NewProcessor::new("c", Language::Cpp);
        c.nubits = 0;
        assert!(
            project.add_processor(&c).is_ok(),
            "C ignora os campos do C±"
        );
    }

    #[test]
    fn clocks_and_frequency_have_ranges() {
        let (_guard, dir) = tmp();
        let mut project = Project::create(&dir, "p").unwrap();
        project
            .add_processor(&NewProcessor::new("calc", Language::Cmm))
            .unwrap();
        for (freq, clocks) in [
            (Some(0), None),
            (None, Some(0)),
            (Some(3_000_000), None),
            (None, Some(3_000_000_000)),
        ] {
            let config = ProcessorConfig {
                frequency_mhz: freq,
                clocks,
                ..ProcessorConfig::default()
            };
            assert!(
                matches!(
                    project.configure_processor("calc", &config),
                    Err(AuroraError::InvalidParameter { .. })
                ),
                "{freq:?} {clocks:?}"
            );
        }
        let config = ProcessorConfig {
            frequency_mhz: Some(MAX_FREQUENCY_MHZ),
            clocks: Some(MAX_CLOCKS),
            ..ProcessorConfig::default()
        };
        assert!(project.configure_processor("calc", &config).is_ok());
    }

    #[test]
    fn concurrent_writes_keep_every_change() {
        let (_guard, dir) = tmp();
        let project = Project::create(&dir, "demo").unwrap();
        let root = project.root().to_owned();
        for i in 0..16 {
            std::fs::write(
                root.join(format!("m{i}.v")),
                format!("module m{i}; endmodule\n"),
            )
            .unwrap();
        }
        // Cada thread abre o seu `Project`, como o Studio e a CLI.
        let handles: Vec<_> = (0..16)
            .map(|i| {
                let root = root.clone();
                std::thread::spawn(move || {
                    let mut project = Project::open(&root).unwrap();
                    project
                        .add_file(
                            crate::FileRole::Synthesizable,
                            root.join(format!("m{i}.v")),
                            None,
                        )
                        .unwrap();
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let project = Project::open(&root).unwrap();
        assert_eq!(project.files(crate::FileRole::Synthesizable).len(), 16);
        // Nenhum temporário sobra ao lado do `.spf`.
        let leftovers: Vec<_> = root
            .read_dir_utf8()
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn long_or_case_colliding_project_names_create_nothing() {
        let (_guard, dir) = tmp();
        let long = format!("a{}", "d".repeat(249));
        assert!(Project::create(&dir, &long).is_err());
        assert!(!dir.join(&long).exists());
        Project::create(&dir, "ok1").unwrap();
        match Project::create(&dir, "Ok1") {
            Err(AuroraError::InvalidName { reason, .. }) => assert!(reason.contains("ok1")),
            other => panic!("{other:?}"),
        }
        // No macOS e no Windows, `dir/Ok1` acha a pasta `ok1`: o que conta é
        // não haver uma entrada com o nome `Ok1`.
        let names: Vec<String> = dir
            .read_dir_utf8()
            .unwrap()
            .map(|e| e.unwrap().file_name().to_owned())
            .collect();
        assert_eq!(names, ["ok1"]);
    }

    #[test]
    fn a_project_inside_another_is_flagged_and_left_out_of_it() {
        let (_guard, dir) = tmp();
        let outer = Project::create(&dir, "pai").unwrap();
        let inner = Project::create(outer.root(), "filho").unwrap();
        std::fs::write(inner.root().join("m.v"), "module m; endmodule\n").unwrap();
        let issues = Project::open(inner.spf_path()).unwrap().issues();
        assert_eq!(issues[0].kind, IssueKind::NestedProject);
        assert!(
            Project::open(outer.spf_path())
                .unwrap()
                .unregistered_hdl()
                .is_empty()
        );
    }

    #[test]
    fn a_new_processor_cannot_take_the_name_of_an_aurora_folder() {
        let (_guard, dir) = tmp();
        let mut project = Project::create(&dir, "p").unwrap();
        for name in ["wave", "Synth", "hierarchy", "lint", "surfer", "cocotb"] {
            let error = project
                .add_processor(&NewProcessor::new(name, Language::Cmm))
                .unwrap_err();
            assert_eq!(error.code(), "invalid_name", "{name}");
        }
        assert!(project.processors().is_empty());
        // Um que já está no `.spf` continua no projeto, sem aviso de nome.
        let root = dir.join("antigo");
        std::fs::create_dir_all(root.join("wave")).unwrap();
        std::fs::write(root.join("wave/wave.cmm"), "void main() {}\n").unwrap();
        std::fs::write(
            root.join("antigo.spf"),
            r#"{"structure": {"processors": [{"name": "wave"}]}}"#,
        )
        .unwrap();
        let old = Project::open(root.join("antigo.spf")).unwrap();
        assert_eq!(old.processors()[0].name, "wave");
        assert!(
            !old.issues()
                .iter()
                .any(|i| i.kind == IssueKind::InvalidProcessorName)
        );
    }

    /// `link` passa a abrir `target`: um link simbólico no Unix, uma junção
    /// no Windows (que não pede administrador).
    fn link_dir(target: &Utf8Path, link: &Utf8Path) {
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, link).unwrap();
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J", link.as_str(), target.as_str()])
                .stdout(std::process::Stdio::null())
                .status()
                .unwrap();
            assert!(status.success());
        }
    }

    #[test]
    fn a_processor_folder_that_is_a_link_keeps_one_record_of_its_verilog() {
        let (_guard, dir) = tmp();
        let mut project = Project::create(&dir, "p").unwrap();
        let soma = project
            .add_processor(&NewProcessor::new("soma", Language::Cmm))
            .unwrap()
            .clone();
        // A pasta do processador mora fora, e o projeto a vê por um link.
        let real = dir.join("fora").join("soma");
        std::fs::create_dir_all(real.parent().unwrap()).unwrap();
        std::fs::rename(&soma.dir, &real).unwrap();
        link_dir(&real, &soma.dir);
        std::fs::create_dir_all(&soma.output_dir).unwrap();
        std::fs::write(soma.verilog_path(), "module soma; endmodule\n").unwrap();
        // Como o build registra: pelo caminho real.
        let registered = project
            .add_file(FileRole::Synthesizable, soma.verilog_path(), None)
            .unwrap();
        assert!(registered.starts_with(&real), "{registered}");

        let sources = crate::synth::project_sources(&project).unwrap();
        assert_eq!(sources, std::slice::from_ref(&registered), "um registro só");
        assert_eq!(
            project.generated_by(&registered).map(|p| &p.name),
            Some(&soma.name)
        );
        assert_eq!(
            project.processor_at(&registered).map(|p| &p.name),
            Some(&soma.name)
        );
        let removed = project.remove_processor("soma").unwrap();
        assert_eq!(removed.unregistered.len(), 1);
        assert!(project.files(FileRole::Synthesizable).is_empty());
    }
}

#[cfg(test)]
mod new_processor_json {
    use super::*;

    #[test]
    fn missing_fields_take_the_defaults_and_nubits_follows_mantissa_and_exponent() {
        let spec: NewProcessor =
            serde_json::from_str(r#"{"name":"soma","language":"cmm"}"#).unwrap();
        assert_eq!(spec, NewProcessor::new("soma", Language::Cmm));

        let spec: NewProcessor =
            serde_json::from_str(r#"{"name":"f","language":"cmm","nbmant":20}"#).unwrap();
        assert_eq!((spec.nbmant, spec.nbexpo, spec.nubits), (20, 6, 27));

        let spec: NewProcessor =
            serde_json::from_str(r#"{"name":"f","language":"cmm","nbexpo":8,"nubits":30}"#)
                .unwrap();
        assert_eq!((spec.nbexpo, spec.nubits), (8, 30), "nubits dado vale");

        let spec: NewProcessor = serde_json::from_str(
            r#"{"name":"f","language":"cpp","input_ports":2,"output_ports":0,"ndstac":4}"#,
        )
        .unwrap();
        assert_eq!(
            (spec.input_ports, spec.output_ports, spec.ndstac),
            (2, 0, Some(4))
        );
        assert!(spec.sdepth.is_none());

        // O JSON de saída continua completo, e volta igual.
        let json = serde_json::to_string(&spec).unwrap();
        let back: NewProcessor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, spec);
        assert!(serde_json::from_str::<NewProcessor>(r#"{"name":"f"}"#).is_err());
    }
}
