//! Tradução da saída das ferramentas em [`Diagnostic`]s.
//!
//! Cada ferramenta tem seu dialeto (todos conferidos executando-as):
//!
//! - `cmmcomp` e `asmcomp` (e o `appcomp` dos relatórios antigos) escrevem
//!   frases em inglês (a AURORA sempre passa `--lang=en`) com a linha no meio
//!   do texto e sem nome de arquivo:
//!   `Error on line 42: there's no variable 'x'!`,
//!   `Syntax error on line 3. You're a confused soul!`,
//!   `Heads up on line 7: ...`, `Info: ...`. O `asmcomp` cita arquivos de
//!   dados: `Error: line 4 of file 'x.txt' isn't a valid integer!`.
//! - `cppcomp`, `iverilog` e o compilador C++ que o Verilator chama usam o
//!   formato de compilador C: `<arquivo>:<linha>[:<coluna>]: error: ...`. O
//!   Icarus também escreve `<arquivo>:<linha>: syntax error`.
//!   `cppcomp` emite `<ferramenta>: <mensagem>`.
//! - Verilator: `%Error: <arquivo>:<linha>:<coluna>: ...`,
//!   `%Warning-<CÓDIGO>: ...`, seguidos de linhas indentadas com o trecho do
//!   fonte, que são anexadas ao diagnóstico anterior.
//! - Yosys: `<arquivo>:<linha>: ERROR: ...`, `Warning: ...`.
//! - Graphviz (`dot`): `Error: <arquivo>: syntax error in line <n> ...`.
//! - Quartus (conferido em relatórios do 13.1, do 17.1 Lite e do 18.0
//!   Lite, não executado): `Error (10161): <mensagem> File: <arquivo> Line:
//!   <n>`, `Critical Warning (332148): ...`, `Warning (18236): ...`. As
//!   submensagens vêm indentadas em 4 espaços e são anexadas à anterior; as
//!   centenas de `Info (...)` ficam de fora.
//!
//! O parser é tolerante: linha não reconhecida vira um diagnóstico com
//! `severity = Unknown`, e todo diagnóstico guarda o texto original em `raw`.
//! A exceção é o stdout de ferramentas cujo stdout é saída do programa e não
//! mensagem (o `$display` de uma simulação, o log do Yosys): ali só entram as
//! linhas reconhecidas, e o texto completo continua no relatório do passo.

use camino::{Utf8Path, Utf8PathBuf};
use schemars::JsonSchema;
use serde::Serialize;

use crate::control::Stream;
use crate::toolchain::Tool;

/// Gravidade de um [`Diagnostic`]. A ordem (`Error < Warning < Info <
/// Unknown`) serve para ordenar do mais grave ao menos grave.
///
/// Em JSON: `"error"`, `"warning"`, `"info"`, `"unknown"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Severity {
    /// A ferramenta recusou a entrada.
    Error,
    /// A ferramenta aceitou, mas avisou (`Heads up on line`, `%Warning-`).
    Warning,
    /// Mensagem informativa (`Info:`, `Sweet:`, resumos de contagem).
    Info,
    /// Linha que o parser não reconheceu.
    Unknown,
}

/// Uma mensagem de uma ferramenta, com os campos que deu para extrair.
///
/// `file`, `line` e `column` são opcionais porque o `cmmcomp` não informa
/// arquivo nem coluna, e várias mensagens não informam nem a linha. Quando a
/// ferramenta não cita o arquivo mas a AURORA sabe qual é (o `cmmcomp` compila
/// um fonte só), `file` vem preenchido pela AURORA.
///
/// Em JSON:
///
/// ```json
/// { "tool": "cmmcomp", "severity": "error",
///   "message": "c'mon dude, declare the variable 'total' properly!",
///   "file": "/p/conta/conta.cmm", "line": 15, "column": null,
///   "raw": "Error on line 15: c'mon dude, declare the variable 'total' properly!" }
/// ```
///
/// No fluxo C, o `cppcomp` cita o arquivo e a linha do fonte, ou do
/// cabeçalho incluído onde está o erro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Diagnostic {
    /// A ferramenta que escreveu a mensagem.
    pub tool: Tool,
    /// A gravidade.
    pub severity: Severity,
    /// A mensagem, sem o prefixo de gravidade e de localização. Vem no idioma
    /// da ferramenta (inglês: a AURORA sempre pede inglês ao YANC).
    pub message: String,
    /// O arquivo a que a mensagem se refere, como a ferramenta o escreveu
    /// (normalmente absoluto, porque a AURORA passa caminhos absolutos).
    // No Windows, um caminho com acento vai relativo ao CWD da ferramenta
    // (`process::tool_path`), e o que ela escreve assim é resolvido a partir
    // do CWD do passo (`pipeline::resolve_relative_files`).
    #[schemars(with = "Option<String>")]
    pub file: Option<Utf8PathBuf>,
    /// Linha, a partir de 1.
    pub line: Option<u32>,
    /// Coluna, a partir de 1 (Verilator e compilador C++).
    pub column: Option<u32>,
    /// O texto exatamente como a ferramenta o escreveu (várias linhas quando
    /// a ferramenta continua a mensagem em linhas indentadas).
    pub raw: String,
}

/// Arruma os diagnósticos de uma operação para quem os lê. Os avisos sobre
/// arquivos do bundle (`bundle`, a raiz dele), que o usuário não escreve nem
/// conserta, como os da biblioteca SAPHO, viram informação, que a CLI e o
/// Problemas do Studio só mostram quando pedidos. E as repetições saem: cada
/// elaboração (o design e cada testbench) repete os mesmos avisos. Num
/// projeto com um contador e dois processadores, o `check` dava 218
/// diagnósticos, 106 deles avisos `timescale for ... inherited from another
/// file` da biblioteca, e os do usuário sumiam no meio.
///
/// O Verilog que o YANC gera no projeto continua com os avisos: o testbench
/// gerado é também onde a simulação de um processador avisa que o programa
/// leu mais valores do que a entrada tem.
pub(crate) fn tidy(diagnostics: &mut Vec<Diagnostic>, bundle: &Utf8Path) {
    for d in diagnostics.iter_mut() {
        if d.severity == Severity::Warning
            && d.file.as_deref().is_some_and(|f| f.starts_with(bundle))
        {
            d.severity = Severity::Info;
        }
    }
    let mut seen = std::collections::HashSet::new();
    diagnostics.retain(|d| {
        seen.insert((
            d.tool,
            d.severity,
            d.file.clone(),
            d.line,
            d.column,
            d.message.clone(),
        ))
    });
}

