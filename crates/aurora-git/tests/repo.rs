//! As operações com o git de verdade, em repositórios temporários. Sem git
//! no `PATH`, os testes avisam e passam.

use std::process::Command;

use aurora_core::Project;
use aurora_git::{
    Change, CommitOptions, Error, Git, Identity, Revision, SwitchOptions, branches, commit,
    commit_files, discard, init, log, read_file, status, switch,
};
use camino::{Utf8Path, Utf8PathBuf};

fn git() -> Option<Git> {
    let path = std::env::var_os("PATH")?;
    let dirs: Vec<Utf8PathBuf> = std::env::split_paths(&path)
        .filter_map(|p| Utf8PathBuf::from_path_buf(p).ok())
        .collect();
    match Git::locate(&dirs) {
        Ok(git) => Some(git),
        Err(error) => {
            eprintln!("no git, skipping: {error}");
            None
        }
    }
}

fn temp() -> (tempfile::TempDir, Utf8PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::from_path_buf(dunce::canonicalize(dir.path()).unwrap()).unwrap();
    (dir, path)
}

/// Roda o git direto, para preparar o cenário.
fn sh(dir: &Utf8Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Um repositório com identidade local e um primeiro commit.
fn repository(dir: &Utf8Path) {
    sh(dir, &["init", "-q", "-b", "main"]);
    sh(dir, &["config", "user.name", "Ana"]);
    sh(dir, &["config", "user.email", "ana@example.com"]);
    sh(dir, &["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.join("a.v"), "module a;\nendmodule\n").unwrap();
    sh(dir, &["add", "-A"]);
    sh(dir, &["commit", "-q", "-m", "Primeiro"]);
}

#[test]
fn outside_a_repository_there_is_none() {
    let Some(git) = git() else { return };
    let (_keep, dir) = temp();
    let state = status(&git, &dir).unwrap();
    assert!(state.repository.is_none());
    assert!(!state.version.is_empty());
}

#[test]
fn the_status_lists_changes_with_accents_and_marks_the_project() {
    let Some(git) = git() else { return };
    let (_keep, top) = temp();
    repository(&top);
    let project = top.join("pasta do projeto");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("ação.v"), "x").unwrap();
    std::fs::write(top.join("a.v"), "mudou").unwrap();
    std::fs::write(top.join("fora.txt"), "y").unwrap();

    let state = status(&git, &project).unwrap();
    let repo = state.repository.unwrap();
    assert_eq!(repo.prefix, "pasta do projeto");
    assert_eq!(repo.branch.name.as_deref(), Some("main"));
    assert!(repo.branch.commit.is_some());
    assert_eq!(repo.identity.unwrap().name, "Ana");
    let files: Vec<(&str, Change, bool)> = repo
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.change, f.in_project))
        .collect();
    assert_eq!(
        files,
        vec![
            ("pasta do projeto/ação.v", Change::Untracked, true),
            ("a.v", Change::Modified, false),
            ("fora.txt", Change::Untracked, false),
        ]
    );
}

#[test]
fn a_commit_takes_only_the_chosen_files_and_keeps_what_was_staged() {
    let Some(git) = git() else { return };
    let (_keep, top) = temp();
    repository(&top);
    std::fs::write(top.join("a.v"), "mudou").unwrap();
    std::fs::write(top.join("novo [1].v"), "novo").unwrap();
    std::fs::write(top.join("preparado.v"), "p").unwrap();
    sh(&top, &["add", "preparado.v"]);

    let mut options = CommitOptions::new("Dois arquivos");
    options.description = Some("Com descrição".into());
    options.paths = Some(vec!["a.v".into(), "novo [1].v".into()]);
    let result = commit(&git, &top, &options).unwrap();
    assert_eq!(result.summary, "Dois arquivos");
    assert_eq!(result.branch.as_deref(), Some("main"));

    let shown = sh(&top, &["show", "--name-only", "--format=%B", "HEAD"]);
    assert!(shown.contains("Dois arquivos\n\nCom descrição"), "{shown}");
    assert!(
        shown.contains("a.v") && shown.contains("novo [1].v"),
        "{shown}"
    );
    assert!(!shown.contains("preparado.v"), "{shown}");
    // O que estava preparado e não foi escolhido continua preparado.
    let staged = sh(&top, &["diff", "--cached", "--name-only"]);
    assert_eq!(staged.trim(), "preparado.v");
}

