// O ícone de cada aba e de cada arquivo do explorador: os arquivos do fluxo
// da AURORA têm ícone próprio (fileIcons.tsx); os outros, o do Lucide pela
// extensão; as vistas, pelo tipo delas.

import {
  AudioWaveform,
  Bot,
  ChartColumn,
  CircuitBoard,
  Cpu,
  File,
  FileCode,
  FileDiff,
  FileText,
  GitCompare,
  GraduationCap,
  Info,
  Microchip,
  MonitorPlay,
  ScrollText,
  Settings,
  Wrench,
  type LucideIcon,
} from 'lucide-react';

import type { Tab } from '../../state/editor';
import { useProject } from '../../state/project';
import { baseName, extension, samePath } from '../../util/paths';
import { FILE_ICONS, type FileKind } from './fileIcons';

/** Testbench pelo projeto (a lista de testbenches do `.spf`) ou pelo nome:
 * `tb_<nome>.v`, `<nome>_tb.v` e o `test_<nome>.py` do cocotb. */
function isTestbench(path: string): boolean {
  if (/^tb_|_tb\.[^.]+$|^test_.+\.py$/i.test(baseName(path))) return true;
  return useProject.getState().snapshot?.testbenches.some((file) => samePath(file.path, path)) ?? false;
}

/** O tipo de um arquivo do fluxo da AURORA, que tem ícone próprio
 * (fileIcons.tsx); os outros ficam sem tipo. */
export function fileKind(path: string): FileKind | null {
  switch (extension(path)) {
    case 'v':
    case 'sv':
    case 'vh':
    case 'svh':
      return isTestbench(path) ? 'testbench' : 'verilog';
    case 'py':
      return isTestbench(path) ? 'testbench' : null;
    case 'cmm':
      return 'cmm';
    case 'asm':
      return 'asm';
    case 'mif':
      return 'mif';
    case 'vcd':
    case 'fst':
    case 'ghw':
      return 'wave';
    case 'spf':
      return 'project';
    default:
      return null;
  }
}

export function fileIcon(path: string): LucideIcon {
  const kind = fileKind(path);
  if (kind) return FILE_ICONS[kind];
  switch (extension(path)) {
    case 'cpp':
    case 'c':
    case 'h':
    case 'py':
      return FileCode;
    case 'txt':
    case 'md':
    case 'log':
      return FileText;
    default:
      return File;
  }
}

export function tabIcon(tab: Tab): LucideIcon {
  switch (tab.kind) {
    case 'file':
      return fileIcon(tab.path ?? '');
    case 'settings':
      return Settings;
    case 'toolchain':
      return Wrench;
    case 'schematic':
      return CircuitBoard;
    case 'synthesis':
      return ChartColumn;
    case 'board':
      return Microchip;
    case 'vboard':
      return MonitorPlay;
    case 'report':
      return ScrollText;
    case 'compare':
      return GitCompare;
    case 'processor':
      return Cpu;
    case 'about':
      return Info;
    case 'wave':
      return AudioWaveform;
    case 'learn':
      return GraduationCap;
    case 'assistant':
      return Bot;
    case 'diff':
      return FileDiff;
  }
}