/// Interpreta a saída de uma execução.
///
/// `source` é o arquivo que o passo compilou; ele vira o `file` das mensagens
/// com número de linha que não citam arquivo (caso do `cmmcomp`, que compila
/// um único arquivo, já que C± não tem `#include`).
pub(crate) fn parse(
    tool: Tool,
    stdout: &str,
    stderr: &str,
    source: Option<&Utf8Path>,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    // stderr primeiro: é onde ficam os erros, que o usuário quer ver no topo.
    parse_stream(tool, stderr, source, true, &mut out);
    parse_stream(tool, stdout, source, stdout_is_messages(tool), &mut out);
    out
}

/// Se `line`, escrita por `tool` em `stream`, volta como [`Diagnostic`] no
/// resultado: tudo do stderr, todo o stdout dos compiladores e, no stdout de
/// uma simulação, só as linhas que o simulador escreve por conta própria
/// (`$finish called at`, `VCD info:`, `ERROR:` do `$error`). O resto é saída
/// do programa, como o `$display` do testbench.
///
/// Decide linha a linha: a continuação indentada de uma mensagem do Icarus ou
/// do Verilator conta como saída comum.
pub(crate) fn is_message(tool: Tool, stream: Stream, line: &str) -> bool {
    !line.trim().is_empty()
        && !is_quartus_noise(tool, line)
        && (stream == Stream::Stderr
            || stdout_is_messages(tool)
            || parse_line(tool, line, None).is_some()
            || (tool == Tool::Vvp && is_vvp_continuation(line))
            || (tool == Tool::Ghdl && is_ghdl_continuation(line)))
}

/// `  instance: /tb/P0`: a linha que o GHDL escreve depois do
/// `ghdl:error: assertion failed`.
fn is_ghdl_continuation(line: &str) -> bool {
    line.starts_with(char::is_whitespace) && line.trim_start().starts_with("instance:")
}

/// `       Time: 1000  Scope: tb`: a segunda linha que o `vvp` escreve
/// depois do `ERROR:` de um `$error` (ou `$fatal`, `$warning`).
fn is_vvp_continuation(line: &str) -> bool {
    line.starts_with(char::is_whitespace)
        && line.trim_start().starts_with("Time:")
        && line.contains("Scope:")
}

/// `TBBmalloc: skip allocation functions replacement in ucrtbase.dll: ...`:
/// a biblioteca de memória do Quartus escreve isto no stderr de todo
/// programa dele no Windows (visto no 25.1 Lite). Não é do design.
fn is_quartus_noise(tool: Tool, line: &str) -> bool {
    tool == Tool::Quartus && line.starts_with("TBBmalloc:")
}

/// O stdout desta ferramenta é feito de mensagens (e não de saída do programa)?
fn stdout_is_messages(tool: Tool) -> bool {
    tool.is_yanc() || tool == Tool::Iverilog
}

fn parse_stream(
    tool: Tool,
    text: &str,
    source: Option<&Utf8Path>,
    keep_unknown: bool,
    out: &mut Vec<Diagnostic>,
) {
    let mut continuable = false;
    // O índice do diagnóstico de `*** These modules were missing:`, até o
    // `***` que fecha a lista.
    let mut missing: Option<usize> = None;
    for line in text.lines() {
        if line.trim().is_empty() || is_quartus_noise(tool, line) {
            continue;
        }
        if tool == Tool::Iverilog {
            // `*** These modules were missing:`, os módulos, um por linha, e
            // `***`: um resumo só (o erro de cada um já veio antes).
            if line.trim() == "*** These modules were missing:" {
                missing = Some(out.len());
                out.push(Diagnostic {
                    tool,
                    severity: Severity::Info,
                    message: "These modules were missing:".into(),
                    file: None,
                    line: None,
                    column: None,
                    raw: line.to_owned(),
                });
                continue;
            }
            if let Some(at) = missing {
                if line.trim() == "***" {
                    missing = None;
                } else {
                    let summary = &mut out[at];
                    summary.message = format!("{} {}", summary.message, line.trim());
                    summary.raw.push('\n');
                    summary.raw.push_str(line);
                }
                continue;
            }
            // `x.v:5:        : Padding 4 high bits of the port.`: com o local,
            // mas continuação do aviso anterior.
            if let Some((_, rest)) = split_location(line.trim())
                && rest.starts_with(':')
                && let Some(last) = out.last_mut()
                && last.severity != Severity::Unknown
            {
                last.message = format!("{} {}", last.message, rest.trim_start_matches(':').trim());
                last.raw.push('\n');
                last.raw.push_str(line);
                continue;
            }
        }
        // `       Time: 0  Scope: tb` depois do `ERROR:` de um `$error`.
        if tool == Tool::Vvp
            && is_vvp_continuation(line)
            && let Some(last) = out.last_mut()
            && last.tool == Tool::Vvp
            && matches!(last.severity, Severity::Error | Severity::Warning)
        {
            let more = line.split_whitespace().collect::<Vec<_>>().join(" ");
            last.message = format!("{} ({more})", last.message);
            last.raw.push('\n');
            last.raw.push_str(line);
            continue;
        }
        // Continuação do Icarus: um aviso longo continua em linhas que começam
        // com `:` (`: confusing timing results.`, `:   -- module x declared
        // here: x.v:1`).
        if tool == Tool::Iverilog
            && line.trim_start().starts_with(':')
            && let Some(last) = out.last_mut()
            && last.severity != Severity::Unknown
        {
            let more = line.trim_start().trim_start_matches(':').trim();
            last.message = format!("{} {}", last.message, more)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            last.raw.push('\n');
            last.raw.push_str(line);
            if last.file.is_none()
                && let Some((_, place)) = more.rsplit_once("declared here: ")
                && let Some((file, number)) = place.rsplit_once(':')
                && let Ok(number) = number.trim().parse()
            {
                last.file = Some(crate::paths::native_separators(file.trim()));
                last.line = Some(number);
            }
            continue;
        }
        // Continuação indentada do Verilator (trecho do fonte, `^~~`, "... See")
        // e as submensagens do Quartus.
        if continuable
            && line.starts_with(' ')
            && let Some(last) = out.last_mut()
        {
            last.raw.push('\n');
            last.raw.push_str(line);
            continue;
        }
        let parsed = parse_line(tool, line, source);
        // O GHDL escreve, depois de cada mensagem com local, a linha do fonte
        // (na indentação do arquivo, que pode ser nenhuma) e a do `^`; depois
        // do `ghdl:error: assertion failed`, o `instance:`.
        if tool == Tool::Ghdl
            && continuable
            && parsed.is_none()
            && split_location(line.trim()).is_none()
            && let Some(last) = out.last_mut()
        {
            last.raw.push('\n');
            last.raw.push_str(line);
            continue;
        }
        continuable =
            matches!(tool, Tool::Verilator | Tool::Quartus | Tool::Ghdl) && parsed.is_some();
        let parsed = match parsed {
            Some(parsed) => parsed,
            None if keep_unknown => Parsed::new(Severity::Unknown, line.trim()),
            None => continue,
        };
        out.push(Diagnostic {
            tool,
            severity: parsed.severity,
            message: parsed.message,
            file: parsed.file,
            line: parsed.line,
            column: parsed.column,
            raw: line.to_owned(),
        });
    }
}