#[test]
fn a_commit_refuses_an_empty_message_and_nothing_to_commit() {
    let Some(git) = git() else { return };
    let (_keep, top) = temp();
    repository(&top);
    assert!(matches!(
        commit(&git, &top, &CommitOptions::new("  ")),
        Err(Error::EmptyMessage)
    ));
    let mut options = CommitOptions::new("Nada");
    options.paths = Some(Vec::new());
    assert!(matches!(
        commit(&git, &top, &options),
        Err(Error::NothingToCommit)
    ));
    assert!(matches!(
        commit(&git, &top, &CommitOptions::new("Nada")),
        Err(Error::NothingToCommit)
    ));
}

#[test]
fn a_commit_without_identity_says_so() {
    let Some(git) = git() else { return };
    let (_keep, top) = temp();
    repository(&top);
    sh(&top, &["config", "--unset", "user.name"]);
    sh(&top, &["config", "--unset", "user.email"]);
    sh(&top, &["config", "user.useConfigOnly", "true"]);
    std::fs::write(top.join("a.v"), "mudou").unwrap();
    let result = commit(&git, &top, &CommitOptions::new("Sem nome"));
    // Com nome e e-mail na configuração global da máquina, o commit passa.
    let global = sh(&top, &["config", "--global", "--get-regexp", "^user\\."]);
    if global.contains("user.name") && global.contains("user.email") {
        return;
    }
    assert!(matches!(result, Err(Error::IdentityMissing)), "{result:?}");
}

#[test]
fn discard_sends_the_version_to_the_trash_and_restores_the_commit() {
    let Some(git) = git() else { return };
    let (_keep, top) = temp();
    repository(&top);
    std::fs::write(top.join("a.v"), "estragado").unwrap();
    std::fs::write(top.join("novo.v"), "novo").unwrap();
    let result = discard(&git, &top, &["a.v".into(), "novo.v".into()]);
    let result = match result {
        Ok(result) => result,
        // Sem lixeira (um servidor sem área de trabalho), o descarte recusa
        // em vez de apagar.
        Err(Error::Io { .. }) => return,
        Err(other) => panic!("{other}"),
    };
    assert_eq!(result.discarded, vec!["a.v", "novo.v"]);
    assert_eq!(result.trashed.len(), 2);
    // Com `core.autocrlf` (o padrão do Git for Windows), volta com CRLF.
    assert_eq!(
        std::fs::read_to_string(top.join("a.v"))
            .unwrap()
            .replace("\r\n", "\n"),
        "module a;\nendmodule\n"
    );
    assert!(!top.join("novo.v").exists());
    let repo = status(&git, &top).unwrap().repository.unwrap();
    assert!(repo.files.is_empty(), "{:?}", repo.files);
}

#[test]
fn reads_a_file_at_head_at_the_index_and_at_a_commit() {
    let Some(git) = git() else { return };
    let (_keep, top) = temp();
    repository(&top);
    let first = sh(&top, &["rev-parse", "HEAD"]).trim().to_owned();
    std::fs::write(top.join("a.v"), "segundo").unwrap();
    sh(&top, &["commit", "-q", "-am", "Segundo"]);
    std::fs::write(top.join("a.v"), "no índice").unwrap();
    sh(&top, &["add", "a.v"]);

    let at = |revision: Revision| read_file(&git, &top, &revision, "a.v").unwrap();
    assert_eq!(at(Revision::Head).text.as_deref(), Some("segundo"));
    assert_eq!(at(Revision::Index).text.as_deref(), Some("no índice"));
    assert_eq!(
        at(Revision::Commit(first)).text.as_deref(),
        Some("module a;\nendmodule\n")
    );
    let missing = read_file(&git, &top, &Revision::Head, "nao_existe.v").unwrap();
    assert!(!missing.exists);
    assert!(read_file(&git, &top, &Revision::Head, "../fora").is_err());
    assert!(read_file(&git, &top, &Revision::Commit("--output=x".into()), "a.v").is_err());
}

#[test]
fn the_log_pages_and_marks_local_commits_and_lists_files() {
    let Some(git) = git() else { return };
    let (_keep, top) = temp();
    repository(&top);
    for n in 0..3 {
        std::fs::write(top.join(format!("f{n}.v")), "x").unwrap();
        sh(&top, &["add", "-A"]);
        sh(&top, &["commit", "-q", "-m", &format!("Commit {n}")]);
    }
    let page = log(&git, &top, 0, 2).unwrap();
    assert_eq!(page.commits.len(), 2);
    assert!(page.more);
    assert_eq!(page.commits[0].summary, "Commit 2");
    assert!(page.commits.iter().all(|c| c.local));
    let rest = log(&git, &top, 2, 10).unwrap();
    assert_eq!(rest.commits.len(), 2);
    assert!(!rest.more);
    assert_eq!(rest.commits[1].summary, "Primeiro");

    let files = commit_files(&git, &top, &page.commits[0].oid).unwrap();
    assert_eq!(files.files.len(), 1);
    assert_eq!(files.files[0].path, "f2.v");
    assert_eq!(files.files[0].change, Change::Added);
    let root = commit_files(&git, &top, &rest.commits[1].oid).unwrap();
    assert_eq!(root.parent, None);
    assert_eq!(root.files[0].path, "a.v");
}

