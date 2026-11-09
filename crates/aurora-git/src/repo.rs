//! As operações no repositório do projeto.

use std::collections::{BTreeMap, HashSet};

use aurora_core::{CancelToken, Project};
use camino::{Utf8Path, Utf8PathBuf};

use crate::error::{Error, Result};
use crate::parse;
use crate::run::Options;
use crate::types::*;
use crate::{Git, MAX_FILE_SIZE, MAX_FILES, failure};

/// O maior diff que o [`diff`] devolve, em bytes.
const MAX_DIFF: usize = 1024 * 1024;

/// O maior pedaço de linha de comando com caminhos, em caracteres: o
/// Windows aceita 32 767 na linha inteira.
const MAX_ARGS_LEN: usize = 8000;

/// Onde fica o repositório de uma pasta.
struct Location {
    top: Utf8PathBuf,
    git_dir: Utf8PathBuf,
    prefix: String,
}

fn read<'a>() -> Options<'a> {
    Options {
        read: true,
        ..Options::default()
    }
}

fn remote(cancel: Option<&CancelToken>) -> Options<'_> {
    Options {
        remote: true,
        cancel,
        ..Options::default()
    }
}

/// O repositório que contém `dir`, ou `None`.
fn locate(git: &Git, dir: &Utf8Path) -> Result<Option<Location>> {
    if !dir.is_dir() {
        return Err(Error::Io {
            context: "Opening the folder",
            path: dir.to_owned(),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        });
    }
    let args = [
        "rev-parse",
        "--show-toplevel",
        "--absolute-git-dir",
        "--show-prefix",
    ];
    let output = git.run(dir, &args, &read())?;
    if !output.success() {
        if output.stderr.contains("not a git repository") {
            return Ok(None);
        }
        return Err(failure(&args, &output.stderr));
    }
    let text = output.text();
    let mut lines = text.lines();
    let (Some(top), Some(git_dir)) = (lines.next(), lines.next()) else {
        return Ok(None);
    };
    Ok(Some(Location {
        top: Utf8PathBuf::from(top.trim()),
        git_dir: Utf8PathBuf::from(git_dir.trim().replace('\\', "/")),
        prefix: lines
            .next()
            .unwrap_or("")
            .trim()
            .trim_end_matches('/')
            .to_owned(),
    }))
}

/// Como [`locate`], com [`Error::NotRepository`] fora de repositório.
fn repository(git: &Git, dir: &Utf8Path) -> Result<Location> {
    locate(git, dir)?.ok_or_else(|| Error::NotRepository {
        path: dir.to_owned(),
    })
}

/// A configuração que o status mostra: identidade, remotos e o remoto de
/// cada ramo.
struct Config {
    entries: Vec<(String, String)>,
}

impl Config {
    fn read(git: &Git, top: &Utf8Path) -> Result<Config> {
        let pattern = r"^(user\.(name|email)|remote\..*\.url|branch\..*\.(remote|merge))$";
        let output = git.run(top, &["config", "-z", "--get-regexp", pattern], &read())?;
        // Sem nenhuma chave, o git sai com 1.
        let entries = if output.success() {
            parse::config(&output.stdout)
        } else {
            Vec::new()
        };
        Ok(Config { entries })
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
            .filter(|v| !v.trim().is_empty())
    }

    fn identity(&self) -> Option<Identity> {
        Some(Identity {
            name: self.get("user.name")?.to_owned(),
            email: self.get("user.email")?.to_owned(),
        })
    }

    fn remotes(&self) -> Vec<Remote> {
        let mut remotes: BTreeMap<String, String> = BTreeMap::new();
        for (key, value) in &self.entries {
            if let Some(name) = key
                .strip_prefix("remote.")
                .and_then(|rest| rest.strip_suffix(".url"))
            {
                remotes
                    .entry(name.to_owned())
                    .or_insert_with(|| value.clone());
            }
        }
        remotes
            .into_iter()
            .map(|(name, url)| Remote {
                github: parse::github_repository(&url),
                url: parse::sanitize_url(&url),
                name,
            })
            .collect()
    }

    /// O remoto para onde `branch` vai: o configurado nele, o `origin` ou o
    /// primeiro que houver.
    fn push_remote(&self, branch: Option<&str>) -> Option<String> {
        if let Some(configured) = branch.and_then(|b| self.get(&format!("branch.{b}.remote"))) {
            return Some(configured.to_owned());
        }
        let remotes = self.remotes();
        remotes
            .iter()
            .find(|r| r.name == "origin")
            .or_else(|| remotes.first())
            .map(|r| r.name.clone())
    }