fn parse_line(tool: Tool, raw: &str, source: Option<&Utf8Path>) -> Option<Parsed> {
    match tool {
        Tool::Cmmcomp | Tool::Appcomp | Tool::Asmcomp => parse_yanc_phrase(raw, source),
        Tool::Cpppp | Tool::Cppcomp => {
            parse_c_style(raw).or_else(|| parse_tool_prefixed(tool, raw))
        }
        Tool::Iverilog | Tool::Vvp => parse_c_style(raw)
            .or_else(|| parse_icarus(raw, tool == Tool::Vvp))
            .map(undefined_macro_is_an_error),
        Tool::Verilator => parse_verilator(raw).or_else(|| parse_c_style(raw)),
        Tool::Ghdl => parse_ghdl(raw),
        Tool::Yosys => parse_yosys(raw),
        Tool::Dot => parse_graphviz(raw),
        Tool::Quartus => parse_quartus(raw),
        _ => None,
    }
}

struct Parsed {
    severity: Severity,
    message: String,
    file: Option<Utf8PathBuf>,
    line: Option<u32>,
    column: Option<u32>,
}

impl Parsed {
    fn new(severity: Severity, message: &str) -> Self {
        Parsed {
            severity,
            message: message.trim().to_owned(),
            file: None,
            line: None,
            column: None,
        }
    }

    fn at(mut self, location: Location<'_>) -> Self {
        self.file = Some(crate::paths::native_separators(location.file));
        self.line = Some(location.line);
        self.column = location.column;
        self
    }
}

/// Prefixos do dialeto de frases, na ordem em que precisam ser testados
/// (`Error on line` antes de `Error:`). O `bool` diz se o prefixo é seguido de
/// `<n>` e de um separador (`:` ou `.`).
const PHRASES: &[(&str, Severity, bool)] = &[
    ("Syntax error on line ", Severity::Error, true),
    ("Error on line ", Severity::Error, true),
    ("Warning on line ", Severity::Warning, true),
    ("Heads up on line ", Severity::Warning, true),
    ("Error:", Severity::Error, false),
    ("Heads up:", Severity::Warning, false),
    ("Warning:", Severity::Warning, false),
    ("Info:", Severity::Info, false),
    ("Sweet:", Severity::Info, false),
];

fn parse_yanc_phrase(raw: &str, source: Option<&Utf8Path>) -> Option<Parsed> {
    let text = raw.trim();
    // `Error in function f: where's the return of ...` (`messages.h`): a
    // única mensagem de erro do cmmcomp sem linha; é do fonte inteiro.
    if text.starts_with("Error in function ") {
        let mut parsed = Parsed::new(Severity::Error, text);
        parsed.file = source.map(Utf8Path::to_owned);
        return Some(parsed);
    }
    for &(prefix, severity, numbered) in PHRASES {
        let Some(rest) = text.strip_prefix(prefix) else {
            continue;
        };
        if numbered {
            let (line, rest) = split_number(rest)?;
            let message = rest.trim_start_matches([':', '.']).trim();
            // "Syntax error on line 3. You're a confused soul!": a frase inteira
            // é a mensagem, porque o resto sozinho não diz que é de sintaxe.
            let message = if prefix.starts_with("Syntax") {
                format!("Syntax error. {message}")
            } else {
                message.to_owned()
            };
            let mut parsed = Parsed::new(severity, &message);
            parsed.file = source.map(Utf8Path::to_owned);
            parsed.line = Some(line);
            return Some(parsed);
        }
        let mut parsed = Parsed::new(severity, rest);
        if let Some((file, line)) = data_file_location(rest) {
            parsed.file = Some(file);
            parsed.line = Some(line);
        }
        return Some(parsed);
    }
    None
}

/// `line <n> of file '<arquivo>'` dentro de uma mensagem do `asmcomp`.
fn data_file_location(message: &str) -> Option<(Utf8PathBuf, u32)> {
    let after = &message[message.find("line ")? + "line ".len()..];
    let (line, rest) = split_number(after)?;
    let rest = rest.strip_prefix(" of file '")?;
    let file = &rest[..rest.find('\'')?];
    Some((crate::paths::native_separators(file), line))
}

/// `"42: resto"` vira `(42, ": resto")`.
fn split_number(s: &str) -> Option<(u32, &str)> {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let n = s[..end].parse().ok()?;
    Some((n, &s[end..]))
}

struct Location<'a> {
    file: &'a str,
    line: u32,
    column: Option<u32>,
}

/// Separa `<arquivo>:<linha>[:<coluna>]: <resto>`. Procura o primeiro `:`
/// seguido de dígitos, para não confundir o `C:` de um caminho Windows.
fn split_location(text: &str) -> Option<(Location<'_>, &str)> {
    for (i, _) in text.match_indices(':') {
        if i == 0 {
            continue;
        }
        let Some((line, rest)) = split_number(&text[i + 1..]) else {
            continue;
        };
        let (column, rest) = match rest.strip_prefix(':').and_then(split_number) {
            Some((col, rest)) => (Some(col), rest),
            None => (None, rest),
        };
        let Some(rest) = rest.strip_prefix(':') else {
            continue;
        };
        let location = Location {
            file: &text[..i],
            line,
            column,
        };
        return Some((location, rest.trim_start()));
    }
    None
}

