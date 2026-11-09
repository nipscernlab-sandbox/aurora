// A conta do GitHub do git desta máquina (recurso experimental `git`): quem
// está conectado e o login pelo código de dispositivo. A vista Perfil, a foto
// na barra de atividades e o "Usar os do GitHub" da vista Controle de versão
// leem daqui.
//
// A conta é lida ao ligar o recurso, ao abrir a vista Perfil e quando a
// janela volta a ter o foco; o servidor guarda a resposta por 10 minutos.
//
// O login: `github.login_start` dá o código que a pessoa digita no GitHub, e
// `github.login_poll` pergunta, a cada `interval` segundos, se ela já
// autorizou. A espera continua com a vista Perfil fora da tela; para com o
// Cancelar, com o código vencido ou com o recurso desligado.

import { create } from 'zustand';

import { api } from '../ipc/api';
import type { Account, DeviceCode } from '../ipc/aurora-types';
import type { IpcError } from '../ipc/types';
import { featureOn, useExperimental } from './experimental';

export type LoginState =
  | { step: 'idle' }
  | { step: 'starting' }
  | { step: 'waiting'; code: DeviceCode }
  | { step: 'expired' }
  | { step: 'denied' }
  | { step: 'failed'; error: IpcError };

interface GitHubState {
  /** `undefined` antes da primeira leitura; `null` sem login. */
  account: Account | null | undefined;
  accountLoading: boolean;
  accountError: IpcError | null;
  login: LoginState;
}

export const useGitHub = create<GitHubState>(() => ({
  account: undefined,
  accountLoading: false,
  accountError: null,
  login: { step: 'idle' },
}));

/** Relê a conta. */
export async function loadAccount(): Promise<void> {
  if (!featureOn('git') || useGitHub.getState().accountLoading) return;
  useGitHub.setState({ accountLoading: true });
  try {
    const account = await api.github.account();
    useGitHub.setState({ account, accountError: null });
  } catch (error) {
    const e = error as IpcError;
    // Sem o curl ou sem rede, a conta fica como estava, com o erro à vista.
    useGitHub.setState((s) => ({ accountError: e, account: s.account === undefined ? null : s.account }));
  } finally {
    useGitHub.setState({ accountLoading: false });
  }
}

/** Muda a cada login começado ou cancelado: a pergunta de um login antigo
 * que ainda estava esperando não continua. */
let attempt = 0;
let pollTimer = 0;

export async function startLogin(): Promise<void> {
  const mine = ++attempt;
  window.clearTimeout(pollTimer);
  useGitHub.setState({ login: { step: 'starting' } });
  try {
    const code = await api.github.loginStart();
    if (mine !== attempt) return;
    useGitHub.setState({ login: { step: 'waiting', code } });
    schedulePoll(mine, code.device_code, code.interval);
  } catch (error) {
    if (mine === attempt) useGitHub.setState({ login: { step: 'failed', error: error as IpcError } });
  }
}

function schedulePoll(mine: number, deviceCode: string, seconds: number): void {
  pollTimer = window.setTimeout(() => void poll(mine, deviceCode), Math.max(1, seconds) * 1000);
}

async function poll(mine: number, deviceCode: string): Promise<void> {
  if (mine !== attempt) return;
  try {
    const answer = await api.github.loginPoll(deviceCode);
    if (mine !== attempt) return;
    switch (answer.state) {
      case 'pending':
        schedulePoll(mine, deviceCode, answer.interval);
        return;
      case 'done':
        useGitHub.setState({ account: answer.account, accountError: null, login: { step: 'idle' } });
        return;
      case 'expired':
        useGitHub.setState({ login: { step: 'expired' } });
        return;
      case 'denied':
        useGitHub.setState({ login: { step: 'denied' } });
        return;
    }
  } catch (error) {
    if (mine === attempt) useGitHub.setState({ login: { step: 'failed', error: error as IpcError } });
  }
}

/** Para de perguntar e volta ao começo. */
export function cancelLogin(): void {
  attempt++;
  window.clearTimeout(pollTimer);
  if (useGitHub.getState().login.step !== 'idle') useGitHub.setState({ login: { step: 'idle' } });
}

/** Tira o login do git desta máquina. */
export async function logout(): Promise<void> {
  cancelLogin();
  await api.github.logout();
  useGitHub.setState({ account: null, accountError: null });
}

/** Uma foto do GitHub no tamanho pedido (`s`, em pixels). */
export function avatarUrl(url: string, size: number): string {
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== 'https:') return url;
    parsed.searchParams.set('s', String(Math.round(size * (window.devicePixelRatio || 1))));
    return parsed.toString();
  } catch {
    return url;
  }
}

// O recurso ligou: a conta, para a foto da barra de atividades. Desligou:
// nada da conta fica na janela.
useExperimental.subscribe((state, previous) => {
  if (state.enabled === previous.enabled) return;
  if (state.enabled.has('git')) void loadAccount();
  else {
    cancelLogin();
    useGitHub.setState({ account: undefined, accountError: null });
  }
});
