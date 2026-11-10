//! VHDL: classificar arquivos, achar as entidades e as instâncias, ler os
//! genéricos e as portas e gerar os arquivos-modelo, sem rodar ferramenta.
//!
//! - [`classify`]: sintetizável ou testbench, pelo conteúdo, com os mesmos
//!   pesos e o mesmo limiar de [`crate::verilog::classify`];
//! - [`entities_in`]: os nomes das entidades declaradas num texto;
//! - [`entities`]: as entidades com os genéricos e as portas;
//! - [`instances_in`]: as instâncias de entidade e de componente das
//!   arquiteturas;
//! - [`design_roots`]: as entidades que nenhuma arquitetura instancia;
//! - [`entity_template`] e [`testbench_template`]: os arquivos que
//!   [`Project::add_hdl`](crate::Project::add_hdl) cria;
//! - [`validate_identifier`]: um nome que serve de entidade.
//!
//! O VHDL não diferencia maiúsculas de minúsculas, e as comparações de nome
//! daqui também não. Os nomes devolvidos ficam como estão no arquivo.
//!
//! ```
//! use aurora_core::FileRole;
//! use aurora_core::vhdl::{classify, entities_in};
//!
//! let tb = "entity and_tb is end entity;\n\
//!           architecture sim of and_tb is begin\n\
//!             process begin wait for 10 ns; std.env.finish; end process;\n\
//!           end architecture;";
//! assert_eq!(classify(tb, "and_tb.vhd"), FileRole::Testbench);
//! assert_eq!(entities_in("entity A is port (x : in bit); end; -- entity b is"), ["A"]);
//! ```

use crate::error::{AuroraError, Result};
use crate::files::FileRole;

/// O modo (a direção) de uma porta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mode {
    /// `in`, também quando a porta não diz o modo.
    In,
    /// `out`.
    Out,
    /// `inout`.
    Inout,
    /// `buffer`: saída que a própria entidade também lê.
    Buffer,
    /// `linkage`.
    Linkage,
}

impl Mode {
    fn parse(word: &str) -> Option<Mode> {
        match word {
            "in" => Some(Mode::In),
            "out" => Some(Mode::Out),
            "inout" => Some(Mode::Inout),
            "buffer" => Some(Mode::Buffer),
            "linkage" => Some(Mode::Linkage),
            _ => None,
        }
    }
}

/// Uma porta de uma entidade.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Port {
    /// O nome, como está no arquivo.
    pub name: String,
    /// O modo.
    pub mode: Mode,
    /// O tipo, como está no arquivo (`std_logic_vector(WIDTH - 1 downto 0)`),
    /// com os espaços em volta de cada palavra reduzidos a um.
    pub type_text: String,
}

/// Um genérico de uma entidade.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Generic {
    /// O nome.
    pub name: String,
    /// O tipo (`positive`).
    pub type_text: String,
    /// O valor padrão (`8`), se a entidade dá um.
    pub default: Option<String>,
}

/// Uma entidade e o cabeçalho dela, na ordem da declaração.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Entity {
    /// O nome, como está no arquivo.
    pub name: String,
    /// Os genéricos.
    pub generics: Vec<Generic>,
    /// As portas.
    pub ports: Vec<Port>,
    /// A linha da declaração (`entity <nome> is`), a partir de 1.
    pub line: u32,
}

/// Como uma arquitetura instancia outra unidade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InstanceKind {
    /// `rotulo : entity work.<nome>`.
    Entity,
    /// `rotulo : [component] <nome> port map (...)`: o componente se liga à
    /// entidade de mesmo nome na elaboração.
    Component,
    /// `rotulo : configuration work.<nome>`.
    Configuration,
}

/// Uma instância dentro de uma arquitetura.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Instance {
    /// O rótulo (`dut`).
    pub label: String,
    /// A unidade instanciada, sem a biblioteca (`contador` em
    /// `entity work.contador`).
    pub unit: String,
    /// Entidade, componente ou configuração.
    pub kind: InstanceKind,
    /// A entidade da arquitetura onde a instância está.
    pub within: String,
    /// A linha do rótulo, a partir de 1.
    pub line: u32,
}

// ------------------------------------------------------------------ texto