    /// O ramo no remoto que `branch` acompanha (`refs/heads/x` vira `x`).
    fn upstream_branch(&self, branch: &str) -> Option<String> {
        self.get(&format!("branch.{branch}.merge"))
            .map(|m| m.strip_prefix("refs/heads/").unwrap_or(m).to_owned())
    }
}

/// O estado do git e do repositório que contém `dir` (a pasta do projeto).
///
/// # Erros
///
/// [`Error::DubiousOwnership`] num repositório de outro usuário; uma falha
/// do git.
pub fn status(git: &Git, dir: &Utf8Path) -> Result<GitStatus> {
    let Some(location) = locate(git, dir)? else {
        return Ok(GitStatus {
            version: git.version().to_owned(),
            repository: None,
        });
    };
    let output = git.run_ok(
        &location.top,
        &[
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
        ],
        &read(),
    )?;
    let (mut branch, entries, truncated) = parse::status(&output.stdout, MAX_FILES);
    let config = Config::read(git, &location.top)?;
    branch.remote = config.push_remote(branch.name.as_deref());

    let prefix = location.prefix.clone();
    let inside =
        |path: &str| prefix.is_empty() || path == prefix || path.starts_with(&format!("{prefix}/"));
    let mut files: Vec<ChangedFile> = entries
        .into_iter()
        .map(|e| ChangedFile {
            in_project: inside(&e.path),
            path: e.path,
            original: e.original,
            change: e.change,
            staged: e.staged,
            unstaged: e.unstaged,
        })
        .collect();
    // Os de dentro do projeto primeiro; a ordem do git (por caminho) dentro
    // de cada grupo.
    files.sort_by_key(|f| !f.in_project);

    Ok(GitStatus {
        version: git.version().to_owned(),
        repository: Some(RepositoryStatus {
            operation: operation(&location.git_dir),
            identity: config.identity(),
            remotes: config.remotes(),
            top: location.top,
            git_dir: location.git_dir,
            prefix: location.prefix,
            branch,
            files,
            truncated,
        }),
    })
}

fn operation(git_dir: &Utf8Path) -> Option<Operation> {
    if git_dir.join("MERGE_HEAD").is_file() {
        Some(Operation::Merge)
    } else if git_dir.join("rebase-merge").is_dir() || git_dir.join("rebase-apply").is_dir() {
        Some(Operation::Rebase)
    } else if git_dir.join("CHERRY_PICK_HEAD").is_file() {
        Some(Operation::CherryPick)
    } else if git_dir.join("REVERT_HEAD").is_file() {
        Some(Operation::Revert)
    } else {
        None
    }
}

/// Um caminho relativo ao topo, com `/`, que não sai do repositório.
fn check_path(path: &str) -> Result<()> {
    let bad = path.is_empty()
        || path.contains('\0')
        || path.contains('\\')
        || path.starts_with('/')
        || path.as_bytes().get(1) == Some(&b':')
        || path.split('/').any(|part| part == "..");
    if bad {
        return Err(Error::InvalidArgument(format!(
            "'{path}' is not a path relative to the repository"
        )));
    }
    Ok(())
}

/// Um identificador de commit: só hexadecimal, para não virar opção do git.
fn check_oid(oid: &str) -> Result<()> {
    if (4..=64).contains(&oid.len()) && oid.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(Error::InvalidArgument(format!(
            "'{oid}' is not a commit id"
        )))
    }
}

/// O repositório tem pelo menos um commit.
fn has_head(git: &Git, top: &Utf8Path) -> Result<bool> {
    Ok(git
        .run(top, &["rev-parse", "--verify", "-q", "HEAD"], &read())?
        .success())
}

/// O diff unificado do que mudou desde o último commit, num arquivo ou em
/// todos (sem os que o git ainda não acompanha). Corta em 1 MiB.
///
/// # Erros
///
/// [`Error::NotRepository`] fora de repositório; um caminho inválido.
pub fn diff(git: &Git, dir: &Utf8Path, path: Option<&str>) -> Result<String> {
    let location = repository(git, dir)?;
    let mut args = vec!["diff", "--no-ext-diff", "--no-color"];
    if has_head(git, &location.top)? {
        args.push("HEAD");
    } else {
        args.push("--cached");
    }
    if let Some(path) = path {
        check_path(path)?;
        args.extend(["--", path]);
    }
    let output = git.run_ok(&location.top, &args, &read())?;
    let mut text = output.text();
    if text.len() > MAX_DIFF {
        let mut cut = MAX_DIFF;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
        text.push_str("\n[diff truncated]\n");
    }
    Ok(text)
}