/// O GHDL. Da análise e da elaboração, no formato do gcc sem o espaço antes
/// do nível (`x.vhd:10:14:error: no declaration for "b"`), no stderr. Da
/// execução, no stdout: `x.vhd:23:5:@120ns:(assertion error): texto`, com
/// `(report note)` e `(assertion note)` como saída do testbench, e não
/// mensagem; `simulation finished @120ns`; e as do próprio GHDL,
/// `ghdl:error: ...` ou `<caminho>ghdl.exe:error: ...`.
fn parse_ghdl(raw: &str) -> Option<Parsed> {
    let text = raw.trim();
    if text.starts_with("simulation finished @") {
        return Some(Parsed::new(Severity::Info, text));
    }
    if let Some((location, rest)) = split_location(text) {
        if let Some(rest) = rest.strip_prefix('@') {
            let (time, rest) = rest.split_once(":(")?;
            let (kind, message) = rest.split_once("):")?;
            let severity = match kind {
                "assertion error" | "assertion failure" | "report error" | "report failure" => {
                    Severity::Error
                }
                "assertion warning" | "report warning" => Severity::Warning,
                _ => return None,
            };
            let message = format!("{} (@{time})", message.trim());
            return Some(Parsed::new(severity, &message).at(location));
        }
        let mut parsed = parse_c_style(text)?;
        // `*command line*:1:1:error: unit "x" has not been analyzed`.
        if parsed
            .file
            .as_ref()
            .is_some_and(|f| f.as_str().starts_with('*'))
        {
            parsed.file = None;
            parsed.line = None;
            parsed.column = None;
        }
        return Some(parsed);
    }
    for (tag, severity) in [
        (":error:", Severity::Error),
        (":warning:", Severity::Warning),
        (":info:", Severity::Info),
    ] {
        if let Some((program, message)) = text.split_once(tag) {
            let name = program
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(program)
                .to_ascii_lowercase();
            if name == "ghdl" || name == "ghdl.exe" {
                return Some(Parsed::new(severity, message));
            }
        }
    }
    None
}

/// `<arquivo>:<linha>[:<coluna>]: error|warning|note|sorry: <mensagem>`
fn parse_c_style(raw: &str) -> Option<Parsed> {
    let (location, rest) = split_location(raw.trim())?;
    let (severity, message) = [
        ("error:", Severity::Error),
        ("fatal error:", Severity::Error),
        ("sorry:", Severity::Error),
        ("warning:", Severity::Warning),
        ("note:", Severity::Info),
    ]
    .into_iter()
    .find_map(|(tag, sev)| rest.strip_prefix(tag).map(|m| (sev, m)))?;
    Some(Parsed::new(severity, message).at(location))
}

/// `cppcomp: internal error: ...`, `cpppp: cannot open 'x'`,
/// `cppcomp: warning: could not create ...`
fn parse_tool_prefixed(tool: Tool, raw: &str) -> Option<Parsed> {
    let rest = raw
        .trim()
        .strip_prefix(tool.binary_name())?
        .strip_prefix(": ")?;
    Some(match rest.strip_prefix("warning: ") {
        Some(message) => Parsed::new(Severity::Warning, message),
        None => Parsed::new(Severity::Error, rest),
    })
}

/// `macro NBITS undefined (and assumed null) at this point.`: o Icarus só
/// avisa e segue com a macro vazia, e o design compila com outra largura
/// (a síntese depois falha). Quase sempre é a ordem dos arquivos: um
/// `` `define `` só vale para os arquivos depois dele na lista do projeto.
/// Vira erro, com a explicação.
fn undefined_macro_is_an_error(mut parsed: Parsed) -> Parsed {
    if parsed.message.contains("undefined (and assumed null)") {
        parsed.severity = Severity::Error;
        parsed.message.push_str(
            " A `define only reaches the files after it in the project list: move its file up, or define it before use.",
        );
    }
    parsed
}

/// O que sobra do Icarus fora do formato C. `simulation` é o `vvp`: lá, um
/// `ERROR:` sem local é o testbench escrevendo (`$display("ERROR: ...")`),
/// e fica como saída dele; o `$error` do Verilog sempre traz o local.
fn parse_icarus(raw: &str, simulation: bool) -> Option<Parsed> {
    let text = raw.trim();
    // `ERROR: tb.v:12: ...` e `FATAL: tb.v:3: ...` do vvp ($error, $fatal):
    // o prefixo vem antes do local, e não faz parte do nome do arquivo.
    for (prefix, severity) in [
        ("ERROR: ", Severity::Error),
        ("FATAL: ", Severity::Error),
        ("WARNING: ", Severity::Warning),
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            // `$readmemb's file name argument contains non-printable
            // characters`: o vvp não abre o arquivo (acento no caminho) e
            // segue sem os dados. É erro, não aviso.
            let unreadable = rest.contains("contains non-printable characters");
            let severity = if unreadable {
                Severity::Error
            } else {
                severity
            };
            return match split_location(rest) {
                Some((location, message)) => Some(Parsed::new(severity, message).at(location)),
                None if simulation && !unreadable => None,
                None => Some(Parsed::new(severity, rest)),
            };
        }
    }
    if let Some((location, rest)) = split_location(text) {
        // `bad.v:2: syntax error`, `x.v:3: Include file y.v not found`,
        // `tb.v:61: $finish called at 985000 (1ps)`
        let lower = rest.to_ascii_lowercase();
        let severity = if lower.contains("error")
            || lower.contains("not found")
            || lower.contains("contains non-printable characters")
        {
            Severity::Error
        } else if lower.contains("warning") {
            Severity::Warning
        } else {
            Severity::Info
        };
        return Some(Parsed::new(severity, rest).at(location));
    }
    if let Some(file) = text.strip_suffix(": No such file or directory") {
        let mut parsed = Parsed::new(Severity::Error, "File not found");
        parsed.file = Some(crate::paths::native_separators(file));
        return Some(parsed);
    }
    // `2 error(s) during elaboration.`: resumo, a contagem já está nos erros.
    if text.contains(" error(s) during ") {
        return Some(Parsed::new(Severity::Info, text));
    }
    // `warning: Some modules have no timescale. ...`, sem local.
    if let Some(rest) = text.strip_prefix("warning: ") {
        return Some(Parsed::new(Severity::Warning, rest));
    }
    if text == "No top level modules, and no -s option." {
        return Some(Parsed::new(Severity::Error, text));
    }
    if text.starts_with("VCD info:") || text.starts_with("FST info:") {
        return Some(Parsed::new(Severity::Info, text));
    }
    None
}

