//! O texto que uma ferramenta do `aurora mcp` devolve ao agente: um resumo do
//! resultado, em inglês, com o que importa para decidir o próximo passo
//! (passou ou falhou, onde, qual relatório), no lugar do JSON inteiro. O
//! JSON vai à parte, em `structuredContent`, com a saída dos passos cortada.
//!
//! Trabalha sobre o JSON do resultado, e não sobre os tipos: o resumo não
//! pode quebrar porque um campo novo entrou, e um campo ausente só fica de
//! fora.

use std::fmt::Write;

use serde_json::Value;

/// Quantos diagnósticos de erro entram no resumo.
const MAX_ERRORS: usize = 20;
/// Quantos valores de uma porta de saída entram no resumo.
const MAX_VALUES: usize = 24;

/// O resumo do resultado de `method` (com o fluxo, para `flow.run`).
pub(crate) fn summarize(method: &str, value: &Value) -> String {
    let text = match method {
        "flow.run" => flow(value),
        "project.status" | "project.create" | "project.testbench" => status(value),
        "project.add" => added(value),
        "project.remove" => removed(value),
        "project.top" => top(value),
        "processor.add" | "processor.set" => processor(value),
        "processor.outputs" => values(value),
        "history.list" => reports(value),
        "history.show" => str_of(value, "text").to_owned(),
        "toolchain.info" => toolchain(value),
        "update.check" | "update.apply" => update(value),
        _ => String::new(),
    };
    if text.trim().is_empty() {
        pretty(value)
    } else {
        text
    }
}

pub(crate) fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn str_of<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn num(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn items<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn strings<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
    items(value, key).iter().filter_map(Value::as_str).collect()
}

fn line(out: &mut String, text: impl AsRef<str>) {
    let _ = writeln!(out, "{}", text.as_ref());
}

fn ms(value: &Value) -> String {
    num(value, "duration_ms").map_or(String::new(), |d| format!(" ({d} ms)"))
}

