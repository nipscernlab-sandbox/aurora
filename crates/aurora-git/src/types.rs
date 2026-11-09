//! O que as operações devolvem e o que recebem: os tipos do protocolo
//! (`git.*`, `github.*`), em `Serialize` e `JsonSchema`.

use camino::Utf8PathBuf;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `git.status`: o git da máquina e o repositório do projeto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct GitStatus {
    /// A versão do git (`2.54.0.windows.1`).
    pub version: String,
    /// O repositório que contém o projeto; `None` fora de repositório.
    pub repository: Option<RepositoryStatus>,
}

/// O estado de um repositório.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct RepositoryStatus {
    /// A pasta de topo do repositório, com `/`.
    #[schemars(with = "String")]
    pub top: Utf8PathBuf,
    /// A pasta do git (`.git`, ou a de um worktree), com `/`: é ela que muda
    /// num commit ou numa troca de ramo feitos fora da AURORA.
    #[schemars(with = "String")]
    pub git_dir: Utf8PathBuf,
    /// A pasta do projeto relativa ao topo, com `/` e sem barra no fim;
    /// vazia quando o projeto é o topo.
    pub prefix: String,
    /// O ramo atual.
    pub branch: BranchState,
    /// Os arquivos com mudança em relação ao último commit, primeiro os de
    /// dentro da pasta do projeto.
    pub files: Vec<ChangedFile>,
    /// A lista parou em [`MAX_FILES`](crate::MAX_FILES): há mais arquivos
    /// mudados do que os listados (falta um `.gitignore`, em geral).
    pub truncated: bool,
    /// Quem assina os commits, pela configuração do git; `None` sem nome ou
    /// sem e-mail.
    pub identity: Option<Identity>,
    /// Os remotos.
    pub remotes: Vec<Remote>,
    /// Uma operação pela metade (um merge com conflito, por exemplo).
    pub operation: Option<Operation>,
}

/// O ramo atual e a relação com o remoto.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct BranchState {
    /// O nome; `None` com o HEAD fora de ramo.
    pub name: Option<String>,
    /// O commit atual; `None` num repositório ainda sem commit.
    pub commit: Option<String>,
    /// O ramo remoto que ele acompanha (`origin/main`); `None` se ainda não
    /// foi publicado.
    pub upstream: Option<String>,
    /// Commits locais que o remoto não tem.
    pub ahead: u32,
    /// Commits do remoto que o local não tem (pelo último fetch).
    pub behind: u32,
    /// O remoto para onde o ramo vai ao publicar: o configurado no ramo, o
    /// `origin` ou o único que houver.
    pub remote: Option<String>,
}

/// Um arquivo com mudança.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct ChangedFile {
    /// O caminho relativo ao topo do repositório, com `/`.
    pub path: String,
    /// O caminho anterior, num arquivo renomeado.
    pub original: Option<String>,
    /// A mudança em relação ao último commit, juntando o que já foi
    /// preparado (o índice) e o que não foi.
    pub change: Change,
    /// Tem mudança preparada no índice (por um `git add` no terminal).
    pub staged: bool,
    /// Tem mudança ainda não preparada.
    pub unstaged: bool,
    /// Está dentro da pasta do projeto.
    pub in_project: bool,
}

/// O tipo de mudança de um arquivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Change {
    /// Novo, já conhecido do git (preparado).
    Added,
    /// Novo, que o git ainda não acompanha.
    Untracked,
    /// Modificado.
    Modified,
    /// Apagado.
    Deleted,
    /// Renomeado.
    Renamed,
    /// Copiado.
    Copied,
    /// Mudou de tipo (arquivo virou link, por exemplo).
    TypeChanged,
    /// Com conflito de merge.
    Conflicted,
}

/// Uma operação pela metade no repositório.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Operation {
    /// Um merge.
    Merge,
    /// Um rebase.
    Rebase,
    /// Um cherry-pick.
    CherryPick,
    /// Um revert.
    Revert,
}

/// Nome e e-mail de quem faz o commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct Identity {
    /// O nome.
    pub name: String,
    /// O e-mail.
    pub email: String,
}

impl Identity {
    /// Nome e e-mail.
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Identity {
            name: name.into(),
            email: email.into(),
        }
    }
}

/// Um remoto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct Remote {
    /// O nome (`origin`).
    pub name: String,
    /// O endereço, sem usuário nem senha embutidos.
    pub url: String,
    /// O repositório do GitHub, quando o remoto é do GitHub.
    pub github: Option<GitHubRepository>,
}

/// Um repositório do GitHub.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct GitHubRepository {
    /// O dono (usuário ou organização).
    pub owner: String,
    /// O nome.
    pub name: String,
}

/// Uma versão de um arquivo, para `git.read_file`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind", content = "oid")]
#[non_exhaustive]
pub enum Revision {
    /// O que está preparado (o índice).
    Index,
    /// O último commit do ramo atual.
    Head,
    /// Um commit, pelo identificador.
    Commit(String),
}

/// `git.read_file`: o conteúdo de um arquivo numa versão.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct FileVersion {
    /// O arquivo existe nessa versão.
    pub exists: bool,
    /// O tamanho, em bytes.
    pub size: u64,
    /// O conteúdo é binário (tem byte nulo no começo).
    pub binary: bool,
    /// Passou de [`MAX_FILE_SIZE`](crate::MAX_FILE_SIZE) e não foi lido.
    pub too_large: bool,
    /// O texto, quando existe, não é binário e não é grande demais.
    pub text: Option<String>,
}