/// O conteúdo de `path` (relativo ao topo) na versão `revision`.
///
/// # Erros
///
/// [`Error::NotRepository`] fora de repositório; um caminho ou commit
/// inválido.
pub fn read_file(
    git: &Git,
    dir: &Utf8Path,
    revision: &Revision,
    path: &str,
) -> Result<FileVersion> {
    check_path(path)?;
    let location = repository(git, dir)?;
    let spec = match revision {
        Revision::Index => format!(":{path}"),
        Revision::Head => format!("HEAD:{path}"),
        Revision::Commit(oid) => {
            check_oid(oid)?;
            format!("{oid}:{path}")
        }
    };
    let missing = FileVersion {
        exists: false,
        size: 0,
        binary: false,
        too_large: false,
        text: None,
    };
    let size = git.run(&location.top, &["cat-file", "-s", &spec], &read())?;
    if !size.success() {
        return Ok(missing);
    }
    let size: u64 = size.text().trim().parse().unwrap_or(0);
    if size > MAX_FILE_SIZE {
        return Ok(FileVersion {
            exists: true,
            size,
            too_large: true,
            ..missing
        });
    }
    let blob = git.run_ok(&location.top, &["cat-file", "blob", &spec], &read())?;
    let binary = blob.stdout.iter().take(8000).any(|&b| b == 0);
    Ok(FileVersion {
        exists: true,
        size,
        binary,
        too_large: false,
        text: (!binary).then(|| String::from_utf8_lossy(&blob.stdout).into_owned()),
    })
}

/// A lista de caminhos para o `--pathspec-from-file` com `--pathspec-file-nul`.
fn pathspec_input(paths: &[String]) -> Vec<u8> {
    let mut input = Vec::new();
    for path in paths {
        input.extend_from_slice(path.as_bytes());
        input.push(0);
    }
    input
}

/// Faz o commit dos arquivos de `options.paths` (todos os mudados, sem a
/// lista), no estado em que estão no disco.
///
/// # Erros
///
/// [`Error::EmptyMessage`], [`Error::NothingToCommit`],
/// [`Error::IdentityMissing`], ou a falha de um hook (em [`Error::Failed`],
/// com o que ele escreveu).
pub fn commit(git: &Git, dir: &Utf8Path, options: &CommitOptions) -> Result<CommitResult> {
    let summary = options.summary.trim();
    if summary.is_empty() {
        return Err(Error::EmptyMessage);
    }
    let location = repository(git, dir)?;
    let top = &location.top;
    let config = Config::read(git, top)?;
    if config.identity().is_none()
        && !git
            .run(top, &["var", "GIT_AUTHOR_IDENT"], &read())?
            .success()
    {
        return Err(Error::IdentityMissing);
    }

    let mut message = vec!["-m".to_owned(), summary.to_owned()];
    if let Some(description) = options.description.as_deref().map(str::trim)
        && !description.is_empty()
    {
        message.extend(["-m".to_owned(), description.to_owned()]);
    }
    let amend = options.amend.then_some("--amend");
    // Num merge pela metade, o git não aceita commit de parte dos arquivos:
    // os escolhidos entram no índice, e o commit leva o índice inteiro.
    let merging = operation(&location.git_dir) == Some(Operation::Merge);

    match &options.paths {
        None => {
            git.run_ok(top, &["add", "-A"], &Options::default())?;
            let mut args: Vec<&str> = vec!["commit"];
            args.extend(message.iter().map(String::as_str));
            args.extend(amend);
            run_commit(git, top, &args, None)?;
        }
        Some(paths) => {
            if paths.is_empty() && !options.amend {
                return Err(Error::NothingToCommit);
            }
            for path in paths {
                check_path(path)?;
            }
            if !paths.is_empty() {
                let input = pathspec_input(paths);
                git.run_ok(
                    top,
                    &["add", "-A", "--pathspec-from-file=-", "--pathspec-file-nul"],
                    &Options {
                        stdin: Some(input.clone()),
                        ..Options::default()
                    },
                )?;
                let mut args: Vec<&str> = vec!["commit"];
                args.extend(message.iter().map(String::as_str));
                args.extend(amend);
                if merging {
                    run_commit(git, top, &args, None)?;
                } else {
                    args.extend(["--only", "--pathspec-from-file=-", "--pathspec-file-nul"]);
                    run_commit(git, top, &args, Some(input))?;
                }
            } else {
                let mut args: Vec<&str> = vec!["commit", "--only"];
                args.extend(message.iter().map(String::as_str));
                args.extend(amend);
                run_commit(git, top, &args, None)?;
            }
        }
    }

    let head = git.run_ok(top, &["log", "-1", "--format=%H%x1f%h"], &read())?;
    let text = head.text();
    let (oid, short) = text
        .trim()
        .split_once('\u{1f}')
        .unwrap_or((text.trim(), ""));
    let branch = git
        .run(top, &["symbolic-ref", "--short", "-q", "HEAD"], &read())?
        .text()
        .trim()
        .to_owned();
    Ok(CommitResult {
        oid: oid.to_owned(),
        short: short.to_owned(),
        summary: summary.to_owned(),
        branch: Some(branch).filter(|b| !b.is_empty()),
    })
}

