// 万象记事一行快录。只映射已有字段，不发明新周期。

export type MemoCycleType = 'once' | 'daily' | 'weekly' | 'interval' | 'deadline';

export interface ParsedMemoLine {
  title: string;
  tag?: string;
  cycle_type: MemoCycleType;
  cycle_val?: string;
  has_progress: boolean;
  target_progress: number;
}

const TOKEN = /(?:^|\s)(#\S+|每日|每周(?:[一二三四五六日]+)?|截止\s+\d{4}-\d{2}-\d{2}|\d+(?:\.\d+)?[dh]|x\d+)(?=\s|$)/gi;
const WEEK_CHAR: Record<string, number> = { 日: 0, 一: 1, 二: 2, 三: 3, 四: 4, 五: 5, 六: 6 };

export function parseMemoLine(raw: string): ParsedMemoLine {
  let tag: string | undefined;
  let cycle: MemoCycleType = 'once';
  let cycleVal: string | undefined;
  let target = 0;
  const stripped = raw.replace(TOKEN, (full, token: string) => {
    const lead = full.startsWith(' ') || full.startsWith('\t') ? full[0] : '';
    const word = token.trim();
    if (word.startsWith('#')) {
      const next = word.slice(1).trim();
      if (next) tag = next;
      return lead;
    }
    if (word === '每日') {
      cycle = 'daily';
      cycleVal = undefined;
      return lead;
    }
    if (word.startsWith('每周')) {
      cycle = 'weekly';
      const days = [...word.slice(2)].map(ch => WEEK_CHAR[ch]).filter(day => day !== undefined);
      cycleVal = days.length ? days.join(',') : undefined;
      return lead;
    }
    if (word.startsWith('截止')) {
      cycle = 'deadline';
      cycleVal = word.replace(/^截止\s*/, '');
      return lead;
    }
    if (/^x\d+$/i.test(word)) {
      const n = parseInt(word.slice(1), 10);
      if (n > 0) target = Math.min(n, 9999);
      return lead;
    }
    if (/^\d+(?:\.\d+)?[dh]$/i.test(word)) {
      cycle = 'interval';
      cycleVal = word.toLowerCase();
      return lead;
    }
    return full;
  });
  return {
    title: stripped.replace(/\s+/g, ' ').trim(),
    tag,
    cycle_type: cycle,
    cycle_val: cycleVal,
    has_progress: target > 0,
    target_progress: target,
  };
}

export function memoLinePreview(parsed: ParsedMemoLine, fallbackTag: string): string {
  const tag = parsed.tag || fallbackTag || '通用';
  const weekDays = (parsed.cycle_val || '').split(',').map(Number).filter(day => day >= 0 && day <= 6);
  const ordered = [...weekDays].sort((a, b) => (a || 7) - (b || 7));
  const weekText = ordered.length
    ? '周' + ordered.map(day => '日一二三四五六'[day]).join('')
    : '每周';
  const cycle = parsed.cycle_type === 'daily'
    ? '每日'
    : parsed.cycle_type === 'weekly'
      ? weekText
      : parsed.cycle_type === 'interval'
        ? `每 ${parsed.cycle_val || ''}`
        : parsed.cycle_type === 'deadline'
          ? `截止 ${parsed.cycle_val || ''}`
          : '一次';
  const count = parsed.has_progress ? ` · 0/${parsed.target_progress}` : '';
  return `${tag} · ${cycle}${count}`;
}