/// `%Error: a.v:2:17: msg`, `%Warning-WIDTH: a.v:3:4: msg`, `%Error: msg`.
fn parse_verilator(raw: &str) -> Option<Parsed> {
    let text = raw.trim_end().strip_prefix('%')?;
    let (tag, rest) = text.split_once(": ")?;
    let severity = if tag.starts_with("Error") {
        Severity::Error
    } else if tag.starts_with("Warning") {
        Severity::Warning
    } else {
        return None;
    };
    // `%Error: Exiting due to 1 error(s)`: resumo.
    if rest.starts_with("Exiting due to") {
        return Some(Parsed::new(Severity::Info, rest));
    }
    let code = tag.split_once('-').map(|(_, code)| code);
    let with_code = |m: &str| match code {
        Some(code) => format!("{m} [{code}]"),
        None => m.to_owned(),
    };
    Some(match split_location(rest) {
        Some((location, message)) => Parsed::new(severity, &with_code(message)).at(location),
        None => Parsed::new(severity, &with_code(rest)),
    })
}

/// `bad.v:2: ERROR: msg`, `x.v:4: Warning: msg`, `ERROR: msg`, `Warning: msg`,
/// com ou sem espaço depois dos dois-pontos (`rom.v:7: ERROR:Can not open`).
///
/// A gravidade no começo da linha vem antes do local: o aviso de latch
/// (`Warning: Latch inferred ... $auto$proc_dlatch.cc:542:...`) traz um
/// `arquivo:linha:` do próprio Yosys no meio da mensagem.
fn parse_yosys(raw: &str) -> Option<Parsed> {
    let text = raw.trim();
    if let Some((severity, message)) = yosys_tag(text) {
        return Some(Parsed::new(severity, message));
    }
    let (location, rest) = split_location(text)?;
    let (severity, message) = yosys_tag(rest)?;
    Some(Parsed::new(severity, message).at(location))
}

/// `ERROR:` ou `Warning:` no começo, e o resto sem espaço na frente.
fn yosys_tag(text: &str) -> Option<(Severity, &str)> {
    [("ERROR:", Severity::Error), ("Warning:", Severity::Warning)]
        .into_iter()
        .find_map(|(tag, sev)| text.strip_prefix(tag).map(|m| (sev, m.trim_start())))
}

/// Graphviz: `Error: x.dot: syntax error in line 3 near 'y'`, `Warning: ...`.
fn parse_graphviz(raw: &str) -> Option<Parsed> {
    let text = raw.trim();
    let (severity, rest) = [
        ("Error: ", Severity::Error),
        ("Warning: ", Severity::Warning),
    ]
    .into_iter()
    .find_map(|(tag, sev)| text.strip_prefix(tag).map(|m| (sev, m)))?;
    let mut parsed = Parsed::new(severity, rest);
    // `<arquivo>: syntax error in line <n> ...`
    if let Some((file, message)) = rest.split_once(": ")
        && let Some(after) = message
            .find(" in line ")
            .map(|i| &message[i + " in line ".len()..])
        && let Some((line, _)) = split_number(after)
    {
        parsed.file = Some(crate::paths::native_separators(file));
        parsed.line = Some(line);
    }
    Some(parsed)
}

/// Quartus: `Error (10161): <mensagem> File: <arquivo> Line: <n>`,
/// `Critical Warning (332148): <mensagem>`, `Warning (18236): <mensagem>`.
/// O número da mensagem vai no fim, entre colchetes, como o código do
/// Verilator: é por ele que se procura na base de conhecimento da Intel.
///
/// Os `Info` ficam de fora: o Quartus escreve centenas deles. O resumo de
/// uma ferramenta que falhou (`Error: Quartus Prime Analysis & Synthesis was
/// unsuccessful. 2 errors, 1 warning`, seguido de `Error:` indentados com
/// memória e tempo) vira informação: os erros já vieram um a um.
fn parse_quartus(raw: &str) -> Option<Parsed> {
    let text = raw.trim();
    let (severity, critical, rest) = [
        ("Critical Warning", Severity::Warning, true),
        ("Error", Severity::Error, false),
        ("Warning", Severity::Warning, false),
    ]
    .into_iter()
    .find_map(|(tag, severity, critical)| {
        text.strip_prefix(tag)
            .map(|rest| (severity, critical, rest))
    })?;
    let (id, message) = match rest.strip_prefix(" (") {
        Some(rest) => {
            let (id, message) = rest.split_once("): ")?;
            if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            (Some(id), message)
        }
        None => (None, rest.strip_prefix(": ")?),
    };
    if id.is_none() && message.contains(" was unsuccessful. ") {
        return Some(Parsed::new(Severity::Info, message));
    }
    let (message, location) = quartus_location(message);
    let tag = match (critical, id) {
        (true, Some(id)) => format!(" [critical {id}]"),
        (true, None) => " [critical]".to_owned(),
        (false, Some(id)) => format!(" [{id}]"),
        (false, None) => String::new(),
    };
    let message = without_knowledge_base(message);
    let mut parsed = Parsed::new(severity, &format!("{}{tag}", message.trim()));
    if let Some((file, line)) = location {
        parsed.file = Some(crate::paths::native_separators(file));
        parsed.line = Some(line);
    }
    Some(parsed)
}

/// O `File: C:/p/contador.v Line: 27` do fim da mensagem (do Quartus II 13.1 em
/// diante), separado do texto. Sem ele, o `at contador.v(27)` do meio do texto,
/// que só traz o nome do arquivo.
fn quartus_location(message: &str) -> (&str, Option<(&str, u32)>) {
    if let Some((before, line)) = message.rsplit_once(" Line: ")
        && let Ok(line) = line.trim().parse()
        && let Some((text, file)) = before.rsplit_once(" File: ")
    {
        return (text, Some((file.trim(), line)));
    }
    let at = message.match_indices(" at ").find_map(|(i, _)| {
        let rest = &message[i + " at ".len()..];
        let (file, after) = rest.split_once('(')?;
        let (line, after) = split_number(after)?;
        (!file.is_empty() && !file.contains(char::is_whitespace) && after.starts_with(')'))
            .then_some((file, line))
    });
    (message, at)
}