fn run_commit(git: &Git, top: &Utf8Path, args: &[&str], stdin: Option<Vec<u8>>) -> Result<()> {
    let output = git.run(
        top,
        args,
        &Options {
            stdin,
            ..Options::default()
        },
    )?;
    if output.success() {
        return Ok(());
    }
    let said = format!("{}\n{}", output.text(), output.stderr);
    if said.contains("nothing to commit") || said.contains("no changes added to commit") {
        return Err(Error::NothingToCommit);
    }
    Err(failure(args, said.trim()))
}

/// Roda `args` com os caminhos em pedaços que cabem numa linha de comando.
fn in_chunks(git: &Git, top: &Utf8Path, args: &[&str], paths: &[String]) -> Result<()> {
    let mut start = 0;
    while start < paths.len() {
        let mut end = start;
        let mut len = 0;
        while end < paths.len() && (end == start || len + paths[end].len() + 3 < MAX_ARGS_LEN) {
            len += paths[end].len() + 3;
            end += 1;
        }
        let mut all: Vec<&str> = args.to_vec();
        all.push("--");
        all.extend(paths[start..end].iter().map(String::as_str));
        git.run_ok(top, &all, &Options::default())?;
        start = end;
    }
    Ok(())
}

/// Descarta as mudanças de `paths` (relativos ao topo): cada um volta ao
/// último commit, e o que é novo sai. A versão descartada de cada arquivo
/// vai antes para a lixeira do sistema, de onde volta.
///
/// # Erros
///
/// Um caminho inválido; a lixeira recusou um arquivo (aí nada daquele
/// arquivo muda).
pub fn discard(git: &Git, dir: &Utf8Path, paths: &[String]) -> Result<DiscardResult> {
    for path in paths {
        check_path(path)?;
    }
    let location = repository(git, dir)?;
    let top = &location.top;
    let output = git.run_ok(
        top,
        &["status", "--porcelain=v2", "-z", "--untracked-files=all"],
        &read(),
    )?;
    let (_, entries, _) = parse::status(&output.stdout, usize::MAX);
    let head = has_head(git, top)?;

    let mut restore = Vec::new();
    let mut unindex = Vec::new();
    let mut discarded = Vec::new();
    let mut trashed = Vec::new();
    let wanted: HashSet<&str> = paths.iter().map(String::as_str).collect();
    for entry in entries.iter().filter(|e| wanted.contains(e.path.as_str())) {
        let file = top.join(&entry.path);
        if file.is_file() || file.is_symlink() {
            trash::delete(file.as_std_path()).map_err(|e| Error::Io {
                context: "Moving to the trash",
                path: file.clone(),
                source: std::io::Error::other(e.to_string()),
            })?;
            trashed.push(file);
        }
        match entry.change {
            Change::Untracked => {}
            Change::Added => unindex.push(entry.path.clone()),
            Change::Renamed | Change::Copied => {
                unindex.push(entry.path.clone());
                if entry.change == Change::Renamed
                    && let Some(original) = &entry.original
                {
                    restore.push(original.clone());
                }
            }
            _ if head => restore.push(entry.path.clone()),
            _ => unindex.push(entry.path.clone()),
        }
        discarded.push(entry.path.clone());
    }
    if !unindex.is_empty() {
        in_chunks(
            git,
            top,
            &["rm", "--cached", "-q", "-r", "--ignore-unmatch"],
            &unindex,
        )?;
    }
    if !restore.is_empty() {
        in_chunks(
            git,
            top,
            &["restore", "--source=HEAD", "--staged", "--worktree"],
            &restore,
        )?;
    }
    Ok(DiscardResult { discarded, trashed })
}

