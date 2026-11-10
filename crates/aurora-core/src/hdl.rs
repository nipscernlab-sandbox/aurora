//! A linguagem de cada arquivo de código do projeto, pela extensão: a regra
//! que o registro de arquivos, a lista dos que estão fora do projeto e cada
//! ferramenta usam para saber o que é de quem.
//!
//! ```
//! use aurora_core::SourceLanguage;
//!
//! assert_eq!(SourceLanguage::of("rtl/alu.sv".as_ref()), Some(SourceLanguage::Verilog));
//! assert_eq!(SourceLanguage::of("rtl/alu.VHD".as_ref()), Some(SourceLanguage::Vhdl));
//! assert_eq!(SourceLanguage::of("test_alu.py".as_ref()), Some(SourceLanguage::Python));
//! assert_eq!(SourceLanguage::of("notas.txt".as_ref()), None);
//! ```

use camino::Utf8Path;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A linguagem de um arquivo do projeto. Em JSON: `"verilog"`, `"vhdl"` ou
/// `"python"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SourceLanguage {
    /// Verilog ou SystemVerilog: `.v` e `.sv`. Simula no Icarus ou no
    /// Verilator; é o que a síntese, a hierarquia pelo Icarus e as placas
    /// leem.
    Verilog,
    /// VHDL: `.vhd` e `.vhdl`, em qualquer caixa. Simula no GHDL.
    Vhdl,
    /// Python: um testbench do cocotb (`.py`).
    Python,
}

impl SourceLanguage {
    /// A linguagem de `path` pela extensão; `None` para o que não é código
    /// do projeto. `.v` e `.sv` só em minúsculas, como a AURORA sempre
    /// aceitou; `.vhd`, `.vhdl` e `.py` em qualquer caixa.
    pub fn of(path: &Utf8Path) -> Option<SourceLanguage> {
        let extension = path.extension()?;
        match extension {
            "v" | "sv" => Some(SourceLanguage::Verilog),
            _ if extension.eq_ignore_ascii_case("vhd")
                || extension.eq_ignore_ascii_case("vhdl") =>
            {
                Some(SourceLanguage::Vhdl)
            }
            _ if extension.eq_ignore_ascii_case("py") => Some(SourceLanguage::Python),
            _ => None,
        }
    }

    /// O nome da linguagem para as mensagens: `Verilog`, `VHDL`, `Python`.
    pub fn label(self) -> &'static str {
        match self {
            SourceLanguage::Verilog => "Verilog",
            SourceLanguage::Vhdl => "VHDL",
            SourceLanguage::Python => "Python",
        }
    }
}

/// `path` é Verilog (`.v` ou `.sv`).
pub(crate) fn is_verilog(path: &Utf8Path) -> bool {
    SourceLanguage::of(path) == Some(SourceLanguage::Verilog)
}

/// `path` é VHDL (`.vhd` ou `.vhdl`).
pub(crate) fn is_vhdl(path: &Utf8Path) -> bool {
    SourceLanguage::of(path) == Some(SourceLanguage::Vhdl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_extension_has_its_language() {
        let of = |p: &str| SourceLanguage::of(Utf8Path::new(p));
        assert_eq!(of("a.v"), Some(SourceLanguage::Verilog));
        assert_eq!(of("a.sv"), Some(SourceLanguage::Verilog));
        assert_eq!(of("a.vhd"), Some(SourceLanguage::Vhdl));
        assert_eq!(of("a.vhdl"), Some(SourceLanguage::Vhdl));
        assert_eq!(of("A.VHDL"), Some(SourceLanguage::Vhdl));
        assert_eq!(of("test_a.py"), Some(SourceLanguage::Python));
        assert_eq!(of("a.V"), None, "o Verilog continua só em minúsculas");
        assert_eq!(of("a.vh"), None, "um include não é registrado");
        assert_eq!(of("a"), None);
    }
}
