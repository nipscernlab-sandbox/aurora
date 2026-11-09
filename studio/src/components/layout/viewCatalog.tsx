// Cada vista da janela: o ícone, o componente e as marcas que ela mostra na
// aba ou no ícone (o número de Problemas, o ponto de saída nova). O rótulo e
// a região padrão ficam no modelo (state/layoutModel.ts), sem React.
//
// As vistas que nasceram na barra lateral (`side`) são colunas que rolam por
// dentro, no fundo das barras laterais; as do painel (`panel`) ocupam a área
// inteira, no fundo do editor. Cada uma leva o seu fundo para a região onde
// estiver.
//
// O ícone da vista Perfil é a foto da conta do GitHub, quando há login
// (`ViewIcon`).

import {
  Activity,
  Binary,
  Bot,
  CircleAlert,
  CircleUserRound,
  CircuitBoard,
  FolderTree,
  GitBranch,
  GraduationCap,
  Hammer,
  ListChecks,
  Logs,
  Microchip,
  ScrollText,
  Search,
  SquareTerminal,
  Workflow,
  type LucideIcon,
} from 'lucide-react';
import { useState } from 'react';

import { useGit } from '../../state/git';
import { avatarUrl, useGitHub } from '../../state/github';
import { useLayout, type ViewId } from '../../state/layout';
import { ConsoleView, ProblemsView, ShellView, useProblemCount } from '../panel/PanelViews';
import { Explorer } from '../sidebar/Explorer';
import { FlowNavigator } from '../sidebar/FlowNavigator';
import { Chat } from '../assistant/Chat';
import { GitPanel } from '../sidebar/GitPanel';
import { LearnPanel } from '../sidebar/LearnPanel';
import { ProfilePanel } from '../sidebar/ProfilePanel';
import { ReportsPanel } from '../sidebar/ReportsPanel';
import { SearchPanel } from '../sidebar/SearchPanel';

export const VIEW_ICONS: Record<ViewId, LucideIcon> = {
  explorer: FolderTree,
  flow: Workflow,
  search: Search,
  git: GitBranch,
  reports: ScrollText,
  learn: GraduationCap,
  profile: CircleUserRound,
  assistant: Bot,
  info: Logs,
  cmm: Hammer,
  asm: Binary,
  verilog: ListChecks,
  wave: Activity,
  rtlViewer: CircuitBoard,
  fpga: Microchip,
  problems: CircleAlert,
  terminal: SquareTerminal,
};

export type ViewKind = 'side' | 'panel';

const SIDE_VIEWS: ReadonlySet<ViewId> = new Set(['explorer', 'flow', 'search', 'git', 'reports', 'learn', 'profile', 'assistant']);

export function viewKind(view: ViewId): ViewKind {
  return SIDE_VIEWS.has(view) ? 'side' : 'panel';
}

/** O ícone de uma vista numa barra de atividades ou numa aba de ícone. O da
 * vista Perfil é a foto da conta do GitHub, redonda, quando há login e ela
 * carrega; sem login, ou sem rede, o ícone de pessoa. */
export function ViewIcon({ view, size, strokeWidth }: { view: ViewId; size: number; strokeWidth: number }) {
  const avatar = useGitHub((s) => (view === 'profile' ? (s.account?.avatar_url ?? null) : null));
  const [failed, setFailed] = useState<string | null>(null);
  if (avatar && failed !== avatar) {
    return (
      <img
        className="view-avatar"
        src={avatarUrl(avatar, size + 2)}
        width={size + 2}
        height={size + 2}
        alt=""
        draggable={false}
        onError={() => setFailed(avatar)}
      />
    );
  }
  const Icon = VIEW_ICONS[view];
  return <Icon size={size} strokeWidth={strokeWidth} />;
}

export function ViewContent({ view }: { view: ViewId }) {
  switch (view) {
    case 'explorer':
      return <Explorer />;
    case 'flow':
      return <FlowNavigator />;
    case 'search':
      return <SearchPanel />;
    case 'reports':
      return <ReportsPanel />;
    case 'learn':
      return <LearnPanel />;
    case 'git':
      return <GitPanel />;
    case 'profile':
      return <ProfilePanel />;
    case 'assistant':
      return <Chat mode="side" />;
    case 'problems':
      return <ProblemsView />;
    case 'terminal':
      return <ShellView />;
    default:
      return <ConsoleView channel={view} />;
  }
}

/**
 * As marcas de uma vista: o número de erros e avisos (em Problemas) e o
 * ponto de saída nova que ainda não foi vista. Numa aba de texto ficam ao
 * lado do nome; num ícone (`compact`), no canto dele.
 */
export function ViewBadge({ view, compact }: { view: ViewId; compact?: boolean }) {
  const unread = useLayout((s) => !!s.unread[view]);
  if (view === 'problems') return <ProblemBadge compact={compact} unread={unread} />;
  if (view === 'git') return <GitBadge compact={compact} />;
  return unread ? <span className={compact ? 'view-badge view-badge--dot' : 'panel__unread'} aria-hidden /> : null;
}

/** O número de arquivos mudados no repositório; com a lista cortada (falta
 * um .gitignore), o número e um +. */
function GitBadge({ compact }: { compact?: boolean }) {
  const count = useGit((s) => s.status?.repository?.files.length ?? 0);
  const truncated = useGit((s) => !!s.status?.repository?.truncated);
  if (count === 0) return null;
  return <span className={`count count--neutral${compact ? ' view-badge view-badge--count' : ''}`}>{truncated ? `${count}+` : count}</span>;
}

function ProblemBadge({ compact, unread }: { compact?: boolean; unread: boolean }) {
  const { errors, warnings } = useProblemCount();
  const total = errors + warnings;
  if (total === 0) return unread ? <span className={compact ? 'view-badge view-badge--dot' : 'panel__unread'} aria-hidden /> : null;
  const tone = errors ? 'count--error' : 'count--warn';
  return <span className={`count ${tone}${compact ? ' view-badge view-badge--count' : ''}`}>{total}</span>;
}