#[test]
fn an_empty_repository_has_an_empty_log() {
    let Some(git) = git() else { return };
    let (_keep, top) = temp();
    sh(&top, &["init", "-q"]);
    let page = log(&git, &top, 0, 10).unwrap();
    assert!(page.commits.is_empty());
    let repo = status(&git, &top).unwrap().repository.unwrap();
    assert_eq!(repo.branch.commit, None);
}

#[test]
fn switching_branches_creates_carries_and_refuses() {
    let Some(git) = git() else { return };
    let (_keep, top) = temp();
    repository(&top);
    let create = SwitchOptions::create("feat/nova");
    assert_eq!(switch(&git, &top, &create).unwrap(), "feat/nova");
    assert!(matches!(
        switch(&git, &top, &create),
        Err(Error::BranchExists { .. })
    ));
    std::fs::write(top.join("a.v"), "na nova").unwrap();
    sh(&top, &["commit", "-q", "-am", "Na nova"]);

    // Uma mudança que o outro ramo sobrescreveria: nada muda.
    std::fs::write(top.join("a.v"), "pendente").unwrap();
    let back = SwitchOptions::to("main");
    match switch(&git, &top, &back) {
        Err(Error::SwitchBlocked { files, .. }) => assert_eq!(files, vec!["a.v"]),
        other => panic!("{other:?}"),
    }
    sh(&top, &["checkout", "--", "a.v"]);
    assert_eq!(switch(&git, &top, &back).unwrap(), "main");

    let all = branches(&git, &top).unwrap();
    assert_eq!(all.current.as_deref(), Some("main"));
    assert_eq!(all.local.len(), 2);
    for bad in ["-x", "a b", "..", ""] {
        let options = SwitchOptions::create(bad);
        assert!(
            matches!(
                switch(&git, &top, &options),
                Err(Error::InvalidBranchName { .. })
            ),
            "{bad}"
        );
    }
    let missing = SwitchOptions::to("nao-existe");
    assert!(matches!(
        switch(&git, &top, &missing),
        Err(Error::BranchNotFound { .. })
    ));
}

#[test]
fn publish_pull_and_push_with_a_local_remote() {
    let Some(git) = git() else { return };
    let (_keep, base) = temp();
    let remote = base.join("remoto.git");
    std::fs::create_dir_all(&remote).unwrap();
    sh(&remote, &["init", "-q", "--bare", "-b", "main"]);
    let mine = base.join("meu");
    std::fs::create_dir_all(&mine).unwrap();
    repository(&mine);
    sh(&mine, &["remote", "add", "origin", remote.as_str()]);

    // Sem upstream: pull recusa, push publica.
    assert!(matches!(
        aurora_git::pull(&git, &mine, None),
        Err(Error::NoUpstream { .. })
    ));
    let pushed = aurora_git::push(&git, &mine, None).unwrap();
    assert!(pushed.published);
    assert_eq!(pushed.upstream.as_deref(), Some("origin/main"));
    let repo = status(&git, &mine).unwrap().repository.unwrap();
    assert_eq!(repo.branch.upstream.as_deref(), Some("origin/main"));
    assert_eq!(repo.remotes[0].name, "origin");
    assert!(
        log(&git, &mine, 0, 5)
            .unwrap()
            .commits
            .iter()
            .all(|c| !c.local)
    );

    // Outro clone manda um commit; o nosso traz.
    let other = base.join("outro");
    sh(&base, &["clone", "-q", remote.as_str(), other.as_str()]);
    sh(&other, &["config", "user.name", "Bia"]);
    sh(&other, &["config", "user.email", "bia@example.com"]);
    std::fs::write(other.join("b.v"), "b").unwrap();
    sh(&other, &["add", "-A"]);
    sh(&other, &["commit", "-q", "-m", "Da Bia"]);
    sh(&other, &["push", "-q"]);

    aurora_git::fetch(&git, &mine, None).unwrap();
    let repo = status(&git, &mine).unwrap().repository.unwrap();
    assert_eq!(repo.branch.behind, 1);
    aurora_git::pull(&git, &mine, None).unwrap();
    assert!(mine.join("b.v").is_file());

    // Commits dos dois lados: o push é recusado, o pull diz que divergiu.
    std::fs::write(other.join("c.v"), "c").unwrap();
    sh(&other, &["add", "-A"]);
    sh(&other, &["commit", "-q", "-m", "Mais da Bia"]);
    sh(&other, &["push", "-q"]);
    std::fs::write(mine.join("d.v"), "d").unwrap();
    commit(&git, &mine, &CommitOptions::new("Meu")).unwrap();
    assert!(matches!(
        aurora_git::push(&git, &mine, None),
        Err(Error::PushRejected { .. })
    ));
    aurora_git::fetch(&git, &mine, None).unwrap();
    assert!(matches!(
        aurora_git::pull(&git, &mine, None),
        Err(Error::Diverged { .. })
    ));
}