/// O texto sem comentários (`--` e `/* */`), sem o conteúdo das strings e
/// sem os literais de caractere (`'1'`), trocados por espaço, com as
/// quebras de linha no lugar: as palavras dentro deles não contam e as
/// posições continuam as do arquivo. O apóstrofo de um atributo
/// (`clk'event`) e de uma expressão qualificada (`unsigned'(...)`) fica.
fn strip(src: &str) -> Vec<char> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = chars.clone();
    let blank = |out: &mut Vec<char>, from: usize, to: usize| {
        for c in out.iter_mut().take(to).skip(from) {
            if *c != '\n' {
                *c = ' ';
            }
        }
    };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '-' && next == Some('-') {
            let start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            blank(&mut out, start, i);
        } else if c == '/' && next == Some('*') {
            let start = i;
            i += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                i += 1;
            }
            i = (i + 2).min(chars.len());
            blank(&mut out, start, i);
        } else if c == '"' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != '\n' {
                if chars[i] == '"' {
                    if chars.get(i + 1) == Some(&'"') {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            blank(&mut out, start, i);
        } else if c == '\'' {
            let previous = i.checked_sub(1).map(|p| chars[p]);
            let tick =
                previous.is_some_and(|p| p.is_alphanumeric() || matches!(p, '_' | ')' | ']'));
            if !tick && chars.get(i + 2) == Some(&'\'') {
                blank(&mut out, i, i + 3);
                i += 3;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Word,
    Number,
    Punct,
}

#[derive(Debug, Clone)]
struct Token {
    kind: Kind,
    text: String,
    lower: String,
    line: u32,
    start: usize,
    end: usize,
}

/// As palavras, os números e os sinais do texto já sem comentários.
fn tokens(chars: &[char]) -> Vec<Token> {
    let mut out = Vec::new();
    let mut line = 1;
    let mut i = 0;
    let n = chars.len();
    let mut push = |kind: Kind, start: usize, end: usize, line: u32| {
        let text: String = chars[start..end].iter().collect();
        let lower = text.to_lowercase();
        out.push(Token {
            kind,
            text,
            lower,
            line,
            start,
            end,
        });
    };
    while i < n {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c.is_alphabetic() {
            let start = i;
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            push(Kind::Word, start, i, line);
        } else if c == '\\' {
            // Identificador estendido: `\nome qualquer\`.
            let start = i;
            i += 1;
            while i < n && chars[i] != '\\' && chars[i] != '\n' {
                i += 1;
            }
            i = (i + 1).min(n);
            push(Kind::Word, start, i, line);
        } else if c.is_ascii_digit() {
            let start = i;
            while i < n && (chars[i].is_ascii_alphanumeric() || matches!(chars[i], '_' | '.' | '#'))
            {
                i += 1;
            }
            push(Kind::Number, start, i, line);
        } else {
            let pair: String = chars[i..(i + 2).min(n)].iter().collect();
            let width = if matches!(
                pair.as_str(),
                "=>" | "<=" | ":=" | ">=" | "/=" | "**" | "<>"
            ) {
                2
            } else {
                1
            };
            push(Kind::Punct, i, i + width, line);
            i += width;
        }
    }
    out
}

/// O que um arquivo declara e instancia.
struct Parsed {
    chars: Vec<char>,
    tokens: Vec<Token>,
}

impl Parsed {
    fn new(text: &str) -> Parsed {
        let chars = strip(text);
        let tokens = tokens(&chars);
        Parsed { chars, tokens }
    }

    fn word(&self, i: usize) -> Option<&str> {
        self.tokens
            .get(i)
            .filter(|t| t.kind == Kind::Word)
            .map(|t| t.lower.as_str())
    }

    fn is(&self, i: usize, text: &str) -> bool {
        self.tokens.get(i).is_some_and(|t| t.lower == text)
    }

    /// O texto entre os tokens `from` e `to` (inclusive), com os espaços
    /// reduzidos a um.
    fn text(&self, from: usize, to: usize) -> String {
        let (start, end) = (self.tokens[from].start, self.tokens[to].end);
        let raw: String = self.chars[start..end].iter().collect();
        raw.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// O índice do `)` que fecha o `(` em `open`.
    fn closing(&self, open: usize) -> Option<usize> {
        let mut depth = 0;
        for (i, t) in self.tokens.iter().enumerate().skip(open) {
            if t.kind == Kind::Punct && t.text == "(" {
                depth += 1;
            } else if t.kind == Kind::Punct && t.text == ")" {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
        None
    }

    fn entities(&self) -> Vec<Entity> {
        let mut out = Vec::new();
        for i in 0..self.tokens.len() {
            if !self.is(i, "entity") || !self.is(i + 2, "is") {
                continue;
            }
            // `end entity` e `rotulo : entity work.x` não declaram.
            if i > 0 && (self.is(i - 1, "end") || self.is(i - 1, ":")) {
                continue;
            }
            let Some(name) = self.tokens.get(i + 1).filter(|t| t.kind == Kind::Word) else {
                continue;
            };
            let mut entity = Entity {
                name: name.text.clone(),
                generics: Vec::new(),
                ports: Vec::new(),
                line: self.tokens[i].line,
            };
            let mut j = i + 3;
            while j < self.tokens.len() {
                if self.is(j, "begin") || self.is(j, "end") {
                    break;
                }
                let header = self.word(j).filter(|w| *w == "generic" || *w == "port");
                if let Some(header) = header
                    && self.is(j + 1, "(")
                    && let Some(close) = self.closing(j + 1)
                {
                    let elements = self.interface_list(j + 2, close);
                    if header == "generic" {
                        entity.generics = elements
                            .into_iter()
                            .map(|e| Generic {
                                name: e.name,
                                type_text: e.type_text,
                                default: e.default,
                            })
                            .collect();
                    } else {
                        entity.ports = elements
                            .into_iter()
                            .map(|e| Port {
                                name: e.name,
                                mode: e.mode.unwrap_or(Mode::In),
                                type_text: e.type_text,
                            })
                            .collect();
                    }
                    j = close + 1;
                } else {
                    j += 1;
                }
            }
            out.push(entity);
        }
        out
    }

    /// Os elementos de uma lista de interface, entre os tokens `from` e
    /// `to` (o `)` que a fecha, de fora).
    fn interface_list(&self, from: usize, to: usize) -> Vec<Element> {
        let mut out = Vec::new();
        let mut start = from;
        let mut depth = 0;
        for i in from..=to {
            let t = &self.tokens[i];
            if t.kind == Kind::Punct && t.text == "(" {
                depth += 1;
            } else if t.kind == Kind::Punct && t.text == ")" && i < to {
                depth -= 1;
            }
            let end_of_element = i == to || (depth == 0 && t.kind == Kind::Punct && t.text == ";");
            if end_of_element {
                if i > start {
                    out.extend(self.element(start, i));
                }
                start = i + 1;
            }
        }
        out
    }

    /// `[classe] a, b : [modo] tipo [:= padrão]`, entre `from` e `to`
    /// (exclusive). Os genéricos de tipo, de pacote e de subprograma do
    /// VHDL-2008 ficam de fora.
    fn element(&self, from: usize, to: usize) -> Vec<Element> {
        let mut i = from;
        if matches!(
            self.word(i),
            Some("type" | "package" | "function" | "procedure" | "impure" | "pure")
        ) {
            return Vec::new();
        }
        if matches!(
            self.word(i),
            Some("signal" | "constant" | "variable" | "file")
        ) {
            i += 1;
        }
        let mut names = Vec::new();
        while i < to && !self.is(i, ":") {
            if self.tokens[i].kind == Kind::Word {
                names.push(self.tokens[i].text.clone());
            }
            i += 1;
        }
        if i >= to {
            return Vec::new();
        }
        i += 1;
        let mode = self.word(i).and_then(Mode::parse);
        if mode.is_some() {
            i += 1;
        }
        let assign = (i..to).find(|&k| self.is(k, ":="));
        let type_end = assign.unwrap_or(to);
        if type_end <= i {
            return Vec::new();
        }
        let mut type_text = self.text(i, type_end - 1);
        if let Some(stripped) = type_text.strip_suffix(" bus") {
            type_text = stripped.to_owned();
        }
        let default = assign
            .filter(|&a| a + 1 < to)
            .map(|a| self.text(a + 1, to - 1));
        names
            .into_iter()
            .map(|name| Element {
                name,
                mode,
                type_text: type_text.clone(),
                default: default.clone(),
            })
            .collect()
    }

    fn instances(&self) -> Vec<Instance> {
        let mut out = Vec::new();
        let mut within = String::new();
        for i in 0..self.tokens.len() {
            if self.is(i, "architecture") && self.is(i + 2, "of") && self.is(i + 4, "is") {
                within = self
                    .tokens
                    .get(i + 3)
                    .map(|t| t.text.clone())
                    .unwrap_or_default();
                continue;
            }
            let label = &self.tokens[i];
            if label.kind != Kind::Word || !self.is(i + 1, ":") {
                continue;
            }
            // Um comando concorrente começa depois de `;`, `begin` ou
            // `generate`; `signal a : t;` e as listas de portas, não.
            let starts = i == 0
                || self.is(i - 1, ";")
                || self.is(i - 1, "begin")
                || self.is(i - 1, "generate");
            if !starts {
                continue;
            }
            let (kind, mut j) = match self.word(i + 2) {
                Some("entity") => (InstanceKind::Entity, i + 3),
                Some("configuration") => (InstanceKind::Configuration, i + 3),
                Some("component") => (InstanceKind::Component, i + 3),
                Some(w) if !is_reserved(w) => (InstanceKind::Component, i + 2),
                _ => continue,
            };
            // `work.contador`: a unidade é o último nome.
            let mut unit = None;
            while let Some(t) = self.tokens.get(j).filter(|t| t.kind == Kind::Word) {
                unit = Some(t.text.clone());
                if self.is(j + 1, ".") {
                    j += 2;
                } else {
                    j += 1;
                    break;
                }
            }
            let Some(unit) = unit else { continue };
            if kind == InstanceKind::Entity && self.is(j, "(") {
                j = self.closing(j).map_or(j, |c| c + 1);
            }
            let maps = matches!(self.word(j), Some("generic" | "port")) && self.is(j + 1, "map");
            let ends = self.is(j, ";");
            // Sem `entity` nem `component`, só um `port map` diz que é instância.
            if kind == InstanceKind::Component && !self.is(i + 2, "component") && !maps {
                continue;
            }
            if !maps && !ends {
                continue;
            }
            out.push(Instance {
                label: label.text.clone(),
                unit,
                kind,
                within: within.clone(),
                line: label.line,
            });
        }
        out
    }
}

struct Element {
    name: String,
    mode: Option<Mode>,
    type_text: String,
    default: Option<String>,
}

// -------------------------------------------------------------- consultas

/// Os nomes das entidades declaradas em `text`, na ordem.
pub fn entities_in(text: &str) -> Vec<String> {
    Parsed::new(text)
        .entities()
        .into_iter()
        .map(|e| e.name)
        .collect()
}

/// As entidades de `text`, com os genéricos e as portas.
///
/// ```
/// use aurora_core::vhdl::{Mode, entities};
///
/// let text = "entity contador is\n\
///               generic (WIDTH : positive := 8);\n\
///               port (clk, rst : in std_logic; q : out std_logic_vector(WIDTH - 1 downto 0));\n\
///             end entity;";
/// let found = entities(text);
/// assert_eq!(found[0].generics[0].default.as_deref(), Some("8"));
/// assert_eq!(found[0].ports.len(), 3);
/// assert_eq!(found[0].ports[2].mode, Mode::Out);
/// assert_eq!(found[0].ports[2].type_text, "std_logic_vector(WIDTH - 1 downto 0)");
/// ```
pub fn entities(text: &str) -> Vec<Entity> {
    Parsed::new(text).entities()
}

/// As instâncias das arquiteturas de `text`: de entidade
/// (`u : entity work.x`), de componente (`u : x port map (...)`) e de
/// configuração.
pub fn instances_in(text: &str) -> Vec<Instance> {
    Parsed::new(text).instances()
}

/// As entidades declaradas em `texts` que nenhuma arquitetura deles
/// instancia, na ordem da declaração: as raízes do design. Um componente
/// conta como instância da entidade de mesmo nome.
pub fn design_roots(texts: &[&str]) -> Vec<String> {
    let parsed: Vec<Parsed> = texts.iter().map(|t| Parsed::new(t)).collect();
    let used: std::collections::HashSet<String> = parsed
        .iter()
        .flat_map(Parsed::instances)
        .filter(|i| i.kind != InstanceKind::Configuration)
        .map(|i| i.unit.to_lowercase())
        .collect();
    let mut roots: Vec<String> = Vec::new();
    for entity in parsed.iter().flat_map(Parsed::entities) {
        let lower = entity.name.to_lowercase();
        if !used.contains(&lower) && !roots.iter().any(|r| r.to_lowercase() == lower) {
            roots.push(entity.name);
        }
    }
    roots
}

// -------------------------------------------------------------- classificar

/// Pontuação a partir da qual o arquivo é testbench, a do Verilog.
const TESTBENCH_THRESHOLD: u32 = 3;

/// Sintetizável ou testbench, pelo conteúdo, com os pesos do Verilog
/// ([`crate::verilog::classify`]) traduzidos: +3 para uma entidade sem
/// portas, +3 para `std.env.finish` ou `stop`, +2 para `wait for`, +1 para
/// `report`, +1 para uma atribuição com `after` (o relógio de um testbench)
/// e +2 para `tb`, `test` ou `testbench` no nome do arquivo. Com 3 ou mais,
/// testbench; na dúvida, sintetizável.
pub fn classify(text: &str, file_name: &str) -> FileRole {
    let parsed = Parsed::new(text);
    let mut score = 0;
    if parsed.entities().iter().any(|e| e.ports.is_empty()) {
        score += 3;
    }
    let words: Vec<&str> = parsed.tokens.iter().map(|t| t.lower.as_str()).collect();
    let calls = |name: &str| {
        words.iter().enumerate().any(|(i, w)| {
            *w == name
                && matches!(words.get(i + 1), Some(&";") | Some(&"("))
                && (i == 0
                    || matches!(words[i - 1], ";" | "." | "begin" | "then" | "else" | "loop"))
        })
    };
    if calls("finish") || calls("stop") {
        score += 3;
    }
    if words.windows(2).any(|w| w[0] == "wait" && w[1] == "for") {
        score += 2;
    }
    if words.contains(&"report") {
        score += 1;
    }
    if words.contains(&"after") {
        score += 1;
    }
    if crate::verilog::name_suggests_testbench(file_name) {
        score += 2;
    }
    if score >= TESTBENCH_THRESHOLD {
        FileRole::Testbench
    } else {
        FileRole::Synthesizable
    }
}

// ----------------------------------------------------------------- nomes

/// As palavras reservadas do VHDL-2008 e do VHDL-2019.
const RESERVED: &[&str] = &[
    "abs",
    "access",
    "after",
    "alias",
    "all",
    "and",
    "architecture",
    "array",
    "assert",
    "assume",
    "assume_guarantee",
    "attribute",
    "begin",
    "block",
    "body",
    "buffer",
    "bus",
    "case",
    "component",
    "configuration",
    "constant",
    "context",
    "cover",
    "default",
    "disconnect",
    "downto",
    "else",
    "elsif",
    "end",
    "entity",
    "exit",
    "fairness",
    "file",
    "for",
    "force",
    "function",
    "generate",
    "generic",
    "group",
    "guarded",
    "if",
    "impure",
    "in",
    "inertial",
    "inout",
    "is",
    "label",
    "library",
    "linkage",
    "literal",
    "loop",
    "map",
    "mod",
    "nand",
    "new",
    "next",
    "nor",
    "not",
    "null",
    "of",
    "on",
    "open",
    "or",
    "others",
    "out",
    "package",
    "parameter",
    "port",
    "postponed",
    "private",
    "procedure",
    "process",
    "property",
    "protected",
    "pure",
    "range",
    "record",
    "register",
    "reject",
    "release",
    "rem",
    "report",
    "restrict",
    "restrict_guarantee",
    "return",
    "rol",
    "ror",
    "select",
    "sequence",
    "severity",
    "shared",
    "signal",
    "sla",
    "sll",
    "sra",
    "srl",
    "strong",
    "subtype",
    "then",
    "to",
    "transport",
    "type",
    "unaffected",
    "units",
    "until",
    "use",
    "variable",
    "view",
    "vmode",
    "vprop",
    "vunit",
    "wait",
    "when",
    "while",
    "with",
    "xnor",
    "xor",
];

/// `word` (em minúsculas) é palavra reservada do VHDL.
fn is_reserved(word: &str) -> bool {
    RESERVED.contains(&word)
}

/// Confere se `name` serve de nome de entidade: letras sem acento, dígitos e
/// `_` simples entre eles, começando por letra, e fora das palavras
/// reservadas do VHDL. `context` diz de onde o nome vem, para a mensagem.
///
/// ```
/// use aurora_core::vhdl::validate_identifier;
///
/// assert!(validate_identifier("contador_8b", "The file name becomes the entity name").is_ok());
/// assert!(validate_identifier("_contador", "x").is_err());
/// assert!(validate_identifier("conta__dor", "x").is_err());
/// assert!(validate_identifier("Signal", "x").is_err());
/// ```
pub fn validate_identifier(name: &str, context: &str) -> Result<()> {
    let shape = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !name.contains("__")
        && !name.ends_with('_');
    if !shape {
        return Err(AuroraError::InvalidName {
            name: name.to_owned(),
            reason: format!(
                "{context}, so use only letters, digits and single '_' between them, starting with a letter"
            ),
        });
    }
    if is_reserved(&name.to_ascii_lowercase()) {
        return Err(AuroraError::InvalidName {
            name: name.to_owned(),
            reason: format!("{context}, and '{name}' is a VHDL reserved word"),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------- modelos

const LIBRARIES: &str =
    "library ieee;\nuse ieee.std_logic_1164.all;\nuse ieee.numeric_std.all;\n\n";

/// A entidade-modelo `name`, com uma entrada e uma saída de 8 bits e a saída
/// ligada à entrada, como o módulo-modelo do Verilog.
pub fn entity_template(name: &str) -> String {
    format!(
        "{LIBRARIES}\
         entity {name} is\n\
         \x20 port (\n\
         \x20   a : in  std_logic_vector(7 downto 0);\n\
         \x20   y : out std_logic_vector(7 downto 0)\n\
         \x20 );\n\
         end entity {name};\n\
         \n\
         architecture rtl of {name} is\n\
         begin\n\
         \x20 y <= a;\n\
         end architecture rtl;\n"
    )
}

/// O testbench-modelo `testbench`, em VHDL-2008. Com `dut`, instancia a
/// entidade por `entity work.<nome>`, com uma constante para cada genérico,
/// um sinal para cada porta (entradas com valor inicial), o relógio de 10 ns
/// numa entrada de relógio (`clk`, `clock`, ...) e o reset ativo nos
/// primeiros 20 ns. Termina com `std.env.finish`, que a AURORA conta como
/// fim normal da simulação.
pub fn testbench_template(testbench: &str, dut: Option<&Entity>) -> String {
    let mut out = format!(
        "{LIBRARIES}entity {testbench} is\nend entity {testbench};\n\narchitecture sim of {testbench} is\n"
    );
    let Some(dut) = dut else {
        out.push_str(
            "begin\n\
             \x20 stimulus : process\n\
             \x20 begin\n\
             \x20   wait for 100 ns;\n\
             \x20   std.env.finish;\n\
             \x20 end process stimulus;\n\
             end architecture sim;\n",
        );
        return out;
    };

    for generic in &dut.generics {
        let value = generic
            .default
            .clone()
            .unwrap_or_else(|| format!("{}'left", generic.type_text));
        out.push_str(&format!(
            "  constant {} : {} := {value};\n",
            generic.name, generic.type_text
        ));
    }
    if !dut.generics.is_empty() {
        out.push('\n');
    }
    let width = dut.ports.iter().map(|p| p.name.len()).max().unwrap_or(0);
    let clock = dut.ports.iter().find(|p| {
        p.mode == Mode::In && crate::verilog::is_clock(&p.name) && is_bit_type(&p.type_text)
    });
    let reset = dut.ports.iter().find_map(|p| {
        let active_high = crate::verilog::reset_polarity(&p.name)?;
        (p.mode == Mode::In && is_bit_type(&p.type_text)).then_some((p, active_high))
    });
    for port in &dut.ports {
        let initial = if port.mode != Mode::In {
            String::new()
        } else if reset.is_some_and(|(r, _)| r.name == port.name) {
            let active = reset.is_some_and(|(_, high)| high);
            format!(" := '{}'", if active { 1 } else { 0 })
        } else if is_bit_type(&port.type_text) {
            " := '0'".to_owned()
        } else if is_vector_type(&port.type_text) {
            " := (others => '0')".to_owned()
        } else {
            String::new()
        };
        out.push_str(&format!(
            "  signal {:<width$} : {}{initial};\n",
            port.name, port.type_text
        ));
    }
    out.push_str("begin\n");

    out.push_str(&format!("  dut : entity work.{}\n", dut.name));
    if !dut.generics.is_empty() {
        let maps: Vec<String> = dut
            .generics
            .iter()
            .map(|g| format!("{0} => {0}", g.name))
            .collect();
        out.push_str(&format!("    generic map ({})\n", maps.join(", ")));
    }
    if dut.ports.is_empty() {
        out.push_str("    ;\n\n");
    } else {
        out.push_str("    port map (\n");
        for (i, port) in dut.ports.iter().enumerate() {
            let comma = if i + 1 < dut.ports.len() { "," } else { "" };
            out.push_str(&format!(
                "      {:<width$} => {}{comma}\n",
                port.name, port.name
            ));
        }
        out.push_str("    );\n\n");
    }
    if let Some(clk) = clock {
        out.push_str(&format!(
            "  -- Relógio de 10 ns.\n  {0} <= not {0} after 5 ns;\n\n",
            clk.name
        ));
    }

    out.push_str("  stimulus : process\n  begin\n");
    if let Some((rst, active_high)) = reset {
        out.push_str(&format!(
            "    wait for 20 ns;\n    {} <= '{}';\n",
            rst.name,
            if active_high { 0 } else { 1 }
        ));
    }
    out.push_str(
        "    wait for 100 ns;\n\
         \x20   std.env.finish;\n\
         \x20 end process stimulus;\n\
         end architecture sim;\n",
    );
    out
}

/// Um tipo de um bit que aceita `'0'` e `not`.
fn is_bit_type(type_text: &str) -> bool {
    matches!(
        type_text.to_ascii_lowercase().as_str(),
        "std_logic" | "std_ulogic" | "bit"
    )
}

/// Um vetor que aceita `(others => '0')`.
fn is_vector_type(type_text: &str) -> bool {
    let lower = type_text.to_ascii_lowercase();
    [
        "std_logic_vector",
        "std_ulogic_vector",
        "bit_vector",
        "unsigned",
        "signed",
    ]
    .iter()
    .any(|t| lower.starts_with(&format!("{t}(")) || lower.starts_with(&format!("{t} (")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTADOR: &str = "\
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

-- entity comentada is end;
entity Contador is
  generic (WIDTH : positive := 8; RESET_VALUE : natural := 0);
  port (
    clk : in  std_logic;
    rst : in  std_logic;  -- reset síncrono
    en  : in  std_logic := '1';
    q   : out std_logic_vector(WIDTH - 1 downto 0)
  );
end entity Contador;

architecture rtl of contador is
  type state_t is (IDLE, COUNTING);
  signal state : state_t;
  signal count : unsigned(WIDTH - 1 downto 0);
begin
  process (clk)
  begin
    if clk'event and clk = '1' then
      report \"entity x is\";
    end if;
  end process;
  q <= std_logic_vector(count);
end architecture rtl;
";

    #[test]
    fn reads_the_entity_header() {
        let found = entities(CONTADOR);
        assert_eq!(
            found.len(),
            1,
            "o comentário e a string não declaram: {found:?}"
        );
        let e = &found[0];
        assert_eq!(e.name, "Contador");
        assert_eq!(e.line, 6);
        let generics: Vec<_> = e
            .generics
            .iter()
            .map(|g| (g.name.as_str(), g.default.as_deref()))
            .collect();
        assert_eq!(generics, [("WIDTH", Some("8")), ("RESET_VALUE", Some("0"))]);
        let ports: Vec<_> = e
            .ports
            .iter()
            .map(|p| (p.name.as_str(), p.mode, p.type_text.as_str()))
            .collect();
        assert_eq!(
            ports,
            [
                ("clk", Mode::In, "std_logic"),
                ("rst", Mode::In, "std_logic"),
                ("en", Mode::In, "std_logic"),
                ("q", Mode::Out, "std_logic_vector(WIDTH - 1 downto 0)"),
            ]
        );
    }

    #[test]
    fn finds_instances_of_entities_and_components() {
        let top = "\
architecture rtl of topo is
  component somador port (a, b : in bit; s : out bit); end component;
  signal a, b, s : bit;
begin
  u0 : entity work.contador(rtl) generic map (WIDTH => 4) port map (clk => clk, q => open);
  u1 : somador port map (a, b, s);
  u2 : component somador port map (a => a, b => b, s => s);
  p0 : process begin wait; end process;
  g : for i in 0 to 3 generate
    u3 : entity work.celula port map (x => a);
  end generate;
end architecture;
";
        let found: Vec<_> = instances_in(top)
            .into_iter()
            .map(|i| (i.label, i.unit, i.kind, i.within))
            .collect();
        let topo = "topo".to_owned();
        assert_eq!(
            found,
            [
                (
                    "u0".to_owned(),
                    "contador".to_owned(),
                    InstanceKind::Entity,
                    topo.clone()
                ),
                (
                    "u1".to_owned(),
                    "somador".to_owned(),
                    InstanceKind::Component,
                    topo.clone()
                ),
                (
                    "u2".to_owned(),
                    "somador".to_owned(),
                    InstanceKind::Component,
                    topo.clone()
                ),
                (
                    "u3".to_owned(),
                    "celula".to_owned(),
                    InstanceKind::Entity,
                    topo
                ),
            ]
        );
    }

    #[test]
    fn roots_are_the_entities_nobody_instantiates() {
        let tb = "entity contador_tb is end;\narchitecture sim of contador_tb is begin\n  dut : entity work.CONTADOR port map (clk => clk);\nend;";
        assert_eq!(design_roots(&[CONTADOR, tb]), ["contador_tb"]);
        assert_eq!(design_roots(&[CONTADOR]), ["Contador"]);
    }

    #[test]
    fn classification_follows_the_verilog_weights() {
        assert_eq!(classify(CONTADOR, "contador.vhd"), FileRole::Synthesizable);
        let tb = "entity t is end;\narchitecture a of t is begin\n  process begin\n    wait for 10 ns;\n    std.env.finish;\n  end process;\nend;";
        assert_eq!(classify(tb, "t.vhd"), FileRole::Testbench);
        // Só o nome não basta, como no Verilog.
        assert_eq!(
            classify(&entity_template("c"), "c_tb.vhd"),
            FileRole::Synthesizable
        );
        // `finish` como nome de sinal não conta.
        let signal = "entity f is port (finish : out bit); end;\narchitecture a of f is begin finish <= '1'; end;";
        assert_eq!(classify(signal, "f.vhd"), FileRole::Synthesizable);
    }

    #[test]
    fn identifiers_follow_the_vhdl_rule() {
        for ok in ["a", "contador_8b", "Alu"] {
            assert!(validate_identifier(ok, "c").is_ok(), "{ok}");
        }
        for bad in ["8bits", "_a", "a_", "a__b", "and", "Process", "çao", "a-b"] {
            assert!(validate_identifier(bad, "c").is_err(), "{bad}");
        }
    }

    #[test]
    fn templates_are_classified_as_intended() {
        let entity = entity_template("alu");
        assert_eq!(classify(&entity, "alu.vhd"), FileRole::Synthesizable);
        assert_eq!(entities(&entity)[0].ports.len(), 2);
        let alone = testbench_template("alu_tb", None);
        assert_eq!(classify(&alone, "alu_tb.vhd"), FileRole::Testbench);
        let dut = &entities(CONTADOR)[0];
        let tb = testbench_template("contador_tb", Some(dut));
        assert_eq!(classify(&tb, "contador_tb.vhd"), FileRole::Testbench);
        assert!(tb.contains("constant WIDTH : positive := 8;"), "{tb}");
        assert!(tb.contains("dut : entity work.Contador"), "{tb}");
        assert!(
            tb.contains("generic map (WIDTH => WIDTH, RESET_VALUE => RESET_VALUE)"),
            "{tb}"
        );
        assert!(tb.contains("clk <= not clk after 5 ns;"), "{tb}");
        assert!(tb.contains("signal rst : std_logic := '1';"), "{tb}");
        assert!(tb.contains("rst <= '0';"), "{tb}");
        assert!(
            tb.contains("signal q   : std_logic_vector(WIDTH - 1 downto 0);"),
            "{tb}"
        );
        assert_eq!(
            instances_in(&tb)
                .iter()
                .map(|i| i.unit.as_str())
                .collect::<Vec<_>>(),
            ["Contador"]
        );
        assert_eq!(design_roots(&[CONTADOR, &tb]), ["contador_tb"]);
    }
}
