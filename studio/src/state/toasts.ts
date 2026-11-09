// Avisos rápidos no canto da janela: erro, sucesso, informação.

import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';

import { errorText } from '../i18n';
import type { IpcError } from '../ipc/types';

export type ToastKind = 'error' | 'success' | 'info' | 'warning';

export interface Toast {
  id: number;
  kind: ToastKind;
  title: string;
  detail?: string;
  /** Um botão no aviso, que faz o que o texto sugere (entrar no GitHub,
   * trazer os commits) e fecha o aviso. */
  action?: { label: string; run: () => void };
}

interface ToastState {
  toasts: Toast[];
  push: (toast: Omit<Toast, 'id'>, ms?: number) => void;
  dismiss: (id: number) => void;
}

let next = 1;

export const useToasts = create<ToastState>((set, get) => ({
  toasts: [],
  push: (toast, ms) => {
    const id = next++;
    set({ toasts: [...get().toasts.slice(-4), { ...toast, id }] });
    const timeout = ms ?? (toast.kind === 'error' ? 9000 : 4000);
    window.setTimeout(() => get().dismiss(id), timeout);
  },
  dismiss: (id) => set({ toasts: get().toasts.filter((t) => t.id !== id) }),
}));

/** Quem reage a um código de erro antes do aviso: devolve `true` se já avisou
 * o usuário do seu jeito, e então o aviso genérico não aparece. */
type ErrorHandler = (error: IpcError) => boolean;

const handlers = new Map<string, ErrorHandler>();

/** Registra quem reage a um código de erro em `showError`: o estado do
 * projeto, por exemplo, liga o aviso de `.spf` apagado no `spf_missing`. */
export function onErrorCode(code: string, handler: ErrorHandler): void {
  handlers.set(code, handler);
}

/** Mostra um erro do backend: o texto traduzido pelo código e, embaixo, a
 * mensagem original do Core. */
export function showError(error: unknown): void {
  const e = error as IpcError;
  const ipc = e && typeof e === 'object' && 'code' in e ? e : { code: 'internal', message: String(error) };
  const title = errorText(ipc);
  // O mesmo erro vai para o log do backend, para quem investiga pelo terminal.
  void invoke('log_frontend', { level: 'warn', message: `${ipc.code}: ${ipc.message}` }).catch(() => undefined);
  if (handlers.get(ipc.code)?.(ipc)) return;
  useToasts.getState().push({
    kind: 'error',
    title,
    detail: title === ipc.message ? undefined : ipc.message,
  });
}

/** Roda `fn` e mostra o erro, se houver. Devolve `undefined` no erro. */
export async function guarded<T>(fn: () => Promise<T>): Promise<T | undefined> {
  try {
    return await fn();
  } catch (error) {
    showError(error);
    return undefined;
  }
}