/// O resultado de um fluxo.
fn flow(value: &Value) -> String {
    let mut out = String::new();
    let name = match str_of(value, "flow") {
        "simulate" => "Simulation",
        "synthesize" => "Synthesis",
        "build" => "Build",
        "check" => "Check",
        "hierarchy" => "Hierarchy",
        "schematic" => "Schematic",
        "fpga_build" => "FPGA build",
        "fpga_program" => "FPGA programming",
        other => other,
    };
    line(&mut out, format!("{name}: {}", str_of(value, "status")));
    let mut errors: Vec<String> = Vec::new();
    let mut warnings = 0usize;
    let mut collect = |result: &Value| {
        for d in items(result, "diagnostics") {
            match str_of(d, "severity") {
                "error" => errors.push(diagnostic(d)),
                "warning" => warnings += 1,
                _ => {}
            }
        }
    };
    for build in items(value, "builds") {
        let failed = match str_of(build, "failed_step") {
            "" => String::new(),
            step => format!(" at {step}"),
        };
        line(
            &mut out,
            format!(
                "  build {}: {}{failed}{}",
                str_of(build, "processor"),
                str_of(build, "status"),
                ms(build)
            ),
        );
        collect(build);
    }
    let not_built = strings(value, "not_built");
    if !not_built.is_empty() {
        line(&mut out, format!("  not built: {}", not_built.join(", ")));
    }
    let registered: Vec<&str> = items(value, "registered")
        .iter()
        .map(|f| str_of(f, "path"))
        .collect();
    if !registered.is_empty() {
        line(
            &mut out,
            format!("  registered in the project: {}", registered.join(", ")),
        );
    }
    if let Some(check) = value.get("check").filter(|c| !c.is_null()) {
        line(
            &mut out,
            format!(
                "  check of {}: {}{}",
                strings(check, "targets").join(", "),
                str_of(check, "status"),
                ms(check)
            ),
        );
        collect(check);
    }
    if let Some(missing) = value.get("lint_missing").and_then(Value::as_str) {
        line(&mut out, format!("  lint skipped: {missing}"));
    }
    if let Some(hierarchy) = value.get("hierarchy").filter(|h| !h.is_null()) {
        line(
            &mut out,
            format!(
                "  hierarchy: {}{}",
                str_of(hierarchy, "status"),
                ms(hierarchy)
            ),
        );
        if let Some(design) = hierarchy.get("design").filter(|d| !d.is_null()) {
            let roots: Vec<&str> = items(design, "roots")
                .iter()
                .map(|r| str_of(r, "module"))
                .filter(|m| !m.is_empty())
                .collect();
            line(
                &mut out,
                format!(
                    "    design: {} (roots: {})",
                    str_of(design, "status"),
                    roots.join(", ")
                ),
            );
            collect(design);
        }
        for tb in items(hierarchy, "testbenches") {
            line(
                &mut out,
                format!(
                    "    testbench {}: {}",
                    str_of(tb, "testbench"),
                    str_of(tb, "status")
                ),
            );
            collect(tb);
        }
        collect(hierarchy);
    }
    if let Some(sim) = value.get("simulation").filter(|s| !s.is_null()) {
        let fast = if sim.get("fast").and_then(Value::as_bool) == Some(true) {
            ", fast"
        } else {
            ""
        };
        line(
            &mut out,
            format!(
                "  simulation of {} on {}{fast}: {}{}",
                str_of(sim, "top"),
                str_of(sim, "simulator"),
                str_of(sim, "status"),
                ms(sim)
            ),
        );
        if let Some(wave) = sim.get("waveform").filter(|w| !w.is_null()) {
            line(&mut out, format!("    waveform: {}", str_of(wave, "path")));
        }
        if let Some(tests) = sim.get("tests").filter(|t| !t.is_null()) {
            line(
                &mut out,
                format!(
                    "    tests: {} passed, {} failed, {} skipped",
                    num(tests, "passed").unwrap_or(0),
                    num(tests, "failed").unwrap_or(0),
                    num(tests, "skipped").unwrap_or(0)
                ),
            );
        }
        let missing = strings(sim, "missing_inputs");
        if !missing.is_empty() {
            line(
                &mut out,
                format!("    missing inputs: {}", missing.join(", ")),
            );
        }
        collect(sim);
    }
    for port in items(value, "outputs") {
        let all: Vec<String> = items(port, "values")
            .iter()
            .filter_map(Value::as_i64)
            .map(|v| v.to_string())
            .collect();
        let shown = all.iter().take(MAX_VALUES).cloned().collect::<Vec<_>>();
        let more = if all.len() > MAX_VALUES {
            format!(" ... ({} values)", all.len())
        } else {
            String::new()
        };
        let error = port
            .get("error")
            .filter(|e| !e.is_null())
            .map(|e| format!(" [{}]", str_of(e, "message")))
            .unwrap_or_default();
        line(
            &mut out,
            format!(
                "  output {}: {}{more}{error}",
                num(port, "port").unwrap_or(0),
                shown.join(" ")
            ),
        );
    }
    if let Some(synth) = value.get("synthesis").filter(|s| !s.is_null()) {
        line(
            &mut out,
            format!(
                "  synthesis of {}: {}{}",
                str_of(synth, "top"),
                str_of(synth, "status"),
                ms(synth)
            ),
        );
        if let Some(netlist) = synth.get("netlist").and_then(Value::as_str) {
            line(&mut out, format!("    netlist: {netlist}"));
        }
        if let Some(stats) = synth.get("statistics").filter(|s| !s.is_null()) {
            let mut parts = Vec::new();
            for key in ["cells", "wires", "memories", "processes", "modules"] {
                if let Some(n) = num(stats, key) {
                    parts.push(format!("{key}: {n}"));
                }
            }
            if !parts.is_empty() {
                line(&mut out, format!("    {}", parts.join(", ")));
            }
        }
        collect(synth);
    }
    if let Some(schematic) = value.get("schematic").filter(|s| !s.is_null()) {
        line(
            &mut out,
            format!("  schematic: {}", str_of(schematic, "svg")),
        );
    }
    for (key, label) in [
        ("schematic_error", "schematic error"),
        ("rtl_viewer_error", "RTL viewer error"),
        ("wave_error", "waveform error"),
    ] {
        if let Some(error) = value.get(key).filter(|e| !e.is_null()) {
            line(
                &mut out,
                format!(
                    "  {label}: {} ({})",
                    str_of(error, "message"),
                    str_of(error, "code")
                ),
            );
        }
    }
    if let Some(viewer) = value.get("rtl_viewer").and_then(Value::as_str) {
        line(&mut out, format!("  RTL viewer: {viewer}"));
    }
    if let Some(wave) = value.get("wave").filter(|w| !w.is_null()) {
        line(
            &mut out,
            format!("  waveform opened: {}", str_of(wave, "waveform")),
        );
    }
    if let Some(fpga) = value.get("fpga").filter(|f| !f.is_null()) {
        line(
            &mut out,
            format!("  FPGA build: {}", str_of(fpga, "status")),
        );
        collect(fpga);
    }
    if let Some(program) = value.get("fpga_program").filter(|f| !f.is_null()) {
        line(
            &mut out,
            format!("  FPGA programming: {}", str_of(program, "status")),
        );
        collect(program);
    }
    if let Some(report) = value.get("report").and_then(Value::as_str) {
        line(
            &mut out,
            format!("  report: {report} (history_show reads it)"),
        );
    }
    if let Some(error) = value.get("report_error").and_then(Value::as_str) {
        line(&mut out, format!("  report not saved: {error}"));
    }
    if !errors.is_empty() {
        line(&mut out, format!("errors ({}):", errors.len()));
        for error in errors.iter().take(MAX_ERRORS) {
            line(&mut out, format!("  {error}"));
        }
        if errors.len() > MAX_ERRORS {
            line(
                &mut out,
                format!("  ... {} more", errors.len() - MAX_ERRORS),
            );
        }
    }
    if warnings > 0 {
        line(
            &mut out,
            format!("warnings: {warnings} (in structuredContent.*.diagnostics)"),
        );
    }
    out
}

