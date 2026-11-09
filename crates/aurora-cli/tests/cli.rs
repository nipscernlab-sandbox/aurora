//! A CLI de ponta a ponta: códigos de saída, texto e JSON.
//!
//! Os testes que rodam ferramentas precisam de um bundle em `AURORA_TEST_BUNDLE`
//! (parcial com o YANC basta para os de build; completo para os demais; ver
//! `crates/aurora-core/tests/common/mod.rs`). Sem elas, avisam e passam.
//!
//! Os caminhos de arquivo da CLI são relativos ao diretório atual, então os
//! testes de arquivo rodam o `aurora` dentro do projeto (`current_dir`).

use assert_cmd::Command;
use camino::{Utf8Path, Utf8PathBuf};
use predicates::prelude::*;
use serde_json::Value;

/// `aurora` sem nenhuma configuração herdada do ambiente de quem roda o teste.
fn aurora() -> Command {
    let mut cmd = Command::cargo_bin("aurora").unwrap();
    cmd.env_remove("AURORA_TOOLCHAIN")
        .env_remove("AURORA_COMPILER")
        .env_remove("AURORA_TEST_BUNDLE")
        .env("NO_COLOR", "1");
    cmd
}

/// `aurora` rodando dentro de `dir`, como quem digita no terminal ali.
fn aurora_in(dir: &Utf8Path) -> Command {
    let mut cmd = aurora();
    cmd.current_dir(dir);
    cmd
}

fn tempdir() -> (tempfile::TempDir, Utf8PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    // dunce: no Windows, `std::fs::canonicalize` devolve `\\?\C:\...`, e a AURORA
    // trabalha com `C:\...`.
    let path = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
    (dir, path)
}

/// Copia um projeto sem `.aurora8/` e sem a pasta de saída dos processadores
/// (`<proc>/<proc>/`), os restos de um build anterior.
fn copy_dir(from: &Utf8Path, to: &Utf8Path) {
    copy_tree(from, to, 0);
}

fn copy_tree(from: &Utf8Path, to: &Utf8Path, depth: usize) {
    std::fs::create_dir_all(to).unwrap();
    for entry in from.read_dir_utf8().unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        // `from` é a pasta de um processador, filha da raiz.
        let output_folder = depth == 1 && from.file_name() == Some(name);
        if name == ".aurora8" || output_folder {
            continue;
        }
        let target = to.join(name);
        if entry.file_type().unwrap().is_dir() {
            copy_tree(entry.path(), &target, depth + 1);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Um projeto de `examples/` numa pasta temporária.
fn example(name: &str) -> (tempfile::TempDir, Utf8PathBuf) {
    let (guard, dir) = tempdir();
    let from = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    let root = dir.join(name);
    copy_dir(&from, &root);
    (guard, root)
}

/// Um projeto vazio criado pela CLI: `(guarda, raiz)`.
fn new_project(name: &str) -> (tempfile::TempDir, Utf8PathBuf) {
    let (guard, dir) = tempdir();
    aurora_in(&dir).args(["new", name]).assert().success();
    (guard, dir.join(name))
}

fn json(cmd: &mut Command, code: i32) -> Value {
    let out = cmd.assert().code(code).get_output().stdout.clone();
    serde_json::from_slice(&out)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out)))
}

fn stdout(cmd: &mut Command, code: i32) -> String {
    let out = cmd.assert().code(code).get_output().stdout.clone();
    String::from_utf8(out).unwrap()
}

fn env_or_skip(var: &str) -> Option<String> {
    let value = std::env::var(var).ok();
    if value.is_none() {
        assert!(std::env::var_os("CI").is_none(), "CI sem {var}");
        eprintln!("PULADO: defina {var}");
    }
    value
}

/// O bundle de teste tem o componente? Sem ele, o trecho que precisa dele
/// avisa e é pulado; com `CI` definido, falha.
fn bundle_has(bundle: &str, component: &str) -> bool {
    let has = Utf8Path::new(bundle)
        .join("components")
        .join(format!("{component}.json"))
        .is_file();
    if !has {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI sem {component} no bundle"
        );
        eprintln!("PULADO: falta {component} no bundle");
    }
    has
}

/// Um caminho relativo do projeto como a CLI o mostra: com o separador do
/// sistema (`rtl\x.v` no Windows). O `.spf` guarda sempre com `/`.
fn shown(path: &str) -> String {
    if cfg!(windows) {
        path.replace('/', "\\")
    } else {
        path.to_owned()
    }
}

fn spf(root: &Utf8Path) -> Value {
    let name = root.file_name().unwrap();
    serde_json::from_str(&std::fs::read_to_string(root.join(format!("{name}.spf"))).unwrap())
        .unwrap()
}

/// Um bundle mínimo (só o manifesto, sem componentes) para testar como a CLI
/// acha e lê o bundle sem precisar das ferramentas.
fn fake_bundle(dir: &Utf8Path) {
    std::fs::create_dir_all(dir.join("yanc/bin")).unwrap();
    let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x64",
        ("macos", "aarch64") => "darwin-arm64",
        ("windows", "x86_64") => "windows-x64",
        _ => "desconhecida",
    };
    std::fs::write(dir.join("yanc/bin/arquivo"), "abc").unwrap();
    let header = serde_json::json!({ "schema": 2, "bundle": "teste", "platform": platform });
    std::fs::write(dir.join("bundle.json"), header.to_string()).unwrap();
    let yanc = serde_json::json!({
        "name": "yanc", "version": "v0", "dir": "yanc", "source": "teste",
        // sha256 de "abc"
        "files": { "yanc/bin/arquivo": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" },
    });
    std::fs::create_dir_all(dir.join("components")).unwrap();
    std::fs::write(dir.join("components/yanc.json"), yanc.to_string()).unwrap();
}

// Linha de comando: ajuda, comandos que saíram, opções excludentes.

#[test]
fn help_shows_both_flows_and_hides_completions() {
    let help = stdout(aurora().arg("--help"), 0);
    for expected in [
        "Verilog",
        "SAPHO",
        "aurora add counter.v counter_tb.v",
        "aurora proc add adder",
        "aurora sim -p adder",
        "AURORA_COMPILER",
    ] {
        assert!(help.contains(expected), "falta {expected:?}:\n{help}");
    }
    for hidden in ["completions", "--toolchain"] {
        assert!(!help.contains(hidden), "{hidden:?} na ajuda:\n{help}");
    }
    assert!(!help.contains('\u{2014}'), "travessão na ajuda");
}

#[test]
fn completions_are_generated() {
    aurora()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("aurora"));
}

/// O que saiu da CLI é recusado pelo clap, com código 2 e `Error:`, como os
/// erros da própria AURORA.
#[test]
fn removed_commands_and_flags_are_rejected() {
    let removed: &[&[&str]] = &[
        &["file", "add", "x.v"],
        &["file", "list"],
        &["proc", "list"],
        &["input", "soma", "0", "1"],
        &["output", "soma", "0"],
        &["config", "show"],
        &["config", "path"],
        &["config", "set-compiler", "/x"],
        &["config", "unset-compiler"],
        &["config"],
        &["config", "--compiler", "/x"],
        &["config", "--reset-compiler"],
        &["build", "--freq", "10"],
        &["build", "--clocks", "10"],
        &["build", "--show-arrays"],
        &["proc", "set", "soma", "--show-arrays", "true"],
        &["sim", "--simulator", "icarus"],
        &["sim", "--vcd"],
        &["sim", "--jobs", "2"],
        &["sim", "--no-build"],
        &["sim", "--freq", "10"],
        &["sim", "--clocks", "10"],
        &["synth", "--no-build"],
        &["synth", "--no-widths"],
    ];
    for args in removed {
        aurora()
            .args(*args)
            .assert()
            .code(2)
            .stderr(predicate::str::contains("Error:"));
    }
}

#[test]
fn exclusive_arguments_are_rejected() {
    let conflicts: &[&[&str]] = &[
        &["sim", "tb.v", "-p", "soma"],
        &["wave", "onda.vcd", "-p", "soma"],
        &["config", "--compiler", "/x", "--reset-compiler"],
        &["synth", "--module", "m"],
        &["add"],
        &["remove"],
    ];
    for args in conflicts {
        aurora()
            .args(*args)
            .assert()
            .code(2)
            .stderr(predicate::str::contains("Error:"));
    }
}

// new, proc, build sem processadores, top sem topo: não dependem do Verilog novo.

#[test]
fn new_prints_spf_and_nothing_else() {
    let (_guard, dir) = tempdir();
    let text = stdout(aurora_in(&dir).args(["new", "proj"]), 0);
    assert_eq!(
        text.trim_end(),
        format!("Project created: {}", dir.join("proj").join("proj.spf"))
    );

    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let created = json(
        aurora_in(&dir).args(["--json", "new", "outro", "--dir", "sub"]),
        0,
    );
    assert_eq!(created["message"], "Project created");
    assert_eq!(
        Utf8Path::new(created["path"].as_str().unwrap()),
        dir.join("sub/outro/outro.spf")
    );

    let err = json(aurora_in(&dir).args(["--json", "new", "proj"]), 2);
    assert_eq!(err["error"]["code"], "project_exists");
}

#[test]
fn top_without_top_level_shows_hint() {
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .arg("top")
        .assert()
        .success()
        .stdout(predicate::str::contains("No top module"))
        .stdout(predicate::str::contains("aurora top <file|module>"));
    let top = json(aurora_in(&root).args(["--json", "top"]), 0);
    assert!(top["top_level"].is_null());
    assert!(top["module"].is_null());
}

/// `aurora` com a pasta de configuração da AURORA (a do `experimental.json`)
/// em `config`: o teste não lê nem grava a do usuário.
fn aurora_with_config(config: &Utf8Path) -> Command {
    let mut cmd = aurora();
    cmd.env("APPDATA", config)
        .env("XDG_CONFIG_HOME", config)
        .env("HOME", config);
    cmd
}

