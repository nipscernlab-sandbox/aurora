// A marca do git de um arquivo, a mesma no Explorador, na vista Controle de
// versão e na aba de diff: a letra da mudança, como no VS Code, na cor dela
// (app.css, `.git-letter--*`).

import { t, type Key } from '../../i18n';
import type { GitChange } from '../../state/git';

/** A letra de cada mudança. */
export const CHANGE_LETTER: Record<GitChange, string> = {
  added: 'A',
  untracked: 'U',
  modified: 'M',
  deleted: 'D',
  renamed: 'R',
  copied: 'C',
  type_changed: 'T',
  conflicted: '!',
};

export function ChangeLetter({ change }: { change: GitChange }) {
  return (
    <span className={`git-letter git-letter--${change}`} title={t(`git.change.${change}` as Key)}>
      {CHANGE_LETTER[change]}
    </span>
  );
}