/// `arquivo:linha: mensagem (ferramenta)`.
fn diagnostic(d: &Value) -> String {
    let mut text = String::new();
    if let Some(file) = d.get("file").and_then(Value::as_str) {
        text.push_str(file);
        if let Some(line) = num(d, "line") {
            let _ = write!(text, ":{line}");
            if let Some(column) = num(d, "column") {
                let _ = write!(text, ":{column}");
            }
        }
        text.push_str(": ");
    }
    text.push_str(str_of(d, "message"));
    let _ = write!(text, " ({})", str_of(d, "tool"));
    text
}

/// O retrato do projeto.
fn status(value: &Value) -> String {
    let mut out = String::new();
    line(
        &mut out,
        format!(
            "Project {} at {} ({})",
            str_of(value, "name"),
            str_of(value, "root"),
            str_of(value, "spf")
        ),
    );
    match value.get("top_level").and_then(Value::as_str) {
        Some(top) => line(
            &mut out,
            format!(
                "  top: {top} (module {})",
                value
                    .get("top_module")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            ),
        ),
        None => line(&mut out, "  top: none"),
    }
    if let Some(error) = value.get("top_module_error").filter(|e| !e.is_null()) {
        line(&mut out, format!("    {}", str_of(error, "message")));
    }
    match value.get("selected_testbench").and_then(Value::as_str) {
        Some(tb) => line(
            &mut out,
            format!(
                "  testbench: {tb} (module {})",
                value
                    .get("testbench_module")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            ),
        ),
        None => line(&mut out, "  testbench: none"),
    }
    let list = |key: &str| -> Vec<String> {
        items(value, key)
            .iter()
            .map(|f| str_of(f, "path").to_owned())
            .collect()
    };
    let modules = list("synthesizable");
    let testbenches = list("testbenches");
    line(
        &mut out,
        format!("  modules ({}): {}", modules.len(), modules.join(", ")),
    );
    line(
        &mut out,
        format!(
            "  testbenches ({}): {}",
            testbenches.len(),
            testbenches.join(", ")
        ),
    );
    let unregistered = strings(value, "unregistered");
    if !unregistered.is_empty() {
        line(
            &mut out,
            format!(
                "  HDL files not in the project ({}): {}",
                unregistered.len(),
                unregistered.join(", ")
            ),
        );
    }
    let processors = items(value, "processors");
    if processors.is_empty() {
        line(&mut out, "  processors: none");
    } else {
        line(&mut out, format!("  processors ({}):", processors.len()));
        for p in processors {
            let built = if p.get("built").and_then(Value::as_bool) == Some(true) {
                "built"
            } else {
                "not built"
            };
            let missing = strings(p, "missing_inputs");
            let missing = if missing.is_empty() {
                String::new()
            } else {
                format!("; missing inputs: {}", missing.join(", "))
            };
            line(
                &mut out,
                format!(
                    "    {} ({}, {} MHz, {} clocks): {built}, source {}{missing}",
                    str_of(p, "name"),
                    str_of(p, "language"),
                    num(p, "frequency_mhz").unwrap_or(0),
                    num(p, "clocks").unwrap_or(0),
                    str_of(p, "source")
                ),
            );
        }
    }
    if let Some(waveform) = value.get("waveform").and_then(Value::as_str) {
        line(&mut out, format!("  last waveform: {waveform}"));
    }
    let issues = items(value, "issues");
    if !issues.is_empty() {
        line(&mut out, format!("  issues ({}):", issues.len()));
        for issue in issues {
            line(&mut out, format!("    {}", str_of(issue, "message")));
        }
    }
    out
}

