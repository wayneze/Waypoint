export interface MingyiShotParse {
  ids: string[];
  format: 'lua-array' | 'json-array' | 'id-list';
}

function stripReturn(raw: string): string {
  return raw.replace(/^\uFEFF/, '').trim().replace(/^return\s*/i, '').trim();
}

function parseNumberList(inner: string): string[] | null {
  const trimmed = inner.trim();
  if (!trimmed) return [];
  if (/[^\d,\s]/.test(trimmed)) return null;
  const ids: string[] = [];
  const seen = new Set<string>();
  for (const part of trimmed.split(',')) {
    const token = part.trim();
    if (!token) continue;
    if (!/^\d+$/.test(token)) return null;
    const id = String(Number(token));
    if (seen.has(id)) continue;
    seen.add(id);
    ids.push(id);
  }
  return ids;
}

/**
 * 茗伊成就快照：明文 `return {0,1,2,...}`（已完成 ID 升序数组）。
 * 也接受 JSON 数组或逗号/换行分隔的 ID 列表。不解析编造的 `["achieve"]` 表。
 */
export function parseMingyiAchievementShot(rawInput: string): MingyiShotParse {
  const stripped = stripReturn(rawInput);
  if (!stripped) {
    throw new Error('空内容');
  }
  if (!stripped.startsWith('return') && !stripped.startsWith('{') && !stripped.startsWith('[')) {
    const head = stripped.slice(0, 12);
    if (/[^\x09\x0a\x0d\x20-\x7e]/.test(head) && !/^\d/.test(stripped)) {
      throw new Error('不是明文成就快照（可能已加密）');
    }
  }

  if (stripped.startsWith('{') && stripped.endsWith('}')) {
    const ids = parseNumberList(stripped.slice(1, -1));
    if (ids) return { ids, format: 'lua-array' };
    throw new Error('不是成就 ID 数组。请粘贴 achievement_acquire_shot.jx3dat，格式为 return {0,1,2,...}');
  }

  if (stripped.startsWith('[') && stripped.endsWith(']')) {
    let parsed: unknown;
    try {
      parsed = JSON.parse(stripped);
    } catch {
      throw new Error('不是成就 ID 数组。请粘贴 achievement_acquire_shot.jx3dat，格式为 return {0,1,2,...}');
    }
    if (!Array.isArray(parsed) || parsed.some((n) => typeof n !== 'number' && (typeof n !== 'string' || !/^\d+$/.test(n)))) {
      throw new Error('JSON 数组里需要全是成就 ID');
    }
    const seen = new Set<string>();
    const ids: string[] = [];
    for (const n of parsed) {
      const id = String(Number(n));
      if (seen.has(id)) continue;
      seen.add(id);
      ids.push(id);
    }
    return { ids, format: 'json-array' };
  }

  const ids = parseNumberList(stripped.replace(/\s+/g, ','));
  if (ids && ids.length) return { ids, format: 'id-list' };
  throw new Error('无法识别。茗伊快照是 return {0,1,2,...}');
}