/// O comentário que abre as linhas da AURORA no `.gitignore`.
const GITIGNORE_HEADER: &str = "# Gerado pela AURORA: simulações, relatórios e o Verilog dos processadores, refeitos a cada compilação";

/// Cria um repositório na pasta do projeto, com um `.gitignore` que deixa
/// de fora o que a AURORA gera: `.aurora8/` e a pasta de saída de cada
/// processador. Um `.gitignore` que já existe ganha só as linhas que faltam.
///
/// # Erros
///
/// [`Error::InvalidArgument`] se a pasta já é o topo de um repositório.
pub fn init(git: &Git, project: &Project) -> Result<InitResult> {
    let root = project.root();
    if let Some(location) = locate(git, root)?
        && same_dir(&location.top, root)
    {
        return Err(Error::InvalidArgument(format!(
            "{root} is already a Git repository"
        )));
    }
    let configured = git
        .run(root, &["config", "--get", "init.defaultBranch"], &read())?
        .text()
        .trim()
        .to_owned();
    let branch = if configured.is_empty() {
        "main".to_owned()
    } else {
        configured.clone()
    };
    if configured.is_empty() && git.at_least([2, 28, 0]) {
        git.run_ok(root, &["init", "-q", "-b", &branch], &Options::default())?;
    } else {
        git.run_ok(root, &["init", "-q"], &Options::default())?;
        if configured.is_empty() {
            git.run_ok(
                root,
                &["symbolic-ref", "HEAD", &format!("refs/heads/{branch}")],
                &Options::default(),
            )?;
        }
    }

    let mut lines = vec![".aurora8/".to_owned()];
    for processor in project.processors() {
        if let Ok(relative) = processor.output_dir.strip_prefix(root) {
            let relative = relative.as_str().replace('\\', "/");
            if !relative.is_empty() {
                lines.push(format!("/{relative}/"));
            }
        }
    }
    let path = root.join(".gitignore");
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(Error::io("Reading", path)(e)),
    };
    let present: HashSet<&str> = existing.lines().map(str::trim).collect();
    let missing: Vec<String> = lines
        .into_iter()
        .filter(|l| !present.contains(l.as_str()))
        .collect();
    if !missing.is_empty() {
        let mut text = existing.clone();
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(GITIGNORE_HEADER);
        text.push('\n');
        for line in &missing {
            text.push_str(line);
            text.push('\n');
        }
        std::fs::write(&path, text).map_err(Error::io("Writing", &path))?;
    }
    Ok(InitResult {
        top: root.to_owned(),
        branch,
        ignored: missing,
    })
}

/// As duas pastas são a mesma, comparando como o git as escreve (`/`) e sem
/// diferença de maiúsculas no Windows.
fn same_dir(a: &Utf8Path, b: &Utf8Path) -> bool {
    let norm = |p: &Utf8Path| {
        let text = dunce::simplified(p.as_std_path())
            .to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_owned();
        if cfg!(windows) {
            text.to_lowercase()
        } else {
            text
        }
    };
    norm(a) == norm(b)
}

/// Uma página do histórico do ramo atual: `limit` commits depois dos
/// `skip` mais novos.
///
/// # Erros
///
/// [`Error::NotRepository`] fora de repositório.
pub fn log(git: &Git, dir: &Utf8Path, skip: usize, limit: usize) -> Result<GitLog> {
    let location = repository(git, dir)?;
    let top = &location.top;
    if !has_head(git, top)? {
        return Ok(GitLog {
            commits: Vec::new(),
            more: false,
        });
    }
    let limit = limit.clamp(1, 500);
    let format = format!("--format={}", parse::LOG_FORMAT);
    let skip_arg = format!("--skip={skip}");
    let count = format!("--max-count={}", limit + 1);
    let output = git.run_ok(
        top,
        &["log", "-z", &format, &skip_arg, &count, "HEAD", "--"],
        &read(),
    )?;
    let mut commits = parse::log(&output.stdout);
    let more = commits.len() > limit;
    commits.truncate(limit);

    let reach = format!("--max-count={}", skip + limit + 1);
    let local: HashSet<String> = git
        .run_ok(
            top,
            &["rev-list", &reach, "HEAD", "--not", "--remotes"],
            &read(),
        )?
        .text()
        .lines()
        .map(str::to_owned)
        .collect();
    for commit in &mut commits {
        commit.local = local.contains(&commit.oid);
    }
    Ok(GitLog { commits, more })
}