fn added(value: &Value) -> String {
    let mut out = String::new();
    let files = value.as_array().map_or(&[][..], Vec::as_slice);
    line(&mut out, format!("Added {} file(s):", files.len()));
    for file in files {
        let created = if file.get("created").and_then(Value::as_bool) == Some(true) {
            ", created from the template"
        } else {
            ""
        };
        let selected = if file.get("selected").and_then(Value::as_bool) == Some(true) {
            ", selected"
        } else {
            ""
        };
        line(
            &mut out,
            format!(
                "  {} ({}{created}{selected})",
                str_of(file, "path"),
                str_of(file, "role")
            ),
        );
    }
    out
}

fn removed(value: &Value) -> String {
    let mut out = String::new();
    let removed = strings(value, "removed");
    let not_registered = strings(value, "not_registered");
    line(
        &mut out,
        format!(
            "Removed from the project ({}): {}",
            removed.len(),
            removed.join(", ")
        ),
    );
    if !not_registered.is_empty() {
        line(
            &mut out,
            format!("Not in the project: {}", not_registered.join(", ")),
        );
    }
    line(&mut out, "The files stay on disk.");
    out
}

fn top(value: &Value) -> String {
    let mut out = String::new();
    match value.get("top_level").and_then(Value::as_str) {
        Some(top) => line(
            &mut out,
            format!(
                "Top: {top} (module {})",
                value
                    .get("module")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            ),
        ),
        None => line(&mut out, "Top: none"),
    }
    if let Some(error) = value.get("module_error").filter(|e| !e.is_null()) {
        line(&mut out, format!("  {}", str_of(error, "message")));
    }
    out
}

fn processor(value: &Value) -> String {
    format!(
        "Processor {} ({}): source {}, {} MHz, {} clocks, arrays {}\n",
        str_of(value, "name"),
        str_of(value, "language"),
        str_of(value, "source"),
        num(value, "frequency_mhz").unwrap_or(0),
        num(value, "clocks").unwrap_or(0),
        if value.get("show_arrays").and_then(Value::as_bool) == Some(true) {
            "exported"
        } else {
            "not exported"
        }
    )
}

