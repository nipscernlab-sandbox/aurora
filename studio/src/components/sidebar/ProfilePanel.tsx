// A vista Perfil (recurso experimental `git`): a conta do GitHub do git
// desta máquina. Sem conta, o login pelo código de dispositivo, todo dentro
// da vista: o código grande, o botão que o copia e abre o GitHub no
// navegador, e a espera pela autorização. Com conta, a foto, o nome, o
// login e a identidade dos commits, com o botão que usa o nome e o e-mail
// do GitHub. O estado fica em state/github.ts.

import { openUrl } from '@tauri-apps/plugin-opener';
import { Copy, ExternalLink, LogIn, LogOut, RefreshCw, TriangleAlert, UserRound } from 'lucide-react';
import { useEffect, useState } from 'react';

import { errorText, t, useT } from '../../i18n';
import type { Account } from '../../ipc/aurora-types';
import { confirm } from '../../state/dialogs';
import { setIdentity, useGit } from '../../state/git';
import { avatarUrl, cancelLogin, loadAccount, logout, startLogin, useGitHub } from '../../state/github';
import { useProject } from '../../state/project';
import { showError, useToasts } from '../../state/toasts';
import { Button, Empty, IconButton, Spinner } from '../common';
import { ViewActions } from '../layout/ViewActions';

function Avatar({ account, size }: { account: Account; size: number }) {
  const [failed, setFailed] = useState(false);
  if (failed) {
    return (
      <span className="profile__avatar profile__avatar--empty" style={{ width: size, height: size }}>
        <UserRound size={size * 0.5} strokeWidth={1.5} />
      </span>
    );
  }
  return (
    <img
      className="profile__avatar"
      src={avatarUrl(account.avatar_url, size)}
      width={size}
      height={size}
      alt=""
      draggable={false}
      onError={() => setFailed(true)}
    />
  );
}

function SignIn() {
  const login = useGitHub((s) => s.login);
  const accountError = useGitHub((s) => s.accountError);
  const [copied, setCopied] = useState(false);

  switch (login.step) {
    case 'starting':
      return (
        <div className="profile">
          <Spinner size={18} />
          <p className="profile__hint">{t('profile.starting')}</p>
        </div>
      );
    case 'waiting': {
      const { code } = login;
      const copyAndOpen = () => {
        void navigator.clipboard
          .writeText(code.user_code)
          .then(() => setCopied(true))
          .catch(() => undefined);
        void openUrl(code.verification_uri).catch(showError);
      };
      return (
        <div className="profile">
          <p className="profile__lead">{t('profile.codeTitle')}</p>
          <p className="profile__code mono" aria-label={t('profile.codeTitle')}>
            {code.user_code}
          </p>
          <Button variant="primary" icon={<Copy size={14} />} onClick={copyAndOpen}>
            {t('profile.copyAndOpen')}
          </Button>
          <p className="profile__hint">
            {copied ? t('profile.copied', { uri: code.verification_uri }) : t('profile.codeHint', { uri: code.verification_uri })}
          </p>
          <p className="profile__waiting">
            <Spinner size={13} /> {t('profile.waiting')}
          </p>
          <Button small onClick={cancelLogin}>
            {t('common.cancel')}
          </Button>
        </div>
      );
    }
    case 'expired':
    case 'denied':
    case 'failed':
      return (
        <div className="profile">
          <p className="profile__problem">
            <TriangleAlert size={14} />
            {login.step === 'failed' ? errorText(login.error) : t(login.step === 'expired' ? 'profile.expired' : 'profile.denied')}
          </p>
          {login.step === 'failed' && errorText(login.error) !== login.error.message && <p className="profile__hint">{login.error.message}</p>}
          <Button variant="primary" icon={<RefreshCw size={14} />} onClick={() => void startLogin()}>
            {t('profile.tryAgain')}
          </Button>
          <Button small variant="ghost" onClick={cancelLogin}>
            {t('common.cancel')}
          </Button>
        </div>
      );
    default:
      return (
        <div className="profile">
          <span className="profile__avatar profile__avatar--empty" style={{ width: 64, height: 64 }}>
            <UserRound size={32} strokeWidth={1.5} />
          </span>
          <p className="profile__lead">{t('profile.signedOut')}</p>
          <p className="profile__hint">{t('profile.signInHint')}</p>
          {accountError && <p className="profile__problem">{errorText(accountError)}</p>}
          <Button variant="primary" icon={<LogIn size={14} />} onClick={() => void startLogin()}>
            {t('profile.signIn')}
          </Button>
        </div>
      );
  }
}

