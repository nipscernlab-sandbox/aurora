//! A leitura da saída do git: só funções puras, testadas com saídas reais.

use crate::types::{
    BranchState, Change, CommitFile, CommitSummary, GitHubRepository, LocalBranch, RemoteBranch,
};

/// A versão de `git --version` (`git version 2.54.0.windows.1`): o texto
/// depois de `git version` e os três primeiros números.
pub(crate) fn version(text: &str) -> Option<(String, [u32; 3])> {
    let rest = text.trim().strip_prefix("git version ")?.trim();
    let full = rest.split_whitespace().next()?.to_owned();
    let mut numbers = [0u32; 3];
    for (slot, part) in numbers.iter_mut().zip(full.split('.')) {
        let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
        *slot = digits.parse().ok()?;
    }
    Some((full, numbers))
}

/// Uma linha de mudança do `git status --porcelain=v2`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub path: String,
    pub original: Option<String>,
    pub change: Change,
    pub staged: bool,
    pub unstaged: bool,
}

/// O `git status --porcelain=v2 -z --branch`: o cabeçalho do ramo e as
/// entradas, até `max` (a terceira posição diz se parou antes).
pub(crate) fn status(bytes: &[u8], max: usize) -> (BranchState, Vec<Entry>, bool) {
    let mut branch = BranchState::default();
    let mut entries = Vec::new();
    let mut truncated = false;
    let mut fields = bytes
        .split(|&b| b == 0)
        .map(|f| String::from_utf8_lossy(f).into_owned());
    while let Some(field) = fields.next() {
        if field.is_empty() {
            continue;
        }
        if let Some(header) = field.strip_prefix("# ") {
            let (key, value) = header.split_once(' ').unwrap_or((header, ""));
            match key {
                "branch.oid" if value != "(initial)" => branch.commit = Some(value.to_owned()),
                "branch.head" if value != "(detached)" => branch.name = Some(value.to_owned()),
                "branch.upstream" => branch.upstream = Some(value.to_owned()),
                "branch.ab" => {
                    for part in value.split_whitespace() {
                        if let Some(n) = part.strip_prefix('+') {
                            branch.ahead = n.parse().unwrap_or(0);
                        } else if let Some(n) = part.strip_prefix('-') {
                            branch.behind = n.parse().unwrap_or(0);
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        if entries.len() >= max {
            truncated = true;
            break;
        }
        let kind = field.as_bytes()[0];
        match kind {
            b'1' => {
                // 1 XY sub mH mI mW hH hI path
                let parts: Vec<&str> = field.splitn(9, ' ').collect();
                if parts.len() == 9 {
                    let (x, y) = xy(parts[1]);
                    entries.push(Entry {
                        path: parts[8].to_owned(),
                        original: None,
                        change: combined(x, y),
                        staged: x != '.',
                        unstaged: y != '.',
                    });
                }
            }
            b'2' => {
                // 2 XY sub mH mI mW hH hI Xscore path, e o caminho antigo no
                // campo seguinte.
                let parts: Vec<&str> = field.splitn(10, ' ').collect();
                let original = fields.next();
                if parts.len() == 10 {
                    let (x, y) = xy(parts[1]);
                    let change = if x == 'C' {
                        Change::Copied
                    } else {
                        Change::Renamed
                    };
                    entries.push(Entry {
                        path: parts[9].to_owned(),
                        original,
                        change,
                        staged: true,
                        unstaged: y != '.',
                    });
                }
            }
            b'u' => {
                // u XY sub m1 m2 m3 mW h1 h2 h3 path
                let parts: Vec<&str> = field.splitn(11, ' ').collect();
                if parts.len() == 11 {
                    entries.push(Entry {
                        path: parts[10].to_owned(),
                        original: None,
                        change: Change::Conflicted,
                        staged: false,
                        unstaged: true,
                    });
                }
            }
            b'?' => {
                if let Some(path) = field.strip_prefix("? ") {
                    entries.push(Entry {
                        path: path.to_owned(),
                        original: None,
                        change: Change::Untracked,
                        staged: false,
                        unstaged: true,
                    });
                }
            }
            _ => {}
        }
    }
    (branch, entries, truncated)
}

fn xy(field: &str) -> (char, char) {
    let mut chars = field.chars();
    (chars.next().unwrap_or('.'), chars.next().unwrap_or('.'))
}

fn letter(c: char) -> Option<Change> {
    Some(match c {
        'A' => Change::Added,
        'M' => Change::Modified,
        'D' => Change::Deleted,
        'R' => Change::Renamed,
        'C' => Change::Copied,
        'T' => Change::TypeChanged,
        'U' => Change::Conflicted,
        _ => return None,
    })
}

/// A mudança de um arquivo em relação ao último commit, juntando o índice
/// (`x`) e a árvore (`y`): um arquivo novo que depois mudou continua novo, e
/// um modificado que depois foi apagado está apagado.
fn combined(x: char, y: char) -> Change {
    match (letter(x), letter(y)) {
        (Some(Change::Added), Some(Change::Deleted)) => Change::Deleted,
        (Some(Change::Added), _) => Change::Added,
        (_, Some(Change::Deleted)) => Change::Deleted,
        (Some(staged), None) => staged,
        (None, Some(unstaged)) => unstaged,
        (Some(staged), Some(_)) => staged,
        (None, None) => Change::Modified,
    }
}

/// `git diff --name-status -z -M` e `git diff-tree --name-status -z`: letra,
/// caminho e, numa renomeação ou cópia, o caminho antigo antes do novo.
pub(crate) fn name_status(bytes: &[u8]) -> Vec<CommitFile> {
    let mut fields = bytes
        .split(|&b| b == 0)
        .map(|f| String::from_utf8_lossy(f).into_owned())
        .filter(|f| !f.is_empty());
    let mut files = Vec::new();
    while let Some(status) = fields.next() {
        let code = status.chars().next().unwrap_or('M');
        let change = letter(code).unwrap_or(Change::Modified);
        if matches!(code, 'R' | 'C') {
            let (Some(original), Some(path)) = (fields.next(), fields.next()) else {
                break;
            };
            files.push(CommitFile {
                path,
                original: Some(original),
                change,
            });
        } else if let Some(path) = fields.next() {
            files.push(CommitFile {
                path,
                original: None,
                change,
            });
        }
    }
    files
}

/// Os campos de cada commit no formato [`LOG_FORMAT`], separados por
/// `0x1f`; os commits, por `0x00` (`-z`).
pub(crate) const LOG_FORMAT: &str = "%H%x1f%h%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%D%x1f%s%x1f%b";

/// A saída de `git log -z --format=LOG_FORMAT`. `local` fica falso: quem
/// chama o liga para os commits que não estão em remoto nenhum.
pub(crate) fn log(bytes: &[u8]) -> Vec<CommitSummary> {
    bytes
        .split(|&b| b == 0)
        .filter_map(|record| {
            let text = String::from_utf8_lossy(record);
            let text = text.trim_start_matches('\n');
            if text.is_empty() {
                return None;
            }
            let parts: Vec<&str> = text.splitn(9, '\u{1f}').collect();
            if parts.len() < 8 {
                return None;
            }
            Some(CommitSummary {
                oid: parts[0].to_owned(),
                short: parts[1].to_owned(),
                parents: parts[2].split_whitespace().map(str::to_owned).collect(),
                author: parts[3].to_owned(),
                email: parts[4].to_owned(),
                time: parts[5].parse().unwrap_or(0),
                refs: parts[6]
                    .split(", ")
                    .map(|r| r.trim().trim_start_matches("HEAD -> ").to_owned())
                    .filter(|r| !r.is_empty() && r != "HEAD")
                    .collect(),
                summary: parts[7].to_owned(),
                body: parts
                    .get(8)
                    .map(|b| b.trim_end().to_owned())
                    .unwrap_or_default(),
                local: false,
            })
        })
        .collect()
}

/// O formato do `git for-each-ref` dos ramos: um ramo por linha, campos
/// separados por `0x00`.
pub(crate) const BRANCH_FORMAT: &str =
    "%(refname)%00%(upstream:short)%00%(upstream:track)%00%(committerdate:unix)";

/// A saída do `git for-each-ref --format=BRANCH_FORMAT refs/heads refs/remotes`.
pub(crate) fn branches(text: &str) -> (Vec<LocalBranch>, Vec<RemoteBranch>) {
    let mut local = Vec::new();
    let mut remote = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('\0').collect();
        if parts.len() < 4 {
            continue;
        }
        let time = parts[3].trim().parse().unwrap_or(0);
        if let Some(name) = parts[0].strip_prefix("refs/heads/") {
            let (ahead, behind, gone) = track(parts[2]);
            local.push(LocalBranch {
                name: name.to_owned(),
                upstream: Some(parts[1]).filter(|u| !u.is_empty()).map(str::to_owned),
                ahead,
                behind,
                gone,
                time,
            });
        } else if let Some(full) = parts[0].strip_prefix("refs/remotes/") {
            let Some((name, branch)) = full.split_once('/') else {
                continue;
            };
            if branch == "HEAD" {
                continue;
            }
            remote.push(RemoteBranch {
                name: full.to_owned(),
                remote: name.to_owned(),
                branch: branch.to_owned(),
                time,
            });
        }
    }
    (local, remote)
}

/// `[ahead 1, behind 2]`, `[gone]` ou vazio.
fn track(text: &str) -> (u32, u32, bool) {
    let inner = text.trim().trim_start_matches('[').trim_end_matches(']');
    let mut ahead = 0;
    let mut behind = 0;
    let mut gone = false;
    for part in inner.split(',') {
        let part = part.trim();
        if let Some(n) = part.strip_prefix("ahead ") {
            ahead = n.parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix("behind ") {
            behind = n.parse().unwrap_or(0);
        } else if part == "gone" {
            gone = true;
        }
    }
    (ahead, behind, gone)
}

/// `git config -z --get-regexp`: chave e valor, separados por `\n`; as
/// entradas, por `0x00`.
pub(crate) fn config(bytes: &[u8]) -> Vec<(String, String)> {
    bytes
        .split(|&b| b == 0)
        .filter(|e| !e.is_empty())
        .map(|entry| {
            let text = String::from_utf8_lossy(entry);
            let (key, value) = text.split_once('\n').unwrap_or((&text, ""));
            (key.to_owned(), value.to_owned())
        })
        .collect()
}

/// As linhas `chave=valor` do `git credential fill`.
pub(crate) fn credential(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

/// O endereço sem usuário e senha: `https://usuario:token@github.com/x`
/// vira `https://github.com/x`. Endereços SSH (`git@github.com:x`) ficam como
/// estão, porque o `git@` não é segredo.
pub(crate) fn sanitize_url(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let (authority, path) = rest.split_once('/').map_or((rest, ""), |(a, p)| (a, p));
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    if path.is_empty() && !rest.contains('/') {
        format!("{scheme}://{host}")
    } else {
        format!("{scheme}://{host}/{path}")
    }
}

/// O dono e o nome de um remoto do GitHub, por HTTPS ou SSH.
pub(crate) fn github_repository(url: &str) -> Option<GitHubRepository> {
    let url = sanitize_url(url);
    let path = if let Some(rest) = url.strip_prefix("git@github.com:") {
        rest
    } else if let Some((_, rest)) = url.split_once("://") {
        let rest = rest.strip_prefix("git@").unwrap_or(rest);
        let (host, path) = rest.split_once('/')?;
        let host = host.split(':').next().unwrap_or(host);
        if !host.eq_ignore_ascii_case("github.com") && !host.eq_ignore_ascii_case("www.github.com")
        {
            return None;
        }
        path
    } else {
        return None;
    };
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, name) = path.split_once('/')?;
    if owner.is_empty() || name.is_empty() || name.contains('/') {
        return None;
    }
    Some(GitHubRepository {
        owner: owner.to_owned(),
        name: name.to_owned(),
    })
}

/// Os arquivos de uma mensagem do git que lista os que seriam
/// sobrescritos (uma linha com tab por arquivo, entre o aviso e o
/// "Please ...").
pub(crate) fn overwritten_files(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|line| line.starts_with('\t'))
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .collect()
}

/// O git recusou por falta de credencial ou credencial errada.
pub(crate) fn is_authentication_failure(stderr: &str) -> bool {
    const SIGNS: &[&str] = &[
        "Authentication failed",
        "could not read Username",
        "could not read Password",
        "terminal prompts disabled",
        "Permission denied (publickey",
        "Invalid username or password",
        "The requested URL returned error: 403",
        "The requested URL returned error: 401",
        "Write access to repository not granted",
        "Permission to ",
    ];
    SIGNS.iter().any(|sign| stderr.contains(sign))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_version_of_each_platform() {
        assert_eq!(
            version("git version 2.54.0.windows.1\n"),
            Some(("2.54.0.windows.1".into(), [2, 54, 0]))
        );
        assert_eq!(
            version("git version 2.39.5 (Apple Git-154)"),
            Some(("2.39.5".into(), [2, 39, 5]))
        );
        assert_eq!(
            version("git version 2.34.1"),
            Some(("2.34.1".into(), [2, 34, 1]))
        );
        assert_eq!(version("bash: git: command not found"), None);
    }

    #[test]
    fn reads_the_status_with_branch_renames_conflicts_and_accents() {
        let raw = b"# branch.oid 1234abcd\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\
1 .M N... 100644 100644 100644 aaaa bbbb src/top.v\0\
1 A. N... 000000 100644 100644 0000 cccc novo.v\0\
1 AM N... 000000 100644 100644 0000 cccc novo2.v\0\
1 MD N... 100644 100644 000000 aaaa bbbb sumiu.v\0\
2 R. N... 100644 100644 100644 aaaa aaaa R100 pasta com espa\xc3\xa7o/n\xc3\xa3o.v\0velho.v\0\
u UU N... 100644 100644 100644 100644 aaaa bbbb cccc conflito.v\0\
? .aurora8/x.txt\0";
        let (branch, entries, truncated) = status(raw, 100);
        assert_eq!(branch.name.as_deref(), Some("main"));
        assert_eq!(branch.commit.as_deref(), Some("1234abcd"));
        assert_eq!(branch.upstream.as_deref(), Some("origin/main"));
        assert_eq!((branch.ahead, branch.behind), (2, 1));
        assert!(!truncated);
        let summary: Vec<(&str, Change, bool, bool)> = entries
            .iter()
            .map(|e| (e.path.as_str(), e.change, e.staged, e.unstaged))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("src/top.v", Change::Modified, false, true),
                ("novo.v", Change::Added, true, false),
                ("novo2.v", Change::Added, true, true),
                ("sumiu.v", Change::Deleted, true, true),
                ("pasta com espaço/não.v", Change::Renamed, true, false),
                ("conflito.v", Change::Conflicted, false, true),
                (".aurora8/x.txt", Change::Untracked, false, true),
            ]
        );
        assert_eq!(entries[4].original.as_deref(), Some("velho.v"));
    }

    #[test]
    fn a_new_repository_and_a_detached_head_have_no_name() {
        let (branch, _, _) = status(b"# branch.oid (initial)\0# branch.head main\0", 10);
        assert_eq!(branch.commit, None);
        assert_eq!(branch.name.as_deref(), Some("main"));
        let (branch, _, _) = status(b"# branch.oid abcd\0# branch.head (detached)\0", 10);
        assert_eq!(branch.name, None);
    }

    #[test]
    fn the_status_stops_at_the_limit() {
        let raw = b"? a\0? b\0? c\0";
        let (_, entries, truncated) = status(raw, 2);
        assert_eq!(entries.len(), 2);
        assert!(truncated);
    }

    #[test]
    fn reads_name_status_with_renames() {
        let raw = b"M\0a.v\0R087\0velho.v\0novo.v\0A\0b.v\0D\0c.v\0";
        let files = name_status(raw);
        assert_eq!(files.len(), 4);
        assert_eq!(files[1].path, "novo.v");
        assert_eq!(files[1].original.as_deref(), Some("velho.v"));
        assert_eq!(files[1].change, Change::Renamed);
        assert_eq!(files[3].change, Change::Deleted);
    }

    #[test]
    fn reads_the_log_with_refs_and_body() {
        let raw = "a1\x1fa\x1fp1 p2\x1fAna\x1fana@x\x1f1700000000\x1fHEAD -> main, origin/main, tag: v1\x1fMerge\x1fcorpo\n\n\0\
b2\x1fb\x1f\x1fBia\x1fbia@x\x1f1600000000\x1f\x1fPrimeiro\x1f\0";
        let commits = log(raw.as_bytes());
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].parents, vec!["p1", "p2"]);
        assert_eq!(commits[0].refs, vec!["main", "origin/main", "tag: v1"]);
        assert_eq!(commits[0].body, "corpo");
        assert_eq!(commits[1].parents, Vec::<String>::new());
        assert_eq!(commits[1].summary, "Primeiro");
        assert_eq!(commits[1].time, 1600000000);
    }

    #[test]
    fn reads_local_and_remote_branches() {
        let text = "refs/heads/main\0origin/main\0[ahead 1, behind 2]\x001700\n\
refs/heads/velho\0origin/velho\0[gone]\x001600\n\
refs/heads/solo\0\0\x001500\n\
refs/remotes/origin/HEAD\0\0\x001700\n\
refs/remotes/origin/feat/x\0\0\x001650\n";
        let (local, remote) = branches(text);
        assert_eq!(local.len(), 3);
        assert_eq!(
            (local[0].ahead, local[0].behind, local[0].gone),
            (1, 2, false)
        );
        assert!(local[1].gone);
        assert_eq!(local[2].upstream, None);
        assert_eq!(remote.len(), 1);
        assert_eq!(remote[0].branch, "feat/x");
        assert_eq!(remote[0].remote, "origin");
    }

    #[test]
    fn hides_credentials_in_urls() {
        assert_eq!(
            sanitize_url("https://ana:ghp_x@github.com/ana/repo.git"),
            "https://github.com/ana/repo.git"
        );
        assert_eq!(
            sanitize_url("git@github.com:ana/repo.git"),
            "git@github.com:ana/repo.git"
        );
        assert_eq!(sanitize_url("https://gitlab.com/x"), "https://gitlab.com/x");
    }

    #[test]
    fn recognizes_github_remotes() {
        for url in [
            "https://github.com/nipscernlab/sapho.git",
            "https://user:tok@github.com/nipscernlab/sapho",
            "git@github.com:nipscernlab/sapho.git",
            "ssh://git@github.com/nipscernlab/sapho.git",
        ] {
            let repo = github_repository(url).unwrap_or_else(|| panic!("{url}"));
            assert_eq!(
                (repo.owner.as_str(), repo.name.as_str()),
                ("nipscernlab", "sapho")
            );
        }
        assert_eq!(
            github_repository("https://gitlab.com/nips-cern/x.git"),
            None
        );
    }

    #[test]
    fn reads_the_files_a_switch_would_overwrite() {
        let stderr = "error: Your local changes to the following files would be overwritten by checkout:\n\ta.v\n\tpasta/b.v\nPlease commit your changes or stash them before you switch branches.\nAborting";
        assert_eq!(overwritten_files(stderr), vec!["a.v", "pasta/b.v"]);
    }
}