fn values(value: &Value) -> String {
    let all: Vec<String> = value
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter_map(Value::as_i64)
        .map(|v| v.to_string())
        .collect();
    format!("{} value(s): {}\n", all.len(), all.join(" "))
}

fn reports(value: &Value) -> String {
    let mut out = String::new();
    let list = value.as_array().map_or(&[][..], Vec::as_slice);
    if list.is_empty() {
        return "No reports in the history yet.\n".to_owned();
    }
    line(&mut out, format!("{} report(s), newest first:", list.len()));
    for r in list {
        line(
            &mut out,
            format!(
                "  {}  {}  {}  {}",
                str_of(r, "id"),
                r.get("timestamp").and_then(Value::as_str).unwrap_or("?"),
                r.get("status").and_then(Value::as_str).unwrap_or("?"),
                r.get("command").and_then(Value::as_str).unwrap_or("")
            ),
        );
    }
    out
}

fn toolchain(value: &Value) -> String {
    let mut out = String::new();
    if value.get("found").and_then(Value::as_bool) != Some(true) {
        line(
            &mut out,
            format!(
                "No bundle found: {}",
                value
                    .get("error")
                    .map(|e| str_of(e, "message").to_owned())
                    .unwrap_or_default()
            ),
        );
        return out;
    }
    line(
        &mut out,
        format!(
            "Bundle {} for {} at {} (found via {})",
            str_of(value, "bundle"),
            str_of(value, "platform"),
            str_of(value, "root"),
            value.get("origin").and_then(Value::as_str).unwrap_or("?")
        ),
    );
    let components: Vec<String> = items(value, "components")
        .iter()
        .map(|c| format!("{} {}", str_of(c, "name"), str_of(c, "version")))
        .collect();
    line(&mut out, format!("  installed: {}", components.join(", ")));
    let missing = strings(value, "not_installed");
    if !missing.is_empty() {
        line(&mut out, format!("  not installed: {}", missing.join(", ")));
    }
    for tool in items(value, "tools") {
        if let Some(error) = tool.get("error").filter(|e| !e.is_null()) {
            line(
                &mut out,
                format!(
                    "  tool {} unavailable: {}",
                    str_of(tool, "name"),
                    str_of(error, "message")
                ),
            );
        }
    }
    for key in ["compiler_error", "quartus_error"] {
        if let Some(error) = value.get(key).filter(|e| !e.is_null()) {
            line(&mut out, format!("  {}", str_of(error, "message")));
        }
    }
    if let Some(cli) = value.get("aurora_cli").and_then(Value::as_str) {
        line(&mut out, format!("  aurora: {cli}"));
    }
    out
}