#[test]
fn fpga_is_experimental_and_off_until_enabled() {
    let (_guard, config) = tempdir();
    let aurora = || aurora_with_config(&config);
    aurora()
        .args(["fpga", "boards"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains(
            "Error: The experimental component fpga (FPGA boards) is disabled",
        ))
        .stderr(predicate::str::contains(
            "Enable it with: aurora experimental enable fpga",
        ));
    let list = stdout(aurora().arg("experimental"), 0);
    assert!(list.contains("  fpga       off  FPGA boards"), "{list}");
    assert!(list.contains("  assistant  off  AI assistant"), "{list}");

    aurora()
        .args(["experimental", "enable", "fpga"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Enabled fpga (FPGA boards)"));
    aurora()
        .args(["fpga", "boards"])
        .assert()
        .success()
        .stdout(predicate::str::contains("de2-115"));
    let report = json(aurora().args(["--json", "experimental"]), 0);
    assert_eq!(report["components"][0]["feature"], "fpga");
    assert_eq!(report["components"][0]["enabled"], true);
    assert!(
        report["path"]
            .as_str()
            .unwrap()
            .ends_with("experimental.json")
    );

    aurora()
        .args(["experimental", "disable", "fpga"])
        .assert()
        .success();
    aurora().args(["fpga", "check"]).assert().code(2);
    aurora()
        .args(["experimental", "enable", "nada"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "[possible values: fpga, assistant, git]",
        ));
}

/// O `aurora mcp` e o `aurora chat` (o assistente de IA) são experimentais:
/// desligados, nenhum dos dois abre, e o erro diz como ligar.
#[test]
fn the_assistant_is_experimental_and_off_until_enabled() {
    let (_guard, config) = tempdir();
    let aurora = || aurora_with_config(&config);
    for command in [&["mcp"][..], &["mcp", "setup", "print"], &["chat"]] {
        aurora()
            .args(command)
            .assert()
            .code(2)
            .stdout("")
            .stderr(predicate::str::contains(
                "Error: The experimental component assistant (AI assistant) is disabled",
            ))
            .stderr(predicate::str::contains(
                "Enable it with: aurora experimental enable assistant",
            ));
    }
    aurora()
        .args(["experimental", "enable", "assistant"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Enabled assistant (AI assistant)"));
    // Ligado, o chat passa da conferência e para no que vem depois: sem
    // terminal, ele recusa com a própria mensagem.
    aurora()
        .arg("chat")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("aurora chat needs a terminal"));
    let report = json(aurora().args(["--json", "experimental"]), 0);
    assert_eq!(report["components"][1]["feature"], "assistant");
    assert_eq!(report["components"][1]["enabled"], true);
}

#[test]
fn proc_add_and_set_with_arrays() {
    let (_guard, root) = new_project("proj");
    aurora_in(&root)
        .args(["proc", "add", "alu", "--lang", "cpp", "--inputs", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Processor created"));
    let source = std::fs::read_to_string(root.join("alu/alu.cpp")).unwrap();
    assert!(source.contains("#pragma yanc nuioin 2"));
    assert!(!source.contains("prname"), "{source}");

    // Nome repetido é erro da AURORA, não do build: código 2, com o código estável.
    let err = json(aurora_in(&root).args(["--json", "proc", "add", "alu"]), 2);
    assert_eq!(err["error"]["code"], "processor_exists");

    aurora_in(&root)
        .args([
            "proc", "set", "alu", "--freq", "50", "--clocks", "300", "--arrays", "true",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("50 MHz, 300 clocks, arrays"));
    let entry = &spf(&root)["structure"]["processors"][0];
    assert_eq!(entry["clk"], 50);
    assert_eq!(entry["numClocks"], 300);
    assert_eq!(entry["showArrays"], true);

    let processor = json(
        aurora_in(&root).args(["--json", "proc", "set", "alu", "--arrays", "false"]),
        0,
    );
    assert_eq!(processor["show_arrays"], false);
    assert_eq!(processor["frequency_mhz"], 50);

    aurora_in(&root)
        .args(["proc", "set", "alu"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Nothing to change"))
        .stderr(predicate::str::contains("--arrays"));
}

#[test]
fn proc_remove_takes_the_processor_out_and_keeps_its_folder() {
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .args(["proc", "add", "soma"])
        .assert()
        .success();
    aurora_in(&root)
        .args(["proc", "add", "filtro", "--lang", "cpp"])
        .assert()
        .success();
    // O Verilog e o testbench que o build gera, registrados e escolhidos.
    std::fs::create_dir_all(root.join("soma/soma")).unwrap();
    std::fs::write(
        root.join("soma/soma/soma.v"),
        "module soma(input clk);\nendmodule\n",
    )
    .unwrap();
    std::fs::write(
        root.join("soma/soma/soma_tb.v"),
        "module soma_tb;\nsoma u(.clk(1'b0));\nendmodule\n",
    )
    .unwrap();
    aurora_in(&root)
        .args(["add", "soma/soma/soma.v", "soma/soma/soma_tb.v"])
        .assert()
        .success();

    aurora_in(&root)
        .args(["proc", "remove", "nada"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("available: soma, filtro"));
    // A lixeira pede um sim; sem terminal, só com --yes, e nada muda.
    aurora_in(&root)
        .args(["proc", "remove", "soma", "--delete"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--delete --yes"));
    aurora_in(&root)
        .args(["proc", "remove", "soma", "--yes"])
        .assert()
        .code(2);
    assert_eq!(spf(&root)["structure"]["processors"][0]["name"], "soma");

    aurora_in(&root)
        .args(["proc", "remove", "soma"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Processor removed from the project: soma",
        ))
        .stdout(predicate::str::contains("(it was the top)"))
        .stdout(predicate::str::contains("(it was the selected testbench)"))
        .stdout(predicate::str::contains("Its folder stays on disk"));
    let doc = spf(&root);
    assert_eq!(
        doc["structure"]["processors"],
        serde_json::json!([{"name": "filtro", "language": "cpp"}])
    );
    assert_eq!(
        doc["structure"]["synthesizableFiles"],
        serde_json::json!([])
    );
    assert_eq!(doc["structure"]["testbenchFiles"], serde_json::json!([]));
    assert_eq!(doc["structure"]["topLevelFile"], "");
    assert!(root.join("soma/soma.cmm").is_file());

    let report = json(
        aurora_in(&root).args(["--json", "proc", "remove", "filtro"]),
        0,
    );
    assert_eq!(report["removed"]["name"], "filtro");
    assert_eq!(report["removed"]["unregistered"], serde_json::json!([]));
    assert_eq!(report["trashed"], false);
}

#[test]
fn build_without_processors_exits_0() {
    let (_guard, root) = new_project("p");
    // Sem bundle nenhum: um projeto só de Verilog nem chega a precisar dele.
    aurora_in(&root)
        .arg("build")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "The project has no SAPHO processors",
        ));
    let report = json(aurora_in(&root).args(["--json", "build"]), 0);
    assert_eq!(report["results"], serde_json::json!([]));
}

#[test]
fn build_adds_the_generated_files_to_the_project() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    if !bundle_has(&tc, "yanc") {
        return;
    }
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .args(["proc", "add", "soma"])
        .assert()
        .success();
    let soma_v = Utf8Path::new("soma").join("soma").join("soma.v");
    let soma_tb = Utf8Path::new("soma").join("soma").join("soma_tb.v");
    aurora_in(&root)
        .env("AURORA_TOOLCHAIN", &tc)
        .arg("build")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Module added: {soma_v} (top)"
        )))
        .stdout(predicate::str::contains(format!(
            "Testbench added: {soma_tb} (selected testbench)"
        )));
    let structure = &spf(&root)["structure"];
    assert_eq!(structure["topLevelFile"], "soma/soma/soma.v");
    assert_eq!(structure["testbenchFile"], "soma/soma/soma_tb.v");

    // Já registrados: o build seguinte não repete. Tirado, volta.
    let report = json(
        aurora_in(&root)
            .env("AURORA_TOOLCHAIN", &tc)
            .args(["--json", "build"]),
        0,
    );
    assert_eq!(report["registered"], serde_json::json!([]));
    aurora_in(&root)
        .args(["remove", soma_v.as_str()])
        .assert()
        .success();
    let report = json(
        aurora_in(&root)
            .env("AURORA_TOOLCHAIN", &tc)
            .args(["--json", "build"]),
        0,
    );
    assert_eq!(report["registered"][0]["role"], "synthesizable");
    let files = spf(&root)["structure"]["synthesizableFiles"].clone();
    assert_eq!(files[0]["path"], "soma/soma/soma.v", "{files:#}");
}

#[test]
fn unknown_processor_lists_available_ones() {
    let (_guard, root) = example("soma");
    // O binário de teste não tem bundle: o nome errado é o erro, antes dele,
    // em todo fluxo que nomeia um processador.
    for command in ["build", "check", "hierarchy", "sim", "synth"] {
        aurora()
            .args(["-C", root.as_str(), command, "-p", "nada"])
            .assert()
            .code(2)
            .stderr(predicate::str::contains("soma, filtro"));
    }
}

#[test]
fn missing_bundle_is_exit_code_2() {
    let (_guard, root) = example("soma");
    // O binário de teste não tem bundle ao lado dele.
    let err = json(aurora().args(["-C", root.as_str(), "--json", "build"]), 2);
    assert_eq!(err["error"]["code"], "bundle_not_found");

    let err = json(
        aurora().args([
            "-C",
            root.as_str(),
            "--json",
            "build",
            "--toolchain",
            "/nao/existe",
        ]),
        2,
    );
    assert_eq!(err["error"]["code"], "invalid_bundle");
}

#[test]
fn bundle_next_to_the_executable_is_found_and_verified() {
    let (_guard, dir) = tempdir();
    let exe = assert_cmd::cargo::cargo_bin("aurora");
    let install = dir.join("instalacao");
    std::fs::create_dir_all(install.join("bin")).unwrap();
    let installed = install
        .join("bin")
        .join(exe.file_name().unwrap().to_str().unwrap());
    std::fs::copy(&exe, &installed).unwrap();
    fake_bundle(&install.join("toolchain"));

    let run = |args: &[&str], code: i32| {
        let mut cmd = std::process::Command::new(installed.as_std_path());
        cmd.args(args)
            .env_remove("AURORA_TOOLCHAIN")
            .env_remove("AURORA_COMPILER")
            .env("NO_COLOR", "1");
        let out = cmd.output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let tools = run(&["--json", "tools", "--verify"], 0);
    assert_eq!(
        Utf8Path::new(tools["root"].as_str().unwrap()),
        install.join("toolchain")
    );
    assert_eq!(tools["bundle"], "teste");
    assert_eq!(tools["verify"], serde_json::json!([]));
    assert_eq!(
        tools["tools"]["yosys"]["error"]["code"],
        "component_missing"
    );

    // Um executável alterado aparece no --verify, com código 1.
    std::fs::write(install.join("toolchain/yanc/bin/arquivo"), "mudou").unwrap();
    let tools = run(&["--json", "tools", "--verify"], 1);
    assert_eq!(tools["verify"][0]["path"], "yanc/bin/arquivo");
}

#[test]
fn compiler_is_declared_by_option_or_variable() {
    let (_guard, dir) = tempdir();
    fake_bundle(&dir.join("bundle"));
    let bundle = dir.join("bundle");

    // Um diretório sem os três programas é recusado, e não esquecido.
    let empty = dir.join("vazio");
    std::fs::create_dir_all(&empty).unwrap();
    aurora()
        .args(["tools", "--toolchain", bundle.as_str()])
        .args(["--compiler", empty.as_str()])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("is missing perl, make"));

    // Um diretório com os três programas (vazios bastam para a conferência).
    let fake = dir.join("compilador");
    std::fs::create_dir_all(&fake).unwrap();
    if cfg!(windows) {
        std::fs::create_dir_all(fake.join("usr/bin")).unwrap();
        std::fs::create_dir_all(fake.join("ucrt64/bin")).unwrap();
        for f in ["usr/bin/perl.exe", "usr/bin/make.exe", "ucrt64/bin/g++.exe"] {
            std::fs::write(fake.join(f), "").unwrap();
        }
    } else {
        for f in ["perl", "make", "g++"] {
            std::fs::write(fake.join(f), "").unwrap();
        }
    }
    let by_option = json(
        aurora()
            .args(["--json", "tools", "--toolchain", bundle.as_str()])
            .args(["--compiler", fake.as_str()]),
        0,
    );
    let by_variable = json(
        aurora()
            .args(["--json", "tools", "--toolchain", bundle.as_str()])
            .env("AURORA_COMPILER", fake.as_str()),
        0,
    );
    for tools in [by_option, by_variable] {
        assert!(
            tools["system_compiler"]["cxx"]
                .as_str()
                .unwrap()
                .contains("g++"),
            "{tools:#}"
        );
        assert!(Utf8Path::new(tools["tools"]["perl"]["path"].as_str().unwrap()).starts_with(&fake));
    }

    // O comando que guardava isso num arquivo saiu.
    aurora().arg("config").assert().code(2);
}

/// As mensagens do Core são neutras; a dica com o comando vem da CLI, no
/// texto e no JSON. O bundle de mentira basta: o erro vem antes de qualquer
/// ferramenta.
#[test]
fn error_hints_name_the_command() {
    let (_guard, root) = new_project("p");
    let bundle = root.join("../bundle");
    fake_bundle(&bundle);
    let run = |args: &[&str]| {
        let mut cmd = aurora_in(&root);
        cmd.args(args).env("AURORA_TOOLCHAIN", bundle.as_str());
        cmd
    };

    run(&["sim"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("has no testbench"))
        .stderr(predicate::str::contains(
            "Create it with: aurora add <name>_tb.v",
        ));
    let err = json(&mut run(&["--json", "sim"]), 2);
    assert_eq!(err["error"]["code"], "no_testbench");
    assert_eq!(
        err["error"]["hint"],
        "Create it with: aurora add <name>_tb.v, or a cocotb one with: aurora add test_<name>.py"
    );

    run(&["synth"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "Choose it with: aurora top <file|module>",
        ));

    let err = json(&mut run(&["--json", "sim", "-p", "nada"]), 2);
    assert_eq!(err["error"]["code"], "processor_not_found");
    assert_eq!(err["error"]["hint"], "Create it with: aurora proc add nada");
}

// Arquivos Verilog: dependem do Core novo (add_verilog, set_top, ...).

#[test]
fn add_creates_module_and_testbench() {
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .args(["add", "and_gate.v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Module created: and_gate.v (top)"));
    let module = std::fs::read_to_string(root.join("and_gate.v")).unwrap();
    assert!(module.contains("module and_gate"), "{module}");

    aurora_in(&root)
        .args(["add", "and_gate_tb.v"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Testbench created: and_gate_tb.v (selected testbench)",
        ));
    let tb = std::fs::read_to_string(root.join("and_gate_tb.v")).unwrap();
    assert!(tb.contains("and_gate "), "instancia o módulo:\n{tb}");
    assert!(tb.contains("$finish"), "{tb}");

    // Um nome que não indica testbench vira testbench com --tb.
    let added = json(
        aurora_in(&root).args(["--json", "add", "--tb", "verifica.v"]),
        0,
    );
    assert_eq!(added["files"][0]["role"], "testbench");
    assert_eq!(added["files"][0]["created"], true);
    assert_eq!(added["files"][0]["selected"], false);

    let doc = spf(&root);
    assert_eq!(doc["structure"]["topLevelFile"], "and_gate.v");
    assert_eq!(doc["structure"]["testbenchFile"], "and_gate_tb.v");
}

#[test]
fn add_registers_existing_files_relative_to_the_shell() {
    let (_guard, root) = new_project("p");
    let rtl = root.join("rtl");
    std::fs::create_dir_all(&rtl).unwrap();
    std::fs::write(
        rtl.join("soma.v"),
        "module soma(input wire [3:0] a, b, output wire [4:0] y);\n  assign y = a + b;\nendmodule\n",
    )
    .unwrap();
    std::fs::write(
        rtl.join("estimulo.v"),
        "module estimulo;\n  initial begin\n    #10 $display(\"ok\");\n    $finish;\n  end\nendmodule\n",
    )
    .unwrap();
    std::fs::write(
        rtl.join("forcado.v"),
        "module forcado(input wire a, output wire y);\n  assign y = a;\nendmodule\n",
    )
    .unwrap();

    // De dentro de rtl/, com -C apontando o projeto: os caminhos são de rtl/.
    let added = json(
        aurora_in(&rtl).args(["-C", "..", "--json", "add", "soma.v", "estimulo.v"]),
        0,
    );
    let files = added["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(
        Utf8Path::new(files[0]["path"].as_str().unwrap()),
        rtl.join("soma.v")
    );
    assert_eq!(files[0]["role"], "synthesizable");
    assert_eq!(files[0]["created"], false);
    assert_eq!(files[0]["selected"], true);
    assert_eq!(files[1]["role"], "testbench", "classificado pelo conteúdo");
    assert_eq!(files[1]["selected"], true);

    aurora_in(&rtl)
        .args(["-C", "..", "add", "--tb", "forcado.v"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Testbench added: {}",
            shown("rtl/forcado.v")
        )));
    let doc = spf(&root);
    assert_eq!(
        doc["structure"]["synthesizableFiles"][0]["path"],
        "rtl/soma.v"
    );
    assert_eq!(
        doc["structure"]["testbenchFiles"][1]["path"],
        "rtl/forcado.v"
    );

    // Arquivo inexistente numa pasta que não existe: o Core cria.
    aurora_in(&root)
        .args(["add", "novo/dentro.v"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Module created: {}",
            shown("novo/dentro.v")
        )));
    assert!(root.join("novo/dentro.v").is_file());
}

#[test]
fn remove_unregisters_without_deleting() {
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .args(["add", "a.v", "b.v"])
        .assert()
        .success();

    aurora_in(&root)
        .args(["remove", "a.v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed from the project: a.v"));
    assert!(root.join("a.v").is_file(), "remover não apaga do disco");

    // Nenhum registrado: erro.
    aurora_in(&root)
        .args(["remove", "a.v"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("a.v is not in the project"));

    // Um registrado e um não: remove o que estava e avisa do outro.
    let removed = json(aurora_in(&root).args(["--json", "remove", "b.v", "a.v"]), 0);
    assert_eq!(
        Utf8Path::new(removed["removed"][0].as_str().unwrap()),
        root.join("b.v")
    );
    assert_eq!(
        Utf8Path::new(removed["not_registered"][0].as_str().unwrap()),
        root.join("a.v")
    );
    aurora_in(&root).args(["add", "a.v"]).assert().success();
    aurora_in(&root)
        .args(["remove", "a.v", "b.v"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Warning: b.v was not in the project",
        ));

    let status = json(aurora_in(&root).args(["--json", "status"]), 0);
    assert_eq!(status["synthesizable"], serde_json::json!([]));
    assert_eq!(status["unregistered"].as_array().unwrap().len(), 2);
}

#[test]
fn top_by_file_and_by_module() {
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .args(["add", "a.v", "b.v"])
        .assert()
        .success();
    aurora_in(&root)
        .arg("top")
        .assert()
        .success()
        .stdout(predicate::str::contains("Top module: a (a.v)"));

    aurora_in(&root)
        .args(["top", "b.v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Top module: b (b.v)"));
    assert_eq!(spf(&root)["structure"]["topLevelFile"], "b.v");

    // Pelo nome do módulo, de qualquer pasta.
    let sub = root.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    let top = json(aurora_in(&sub).args(["-C", "..", "--json", "top", "a"]), 0);
    assert_eq!(
        Utf8Path::new(top["top_level"].as_str().unwrap()),
        root.join("a.v")
    );
    assert_eq!(top["module"], "a");
}

#[test]
fn move_keeps_files_in_the_project() {
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .args(["add", "a.v", "a_tb.v"])
        .assert()
        .success();
    let rtl_a = Utf8Path::new("rtl").join("a.v");
    aurora_in(&root)
        .args(["move", "a.v", rtl_a.as_str()])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("Moved: a.v -> {rtl_a}")))
        .stdout(predicate::str::contains(format!(
            "{rtl_a} (still in the project, top)"
        )));
    assert!(root.join("rtl/a.v").is_file() && !root.join("a.v").exists());
    assert_eq!(spf(&root)["structure"]["topLevelFile"], "rtl/a.v");

    // Para uma pasta que existe, cada origem vai para dentro dela.
    std::fs::create_dir(root.join("sim")).unwrap();
    std::fs::write(root.join("leia.txt"), "x").unwrap();
    let moved = json(
        aurora_in(&root).args(["--json", "move", "a_tb.v", "leia.txt", "sim"]),
        0,
    );
    assert_eq!(moved["moved"].as_array().unwrap().len(), 2, "{moved:#}");
    let file = &moved["moved"][0]["files"][0];
    assert_eq!(file["role"], "testbench");
    assert_eq!(file["top_level"], true);
    assert_eq!(moved["moved"][1]["files"], serde_json::json!([]));
    assert_eq!(spf(&root)["structure"]["testbenchFile"], "sim/a_tb.v");

    // Várias origens precisam de uma pasta; o .spf não se move; o destino
    // que existe não é sobrescrito.
    aurora_in(&root)
        .args(["move", "sim/a_tb.v", "sim/leia.txt", "novo.v"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("must be an existing folder"));
    let refused = json(
        aurora_in(&root).args(["--json", "move", "p.spf", "q.spf"]),
        2,
    );
    assert_eq!(refused["error"]["code"], "cannot_move");
    let exists = json(
        aurora_in(&root).args(["--json", "move", "rtl/a.v", "sim/a_tb.v"]),
        2,
    );
    assert_eq!(exists["error"]["code"], "path_exists");
    assert!(root.join("rtl/a.v").is_file());

    // Destino com `/` no fim é pasta, criada se faltar.
    aurora_in(&root)
        .args(["move", "rtl/a.v", "hdl/"])
        .assert()
        .success();
    assert!(root.join("hdl/a.v").is_file());
    assert_eq!(spf(&root)["structure"]["topLevelFile"], "hdl/a.v");
}

#[test]
fn move_after_records_what_an_editor_moved() {
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .args(["add", "a.v", "a_tb.v"])
        .assert()
        .success();

    // A pasta renomeada: `hdl` é a própria `rtl`, não a pasta de destino.
    std::fs::create_dir(root.join("rtl")).unwrap();
    aurora_in(&root)
        .args(["move", "a.v", "rtl/"])
        .assert()
        .success();
    std::fs::rename(root.join("rtl"), root.join("hdl")).unwrap();
    let hdl_a = Utf8Path::new("hdl").join("a.v");
    aurora_in(&root)
        .args(["move", "--after", "rtl", "hdl"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Recorded move: rtl -> hdl"))
        .stdout(predicate::str::contains(format!(
            "{hdl_a} (still in the project, top)"
        )));
    assert_eq!(spf(&root)["structure"]["topLevelFile"], "hdl/a.v");

    // Arrastado para dentro de uma pasta que já existia.
    std::fs::rename(root.join("a_tb.v"), root.join("hdl/a_tb.v")).unwrap();
    let moved = json(
        aurora_in(&root).args(["--json", "move", "--after", "a_tb.v", "hdl"]),
        0,
    );
    assert_eq!(
        Utf8Path::new(moved["moved"][0]["to"].as_str().unwrap()),
        root.join("hdl/a_tb.v")
    );
    assert_eq!(spf(&root)["structure"]["testbenchFile"], "hdl/a_tb.v");

    // O que não foi movido é recusado, e nada muda.
    let refused = json(
        aurora_in(&root).args(["--json", "move", "--after", "hdl/a.v", "b.v"]),
        2,
    );
    assert_eq!(refused["error"]["code"], "invalid_project", "{refused:#}");
    assert!(root.join("hdl/a.v").is_file());
}

#[test]
fn hierarchy_shows_design_and_testbench() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    if !bundle_has(&tc, "icarus") {
        return;
    }
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .args(["add", "porta.v", "porta_tb.v"])
        .assert()
        .success();
    aurora_in(&root)
        .env("AURORA_TOOLCHAIN", &tc)
        .arg("hierarchy")
        .assert()
        .success()
        .stdout(predicate::str::contains("Design\n  porta\n"))
        .stdout(predicate::str::contains(
            "Testbench porta_tb.v\n  porta_tb\n    dut: porta",
        ))
        .stdout(predicate::str::contains("Hierarchy (iverilog): finished"));
    // Não compila nem grava relatório.
    assert!(!root.join(".aurora8/reports").exists());

    let result = json(
        aurora_in(&root)
            .env("AURORA_TOOLCHAIN", &tc)
            .args(["--json", "hierarchy"]),
        0,
    );
    assert_eq!(result["design"]["roots"][0]["module"], "porta");
    assert_eq!(
        result["testbenches"][0]["roots"][0]["children"][0]["name"],
        "dut"
    );

    // Verilog que não elabora: código 1, com o erro e a linha.
    std::fs::write(
        root.join("porta.v"),
        "module porta(input a, output y);\n  assign y = ;\nendmodule\n",
    )
    .unwrap();
    aurora_in(&root)
        .env("AURORA_TOOLCHAIN", &tc)
        .arg("hierarchy")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("did not elaborate"))
        .stdout(predicate::str::contains("porta.v:2:"));
}

#[test]
fn top_refuses_a_testbench_name() {
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .args(["add", "a.v", "a_tb.v"])
        .assert()
        .success();
    let error = json(aurora_in(&root).args(["--json", "top", "a_tb.v"]), 2);
    assert_eq!(error["error"]["code"], "invalid_name");
    assert_eq!(spf(&root)["structure"]["topLevelFile"], "a.v");
}

#[test]
fn top_unknown_module_lists_modules() {
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .args(["add", "and_gate.v"])
        .assert()
        .success();
    aurora_in(&root)
        .args(["top", "nada"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("nada"))
        .stderr(predicate::str::contains("and_gate"));
    let err = json(aurora_in(&root).args(["--json", "top", "nada"]), 2);
    assert_eq!(err["error"]["code"], "module_not_found");
}

#[test]
fn status_of_an_empty_project_explains_both_flows() {
    let (_guard, root) = new_project("p");
    let text = stdout(aurora_in(&root).arg("status"), 0);
    assert!(text.contains("Empty project"), "{text}");
    assert!(text.contains("aurora add <name>.v"), "{text}");
    assert!(text.contains("aurora proc add <name>"), "{text}");
}

#[test]
fn status_lists_unregistered_files() {
    let (_guard, root) = new_project("p");
    std::fs::create_dir_all(root.join("rtl")).unwrap();
    std::fs::write(root.join("rtl/solto.v"), "module solto; endmodule\n").unwrap();
    let text = stdout(aurora_in(&root).arg("status"), 0);
    assert!(text.contains("Untracked files"), "{text}");
    assert!(text.contains(&shown("rtl/solto.v")), "{text}");
    assert!(text.contains("Add one with: aurora add <file>"), "{text}");

    let status = json(aurora_in(&root).args(["--json", "status"]), 0);
    assert_eq!(
        Utf8Path::new(status["unregistered"][0].as_str().unwrap()),
        root.join("rtl/solto.v")
    );
}

#[test]
fn status_shows_both_flows() {
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .args(["add", "and_gate.v", "and_gate_tb.v"])
        .assert()
        .success();
    aurora_in(&root)
        .args(["proc", "add", "soma"])
        .assert()
        .success();
    let text = stdout(aurora_in(&root).arg("status"), 0);
    for expected in [
        "Modules",
        "and_gate.v  top: and_gate",
        "Testbenches",
        "and_gate_tb.v  selected",
        "Processors",
        "soma",
        "not built",
    ] {
        assert!(text.contains(expected), "falta {expected:?}:\n{text}");
    }
    assert!(!text.contains("Empty project"), "{text}");

    let status = json(aurora_in(&root).args(["--json", "status"]), 0);
    assert_eq!(status["name"], "p");
    assert_eq!(status["top_module"], "and_gate");
    assert_eq!(status["testbench_module"], "and_gate_tb");
    assert_eq!(status["processors"][0]["name"], "soma");
    assert_eq!(status["processors"][0]["built"], false);
    assert_eq!(status["unregistered"], serde_json::json!([]));
}

#[test]
fn check_of_an_empty_project_hints_both_flows() {
    let (_guard, root) = new_project("p");
    let bundle = root.join("../bundle");
    fake_bundle(&bundle);
    let err = json(
        aurora_in(&root)
            .args(["--json", "check"])
            .env("AURORA_TOOLCHAIN", bundle.as_str()),
        2,
    );
    assert_eq!(err["error"]["code"], "empty_project");
    assert!(
        err["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("aurora proc add")
    );
}

#[test]
fn wave_of_a_processor_before_simulating_hints_sim() {
    let (_guard, root) = example("soma");
    let err = json(aurora_in(&root).args(["--json", "wave", "-p", "soma"]), 2);
    assert_eq!(err["error"]["code"], "not_built");
    assert_eq!(
        err["error"]["hint"],
        "Build and simulate with: aurora sim -p soma"
    );
}

// Com ferramentas (AURORA_TEST_BUNDLE).

#[test]
fn build_ok_is_exit_code_0() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("soma");
    let report = json(
        aurora()
            .args(["-C", root.as_str(), "--json", "build"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    let results = report["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r["status"] == "succeeded"));
}

#[test]
fn build_error_is_exit_code_1_with_file_and_line() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("com_erro");
    let line = 15;
    aurora()
        .args(["-C", root.as_str(), "build", "-p", "conta"])
        .env("AURORA_TOOLCHAIN", &tc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains(format!(
            "conta.cmm:{line}: error:"
        )));
}

#[test]
fn build_reports_all_failures_but_sim_stops_at_first() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("com_erro");
    let run = |args: &[&str], code: i32| {
        let mut cmd = aurora();
        cmd.args(["-C", root.as_str(), "--json"])
            .args(args)
            .env("AURORA_TOOLCHAIN", &tc);
        json(&mut cmd, code)
    };
    let build = run(&["build"], 1);
    assert_eq!(build["results"].as_array().unwrap().len(), 2);

    let sim = run(&["sim"], 1);
    assert_eq!(sim["builds"].as_array().unwrap().len(), 1);
    assert!(sim["simulation"].is_null());
}

#[test]
fn check_does_not_build_the_processors() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("soma");
    let check = |args: &[&str], code: i32| {
        json(
            aurora_in(&root)
                .args(["--json", "check"])
                .args(args)
                .env("AURORA_TOOLCHAIN", &tc),
            code,
        )
    };
    // Sem compilar antes, não há o Verilog do processador para verificar.
    for args in [&[][..], &["-p", "soma"][..]] {
        let err = check(args, 2);
        assert_eq!(err["error"]["code"], "not_built", "{args:?}");
        assert_eq!(err["error"]["hint"], "Build it with: aurora build -p soma");
    }
    // Com um processador compilado e o outro não, o check roda e avisa do
    // que ficou de fora.
    aurora_in(&root)
        .args(["build", "-p", "soma"])
        .env("AURORA_TOOLCHAIN", &tc)
        .assert()
        .success();
    let partial = check(&[], 0);
    let warned = partial["check"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| {
            d["message"] == "Processor filtro has not been built: its Verilog is not in the check"
        });
    assert!(warned, "{partial}");
    // Compilado, o check só roda o Icarus: nenhum passo do YANC, e o
    // Verilog gerado não é reescrito.
    aurora_in(&root)
        .args(["build"])
        .env("AURORA_TOOLCHAIN", &tc)
        .assert()
        .success();
    let verilog = root.join("soma/soma/soma.v");
    let written = std::fs::metadata(&verilog).unwrap().modified().unwrap();
    let report = check(&["-p", "soma"], 0);
    assert!(report.get("builds").is_none(), "{report}");
    let tools: Vec<&str> = report["check"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["tool"].as_str().unwrap())
        .collect();
    assert!(tools.iter().all(|t| *t == "iverilog"), "{tools:?}");
    assert_eq!(
        std::fs::metadata(&verilog).unwrap().modified().unwrap(),
        written
    );
}

#[test]
fn processor_simulation_prints_outputs() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("soma");
    aurora_in(&root)
        .args(["sim", "-p", "soma"])
        .env("AURORA_TOOLCHAIN", &tc)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Build of soma (C±, 100 MHz, 2000 clocks): finished in",
        ))
        .stdout(predicate::str::contains(
            "Simulation of soma_tb (Icarus): finished in",
        ))
        .stdout(predicate::str::contains(format!(
            "testbench           {}",
            shown("soma/soma/soma_tb.v")
        )))
        .stdout(predicate::str::contains("Output 0: 55"))
        .stdout(predicate::str::contains(
            "Open the waveform with: aurora wave -p soma",
        ));
    // O que o YANC gera fica na pasta de saída, como na AURORA: o testbench
    // e a saída da simulação, que roda nela.
    assert!(root.join("soma/soma/soma_tb.v").is_file());
    assert!(root.join("soma/soma/output_0.txt").is_file());

    let report = json(
        aurora_in(&root)
            .args(["--json", "sim", "-p", "filtro"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    assert_eq!(report["builds"].as_array().unwrap().len(), 1);
    assert_eq!(report["builds"][0]["processor"], "filtro");
    assert_eq!(report["simulation"]["status"], "succeeded");
    assert_eq!(report["outputs"][0]["port"], 0);
    assert_eq!(report["outputs"][0]["values"], serde_json::json!([55]));
}

/// Um programa que lê a porta 0: sem o arquivo de entrada, a CLI diz qual
/// criar; com ele, a saída muda.
#[test]
fn processor_simulation_names_missing_inputs() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = new_project("p");
    aurora_in(&root)
        .args(["proc", "add", "eco"])
        .assert()
        .success();
    let source = root.join("eco/eco.cmm");
    let header = std::fs::read_to_string(&source).unwrap();
    let header = &header[..header.find("void main").unwrap()];
    std::fs::write(
        &source,
        format!("{header}void main()\n{{\n    int x = in(0);\n    out(0, x + 1);\n}}\n"),
    )
    .unwrap();

    aurora_in(&root)
        .args(["sim", "-p", "eco"])
        .env("AURORA_TOOLCHAIN", &tc)
        .assert()
        .success()
        // O caminho vem com o separador do sistema.
        .stdout(predicate::str::contains(format!(
            "Create {} with one value per line",
            std::path::Path::new("eco").join("input_0.txt").display()
        )));

    std::fs::write(root.join("eco/input_0.txt"), "4\n").unwrap();
    let report = json(
        aurora_in(&root)
            .args(["--json", "sim", "-p", "eco"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    assert_eq!(
        report["simulation"]["missing_inputs"],
        serde_json::json!([])
    );
    assert_eq!(report["outputs"][0]["values"], serde_json::json!([5]));
}

/// Numa pasta com acento, outro alfabeto e espaço, cada operação roda como
/// em qualquer outra. No Windows, as ferramentas abrem arquivos pela página
/// de código ANSI: os caminhos com caractere fora do ASCII vão relativos ao
/// CWD delas, e o Verilator nasce no nome curto da pasta. Antes, a simulação
/// era recusada, a síntese não abria o log e a hierarquia não lia o `.vvp`.
#[test]
fn every_operation_runs_in_a_folder_with_accents_and_other_scripts() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, dir) = tempdir();
    let parent = dir.join("Área de Trabalho Δσ 日本");
    std::fs::create_dir(&parent).unwrap();
    aurora_in(&parent).args(["new", "p"]).assert().success();
    let root = parent.join("p");
    aurora_in(&root)
        .args(["proc", "add", "eco"])
        .assert()
        .success();
    let source = root.join("eco/eco.cmm");
    let header = std::fs::read_to_string(&source).unwrap();
    let header = &header[..header.find("void main").unwrap()];
    std::fs::write(
        &source,
        format!("{header}void main()\n{{\n    int x = in(0);\n    out(0, x + 1);\n}}\n"),
    )
    .unwrap();
    std::fs::write(root.join("eco/input_0.txt"), "4\n").unwrap();
    // O contador do exemplo como topo e testbench do projeto.
    let rtl = Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/contador/rtl");
    std::fs::create_dir(root.join("rtl")).unwrap();
    for file in ["contador.v", "contador_tb.v"] {
        std::fs::copy(rtl.join(file), root.join("rtl").join(file)).unwrap();
    }
    aurora_in(&root)
        .args(["add", "rtl/contador.v", "rtl/contador_tb.v"])
        .env("AURORA_TOOLCHAIN", &tc)
        .assert()
        .success();
    let run = |args: &[&str]| {
        json(
            aurora_in(&root)
                .arg("--json")
                .args(args)
                .env("AURORA_TOOLCHAIN", &tc),
            0,
        )
    };

    let sim = run(&["sim", "-p", "eco"]);
    assert_eq!(sim["outputs"][0]["values"], serde_json::json!([5]));
    run(&["sim"]);
    run(&["check"]);
    let hierarchy = run(&["hierarchy"]);
    let top = &hierarchy["design"]["roots"][0];
    assert_eq!(top["module"], "contador", "{hierarchy:#}");
    assert_eq!(
        top["file"],
        root.join("rtl").join("contador.v").as_str(),
        "{hierarchy:#}"
    );
    let signals = run(&["wave", "signals"]);
    assert!(!signals["root"].is_null(), "{signals:#}");
    run(&["synth"]);
    if bundle_has(&tc, "verilator") {
        let fast = run(&["sim", "-p", "eco", "--fast"]);
        assert_eq!(fast["outputs"][0]["values"], serde_json::json!([5]));
        run(&["check", "--lint"]);
    }
}

/// O `$display` do testbench sai sempre, sem -v, e as linhas que a AURORA
/// interpreta (`$finish called at`) não se repetem nele.
#[test]
fn project_simulation_always_prints_testbench_stdout() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("contador");
    let text = stdout(aurora_in(&root).arg("sim").env("AURORA_TOOLCHAIN", &tc), 0);
    assert!(
        text.contains("Simulation of contador_tb (Icarus): finished in"),
        "{text}"
    );
    assert!(text.contains("| q = 10"), "{text}");
    assert!(!text.contains("$finish called"), "{text}");

    let sim = json(
        aurora_in(&root)
            .args(["--json", "sim"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    assert_eq!(sim["simulation"]["waveform"]["format"], "fst");
    let wave = Utf8PathBuf::from(sim["simulation"]["waveform"]["path"].as_str().unwrap());
    assert_eq!(wave.parent(), Some(root.as_path()), "a onda fica na raiz");
    assert_eq!(sim["outputs"], serde_json::json!([]));
}

#[test]
fn sim_with_testbench_argument_selects_it() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("contador");
    let original = std::fs::read_to_string(root.join("rtl/contador_tb.v")).unwrap();
    std::fs::write(
        root.join("rtl/outro_tb.v"),
        original
            .replace("module contador_tb;", "module outro_tb;")
            .replace("q = ", "outro q = "),
    )
    .unwrap();

    // De dentro de rtl/: o argumento é relativo ao diretório atual.
    let rtl = root.join("rtl");
    let text = stdout(
        aurora_in(&rtl)
            .args(["-C", "..", "sim", "outro_tb.v"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    assert!(
        text.contains("Simulation of outro_tb (Icarus): finished in"),
        "{text}"
    );
    assert!(text.contains("| outro q = 10"), "{text}");
    assert_eq!(spf(&root)["structure"]["testbenchFile"], "rtl/outro_tb.v");

    // Sem argumento, continua no escolhido.
    let sim = json(
        aurora_in(&root)
            .args(["--json", "sim"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    assert_eq!(sim["simulation"]["top"], "outro_tb");
}

#[test]
fn synthesis_of_the_top_level() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("contador");
    let synth = json(
        aurora_in(&root)
            .args(["--json", "synth", "--svg"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    assert_eq!(synth["builds"], serde_json::json!([]));
    assert_eq!(
        synth["synthesis"]["modules"],
        serde_json::json!(["contador"])
    );
    assert!(Utf8Path::new(synth["schematic"]["svg"].as_str().unwrap()).is_file());
}

#[test]
fn synthesis_of_a_processor_builds_it_first() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("soma");
    let synth = json(
        aurora_in(&root)
            .args(["--json", "synth", "-p", "soma"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    assert_eq!(synth["builds"][0]["status"], "succeeded");
    assert_eq!(synth["synthesis"]["top"], "soma");
    assert!(synth["schematic"].is_null());
}

/// Sem display o Surfer não abre janela nenhuma: morre logo, e o comando
/// precisa dizer isso em vez de "aberto". Só no Linux (e outros Unix com X11
/// ou Wayland): no macOS e no Windows a interface gráfica não depende de
/// `DISPLAY`, e o Surfer abriria de verdade.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn wave_with_relative_path_and_without_display() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("contador");
    let sim = json(
        aurora_in(&root)
            .args(["--json", "sim"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    let wave = Utf8PathBuf::from(sim["simulation"]["waveform"]["path"].as_str().unwrap());
    // Relativo ao diretório atual, que não é a raiz do projeto.
    let relative = format!("../{}", wave.file_name().unwrap());
    let err = json(
        aurora_in(&root.join("rtl"))
            .args(["--json", "wave", &relative])
            .env("AURORA_TOOLCHAIN", &tc)
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY"),
        2,
    );
    assert_eq!(err["error"]["code"], "process_exited_early");
}

// Com ferramentas e com o Core novo.

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn wave_without_argument_uses_the_project_waveform() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("contador");
    aurora_in(&root)
        .args(["wave"])
        .env("AURORA_TOOLCHAIN", &tc)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("simulate first with: aurora sim"));

    let sim = json(
        aurora_in(&root)
            .args(["--json", "sim"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    let wave = Utf8PathBuf::from(sim["simulation"]["waveform"]["path"].as_str().unwrap());
    let err = json(
        aurora_in(&root)
            .args(["--json", "wave"])
            .env("AURORA_TOOLCHAIN", &tc)
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY"),
        2,
    );
    assert_eq!(err["error"]["code"], "process_exited_early");
    // O log do Surfer fica na pasta oculta do projeto, não ao lado da onda.
    let name = wave.file_name().unwrap();
    assert!(
        err["error"]["message"]
            .as_str()
            .unwrap()
            .contains(&format!(".aurora8/Temp/surfer/{name}.log")),
        "{err}"
    );
    let beside: Vec<_> = std::fs::read_dir(wave.parent().unwrap())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|f| f.contains(".log"))
        .collect();
    assert!(beside.is_empty(), "log ao lado da onda: {beside:?}");
}

#[test]
fn check_project_single_file_and_lint() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("contador");
    let run = |args: &[&str], code: i32| {
        let mut cmd = aurora_in(&root);
        cmd.arg("--json").args(args).env("AURORA_TOOLCHAIN", &tc);
        json(&mut cmd, code)
    };

    // O check não compila: o JSON não tem `builds`.
    let check = run(&["check"], 0);
    assert!(check.get("builds").is_none(), "{check}");
    assert_eq!(check["check"]["status"], "succeeded");
    let targets = check["check"]["targets"].as_array().unwrap();
    assert!(targets.contains(&"contador".into()), "{targets:?}");
    assert!(targets.contains(&"contador_tb".into()), "{targets:?}");

    let single = run(&["check", "rtl/contador.v"], 0);
    assert_eq!(single["check"]["targets"], serde_json::json!(["contador"]));

    // O lint é do Verilator, que um bundle parcial pode não ter.
    if bundle_has(&tc, "verilator") {
        let lint = run(&["check", "--lint"], 0);
        let steps = lint["check"]["steps"].as_array().unwrap();
        assert!(steps.iter().any(|s| s["step"] == "lint"), "{steps:?}");
    }

    // Erro de sintaxe: código 1, com arquivo e linha no texto.
    let file = root.join("rtl/contador.v");
    let broken = std::fs::read_to_string(&file)
        .unwrap()
        .replace("q <= q + 4'd1;", "q <= q + ;");
    std::fs::write(&file, broken).unwrap();
    aurora_in(&root)
        .arg("check")
        .env("AURORA_TOOLCHAIN", &tc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("contador.v:"))
        .stdout(predicate::str::contains("error"));
}

/// O fluxo Verilog inteiro a partir de um projeto vazio: criar módulo e
/// testbench, verificar e simular, sem escrever Verilog à mão.
#[test]
fn verilog_flow_from_scratch() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = new_project("demo");
    let run = |args: &[&str], code: i32| {
        let mut cmd = aurora_in(&root);
        cmd.arg("--json").args(args).env("AURORA_TOOLCHAIN", &tc);
        json(&mut cmd, code)
    };

    let added = run(&["add", "and_gate.v", "and_gate_tb.v"], 0);
    assert_eq!(added["files"][0]["role"], "synthesizable");
    assert_eq!(added["files"][1]["role"], "testbench");

    let check = run(&["check"], 0);
    assert_eq!(check["check"]["status"], "succeeded");

    let sim = run(&["sim"], 0);
    assert_eq!(sim["simulation"]["status"], "succeeded");
    assert_eq!(sim["simulation"]["top"], "and_gate_tb");
    let wave = Utf8PathBuf::from(sim["simulation"]["waveform"]["path"].as_str().unwrap());
    assert_eq!(wave.parent(), Some(root.as_path()));
    assert!(wave.is_file());

    let synth = run(&["synth"], 0);
    assert_eq!(synth["synthesis"]["top"], "and_gate");
}

// Cancelamento, prazo e saída ao vivo.

/// Imprime uma linha e nunca chega ao `$finish`.
const ENDLESS_TB: &str = "\
module infinito_tb;
    reg clk = 0;
    always #5 clk = ~clk;
    initial $display(\"comecou\");
endmodule
";

/// Um projeto com o testbench que não termina, escolhido para simular.
fn endless_project() -> (tempfile::TempDir, Utf8PathBuf) {
    let (guard, root) = new_project("demo");
    std::fs::write(root.join("infinito_tb.v"), ENDLESS_TB).unwrap();
    aurora_in(&root)
        .args(["add", "infinito_tb.v"])
        .assert()
        .success();
    (guard, root)
}

#[test]
fn sim_timeout_stops_a_testbench_without_finish() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = endless_project();

    let text = stdout(
        aurora_in(&root)
            .args(["sim", "--timeout", "1"])
            .env("AURORA_TOOLCHAIN", &tc),
        1,
    );
    assert!(
        text.contains("| comecou"),
        "a saída antes do prazo:\n{text}"
    );
    assert!(
        text.contains("timed out, vvp stopped at the 1 s limit"),
        "{text}"
    );
    assert!(text.contains("$finish"), "a dica do $finish:\n{text}");

    let sim = json(
        aurora_in(&root)
            .args(["--json", "sim", "--timeout", "1"])
            .env("AURORA_TOOLCHAIN", &tc),
        1,
    );
    assert_eq!(sim["simulation"]["status"], "timed_out");
    assert_eq!(sim["simulation"]["failed_step"], "simulate");
    let steps = sim["simulation"]["steps"].as_array().unwrap();
    assert_eq!(steps.last().unwrap()["termination"]["kind"], "timed_out");

    aurora_in(&root)
        .args(["sim", "--timeout", "0"])
        .assert()
        .code(2);
}

#[test]
fn events_are_one_json_object_per_line_ending_with_the_result() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("contador");
    let text = stdout(
        aurora_in(&root)
            .args(["--events", "sim"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    let events: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}: {line}")))
        .collect();
    let kinds: Vec<&str> = events
        .iter()
        .map(|e| e["event"].as_str().unwrap())
        .collect();
    assert_eq!(kinds.first(), Some(&"step_started"), "{kinds:?}");
    assert_eq!(kinds.last(), Some(&"result"), "{kinds:?}");
    assert_eq!(kinds.iter().filter(|k| **k == "result").count(), 1);

    // O $display do testbench chega como saída do programa; o `$finish
    // called at` do vvp, como mensagem da ferramenta.
    let output = |line: &str| {
        events
            .iter()
            .find(|e| e["event"] == "output" && e["line"].as_str().unwrap().contains(line))
            .unwrap_or_else(|| panic!("sem a linha {line}: {kinds:?}"))
    };
    assert_eq!(output("q = 10")["diagnostic"], false);
    assert_eq!(output("q = 10")["step"], "simulate");
    assert_eq!(output("$finish called")["diagnostic"], true);

    // A última linha é o mesmo objeto do --json.
    let result = &events.last().unwrap()["result"];
    assert_eq!(result["simulation"]["status"], "succeeded");
    assert!(result["simulation"]["steps"].is_array());
}

#[test]
fn events_report_errors_in_the_result_line() {
    let (dir, path) = tempdir();
    let text = stdout(
        aurora_in(&path).args(["--events", "-C", "nao_existe", "status"]),
        2,
    );
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1, "{text}");
    let line: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(line["event"], "result");
    assert!(line["result"]["error"]["code"].is_string(), "{line}");
    drop(dir);
}

/// O Ctrl+C chega ao `aurora` e não ao simulador, que roda num grupo de
/// processos próprio: quem encerra o vvp é a AURORA. Sem isso, o vvp de um
/// testbench sem `$finish` continuaria rodando depois que a AURORA saísse.
#[cfg(unix)]
#[test]
fn interrupt_cancels_and_leaves_no_simulator_behind() {
    use std::io::{BufRead, BufReader};

    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = endless_project();
    let exe = assert_cmd::cargo::cargo_bin("aurora");
    let mut child = std::process::Command::new(exe)
        .args(["--json", "sim"])
        .current_dir(&root)
        .env_remove("AURORA_TEST_BUNDLE")
        .env("AURORA_TOOLCHAIN", &tc)
        .env_remove("AURORA_COMPILER")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    // Espera o vvp começar: o .vvp aparece antes da simulação.
    let image = root.join(".aurora8/Temp/infinito_tb.vvp");
    let started = std::time::Instant::now();
    while !image.is_file() {
        assert!(started.elapsed().as_secs() < 30, "a simulação não começou");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    let pid = child.id().to_string();
    let sent = std::process::Command::new("/bin/kill")
        .args(["-INT", &pid])
        .status()
        .unwrap();
    assert!(sent.success());

    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(130));
    let mut out = String::new();
    for line in BufReader::new(child.stdout.take().unwrap()).lines() {
        out.push_str(&line.unwrap());
    }
    let sim: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(sim["simulation"]["status"], "cancelled", "{out}");

    // Nenhum processo segue rodando com o .vvp deste projeto.
    let left = std::process::Command::new("pgrep")
        .args(["-f", image.as_str()])
        .output()
        .unwrap();
    assert!(
        left.stdout.is_empty(),
        "sobrou: {}",
        String::from_utf8_lossy(&left.stdout)
    );
}

// Desinstalar.

/// Um `aurora` fora de uma instalação (este, em `target/`) não apaga nada.
#[test]
fn uninstall_outside_an_installation_is_refused() {
    aurora()
        .args(["uninstall", "--yes"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "was not installed by the installer",
        ));
}

/// Uma instalação como a do instalador: `bin/aurora`, `toolchain/`,
/// `install.json`, o `uninstall.sh` que ele gera e o atalho. O script de
/// verdade é testado em `crates/aurora-installer/tests/install.rs`; aqui, o
/// que importa é que `aurora uninstall` o ache e o rode.
#[cfg(unix)]
#[test]
fn uninstall_removes_the_installation_the_bundle_and_the_old_config() {
    let (_guard, dir) = tempdir();
    let prefix = dir.join("share/aurora");
    std::fs::create_dir_all(prefix.join("bin")).unwrap();
    let exe = assert_cmd::cargo::cargo_bin("aurora");
    std::fs::copy(&exe, prefix.join("bin/aurora")).unwrap();
    fake_bundle(&prefix.join("toolchain"));
    std::fs::write(prefix.join("install.json"), "{}").unwrap();
    let link = dir.join("bin/aurora");
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(prefix.join("bin/aurora"), &link).unwrap();
    std::fs::write(
        prefix.join("uninstall.sh"),
        format!(
            "#!/bin/sh\nset -e\nprefix='{prefix}'\nlink='{link}'\n\
             if [ -L \"$link\" ] && [ \"$(readlink \"$link\")\" = \"$prefix/bin/aurora\" ]; then rm -f \"$link\"; fi\n\
             rm -rf \"$prefix/toolchain\" \"$prefix/bin/aurora\" \"$prefix/install.json\"\n\
             rmdir \"$prefix/bin\" 2>/dev/null || true\n\
             rm -f \"$prefix/uninstall.sh\"\n\
             rmdir \"$prefix\" 2>/dev/null || true\n\
             echo \"Removed AURORA from $prefix\"\n"
        ),
    )
    .unwrap();
    // A configuração da 0.1.0, que nenhuma versão nova lê.
    let config = dir.join("config");
    std::fs::create_dir_all(config.join("aurora")).unwrap();
    std::fs::write(config.join("aurora/config.json"), "{}").unwrap();

    let installed = || {
        let mut cmd = std::process::Command::new(link.as_std_path());
        cmd.env_remove("AURORA_TOOLCHAIN")
            .env_remove("AURORA_COMPILER")
            .env("XDG_CONFIG_HOME", &config)
            .env("NO_COLOR", "1")
            .stdin(std::process::Stdio::null());
        cmd
    };

    // Sem terminal e sem --yes: recusa, e nada sai.
    let out = installed().arg("uninstall").output().unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("--yes"));
    assert!(prefix.join("toolchain/bundle.json").is_file());

    // Pelo atalho, como quem digita `aurora uninstall --yes`.
    let out = installed()
        .args(["--json", "uninstall", "--yes"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    // O stdout é só o JSON; o que o script escreve vai para o stderr.
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(String::from_utf8_lossy(&out.stderr).contains("Removed AURORA"));
    assert_eq!(report["removed"], true);
    assert_eq!(Utf8Path::new(report["prefix"].as_str().unwrap()), prefix);
    assert!(!prefix.exists(), "a instalação ficou");
    assert!(std::fs::symlink_metadata(&link).is_err(), "o atalho ficou");
    assert!(
        !config.join("aurora").exists(),
        "a configuração antiga ficou"
    );
}

// Acrescentar componentes.

#[test]
fn install_checks_the_names_and_the_installation_before_downloading() {
    // Nome errado: a lista do bundle, sem tentar baixar nada.
    aurora()
        .args(["install", "vivado"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Unknown app: vivado"))
        .stderr(predicate::str::contains("verilator"));
    // Este aurora, de target/, não foi instalado pelo instalador.
    aurora()
        .args(["install", "verilator"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "was not installed by the installer",
        ));
}

// Histórico de relatórios.

#[test]
fn report_without_history_explains_itself() {
    let (_guard, root) = new_project("p");
    let error = json(aurora_in(&root).args(["--json", "report"]), 2);
    assert_eq!(error["error"]["code"], "no_reports");
    let list = json(aurora_in(&root).args(["--json", "report", "list"]), 0);
    assert_eq!(list["reports"].as_array().unwrap().len(), 0);
    assert!(stdout(aurora_in(&root).args(["report", "list"]), 0).contains("No reports yet"));
    let error = json(aurora_in(&root).args(["--json", "report", "show", "5"]), 2);
    assert_eq!(error["error"]["code"], "report_not_found");
    aurora_in(&root)
        .args(["report", "show", "5"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Report run-000005 not found"))
        .stderr(predicate::str::contains("aurora report list"));
    let error = json(aurora_in(&root).args(["--json", "report", "compare"]), 2);
    assert_eq!(error["error"]["code"], "no_reports");
    // `list` não aceita limite zero.
    aurora_in(&root)
        .args(["report", "list", "--limit", "0"])
        .assert()
        .code(2);
}

/// Relatórios de mentira: `clean` não lê o conteúdo.
fn fake_reports(root: &Utf8Path, numbers: &[u32]) {
    for n in numbers {
        let dir = root.join(format!(".aurora8/reports/run-{n:06}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("report.txt"), "x").unwrap();
    }
}

fn report_ids(root: &Utf8Path) -> Vec<String> {
    let list = json(aurora_in(root).args(["--json", "report", "list"]), 0);
    list["reports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn report_clean_removes_named_old_or_all_reports() {
    let (_guard, root) = new_project("p");
    assert!(stdout(aurora_in(&root).args(["report", "clean"]), 0).contains("No reports to remove"));
    fake_reports(&root, &[1, 2, 3, 4, 5, 6]);

    // Pelo nome: sem perguntar, com o número ou o identificador.
    let clean = json(
        aurora_in(&root).args(["--json", "report", "clean", "2", "run-000004"]),
        0,
    );
    assert_eq!(
        clean["removed"],
        serde_json::json!(["run-000002", "run-000004"])
    );
    assert_eq!(clean["kept"], 4);
    // Um que não existe: nada sai.
    let error = json(
        aurora_in(&root).args(["--json", "report", "clean", "1", "9"]),
        2,
    );
    assert_eq!(error["error"]["code"], "report_not_found");
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("run-000009")
    );
    assert_eq!(report_ids(&root).len(), 4);

    // Em massa, sem terminal: só com --yes.
    aurora_in(&root)
        .args(["report", "clean", "--keep", "1"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("use aurora report clean --yes"));
    let text = stdout(
        aurora_in(&root).args(["report", "clean", "--keep", "1", "--yes"]),
        0,
    );
    assert!(
        text.contains("Removed 3 reports: run-000001, run-000003, run-000005"),
        "{text}"
    );
    assert_eq!(report_ids(&root), ["run-000006"]);
    aurora_in(&root)
        .args(["report", "clean", "1", "--keep", "1"])
        .assert()
        .code(2);

    let text = stdout(aurora_in(&root).args(["report", "clean", "--yes"]), 0);
    assert!(text.contains("Removed run-000006"), "{text}");
    assert!(text.contains("report numbers are not reused"), "{text}");
    assert!(report_ids(&root).is_empty());
    // O número não volta: o próximo relatório seria o run-000007.
    let sequence = std::fs::read_to_string(root.join(".aurora8/reports/sequence")).unwrap();
    assert_eq!(sequence.trim(), "6");
}

#[test]
fn operations_store_reports_that_list_show_and_compare() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    if !bundle_has(&tc, "yosys") || !bundle_has(&tc, "icarus") {
        return;
    }
    let (_guard, root) = example("soma");
    let run = |args: &[&str], code: i32| {
        let mut cmd = aurora();
        cmd.args(["-C", root.as_str(), "--json"])
            .args(args)
            .env("AURORA_TOOLCHAIN", &tc);
        json(&mut cmd, code)
    };
    assert_eq!(run(&["synth", "-p", "soma"], 0)["report"], "run-000001");
    assert_eq!(run(&["sim", "-p", "soma"], 0)["report"], "run-000002");
    let synth = run(&["synth", "-p", "soma"], 0);
    assert_eq!(synth["report"], "run-000003");
    assert!(synth["synthesis"]["statistics"]["cells"].as_u64().unwrap() > 0);

    let list = run(&["report", "list", "--limit", "2"], 0);
    let ids: Vec<_> = list["reports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["run-000003", "run-000002"]);

    let show = run(&["report", "show", "2"], 0);
    assert_eq!(show["id"], "run-000002");
    assert!(
        show["text"]
            .as_str()
            .unwrap()
            .contains("AURORA OPERATION REPORT")
    );
    assert!(show["record"]["simulation"]["execution_ms"].is_u64());

    // A referência automática pula a simulação e pega a síntese anterior.
    let compare = run(&["report", "compare"], 0);
    assert_eq!(compare["current_id"], "run-000003");
    assert_eq!(compare["baseline_id"], "run-000001");
    assert!(compare["synthesis"].is_object());
    assert!(compare["simulation"].is_null());

    let text = {
        let mut cmd = aurora();
        cmd.args(["-C", root.as_str(), "report", "compare", "--summary"])
            .env("AURORA_TOOLCHAIN", &tc);
        stdout(&mut cmd, 0)
    };
    assert!(text.contains("COMPARISON SUMMARY"), "{text}");
    assert!(text.contains("Baseline:  run-000001"), "{text}");
}

#[test]
fn a_failed_build_says_what_ran_and_what_did_not() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("com_erro");
    let text = stdout(
        aurora_in(&root)
            .args(["sim", "-p", "conta"])
            .env("AURORA_TOOLCHAIN", &tc),
        1,
    );
    for expected in [
        "Build of conta (C±, 100 MHz, 2000 clocks): failed after".to_owned(),
        "cmmcomp exited with code 1".to_owned(),
        "not run   assemble      asmcomp".to_owned(),
        "Not generated:".to_owned(),
        format!("testbench           {}", shown("conta/conta/conta_tb.v")),
        "Simulation not run: the build of conta did not finish".to_owned(),
    ] {
        assert!(text.contains(&expected), "{expected}:\n{text}");
    }
}

// O projeto de qualquer pasta dele.

#[test]
fn commands_find_the_project_from_any_inner_folder() {
    let (_guard, root) = new_project("demo");
    aurora_in(&root)
        .args(["proc", "add", "soma"])
        .assert()
        .success();
    // A pasta de saída nasce no primeiro build.
    std::fs::create_dir_all(root.join("soma/soma")).unwrap();
    for inner in ["soma", "soma/soma"] {
        let status = json(aurora_in(&root.join(inner)).args(["--json", "status"]), 0);
        assert!(
            status["spf"].as_str().unwrap().ends_with("demo.spf"),
            "de {inner}: {status}"
        );
    }

    // Fora de qualquer projeto.
    let (_outside_guard, outside) = tempdir();
    let error = json(aurora_in(&outside).args(["--json", "status"]), 2);
    assert_eq!(error["error"]["code"], "project_not_found");
    aurora_in(&outside)
        .arg("status")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("aurora new <name>"));
}

#[test]
fn status_and_proc_set_know_the_processor_of_the_folder() {
    let (_guard, root) = new_project("demo");
    for name in ["soma", "filtro"] {
        aurora_in(&root)
            .args(["proc", "add", name])
            .assert()
            .success();
    }
    let here = root.join("filtro/filtro");
    std::fs::create_dir_all(&here).unwrap();

    let status = json(aurora_in(&here).args(["--json", "status"]), 0);
    assert_eq!(status["here"], "filtro");
    let status = json(aurora_in(&root).args(["--json", "status"]), 0);
    assert!(status["here"].is_null());
    let text = stdout(aurora_in(&here).arg("status"), 0);
    assert!(text.contains("(this folder)"), "{text}");

    let set = json(
        aurora_in(&here).args(["--json", "proc", "set", "--freq", "50"]),
        0,
    );
    assert_eq!(set["name"], "filtro");
    assert_eq!(set["frequency_mhz"], 50);
    // Na raiz não há processador da pasta: precisa do nome.
    aurora_in(&root)
        .args(["proc", "set", "--freq", "50"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Which processor?"));
    let set = json(
        aurora_in(&root).args(["--json", "proc", "set", "soma", "--clocks", "10"]),
        0,
    );
    assert_eq!(set["name"], "soma");
}

#[test]
fn build_check_and_synth_inside_a_processor_folder_act_on_it() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    if !bundle_has(&tc, "yosys") || !bundle_has(&tc, "icarus") {
        return;
    }
    let (_guard, root) = example("soma");
    let here = root.join("filtro");
    let run = |dir: &Utf8Path, args: &[&str]| {
        let mut cmd = aurora_in(dir);
        cmd.arg("--json").args(args).env("AURORA_TOOLCHAIN", &tc);
        json(&mut cmd, 0)
    };
    let processors = |builds: &Value| -> Vec<String> {
        builds
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["processor"].as_str().unwrap().to_owned())
            .collect()
    };

    assert_eq!(processors(&run(&here, &["build"])["results"]), ["filtro"]);
    // O check verifica o filtro que o build acima compilou, sem compilar.
    let check = run(&here, &["check"]);
    assert!(check.get("builds").is_none(), "{check}");
    assert_eq!(
        check["check"]["targets"],
        serde_json::json!(["filtro", "filtro_tb"])
    );
    let synth = run(&here, &["synth"]);
    assert_eq!(processors(&synth["builds"]), ["filtro"]);
    assert_eq!(synth["synthesis"]["top"], "filtro");

    // Da raiz: todos, e o -p escolhe de qualquer pasta.
    assert_eq!(
        processors(&run(&root, &["build"])["results"]),
        ["soma", "filtro"]
    );
    let check = run(&here, &["check", "-p", "soma"]);
    assert_eq!(
        check["check"]["targets"],
        serde_json::json!(["soma", "soma_tb"])
    );
}

#[test]
fn sim_inside_a_processor_folder_simulates_that_processor() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    let (_guard, root) = example("soma");
    // Da pasta do código do filtro: o filtro, sem -p.
    let sim = json(
        aurora_in(&root.join("filtro"))
            .args(["--json", "sim"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    assert_eq!(sim["builds"].as_array().unwrap().len(), 1);
    assert_eq!(sim["builds"][0]["processor"], "filtro");
    assert_eq!(sim["simulation"]["top"], "filtro_tb");

    // De uma pasta do projeto fora dos processadores: a simulação do
    // projeto.
    let (_guard, contador) = example("contador");
    let sim = json(
        aurora_in(&contador.join("rtl"))
            .args(["--json", "sim"])
            .env("AURORA_TOOLCHAIN", &tc),
        0,
    );
    assert_eq!(sim["simulation"]["top"], "contador_tb");
}

#[test]
fn cocotb_testbench_from_the_command_line() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    if !bundle_has(&tc, "icarus") || !bundle_has(&tc, "cocotb") {
        return;
    }
    let (_guard, root) = new_project("c");
    std::fs::write(
        root.join("somador.v"),
        "module somador(input [3:0] a, input [3:0] b, output [4:0] y);\n    assign y = a + b;\nendmodule\n",
    )
    .unwrap();
    let aurora = |args: &[&str]| {
        let mut cmd = aurora_in(&root);
        cmd.args(args).env("AURORA_TOOLCHAIN", &tc);
        cmd
    };
    aurora(&["add", "somador.v", "test_somador.py"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Testbench created: test_somador.py",
        ));
    let sim = json(&mut aurora(&["--json", "sim"]), 0);
    assert_eq!(sim["simulation"]["top"], "somador");
    assert_eq!(sim["simulation"]["tests"]["passed"], 1, "{sim}");
    assert_eq!(
        sim["simulation"]["tests"]["cases"][0]["name"],
        "test_somador.basic_test"
    );
    let text = stdout(&mut aurora(&["sim"]), 0);
    assert!(text.contains("Tests: 1 of 1 passed"), "{text}");

    // A simulação rápida roda os testes no Icarus, sem a onda.
    std::fs::remove_file(root.join("test_somador.fst")).unwrap();
    let fast = json(&mut aurora(&["--json", "sim", "--fast"]), 0);
    assert_eq!(fast["simulation"]["fast"], true, "{fast}");
    assert_eq!(fast["simulation"]["simulator"], "icarus");
    assert_eq!(fast["simulation"]["waveform"], Value::Null);
    assert_eq!(fast["simulation"]["tests"]["passed"], 1, "{fast}");
    assert!(!root.join("test_somador.fst").exists());

    // E no Verilator, com a onda em VCD.
    if bundle_has(&tc, "verilator") {
        let sim = json(&mut aurora(&["--json", "sim", "--verilator"]), 0);
        assert_eq!(sim["simulation"]["simulator"], "verilator", "{sim}");
        assert_eq!(sim["simulation"]["tests"]["passed"], 1, "{sim}");
        assert_eq!(sim["simulation"]["waveform"]["format"], "vcd");
        assert!(root.join("test_somador.vcd").is_file());
    }
}

#[test]
fn fast_sim_runs_on_verilator_without_waveform() {
    let Some(tc) = env_or_skip("AURORA_TEST_BUNDLE") else {
        return;
    };
    if !bundle_has(&tc, "verilator") {
        return;
    }
    let (_guard, root) = example("contador");
    let aurora = |args: &[&str]| {
        let mut cmd = aurora_in(&root);
        cmd.args(args).env("AURORA_TOOLCHAIN", &tc);
        cmd
    };
    let text = stdout(&mut aurora(&["sim", "--fast"]), 0);
    assert!(
        text.contains("Fast simulation of contador_tb (Verilator): finished in"),
        "{text}"
    );
    assert!(text.contains("| q = 10"), "{text}");
    assert!(!text.contains("Open the waveform"), "{text}");
    for wave in ["contador_tb.vcd", "contador_tb.fst"] {
        assert!(!root.join(wave).exists(), "{wave}");
    }

    let sim = json(&mut aurora(&["--json", "sim", "--fast"]), 0);
    assert_eq!(sim["simulation"]["fast"], true, "{sim}");
    assert_eq!(sim["simulation"]["simulator"], "verilator");
    assert_eq!(sim["simulation"]["waveform"], Value::Null);
    // O relatório diz que foi a rápida.
    let report = json(&mut aurora(&["--json", "report"]), 0);
    assert!(
        report
            .to_string()
            .contains("Fast simulation contador_tb (Verilator)"),
        "{report}"
    );

    // Sem onda não há o que abrir.
    aurora(&["sim", "--fast", "--open"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--open"));
}