/// A mensagem sem o parágrafo da base de conhecimento que o Quartus põe em
/// alguns erros ("The Intel FPGA Knowledge Database contains many articles
/// ... search for this specific error message number."): o número da
/// mensagem, no fim, já diz o que procurar.
fn without_knowledge_base(message: &str) -> String {
    const END: &str = "error message number.";
    let Some(kb) = message.find(" Knowledge Database contains") else {
        return message.to_owned();
    };
    let Some(end) = message[kb..].find(END).map(|e| kb + e + END.len()) else {
        return message.to_owned();
    };
    let start = message[..kb].rfind(" The ").unwrap_or(kb);
    format!("{}{}", &message[..start], &message[end..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(tool: Tool, line: &str) -> Diagnostic {
        let mut d = parse(tool, "", line, Some(Utf8Path::new("/p/Software/prog.cmm")));
        assert_eq!(d.len(), 1);
        d.remove(0)
    }

    // As linhas copiadas da saída do GHDL 6.0.0 (mcode) do ucrt64-v3.
    #[test]
    fn ghdl_analysis_errors_keep_the_source_and_caret_lines() {
        let stderr = "\
erro.vhd:10:14:error: no declaration for \"b\"
  y <= a and b;
             ^
sintaxe.vhd:2:11:error: missing \";\" at end of entity
end entity
          ^
";
        let d = parse(Tool::Ghdl, "", stderr, None);
        assert_eq!(d.len(), 2, "{d:?}");
        assert_eq!(d[0].severity, Severity::Error);
        assert_eq!(d[0].message, "no declaration for \"b\"");
        assert_eq!(d[0].file.as_deref(), Some(Utf8Path::new("erro.vhd")));
        assert_eq!((d[0].line, d[0].column), (Some(10), Some(14)));
        assert!(d[0].raw.ends_with("             ^"), "{}", d[0].raw);
        assert_eq!(d[1].line, Some(2));
        assert!(
            d[1].raw.contains("\nend entity\n"),
            "a linha sem indentação também vem"
        );
    }

    #[test]
    fn ghdl_run_messages_separate_reports_from_failures() {
        let stdout = "\
contador_tb.vhd:22:5:@120ns:(report note): q = 0A
contador_tb.vhd:23:5:@120ns:(assertion error): q nao e zero
simulation finished @120ns
falha_tb.vhd:16:5:@10ns:(assertion failure): parou aqui
ghdl:error: assertion failed
  instance: /falha_tb/P0
C:\\ghdl\\bin\\ghdl.exe:error: cannot find entity or configuration tb
*command line*:1:1:error: unit entity \"tb\" has not been analyzed
";
        let d = parse(Tool::Ghdl, stdout, "", None);
        let found: Vec<_> = d.iter().map(|d| (d.severity, d.message.as_str())).collect();
        assert_eq!(
            found,
            [
                (Severity::Error, "q nao e zero (@120ns)"),
                (Severity::Info, "simulation finished @120ns"),
                (Severity::Error, "parou aqui (@10ns)"),
                (Severity::Error, "assertion failed"),
                (Severity::Error, "cannot find entity or configuration tb"),
                (Severity::Error, "unit entity \"tb\" has not been analyzed"),
            ]
        );
        assert_eq!(d[0].file.as_deref(), Some(Utf8Path::new("contador_tb.vhd")));
        assert!(d[3].raw.ends_with("instance: /falha_tb/P0"));
        assert_eq!(d[5].file, None, "a linha de comando não é arquivo");
        assert!(!is_message(
            Tool::Ghdl,
            Stream::Stdout,
            "x.vhd:22:5:@1ns:(report note): oi"
        ));
        assert!(is_message(Tool::Ghdl, Stream::Stdout, "  instance: /tb/P0"));
    }

    #[test]
    fn bundle_warnings_are_quieted_and_repetitions_go() {
        let stderr = "\
/b/yanc/SAPHO/processor.v:23: warning: timescale for pc inherited from another file.
/p/contador.v:1: ...: The inherited timescale is here.
/p/top.v:4: warning: Port 2 (b) of x expects 8 bits, got 1.
/b/yanc/SAPHO/processor.v:23: warning: timescale for pc inherited from another file.
/p/contador.v:1: ...: The inherited timescale is here.
/p/top.v:4: warning: Port 2 (b) of x expects 8 bits, got 1.
";
        let mut d = parse(Tool::Iverilog, "", stderr, None);
        assert_eq!(d.len(), 6);
        tidy(&mut d, Utf8Path::new("/b"));
        let summary: Vec<(Severity, String)> = d
            .iter()
            .map(|d| (d.severity, d.file.clone().unwrap().into_string()))
            .collect();
        let path = |p: &str| crate::paths::native_separators(p).into_string();
        assert_eq!(
            summary,
            [
                // A biblioteca do bundle não é do usuário.
                (Severity::Info, path("/b/yanc/SAPHO/processor.v")),
                (d[1].severity, path("/p/contador.v")),
                // O do usuário continua aviso.
                (Severity::Warning, path("/p/top.v")),
            ]
        );
    }

    #[test]
    fn cmmcomp_error_with_line() {
        let d = one(
            Tool::Cmmcomp,
            "Error on line 19: c'mon dude, declare the variable 'zz' properly!",
        );
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.line, Some(19));
        assert_eq!(
            d.file.as_deref(),
            Some(Utf8Path::new("/p/Software/prog.cmm"))
        );
        assert_eq!(d.message, "c'mon dude, declare the variable 'zz' properly!");
    }

    #[test]
    fn cmmcomp_syntax_error() {
        let d = one(
            Tool::Cmmcomp,
            "Syntax error on line 3. You're a confused soul!",
        );
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.line, Some(3));
        assert_eq!(d.message, "Syntax error. You're a confused soul!");
    }

    #[test]
    fn cmmcomp_warning() {
        let d = one(
            Tool::Cmmcomp,
            "Heads up on line 7: if you don't want this warning, use 'out'.",
        );
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(d.line, Some(7));
    }

    #[test]
    fn error_without_line_has_no_file() {
        let d = one(Tool::Cmmcomp, "Error: yo, where's the main() function?");
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.line, None);
        assert_eq!(d.file, None);
    }

    #[test]
    fn asmcomp_data_file_error() {
        let d = one(
            Tool::Asmcomp,
            "Error: line 4 of file 'dados.txt' isn't a valid integer!",
        );
        assert_eq!(d.file.as_deref(), Some(Utf8Path::new("dados.txt")));
        assert_eq!(d.line, Some(4));
    }

    #[test]
    fn cppcomp_error_windows_path() {
        let d = one(
            Tool::Cppcomp,
            r"C:\proj\Temp\pp.cpp:12: error: unknown type 'foo'",
        );
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(
            d.file.as_deref(),
            Some(Utf8Path::new(r"C:\proj\Temp\pp.cpp"))
        );
        assert_eq!(d.line, Some(12));
        assert_eq!(d.message, "unknown type 'foo'");
    }

    #[test]
    fn cppcomp_tool_prefixed() {
        let d = one(Tool::Cppcomp, "cppcomp: internal error: bad node");
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.message, "internal error: bad node");

        let d = one(
            Tool::Cppcomp,
            "cppcomp: warning: could not create /x (errno=13)",
        );
        assert_eq!(d.severity, Severity::Warning);
    }

    #[test]
    fn unknown_lines_are_kept() {
        let d = one(Tool::Asmcomp, "Bad format!");
        assert_eq!(d.severity, Severity::Unknown);
        assert_eq!(d.raw, "Bad format!");
    }

    #[test]
    fn stderr_comes_first_and_blank_lines_are_skipped() {
        let d = parse(
            Tool::Cmmcomp,
            "Info: 16 assembly instructions generated\n\n",
            "Error: x\n",
            None,
        );
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].severity, Severity::Error);
        assert_eq!(d[1].severity, Severity::Info);
    }

    #[test]
    fn iverilog_formats() {
        let d = parse(
            Tool::Iverilog,
            "",
            "bad.v:2: syntax error\nbad.v:2: error: Syntax error in continuous assignment\n2 error(s) during elaboration.\nnope.v: No such file or directory\n",
            None,
        );
        assert_eq!(d.len(), 4);
        assert_eq!((d[0].severity, d[0].line), (Severity::Error, Some(2)));
        assert_eq!(d[1].message, "Syntax error in continuous assignment");
        assert_eq!(d[2].severity, Severity::Info);
        assert_eq!(d[3].file.as_deref(), Some(Utf8Path::new("nope.v")));
    }

    #[test]
    fn verilator_with_continuation_lines() {
        let stderr = "%Error: bad.v:2:17: syntax error, unexpected ';'\n    2 |   assign b = a &;\n      |                 ^\n%Warning-WIDTH: C:\\p\\w.v:3:14: Operator ASSIGNW expects 4 bits\n%Error: Exiting due to 1 error(s)\n";
        let d = parse(Tool::Verilator, "", stderr, None);
        assert_eq!(d.len(), 3);
        assert_eq!((d[0].line, d[0].column), (Some(2), Some(17)));
        assert!(d[0].raw.contains("assign b = a &;"));
        assert_eq!(d[1].severity, Severity::Warning);
        assert_eq!(d[1].file.as_deref(), Some(Utf8Path::new("C:\\p\\w.v")));
        assert!(d[1].message.ends_with("[WIDTH]"));
        assert_eq!(d[2].severity, Severity::Info);
    }

    #[test]
    fn yosys_formats() {
        let d = parse(
            Tool::Yosys,
            "",
            "bad.v:2: ERROR: syntax error, unexpected ';'\nERROR: Module `nada' not found!\n",
            None,
        );
        assert_eq!((d[0].severity, d[0].line), (Severity::Error, Some(2)));
        assert_eq!(d[1].message, "Module `nada' not found!");
        assert_eq!(d[1].file, None);
    }

    #[test]
    fn simulation_stdout_is_not_diagnostics() {
        let d = parse(
            Tool::Vvp,
            "resultado = 55\nVCD info: dumpfile x.vcd opened for output.\n",
            "",
            None,
        );
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].severity, Severity::Info);
    }

    #[test]
    fn graphviz_errors() {
        let d = parse(
            Tool::Dot,
            "",
            "Error: /t/soma.dot: syntax error in line 3 near 'x'\nWarning: node a, port b unrecognized\n",
            None,
        );
        assert_eq!(d[0].severity, Severity::Error);
        assert_eq!(d[0].file.as_deref(), Some(Utf8Path::new("/t/soma.dot")));
        assert_eq!(d[0].line, Some(3));
        assert_eq!(d[1].severity, Severity::Warning);
    }

    #[test]
    fn icarus_warning_continues_on_colon_lines() {
        // Como o iverilog 14 imprime: continuação com espaços e `:`.
        let stderr = "warning: Some modules have no timescale. This may cause\n       : confusing timing results.\tAffected modules are:\n       :   -- module and_gate declared here: /p/and_gate.v:1\n";
        let found = parse(Tool::Iverilog, "", stderr, None);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].severity, Severity::Warning);
        assert_eq!(
            found[0].message,
            "Some modules have no timescale. This may cause confusing timing results. Affected modules are: -- module and_gate declared here: /p/and_gate.v:1"
        );
        assert_eq!(
            found[0].file.as_deref(),
            Some(Utf8Path::new("/p/and_gate.v"))
        );
        assert_eq!(found[0].line, Some(1));
    }

    #[test]
    fn yosys_error_without_space_and_latch_warning() {
        let d = one(
            Tool::Yosys,
            "rom.v:7: ERROR:Can not open file `dados.mif` for reading.",
        );
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.file.as_deref(), Some(Utf8Path::new("rom.v")));
        assert_eq!(d.line, Some(7));
        assert!(d.message.starts_with("Can not open"));
        let d = one(
            Tool::Yosys,
            "Warning: Latch inferred for signal `\\top.\\q' from process `\\top.$proc$x.v:5$1': $auto$proc_dlatch.cc:542:proc_dlatch$7",
        );
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(d.file, None);
        assert!(d.message.starts_with("Latch inferred"));
    }

    #[test]
    fn cmmcomp_function_without_return() {
        let d = one(
            Tool::Cmmcomp,
            "Error in function f: where's the return? The function is declared int.",
        );
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(
            d.file.as_deref(),
            Some(Utf8Path::new("/p/Software/prog.cmm"))
        );
        assert!(d.message.starts_with("Error in function f"));
    }

    #[test]
    fn undefined_macro_is_an_error() {
        let d = one(
            Tool::Iverilog,
            "/p/alu.v:3: warning: macro NBITS undefined (and assumed null) at this point.",
        );
        assert_eq!(d.severity, Severity::Error);
        assert!(d.message.contains("project list"));
    }

    #[test]
    fn icarus_continuations_and_missing_modules_are_merged() {
        let stderr = "/p/top.v:5: warning: Port 1 (a) of m expects 8 bits, got 4.\n\
                      /p/top.v:5:        : Padding 4 high bits of the port.\n\
                      /p/top.v:2: error: Unknown module type: falta\n\
                      *** These modules were missing:\n\
                      \x20       falta referenced 1 times.\n\
                      ***\n";
        let found = parse(Tool::Iverilog, "", stderr, None);
        assert_eq!(found.len(), 3, "{found:#?}");
        assert!(
            found[0]
                .message
                .ends_with("Padding 4 high bits of the port.")
        );
        assert_eq!(found[1].severity, Severity::Error);
        assert_eq!(found[2].severity, Severity::Info);
        assert_eq!(
            found[2].message,
            "These modules were missing: falta referenced 1 times."
        );
    }

    #[test]
    fn what_the_testbench_prints_stays_its_output() {
        let stdout = "ERROR: valor errado na saida\n\
                      ERROR: /p/tb.v:12: boom\n\
                      \x20      Time: 1000  Scope: tb\n";
        let found = parse(Tool::Vvp, stdout, "", None);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].line, Some(12));
        assert_eq!(found[0].message, "boom (Time: 1000 Scope: tb)");
        assert!(!is_message(
            Tool::Vvp,
            Stream::Stdout,
            "ERROR: valor errado na saida"
        ));
        assert!(is_message(
            Tool::Vvp,
            Stream::Stdout,
            "       Time: 1000  Scope: tb"
        ));
    }

    /// Linhas de relatórios reais do Quartus: o 17.1 Lite (o erro com
    /// `File:` e `Line:`, o resumo da falha), o 18.0 Lite (o parágrafo da
    /// base de conhecimento), o 13.1 (o aviso crítico sem `File:`) e o 14.0
    /// (o tempo não atendido). Os caminhos foram trocados.
    #[test]
    fn quartus_messages() {
        let stdout = "Info: *******************************************************************\n\
             Info: Running Quartus Prime Analysis & Synthesis\n\
             \x20   Info: Version 17.1.0 Build 590 10/25/2017 SJ Lite Edition\n\
             Info: Command: quartus_map --read_settings_files=on --write_settings_files=off contador -c contador\n\
             Warning (18236): Number of processors has not been specified which may cause overloading on shared machines.  Set the global assignment NUM_PARALLEL_PROCESSORS in your QSF to an appropriate value for best performance.\n\
             Info (20030): Parallel compilation is enabled and will use 6 of the 6 processors detected\n\
             Warning (12125): Using design file contador.v, which is not specified as a design file for the current project, but contains definitions for 1 design units and 1 entities in project\n\
             \x20   Info (12023): Found entity 1: t1 File: C:/p/contador.v Line: 3\n\
             Error (10161): Verilog HDL error at contador.v(27): object \"conta\" is not declared. Verify the object name is correct. If the name is correct, declare the object. File: C:/p/contador.v Line: 27\n\
             Error (10170): Verilog HDL syntax error at filtro_fir.sv(118) near text: \"always_comb\";  expecting \"end\". Check for and fix any syntax errors that appear immediately before or at the specified keyword. The Intel FPGA Knowledge Database contains many articles with specific details on how to resolve this error. Visit the Knowledge Database at https://www.altera.com/support/support-resources/knowledge-base/search.html and search for this specific error message number. File: C:/p/My Files/filtro_fir.sv Line: 118\n\
             Critical Warning (10191): Verilog HDL Compiler Directive warning at ula.v(41): text macro \"LARGURA\" is undefined\n\
             Critical Warning (332148): Timing requirements not met\n\
             \x20   Info (11105): For recommendations on closing timing, run Report Timing Closure Recommendations in the TimeQuest Timing Analyzer.\n\
             Info (332146): Worst-case setup slack is -2.318\n\
             \x20   Info (332119):     Slack       End Point TNS Clock \n\
             Error: Quartus Prime Analysis & Synthesis was unsuccessful. 2 errors, 3 warnings\n\
             \x20   Error: Peak virtual memory: 4812 megabytes\n\
             \x20   Error: Processing ended: Thu Oct 08 10:12:30 2026\n";
        let found = parse(Tool::Quartus, stdout, "", None);
        let summary: Vec<_> = found
            .iter()
            .map(|d| (d.severity, d.message.as_str(), d.line))
            .collect();
        assert_eq!(found.len(), 7, "{summary:#?}");

        assert_eq!(found[0].severity, Severity::Warning);
        assert!(found[0].message.ends_with("best performance. [18236]"));
        // A submensagem fica no texto original, e o local dela não é o do
        // aviso.
        assert!(
            found[1].raw.contains("Found entity 1"),
            "{:?}",
            found[1].raw
        );
        assert_eq!(found[1].file, None);

        let error = &found[2];
        assert_eq!(error.severity, Severity::Error);
        assert_eq!(
            error.message,
            "Verilog HDL error at contador.v(27): object \"conta\" is not declared. Verify the object name is correct. If the name is correct, declare the object. [10161]"
        );
        assert_eq!(
            error.file,
            Some(crate::paths::native_separators("C:/p/contador.v"))
        );
        assert_eq!(error.line, Some(27));

        let syntax = &found[3];
        assert!(
            syntax
                .message
                .ends_with("before or at the specified keyword. [10170]"),
            "{}",
            syntax.message
        );
        assert_eq!(
            syntax.file,
            Some(crate::paths::native_separators(
                "C:/p/My Files/filtro_fir.sv"
            ))
        );
        assert_eq!(syntax.line, Some(118));

        let macro_warning = &found[4];
        assert_eq!(macro_warning.severity, Severity::Warning);
        assert!(macro_warning.message.ends_with("[critical 10191]"));
        assert_eq!(
            macro_warning.file,
            Some(crate::paths::native_separators("ula.v"))
        );
        assert_eq!(macro_warning.line, Some(41));

        assert_eq!(
            found[5].message,
            "Timing requirements not met [critical 332148]"
        );
        assert!(found[5].raw.contains("Timing Closure Recommendations"));

        let end = &found[6];
        assert_eq!(end.severity, Severity::Info);
        assert!(end.message.ends_with("2 errors, 3 warnings"));
        assert!(end.raw.contains("Peak virtual memory"));

        assert!(is_message(
            Tool::Quartus,
            Stream::Stdout,
            "Error (10161): x File: C:/p/contador.v Line: 27"
        ));
        assert!(!is_message(
            Tool::Quartus,
            Stream::Stdout,
            "Info (12021): Found 1 design units, including 1 entities, in source file contador.v"
        ));
        // O aviso da biblioteca de memória do Quartus 25.1 no stderr.
        let noise = "TBBmalloc: skip allocation functions replacement in ucrtbase.dll: unknown prologue for function _msize\r\n";
        assert!(parse(Tool::Quartus, "", noise, None).is_empty());
        assert!(!is_message(Tool::Quartus, Stream::Stderr, noise.trim_end()));
    }
}