fn update(value: &Value) -> String {
    let mut out = String::new();
    let aurora = value.get("aurora").cloned().unwrap_or(Value::Null);
    let newer = aurora.get("newer").and_then(Value::as_bool) == Some(true);
    line(
        &mut out,
        format!(
            "AURORA {} installed, latest release {}{}",
            str_of(&aurora, "installed"),
            str_of(&aurora, "latest"),
            if newer { " (newer)" } else { " (up to date)" }
        ),
    );
    line(
        &mut out,
        format!(
            "  action: {}{}",
            str_of(value, "action"),
            value
                .get("method")
                .and_then(Value::as_str)
                .map(|m| format!(" by {m}"))
                .unwrap_or_default()
        ),
    );
    if let Some(prefix) = value.get("prefix").and_then(Value::as_str) {
        line(&mut out, format!("  installation: {prefix}"));
    } else {
        line(
            &mut out,
            "  installation: none (this aurora was not installed by the installer)",
        );
    }
    for c in items(value, "components") {
        let release_newer = c.get("release_newer").and_then(Value::as_bool) == Some(true);
        let upstream_newer = c.get("upstream_newer").and_then(Value::as_bool) == Some(true);
        if release_newer || upstream_newer {
            line(
                &mut out,
                format!(
                    "  {}: installed {}, release {}, upstream {}",
                    str_of(c, "name"),
                    str_of(c, "installed"),
                    c.get("release").and_then(Value::as_str).unwrap_or("?"),
                    c.get("upstream").and_then(Value::as_str).unwrap_or("?")
                ),
            );
        }
    }
    if let Some(changes) = value.get("changes").filter(|c| !c.is_null()) {
        line(
            &mut out,
            format!(
                "  changes: {} ({} files in, {} out, {} bytes)",
                strings(changes, "components").join(", "),
                num(changes, "files").unwrap_or(0),
                num(changes, "removed").unwrap_or(0),
                num(changes, "download_bytes").unwrap_or(0)
            ),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_simulation_outcome_reads_as_a_summary() {
        let outcome = json!({
            "flow": "simulate", "status": "failed",
            "builds": [
                {"processor": "soma", "status": "succeeded", "failed_step": null, "duration_ms": 120, "diagnostics": []},
                {"processor": "filtro", "status": "failed", "failed_step": "compile", "duration_ms": 30,
                 "diagnostics": [{"severity": "error", "message": "unexpected token", "file": "/p/filtro/filtro.cpp", "line": 7, "column": 3, "tool": "cppcomp"},
                                 {"severity": "warning", "message": "unused", "file": null, "line": null, "column": null, "tool": "cppcomp"}]}
            ],
            "not_built": [], "registered": [], "check": null, "hierarchy": null,
            "simulation": null, "outputs": [{"port": 0, "path": "/p/o.txt", "values": [1, 2, 3], "error": null}],
            "synthesis": null, "schematic": null, "schematic_error": null, "rtl_viewer": null,
            "wave": null, "wave_error": {"code": "no_waveform", "message": "Waveform missing"},
            "report": "run-000007", "report_error": null
        });
        let text = summarize("flow.run", &outcome);
        assert!(text.starts_with("Simulation: failed\n"), "{text}");
        assert!(
            text.contains("build filtro: failed at compile (30 ms)"),
            "{text}"
        );
        assert!(text.contains("output 0: 1 2 3"), "{text}");
        assert!(
            text.contains("waveform error: Waveform missing (no_waveform)"),
            "{text}"
        );
        assert!(text.contains("report: run-000007"), "{text}");
        assert!(
            text.contains("errors (1):\n  /p/filtro/filtro.cpp:7:3: unexpected token (cppcomp)"),
            "{text}"
        );
        assert!(text.contains("warnings: 1"), "{text}");
    }

    #[test]
    fn a_project_status_lists_what_matters() {
        let status = json!({
            "name": "soma", "root": "/p", "spf": "/p/soma.spf",
            "synthesizable": [{"path": "/p/soma/soma/soma.v"}], "testbenches": [{"path": "/p/tb.v"}],
            "top_level": "/p/soma/soma/soma.v", "top_module": "soma", "top_module_error": null,
            "selected_testbench": "/p/tb.v", "testbench_module": "tb", "unregistered": ["/p/x.v"],
            "processors": [{"name": "soma", "language": "cmm", "frequency_mhz": 100, "clocks": 2000, "source": "/p/soma/soma.cmm", "built": true, "missing_inputs": []}],
            "waveform": null, "issues": [{"message": "duplicate file"}]
        });
        let text = summarize("project.status", &status);
        assert!(text.contains("Project soma at /p"), "{text}");
        assert!(
            text.contains("top: /p/soma/soma/soma.v (module soma)"),
            "{text}"
        );
        assert!(text.contains("modules (1)"), "{text}");
        assert!(
            text.contains("soma (cmm, 100 MHz, 2000 clocks): built"),
            "{text}"
        );
        assert!(
            text.contains("HDL files not in the project (1): /p/x.v"),
            "{text}"
        );
        assert!(text.contains("issues (1):\n    duplicate file"), "{text}");
    }

    #[test]
    fn unknown_results_fall_back_to_json() {
        let value = json!({"a": 1});
        assert_eq!(summarize("history.compare", &value), pretty(&value));
        assert_eq!(
            summarize("history.list", &json!([])),
            "No reports in the history yet.\n"
        );
    }
}