/// Os arquivos que o commit `oid` mudou, em relação ao primeiro pai.
///
/// # Erros
///
/// [`Error::NotRepository`]; um `oid` que não é commit.
pub fn commit_files(git: &Git, dir: &Utf8Path, oid: &str) -> Result<CommitFiles> {
    check_oid(oid)?;
    let location = repository(git, dir)?;
    let top = &location.top;
    let parents = git.run_ok(top, &["rev-list", "--parents", "-n", "1", oid], &read())?;
    let text = parents.text();
    let mut ids = text.split_whitespace();
    let oid = ids.next().unwrap_or(oid).to_owned();
    let parent = ids.next().map(str::to_owned);
    let output = match &parent {
        Some(parent) => git.run_ok(
            top,
            &[
                "diff",
                "-z",
                "-M",
                "--name-status",
                "--no-ext-diff",
                parent,
                &oid,
            ],
            &read(),
        )?,
        None => git.run_ok(
            top,
            &[
                "diff-tree",
                "-r",
                "-z",
                "-M",
                "--name-status",
                "--root",
                "--no-commit-id",
                &oid,
            ],
            &read(),
        )?,
    };
    Ok(CommitFiles {
        files: parse::name_status(&output.stdout),
        oid,
        parent,
    })
}

/// Os ramos locais e os remotos sem ramo local correspondente.
///
/// # Erros
///
/// [`Error::NotRepository`] fora de repositório.
pub fn branches(git: &Git, dir: &Utf8Path) -> Result<Branches> {
    let location = repository(git, dir)?;
    let top = &location.top;
    let format = format!("--format={}", parse::BRANCH_FORMAT);
    let output = git.run_ok(
        top,
        &[
            "for-each-ref",
            &format,
            "--sort=-committerdate",
            "refs/heads",
            "refs/remotes",
        ],
        &read(),
    )?;
    let (local, mut remote) = parse::branches(&output.text());
    let current = current_branch(git, top)?;
    let followed: HashSet<&str> = local.iter().filter_map(|b| b.upstream.as_deref()).collect();
    let names: HashSet<&str> = local.iter().map(|b| b.name.as_str()).collect();
    remote.retain(|r| !followed.contains(r.name.as_str()) && !names.contains(r.branch.as_str()));
    Ok(Branches {
        current,
        local,
        remote,
    })
}

fn current_branch(git: &Git, top: &Utf8Path) -> Result<Option<String>> {
    let output = git.run(top, &["symbolic-ref", "--short", "-q", "HEAD"], &read())?;
    Ok(Some(output.text().trim().to_owned()).filter(|b| output.success() && !b.is_empty()))
}

fn ref_exists(git: &Git, top: &Utf8Path, reference: &str) -> Result<bool> {
    Ok(git
        .run(
            top,
            &["show-ref", "--verify", "--quiet", reference],
            &read(),
        )?
        .success())
}

fn check_branch_name(git: &Git, top: &Utf8Path, name: &str) -> Result<()> {
    let invalid = || Error::InvalidBranchName {
        name: name.to_owned(),
    };
    if name.is_empty() || name.starts_with('-') {
        return Err(invalid());
    }
    if !git
        .run(top, &["check-ref-format", "--branch", name], &read())?
        .success()
    {
        return Err(invalid());
    }
    Ok(())
}