#[test]
fn init_creates_the_repository_and_ignores_what_aurora_generates() {
    let Some(git) = git() else { return };
    let (_keep, dir) = temp();
    let project = Project::create(&dir, "novo").unwrap();
    std::fs::write(project.root().join(".gitignore"), "segredo.txt").unwrap();
    let result = init(&git, &project).unwrap();
    assert_eq!(result.ignored, vec![".aurora8/"]);
    let text = std::fs::read_to_string(project.root().join(".gitignore")).unwrap();
    assert!(
        text.starts_with("segredo.txt\n\n# Gerado pela AURORA"),
        "{text}"
    );
    assert!(text.ends_with(".aurora8/\n"), "{text}");
    let repo = status(&git, project.root()).unwrap().repository.unwrap();
    assert_eq!(repo.branch.name.as_deref(), Some(result.branch.as_str()));
    assert!(matches!(
        init(&git, &project),
        Err(Error::InvalidArgument(_))
    ));
}

#[test]
fn identity_and_remote_are_validated() {
    let Some(git) = git() else { return };
    let (_keep, top) = temp();
    repository(&top);
    let empty = Identity::new(" ", "x@y");
    assert!(aurora_git::set_identity(&git, &top, &empty, false).is_err());
    let good = Identity::new("Ana Souza", "ana@souza.dev");
    aurora_git::set_identity(&git, &top, &good, false).unwrap();
    assert_eq!(
        sh(&top, &["config", "--local", "user.name"]).trim(),
        "Ana Souza"
    );

    assert!(aurora_git::set_remote(&git, &top, "-x", "https://e.x/r.git").is_err());
    assert!(aurora_git::set_remote(&git, &top, "origin", "--upload-pack=x").is_err());
    aurora_git::set_remote(&git, &top, "origin", "https://ana:tok@github.com/ana/r.git").unwrap();
    let repo = status(&git, &top).unwrap().repository.unwrap();
    assert_eq!(repo.remotes[0].url, "https://github.com/ana/r.git");
    assert_eq!(repo.remotes[0].github.as_ref().unwrap().owner, "ana");
    // O mesmo nome troca o endereço.
    aurora_git::set_remote(&git, &top, "origin", "https://github.com/ana/certo.git").unwrap();
    let repo = status(&git, &top).unwrap().repository.unwrap();
    assert_eq!(repo.remotes.len(), 1);
    assert_eq!(repo.remotes[0].url, "https://github.com/ana/certo.git");
    aurora_git::remove_remote(&git, &top, "origin").unwrap();
    let repo = status(&git, &top).unwrap().repository.unwrap();
    assert!(repo.remotes.is_empty());
    assert!(aurora_git::remove_remote(&git, &top, "origin").is_err());
}

#[test]
fn a_wrong_remote_address_is_reported_and_can_be_fixed() {
    let Some(git) = git() else { return };
    let (_keep, base) = temp();
    let remote = base.join("certo.git");
    std::fs::create_dir_all(&remote).unwrap();
    sh(&remote, &["init", "-q", "--bare", "-b", "main"]);
    let mine = base.join("meu");
    std::fs::create_dir_all(&mine).unwrap();
    repository(&mine);
    let wrong = base.join("errado.git");
    aurora_git::set_remote(&git, &mine, "origin", wrong.as_str()).unwrap();
    match aurora_git::push(&git, &mine, None) {
        Err(Error::RemoteUnreachable { remote, url, .. }) => {
            assert_eq!(remote, "origin");
            assert_eq!(url, wrong.as_str());
        }
        other => panic!("{other:?}"),
    }
    aurora_git::set_remote(&git, &mine, "origin", remote.as_str()).unwrap();
    assert!(aurora_git::push(&git, &mine, None).unwrap().published);
}
