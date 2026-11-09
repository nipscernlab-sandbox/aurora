//! Vigia a pasta do projeto e avisa a interface do que mudou no disco.
//!
//! A interface usa o aviso para atualizar a árvore de arquivos, o retrato
//! do projeto (o `.spf` pode ter mudado pela CLI no terminal) e as abas
//! abertas cujo arquivo mudou por fora.
//!
//! O evento é `studio://fs-changed`, com `{ paths: [...] }`, no máximo um a
//! cada 300 ms. O que muda em `.aurora8/` (temporários, relatórios) e em
//! `.git/` não é avisado: muda o tempo todo durante uma operação e não
//! aparece na árvore. Do `.git/` passam só o `HEAD`, o `index`, o
//! `packed-refs` e o que está em `refs/`, sem as travas (`*.lock`): é o que
//! muda num commit, numa troca de ramo ou num fetch feitos no terminal, e a
//! vista Controle de versão se atualiza com eles.
//!
//! Só mudança conta: criar, gravar, apagar, renomear, mudar atributos. O
//! inotify também avisa quando um arquivo é aberto para leitura, e avisar
//! disso faria a interface reler o projeto a cada leitura, inclusive a dela
//! mesma (reler o projeto abre o `.spf`, que avisaria de novo, num ciclo).

use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use notify::event::{AccessKind, AccessMode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::state::AppState;

/// O nome do evento.
pub const FS_CHANGED: &str = "studio://fs-changed";

/// Quanto juntar os eventos antes de avisar.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// O que vai no evento.
#[derive(Debug, Clone, Serialize)]
pub struct FsChanged {
    /// Os caminhos que mudaram (criados, alterados ou apagados).
    pub paths: Vec<Utf8PathBuf>,
}

/// O evento muda alguma coisa no disco. Abrir, ler e fechar sem gravar, não.
fn is_change(kind: &EventKind) -> bool {
    match kind {
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => true,
        EventKind::Access(_) => false,
        _ => true,
    }
}

/// O caminho, dentro de `git_dir`, é do que o git muda num commit, numa
/// troca de ramo ou num fetch: o `HEAD`, o `index`, o `packed-refs` e as
/// referências. As travas (`index.lock`, `refs/heads/main.lock`) não contam:
/// o arquivo de verdade chega logo depois, no rename.
fn is_git_state(git_dir: &Utf8Path, path: &Utf8Path) -> bool {
    let Ok(inside) = path.strip_prefix(git_dir) else {
        return false;
    };
    if inside.extension() == Some("lock") {
        return false;
    }
    matches!(inside.as_str(), "HEAD" | "index" | "packed-refs") || inside.starts_with("refs")
}

/// Começa a vigiar `root`, trocando o vigia anterior. Uma falha só vira
/// aviso no log: sem vigia, a interface ainda atualiza por conta própria.
pub fn watch(app: &AppHandle, state: &AppState, root: &Utf8Path) {
    stop(state);
    let (tx, rx) = mpsc::channel::<Utf8PathBuf>();
    let watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        let Ok(event) = result else { return };
        if !is_change(&event.kind) {
            return;
        }
        for path in event.paths {
            if let Ok(path) = Utf8PathBuf::from_path_buf(path) {
                let _ = tx.send(path);
            }
        }
    });
    let mut watcher: RecommendedWatcher = match watcher {
        Ok(watcher) => watcher,
        Err(error) => {
            tracing::warn!("Could not start the file watcher: {error}");
            return;
        }
    };
    if let Err(error) = watcher.watch(root.as_std_path(), RecursiveMode::Recursive) {
        tracing::warn!("Could not watch {root}: {error}");
        return;
    }

    // Junta os caminhos por 300 ms e avisa uma vez. A thread acaba quando o
    // vigia sai (o `tx` vai junto, e o `recv` falha).
    let app = app.clone();
    let aurora_dir = root.join(".aurora8");
    let git_dir = root.join(".git");
    std::thread::spawn(move || {
        while let Ok(first) = rx.recv() {
            let mut paths = vec![first];
            let deadline = Instant::now() + DEBOUNCE;
            loop {
                match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                    Ok(path) => paths.push(path),
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
            paths.retain(|p| {
                !p.starts_with(&aurora_dir)
                    && (!p.starts_with(&git_dir) || is_git_state(&git_dir, p))
            });
            paths.sort();
            paths.dedup();
            if !paths.is_empty() {
                let _ = app.emit(FS_CHANGED, FsChanged { paths });
            }
        }
    });
    *state.watcher.lock().expect("watcher lock") = Some(watcher);
}

/// Para de vigiar.
pub fn stop(state: &AppState) {
    state.watcher.lock().expect("watcher lock").take();
}

#[cfg(test)]
mod tests {
    use notify::event::{CreateKind, DataChange, ModifyKind};

    use super::*;

    #[test]
    fn reading_is_not_a_change() {
        assert!(!is_change(&EventKind::Access(AccessKind::Open(
            AccessMode::Any
        ))));
        assert!(!is_change(&EventKind::Access(AccessKind::Close(
            AccessMode::Read
        ))));
        assert!(is_change(&EventKind::Access(AccessKind::Close(
            AccessMode::Write
        ))));
        assert!(is_change(&EventKind::Modify(ModifyKind::Data(
            DataChange::Any
        ))));
        assert!(is_change(&EventKind::Create(CreateKind::File)));
    }

    #[test]
    fn only_the_git_state_passes() {
        let git = Utf8Path::new("/p/.git");
        for path in [
            "HEAD",
            "index",
            "packed-refs",
            "refs/heads/main",
            "refs/remotes/origin/main",
        ] {
            assert!(is_git_state(git, &git.join(path)), "{path}");
        }
        for path in [
            "index.lock",
            "refs/heads/main.lock",
            "objects/ab/cdef",
            "logs/HEAD",
            "ORIG_HEAD",
            "config",
        ] {
            assert!(!is_git_state(git, &git.join(path)), "{path}");
        }
        assert!(!is_git_state(git, Utf8Path::new("/p/src/HEAD")));
    }
}