/// Troca de ramo, ou cria um a partir do commit atual. As mudanças sem
/// commit vão junto quando o git consegue levá-las; quando não consegue,
/// nada muda e o erro lista os arquivos.
///
/// Devolve o ramo atual depois da troca.
///
/// # Erros
///
/// [`Error::InvalidBranchName`], [`Error::BranchExists`],
/// [`Error::BranchNotFound`], [`Error::SwitchBlocked`].
pub fn switch(git: &Git, dir: &Utf8Path, options: &SwitchOptions) -> Result<String> {
    let location = repository(git, dir)?;
    let top = &location.top;
    let name = options.branch.trim();
    check_branch_name(git, top, name)?;
    let (args, target): (Vec<String>, String) = if options.create {
        if ref_exists(git, top, &format!("refs/heads/{name}"))? {
            return Err(Error::BranchExists {
                name: name.to_owned(),
            });
        }
        (
            vec!["switch".into(), "-c".into(), name.into()],
            name.to_owned(),
        )
    } else if ref_exists(git, top, &format!("refs/heads/{name}"))? {
        (vec!["switch".into(), name.into()], name.to_owned())
    } else if ref_exists(git, top, &format!("refs/remotes/{name}"))? {
        // Um ramo remoto (`origin/x`) vira o local `x`, que o acompanha; se
        // o `x` local já existe, é para ele que se vai.
        let local = name.split_once('/').map_or(name, |(_, b)| b).to_owned();
        if ref_exists(git, top, &format!("refs/heads/{local}"))? {
            (vec!["switch".into(), local.clone()], local)
        } else {
            (vec!["switch".into(), "--track".into(), name.into()], local)
        }
    } else {
        return Err(Error::BranchNotFound {
            name: name.to_owned(),
        });
    };
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = git.run(top, &args, &Options::default())?;
    if !output.success() {
        let files = parse::overwritten_files(&output.stderr);
        if !files.is_empty() {
            return Err(Error::SwitchBlocked {
                branch: target,
                files,
            });
        }
        return Err(failure(&args, &output.stderr));
    }
    Ok(target)
}

fn remote_failure(args: &[&str], remote: &str, stderr: &str) -> Error {
    if parse::is_authentication_failure(stderr) {
        Error::Authentication {
            remote: remote.to_owned(),
            stderr: stderr.trim().to_owned(),
        }
    } else {
        failure(args, stderr)
    }
}

/// Traz do remoto do ramo atual (ou do `origin`) os ramos e commits novos,
/// sem mexer nos arquivos.
///
/// # Erros
///
/// [`Error::NoRemote`], [`Error::Authentication`], [`Error::Cancelled`],
/// [`Error::TimedOut`].
pub fn fetch(git: &Git, dir: &Utf8Path, cancel: Option<&CancelToken>) -> Result<SyncResult> {
    let location = repository(git, dir)?;
    let top = &location.top;
    let config = Config::read(git, top)?;
    let branch = current_branch(git, top)?;
    let remote_name = config
        .push_remote(branch.as_deref())
        .ok_or(Error::NoRemote)?;
    let args = ["fetch", "--prune", "--progress", remote_name.as_str()];
    let output = git.run(top, &args, &remote(cancel))?;
    if !output.success() {
        return Err(remote_failure(&args, &remote_name, &output.stderr));
    }
    let upstream = branch
        .as_deref()
        .and_then(|b| config.upstream_branch(b))
        .map(|u| format!("{remote_name}/{u}"));
    Ok(SyncResult {
        remote: remote_name,
        branch,
        upstream,
        published: false,
    })
}

/// Traz os commits do ramo remoto para o ramo atual, só quando é um avanço
/// direto (sem merge).
///
/// # Erros
///
/// [`Error::DetachedHead`], [`Error::NoUpstream`], [`Error::Diverged`],
/// [`Error::SwitchBlocked`] (mudanças locais no caminho),
/// [`Error::Authentication`].
pub fn pull(git: &Git, dir: &Utf8Path, cancel: Option<&CancelToken>) -> Result<SyncResult> {
    let location = repository(git, dir)?;
    let top = &location.top;
    let config = Config::read(git, top)?;
    let branch = current_branch(git, top)?.ok_or(Error::DetachedHead)?;
    let upstream = config
        .upstream_branch(&branch)
        .ok_or_else(|| Error::NoUpstream {
            branch: branch.clone(),
        })?;
    let remote_name = config.push_remote(Some(&branch)).ok_or(Error::NoRemote)?;
    let args = ["pull", "--ff-only", "--progress"];
    let output = git.run(top, &args, &remote(cancel))?;
    if !output.success() {
        let stderr = &output.stderr;
        if stderr.contains("Not possible to fast-forward")
            || stderr.contains("diverging branches")
            || stderr.contains("have diverged")
        {
            return Err(Error::Diverged { branch });
        }
        let files = parse::overwritten_files(stderr);
        if !files.is_empty() {
            return Err(Error::SwitchBlocked { branch, files });
        }
        return Err(remote_failure(&args, &remote_name, stderr));
    }
    Ok(SyncResult {
        upstream: Some(format!("{remote_name}/{upstream}")),
        remote: remote_name,
        branch: Some(branch),
        published: false,
    })
}