/// O que `git.commit` recebe.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct CommitOptions {
    /// A primeira linha da mensagem.
    pub summary: String,
    /// O resto da mensagem, depois de uma linha em branco.
    #[serde(default)]
    pub description: Option<String>,
    /// Os arquivos que entram, relativos ao topo do repositório, com `/`;
    /// `None`, todos os mudados. Os outros, mesmo preparados no terminal,
    /// ficam de fora e continuam como estavam.
    #[serde(default)]
    pub paths: Option<Vec<String>>,
    /// Refaz o último commit em vez de criar outro.
    #[serde(default)]
    pub amend: bool,
}

impl CommitOptions {
    /// Um commit de todos os arquivos mudados.
    pub fn new(summary: impl Into<String>) -> Self {
        CommitOptions {
            summary: summary.into(),
            ..CommitOptions::default()
        }
    }
}

/// `git.commit`: o commit criado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct CommitResult {
    /// O identificador completo.
    pub oid: String,
    /// O identificador curto.
    pub short: String,
    /// A primeira linha da mensagem.
    pub summary: String,
    /// O ramo; `None` com o HEAD fora de ramo.
    pub branch: Option<String>,
}

/// `git.discard`: o que foi descartado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct DiscardResult {
    /// Os arquivos que voltaram ao último commit ou saíram, relativos ao topo.
    pub discarded: Vec<String>,
    /// As versões descartadas que foram para a lixeira do sistema, de onde
    /// voltam: o caminho absoluto de cada uma, antes de ir.
    #[schemars(with = "Vec<String>")]
    pub trashed: Vec<Utf8PathBuf>,
}

/// `git.init`: o repositório criado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct InitResult {
    /// A pasta do repositório (a do projeto).
    #[schemars(with = "String")]
    pub top: Utf8PathBuf,
    /// O ramo inicial.
    pub branch: String,
    /// As linhas acrescentadas ao `.gitignore`.
    pub ignored: Vec<String>,
}

/// `git.log`: uma página do histórico do ramo atual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct GitLog {
    /// Os commits, do mais novo para o mais velho.
    pub commits: Vec<CommitSummary>,
    /// Há mais commits depois desta página.
    pub more: bool,
}

/// Um commit do histórico.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct CommitSummary {
    /// O identificador completo.
    pub oid: String,
    /// O identificador curto.
    pub short: String,
    /// Os pais (dois num merge, nenhum no primeiro commit).
    pub parents: Vec<String>,
    /// O autor.
    pub author: String,
    /// O e-mail do autor.
    pub email: String,
    /// A data do autor, em segundos desde 1970 (UTC).
    pub time: i64,
    /// A primeira linha da mensagem.
    pub summary: String,
    /// O resto da mensagem.
    pub body: String,
    /// Os ramos e tags que apontam para ele (`origin/main`, `tag: v1.0`).
    pub refs: Vec<String>,
    /// Ainda não está em nenhum ramo remoto: só existe nesta máquina.
    pub local: bool,
}

/// `git.commit_files`: os arquivos que um commit mudou.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct CommitFiles {
    /// O commit.
    pub oid: String,
    /// O pai com que ele é comparado (o primeiro, num merge); `None` no
    /// primeiro commit.
    pub parent: Option<String>,
    /// Os arquivos.
    pub files: Vec<CommitFile>,
}

/// Um arquivo mudado por um commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct CommitFile {
    /// O caminho, relativo ao topo, com `/`.
    pub path: String,
    /// O caminho anterior, num arquivo renomeado.
    pub original: Option<String>,
    /// A mudança.
    pub change: Change,
}

/// `git.branches`: os ramos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct Branches {
    /// O atual; `None` com o HEAD fora de ramo.
    pub current: Option<String>,
    /// Os locais, do usado mais recentemente para o mais antigo.
    pub local: Vec<LocalBranch>,
    /// Os remotos sem ramo local com o mesmo nome.
    pub remote: Vec<RemoteBranch>,
}

/// Um ramo local.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct LocalBranch {
    /// O nome.
    pub name: String,
    /// O ramo remoto que ele acompanha.
    pub upstream: Option<String>,
    /// Commits que o remoto não tem.
    pub ahead: u32,
    /// Commits do remoto que ele não tem.
    pub behind: u32,
    /// O ramo remoto foi apagado.
    pub gone: bool,
    /// A data do último commit, em segundos desde 1970.
    pub time: i64,
}

/// Um ramo remoto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct RemoteBranch {
    /// O nome completo (`origin/feature`).
    pub name: String,
    /// O remoto (`origin`).
    pub remote: String,
    /// O ramo no remoto (`feature`).
    pub branch: String,
    /// A data do último commit, em segundos desde 1970.
    pub time: i64,
}

/// O que `git.switch` recebe.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct SwitchOptions {
    /// O ramo: um local, um remoto (`origin/feature`, que vira um local que o
    /// acompanha) ou, com `create`, o nome do novo.
    pub branch: String,
    /// Cria o ramo a partir do commit atual.
    #[serde(default)]
    pub create: bool,
}

impl SwitchOptions {
    /// Ir para um ramo que existe.
    pub fn to(branch: impl Into<String>) -> Self {
        SwitchOptions {
            branch: branch.into(),
            create: false,
        }
    }

    /// Criar o ramo e ir para ele.
    pub fn create(branch: impl Into<String>) -> Self {
        SwitchOptions {
            branch: branch.into(),
            create: true,
        }
    }
}

/// `git.fetch`, `git.pull` e `git.push`: o que foi feito com o remoto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[non_exhaustive]
pub struct SyncResult {
    /// O remoto.
    pub remote: String,
    /// O ramo.
    pub branch: Option<String>,
    /// O ramo remoto que ele acompanha, depois da operação.
    pub upstream: Option<String>,
    /// O ramo foi publicado agora (não existia no remoto).
    pub published: bool,
}