async function signOut(account: Account) {
  const answer = await confirm({
    title: t('profile.signOut.title'),
    message: t('profile.signOut.message', { login: account.login }),
    buttons: [
      { label: t('profile.signOut.confirm'), value: 'yes', danger: true },
      { label: t('common.cancel'), value: 'no' },
    ],
  });
  if (answer !== 'yes') return;
  try {
    await logout();
    useToasts.getState().push({ kind: 'info', title: t('profile.signedOutDone') });
  } catch (error) {
    showError(error);
  }
}

function AccountCard({ account }: { account: Account }) {
  const hasProject = useProject((s) => s.snapshot !== null);
  const repo = useGit((s) => s.status?.repository ?? null);
  const busy = useGit((s) => s.busy);
  const identity = repo?.identity ?? null;
  const githubName = account.name || account.login;
  const same = !!identity && identity.name === githubName && identity.email === account.commit_email;

  return (
    <div className="profile">
      <Avatar account={account} size={72} />
      <p className="profile__name">{githubName}</p>
      <button type="button" className="link profile__login" onClick={() => void openUrl(account.html_url).catch(showError)}>
        @{account.login}
        <ExternalLink size={12} />
      </button>
      {!account.saved && (
        <p className="git-card git-card--warn profile__notice">
          <TriangleAlert size={13} /> {t('profile.notSaved')}
        </p>
      )}

      <section className="profile__section">
        <h4 className="profile__heading">{t('profile.commits')}</h4>
        {repo ? (
          identity ? (
            <p className="profile__identity">
              <span>{identity.name}</span>
              <span className="mono">{identity.email}</span>
            </p>
          ) : (
            <p className="profile__hint">{t('profile.noIdentity')}</p>
          )
        ) : (
          <p className="profile__hint">{t(hasProject ? 'profile.identityNoRepo' : 'profile.identityNoProject')}</p>
        )}
        {!same && (
          <>
            <Button
              small
              disabled={!hasProject || !!busy}
              title={hasProject ? undefined : t('profile.identityNoProject')}
              onClick={() => void setIdentity(githubName, account.commit_email, true)}
            >
              {busy === 'identity' && <Spinner size={12} />}
              {t('profile.useGitHubIdentity')}
            </Button>
            <p className="profile__hint">{t('profile.useGitHubHint', { email: account.commit_email })}</p>
          </>
        )}
      </section>

      <section className="profile__section">
        <Button small variant="danger" icon={<LogOut size={13} />} onClick={() => void signOut(account)}>
          {t('profile.signOut.action')}
        </Button>
        <p className="profile__hint">{t('profile.signOut.hint')}</p>
      </section>
    </div>
  );
}

export function ProfilePanel() {
  useT();
  const account = useGitHub((s) => s.account);
  const loading = useGitHub((s) => s.accountLoading);

  // Abrir a vista relê a conta; sair dela para o login que estava esperando.
  // O login continua com a vista fora da tela: a pessoa sai para o navegador
  // e pode mexer na IDE enquanto autoriza. Só o Cancelar, o código vencido ou
  // desligar o recurso param a espera.
  useEffect(() => {
    void loadAccount();
  }, []);

  return (
    <>
      <ViewActions>
        <IconButton label={t('common.refresh')} disabled={loading} onClick={() => void loadAccount()}>
          {loading ? <Spinner size={13} /> : <RefreshCw size={14} />}
        </IconButton>
      </ViewActions>
      <div className="sidebar__body">
        {account === undefined ? (
          <Empty>
            <Spinner />
          </Empty>
        ) : account ? (
          <AccountCard account={account} />
        ) : (
          <SignIn />
        )}
      </div>
    </>
  );
}
