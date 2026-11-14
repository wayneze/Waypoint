// 万象记事备注：步骤行可打钩，http 链接可点开。其余仍是纯文本。

export type MemoNotePart =
  | { kind: 'text'; text: string }
  | { kind: 'link'; text: string; url: string }
  | { kind: 'step'; index: number; done: boolean; text: string };

const STEP = /^(\s*)-\s+\[([ xX])\]\s?(.*)$/;
const URL_RE = /https?:\/\/[^\s<>]+/g;

export function noteParts(note: string): MemoNotePart[] {
  const parts: MemoNotePart[] = [];
  let step = 0;
  for (const line of note.split('\n')) {
    const matched = line.match(STEP);
    if (matched) {
      parts.push({ kind: 'step', index: step, done: matched[2].toLowerCase() === 'x', text: matched[3] });
      step += 1;
      continue;
    }
    let cursor = 0;
    for (const hit of line.matchAll(URL_RE)) {
      const start = hit.index ?? 0;
      if (start > cursor) parts.push({ kind: 'text', text: line.slice(cursor, start) });
      let url = hit[0];
      let tail = '';
      while (/[)\].,;:!?，。、）】]$/.test(url)) {
        tail = url.slice(-1) + tail;
        url = url.slice(0, -1);
      }
      parts.push({ kind: 'link', text: url, url });
      if (tail) parts.push({ kind: 'text', text: tail });
      cursor = start + hit[0].length;
    }
    if (cursor < line.length) parts.push({ kind: 'text', text: line.slice(cursor) });
    parts.push({ kind: 'text', text: '\n' });
  }
  if (parts.length && parts[parts.length - 1].kind === 'text' && parts[parts.length - 1].text === '\n') {
    parts.pop();
  }
  return parts;
}

export function toggleNoteStep(note: string, index: number): string {
  let step = 0;
  return note.split('\n').map(line => {
    const matched = line.match(STEP);
    if (!matched) return line;
    const current = step;
    step += 1;
    if (current !== index) return line;
    const mark = matched[2].toLowerCase() === 'x' ? ' ' : 'x';
    return `${matched[1]}- [${mark}] ${matched[3]}`;
  }).join('\n');
}

export function noteStepProgress(note: string): { done: number; total: number } {
  const steps = noteParts(note).filter(part => part.kind === 'step');
  return { done: steps.filter(part => part.done).length, total: steps.length };
}

export const WEEKDAY_LABELS = ['日', '一', '二', '三', '四', '五', '六'] as const;

export function parseWeekdays(cycleVal?: string): number[] {
  if (!cycleVal) return [];
  return [...new Set(
    cycleVal.split(',').map(part => Number(part.trim())).filter(day => day >= 0 && day <= 6)
  )].sort((a, b) => a - b);
}

export function formatWeekdays(days: number[]): string {
  return [...new Set(days.filter(day => day >= 0 && day <= 6))].sort((a, b) => a - b).join(',');
}

export function weekdayLabel(days: number[]): string {
  if (!days.length || days.length === 7) return '每周';
  const ordered = [...days].sort((a, b) => (a || 7) - (b || 7));
  return '周' + ordered.map(day => WEEKDAY_LABELS[day]).join('');
}

export function isWeekdayActive(cycleVal: string | undefined, now = new Date()): boolean {
  const days = parseWeekdays(cycleVal);
  if (!days.length) return true;
  return days.includes(now.getDay());
}