/// Envia o ramo atual. Sem ramo remoto ainda, publica: cria o ramo com o
/// mesmo nome no remoto e passa a acompanhá-lo.
///
/// # Erros
///
/// [`Error::DetachedHead`], [`Error::NoRemote`], [`Error::PushRejected`],
/// [`Error::Authentication`].
pub fn push(git: &Git, dir: &Utf8Path, cancel: Option<&CancelToken>) -> Result<SyncResult> {
    let location = repository(git, dir)?;
    let top = &location.top;
    let config = Config::read(git, top)?;
    let branch = current_branch(git, top)?.ok_or(Error::DetachedHead)?;
    let remote_name = config.push_remote(Some(&branch)).ok_or(Error::NoRemote)?;
    let upstream = config.upstream_branch(&branch);
    let refspec = match &upstream {
        Some(target) => format!("HEAD:refs/heads/{target}"),
        None => branch.clone(),
    };
    let mut args = vec!["push", "--progress"];
    if upstream.is_none() {
        args.push("--set-upstream");
    }
    args.extend([remote_name.as_str(), refspec.as_str()]);
    let output = git.run(top, &args, &remote(cancel))?;
    if !output.success() {
        let stderr = &output.stderr;
        if stderr.contains("[rejected]") || stderr.contains("non-fast-forward") {
            return Err(Error::PushRejected {
                remote: remote_name,
                branch,
            });
        }
        return Err(remote_failure(&args, &remote_name, stderr));
    }
    let target = upstream.clone().unwrap_or_else(|| branch.clone());
    Ok(SyncResult {
        upstream: Some(format!("{remote_name}/{target}")),
        remote: remote_name,
        branch: Some(branch),
        published: upstream.is_none(),
    })
}

/// Grava o nome e o e-mail de quem faz o commit: na configuração global do
/// git (`global`) ou só neste repositório.
///
/// # Erros
///
/// [`Error::InvalidArgument`] com nome ou e-mail vazio;
/// [`Error::NotRepository`] fora de repositório sem `global`.
pub fn set_identity(git: &Git, dir: &Utf8Path, identity: &Identity, global: bool) -> Result<()> {
    let name = identity.name.trim();
    let email = identity.email.trim();
    if name.is_empty() || email.is_empty() || !email.contains('@') {
        return Err(Error::InvalidArgument(
            "The name and the e-mail must not be empty".to_owned(),
        ));
    }
    let cwd = if global {
        dir.to_owned()
    } else {
        repository(git, dir)?.top
    };
    let scope = if global { "--global" } else { "--local" };
    git.run_ok(
        &cwd,
        &["config", scope, "user.name", name],
        &Options::default(),
    )?;
    git.run_ok(
        &cwd,
        &["config", scope, "user.email", email],
        &Options::default(),
    )?;
    Ok(())
}

/// Acrescenta um remoto.
///
/// # Erros
///
/// [`Error::InvalidArgument`] com nome ou endereço inválido, ou nome que já
/// existe.
pub fn add_remote(git: &Git, dir: &Utf8Path, name: &str, url: &str) -> Result<()> {
    let location = repository(git, dir)?;
    let name = name.trim();
    let url = url.trim();
    if name.is_empty()
        || name.starts_with('-')
        || name
            .chars()
            .any(|c| c.is_whitespace() || c == '/' || c == ':')
    {
        return Err(Error::InvalidArgument(format!(
            "'{name}' is not a valid remote name"
        )));
    }
    if url.is_empty() || url.starts_with('-') || url.chars().any(char::is_whitespace) {
        return Err(Error::InvalidArgument(format!(
            "'{url}' is not a valid address"
        )));
    }
    let config = Config::read(git, &location.top)?;
    if config.remotes().iter().any(|r| r.name == name) {
        return Err(Error::InvalidArgument(format!(
            "The remote '{name}' already exists"
        )));
    }
    git.run_ok(
        &location.top,
        &["remote", "add", name, url],
        &Options::default(),
    )?;
    Ok(())
}
