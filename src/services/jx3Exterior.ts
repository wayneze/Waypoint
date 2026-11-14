export interface Jx3PriceBookEntry {
  name: string;
  alias: string;
  goodsId: string;
  band: string;
  url: string;
}

const LINK = /\[([^\]]+)\]\((https?:\/\/(?:www\.)?jiansanzs\.(?:cn|com)\/goods\/detail\/\?id=([0-9a-fA-F]+))\)/g;
const BAND = /^(?:\d+\+|\d+下|便宜货|限量无价格|免费)$/;

export function parseJx3PriceBook(markdown: string): Jx3PriceBookEntry[] {
  const entries: Jx3PriceBookEntry[] = [];
  const seen = new Set<string>();
  let band = '';
  for (const raw of markdown.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line) continue;
    if (BAND.test(line)) {
      band = line;
      continue;
    }
    for (const hit of line.matchAll(LINK)) {
      const goodsId = hit[3];
      if (seen.has(goodsId)) continue;
      seen.add(goodsId);
      const words = hit[1].replace(/^#+\s*/, '').trim().split(/\s+/).filter(Boolean);
      entries.push({
        name: words[0] || '',
        alias: words.slice(1).join(' '),
        goodsId,
        band,
        url: hit[2],
      });
    }
  }
  return entries;
}

export function compactAppearanceName(name: string): string {
  return name.replace(/[·•.\s]/g, '').toLowerCase();
}

export interface ExteriorAddInput {
  name: string;
  goodsId: string;
}

const GOODS_URL = /(?:https?:\/\/(?:www\.)?jiansanzs\.(?:cn|com)\/goods\/detail\/\?id=)([0-9a-fA-F]{24})/;
const MARKDOWN_GOODS = /\[([^\]]+)\]\((https?:\/\/(?:www\.)?jiansanzs\.(?:cn|com)\/goods\/detail\/\?id=([0-9a-fA-F]{24}))\)/;

export function parseExteriorAddInput(raw: string): ExteriorAddInput {
  const text = raw.trim();
  const markdown = text.match(MARKDOWN_GOODS);
  if (markdown) {
    const words = markdown[1].replace(/^#+\s*/, '').trim().split(/\s+/).filter(Boolean);
    return { name: words[0] || '', goodsId: markdown[3] };
  }
  const url = text.match(GOODS_URL);
  if (url) {
    const name = text.slice(0, url.index ?? 0).trim().split(/\s+/).filter(Boolean)[0] || '';
    return { name, goodsId: url[1] };
  }
  if (/^[0-9a-fA-F]{24}$/.test(text)) return { name: '', goodsId: text };
  return { name: text, goodsId: '' };
}

export interface OwnedExterior {
  key: string;
  kind: string;
  id: number;
  name: string;
  goodsId: string;
  source: 'snapshot' | 'manual';
}

export function isPlaceholderExteriorName(name: string): boolean {
  return /^(body|weapon|hair)#\d+$/.test(name);
}

export function mergeOwnedExteriors(
  snapshot: { kind: string; id: number; name: string }[],
  manual: { name: string; goodsId: string }[],
): OwnedExterior[] {
  const rows: OwnedExterior[] = snapshot
    .filter(item => item.name || item.id > 0)
    .map(item => ({
      key: `snap:${item.kind}:${item.id}:${item.name}`,
      kind: item.kind,
      id: item.id,
      name: item.name || `${item.kind}#${item.id}`,
      goodsId: '',
      source: 'snapshot' as const,
    }));
  const seen = new Set(rows.map(row => compactAppearanceName(row.name)).filter(Boolean));
  for (const item of manual) {
    const name = item.name.trim();
    const compact = compactAppearanceName(name);
    if (!name || seen.has(compact)) continue;
    seen.add(compact);
    rows.push({
      key: `manual:${compact}`,
      kind: 'manual',
      id: 0,
      name,
      goodsId: item.goodsId,
      source: 'manual',
    });
  }
  return rows;
}

export type ExteriorPriceStatus = 'priced' | 'unpriced' | 'failed';

export function exteriorPriceStatus(price: number | null, failedAt: string): ExteriorPriceStatus {
  if (failedAt) return 'failed';
  if (price && price > 0) return 'priced';
  return 'unpriced';
}

export function exteriorPricedTotal(rows: { price: number | null; status: ExteriorPriceStatus }[]): number {
  return rows.reduce((sum, row) => sum + (row.status === 'priced' ? (row.price || 0) : 0), 0);
}

export function formatExteriorPrice(value: number): string {
  return value.toLocaleString('zh-CN');
}

export interface ExteriorSuggest {
  key: string;
  name: string;
  alias: string;
  goodsId: string;
  band: string;
  price: number | null;
  priceSource?: string;
  source: 'book' | 'market';
}

export function suggestFromPriceBook(book: Jx3PriceBookEntry[], query: string, limit = 8): ExteriorSuggest[] {
  const compact = compactAppearanceName(query);
  if (!compact) return [];
  return book
    .filter(entry => {
      const name = compactAppearanceName(entry.name);
      const alias = compactAppearanceName(entry.alias);
      return name.includes(compact) || alias.includes(compact) || entry.name.includes(query.trim());
    })
    .slice(0, limit)
    .map(entry => ({
      key: `book:${entry.goodsId}`,
      name: entry.name,
      alias: entry.alias,
      goodsId: entry.goodsId,
      band: entry.band,
      price: null,
      source: 'book' as const,
    }));
}
