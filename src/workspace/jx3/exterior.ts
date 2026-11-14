// 剑网3已收集外观，对剑三交易助手当前成交价。
import { computed, ref } from 'vue';
import type { Jx3ExteriorScan, Jx3ExteriorSnapshot, Jx3GoodsQuote } from '../../types';
import {
  fetchJx3GoodsByName,
  fetchJx3GoodsQuote,
  scanJx3Exterior,
  searchJx3Goods,
} from '../../services/jx3Api';
import { readObsidianFile, scanObsidianVault } from '../../services/systemApi';
import {
  compactAppearanceName,
  exteriorPriceStatus,
  exteriorPricedTotal,
  formatExteriorPrice,
  isPlaceholderExteriorName,
  mergeOwnedExteriors,
  parseExteriorAddInput,
  parseJx3PriceBook,
  suggestFromPriceBook,
  type ExteriorSuggest,
  type Jx3PriceBookEntry,
  type OwnedExterior,
} from '../../services/jx3Exterior';
import { kvGetJson, kvSet } from '../../services/kv';
import { cacheGet, cacheSet } from '../../services/fetchCache';
import { activeCharacter, characterList, obsidianVaultPath, showToast } from '../core';

// 报价是网络数据，放 cache.db；手动添加的外观是用户数据，留在设置库。
const QUOTE_KEY = 'jx3.exterior_quotes';
const MANUAL_KEY = 'waypoint_jx3_exterior_manual_v1';

export interface ExteriorRow {
  key: string;
  name: string;
  alias: string;
  band: string;
  price: number | null;
  priceSource: string;
  url: string;
  status: 'priced' | 'unpriced' | 'failed';
  failedAt: string;
  source: 'snapshot' | 'manual';
  imageUrl: string;
  iconUrl: string;
}

export interface ManualExterior {
  name: string;
  goodsId: string;
}

export const jx3ExteriorScan = ref<Jx3ExteriorScan | null>(null);
export const jx3ExteriorBusy = ref(false);
export const jx3ExteriorError = ref('');
export const jx3ExteriorAddText = ref('');
export const jx3ExteriorSuggests = ref<ExteriorSuggest[]>([]);
export const jx3ExteriorSuggestIndex = ref(0);
const priceBook = ref<Jx3PriceBookEntry[]>([]);
let suggestToken = 0;
const quotes = ref<Record<string, Jx3GoodsQuote>>(cacheGet<Record<string, Jx3GoodsQuote>>(QUOTE_KEY)?.data ?? {});
const manualByCharacter = ref<Record<string, ManualExterior[]>>(kvGetJson(MANUAL_KEY, {}));

export function reloadManualFromKv() {
  quotes.value = cacheGet<Record<string, Jx3GoodsQuote>>(QUOTE_KEY)?.data ?? {};
  manualByCharacter.value = kvGetJson(MANUAL_KEY, {});
}

function saveQuotes() {
  cacheSet(QUOTE_KEY, quotes.value);
}

function saveManual() {
  kvSet(MANUAL_KEY, JSON.stringify(manualByCharacter.value));
}

function manualForActive(): ManualExterior[] {
  const id = activeCharacter.value;
  if (!id) return [];
  return manualByCharacter.value[id] || [];
}

function activeRole() {
  return characterList.value.find(row => row.id === activeCharacter.value) || null;
}

function activeRoleName(): string {
  return activeRole()?.name || '';
}

function characterIdForRole(role: string, server: string): string {
  const named = characterList.value.filter(row => row.game_id === 'jx3' && row.name === role);
  const exact = named.find(row => row.server && server && row.server === server);
  if (exact) return exact.id;
  if (named.length === 1) return named[0].id;
  return '';
}

function sameServer(shotServer: string, characterServer: string): boolean {
  const left = shotServer.trim();
  const right = characterServer.trim();
  if (!left || !right) return true;
  return left === right;
}

function snapshotForActive(): Jx3ExteriorSnapshot | null {
  const role = activeRole();
  const scan = jx3ExteriorScan.value;
  if (!role?.name || !scan) return null;
  const named = scan.snapshots.filter(shot => shot.role === role.name && sameServer(shot.server, role.server || ''));
  if (named.length === 1) return named[0];
  if (named.length > 1) {
    const exact = named.find(shot => shot.server && role.server && shot.server === role.server);
    return exact || named[0];
  }
  return null;
}

const ownedSnapshot = computed(() => snapshotForActive());

const ownedItems = computed(() => mergeOwnedExteriors(
  ownedSnapshot.value?.items || [],
  manualForActive(),
));

function quoteOf(name: string): Jx3GoodsQuote | undefined {
  return quotes.value[name] || quotes.value[compactAppearanceName(name)];
}

function rowFromOwned(item: OwnedExterior): ExteriorRow {
  const quote = quoteOf(item.name);
  const price = quote?.price && quote.price > 0 ? quote.price : null;
  const failedAt = quote?.failed_at || '';
  return {
    key: item.key,
    name: item.name,
    alias: quote?.alias || '',
    band: quote?.note_band || '',
    price,
    priceSource: price ? quote?.price_source || '' : '',
    url: quote?.url || '',
    status: exteriorPriceStatus(price, failedAt),
    failedAt,
    source: item.source,
    imageUrl: quote?.image_url || '',
    iconUrl: quote?.icon_url || '',
  };
}

export const jx3ExteriorRows = computed<ExteriorRow[]>(() =>
  ownedItems.value
    .map(rowFromOwned)
    .sort((a, b) => (b.price || 0) - (a.price || 0) || a.name.localeCompare(b.name, 'zh'))
);

export const jx3ExteriorTotal = computed(() => exteriorPricedTotal(jx3ExteriorRows.value));
export const jx3ExteriorTotalLabel = computed(() => formatExteriorPrice(jx3ExteriorTotal.value));
export const jx3ExteriorPricedCount = computed(() =>
  jx3ExteriorRows.value.filter(row => row.status === 'priced').length
);

export const jx3ExteriorUnpriced = computed(() =>
  jx3ExteriorRows.value.filter(row => row.status === 'unpriced').length
);

export const jx3ExteriorFailed = computed(() =>
  jx3ExteriorRows.value.filter(row => row.status === 'failed').length
);

export const jx3ExteriorMissingSnapshot = computed(() =>
  !!activeRoleName() && !!jx3ExteriorScan.value && !ownedSnapshot.value
);

async function loadPriceBook(): Promise<Jx3PriceBookEntry[]> {
  if (priceBook.value.length) return priceBook.value;
  const vault = obsidianVaultPath.value.trim();
  if (!vault) return [];
  const files = await scanObsidianVault(vault);
  const notes = files.filter(file => /外观\.md$/i.test(file.name) && file.relative_path.includes('剑网3'));
  const books = await Promise.all(notes.map(file => readObsidianFile(file.full_path)));
  priceBook.value = books.flatMap(parseJx3PriceBook);
  return priceBook.value;
}

export function moveJx3ExteriorSuggest(delta: number) {
  const total = jx3ExteriorSuggests.value.length;
  if (!total) return;
  jx3ExteriorSuggestIndex.value = (jx3ExteriorSuggestIndex.value + delta + total) % total;
}

export async function refreshJx3ExteriorSuggests(raw = jx3ExteriorAddText.value) {
  const query = raw.trim();
  const token = ++suggestToken;
  if (query.length < 1 || /jiansanzs\./.test(query) || /^[0-9a-fA-F]{24}$/.test(query)) {
    jx3ExteriorSuggests.value = [];
    return;
  }
  const book = await loadPriceBook().catch(() => priceBook.value);
  if (token !== suggestToken) return;
  const local = suggestFromPriceBook(book, query);
  jx3ExteriorSuggests.value = local;
  jx3ExteriorSuggestIndex.value = 0;
  if (query.length < 2) return;
  try {
    const remote = await searchJx3Goods(query);
    if (token !== suggestToken) return;
    const seen = new Set(local.map(row => row.goodsId));
    const extra = remote
      .filter(row => row.id && !seen.has(row.id))
      .map(row => ({
        key: `market:${row.id}`,
        name: row.name,
        alias: row.alias,
        goodsId: row.id,
        band: '',
        price: row.price ?? null,
        priceSource: row.price_source || '',
        source: 'market' as const,
      }));
    jx3ExteriorSuggests.value = [...local, ...extra].slice(0, 8);
  } catch {
    if (token === suggestToken && !local.length) jx3ExteriorSuggests.value = [];
  }
}

function bookHit(book: Jx3PriceBookEntry[], name: string): Jx3PriceBookEntry | undefined {
  const compact = compactAppearanceName(name);
  return book.find(entry =>
    entry.name === name
    || compactAppearanceName(entry.name) === compact
    || entry.alias === name
    || compactAppearanceName(entry.alias) === compact
  );
}

function stampFailure(name: string, hit: Jx3PriceBookEntry | undefined, goodsId: string, message: string) {
  const previous = quotes.value[name];
  quotes.value[name] = {
    id: previous?.id || goodsId || hit?.goodsId || '',
    name: previous?.name || name,
    alias: previous?.alias || hit?.alias || '',
    price: previous?.price,
    note_band: hit?.band || previous?.note_band || '',
    url: previous?.url || hit?.url || (goodsId ? `https://www.jiansanzs.cn/goods/detail/?id=${goodsId}` : ''),
    updated: previous?.updated || '',
    failed_at: new Date().toISOString(),
    image_url: previous?.image_url,
    icon_url: previous?.icon_url,
  };
  jx3ExteriorError.value = message;
}

async function priceOwned(items: OwnedExterior[], book: Jx3PriceBookEntry[]): Promise<number> {
  let failed = 0;
  for (const item of items) {
    if (isPlaceholderExteriorName(item.name)) {
      quotes.value[item.name] = {
        id: '',
        name: item.name,
        alias: '',
        note_band: '',
        url: '',
        updated: '',
        failed_at: '',
      };
      continue;
    }
    const hit = bookHit(book, item.name);
    const goodsId = item.goodsId || hit?.goodsId || '';
    try {
      const quote = goodsId
        ? await fetchJx3GoodsQuote(goodsId)
        : await fetchJx3GoodsByName(item.name);
      quote.note_band = hit?.band || '';
      if (!quote.alias && hit?.alias) quote.alias = hit.alias;
      quote.failed_at = '';
      // 只按清单里的名字记。按商品正式名再记一份，会让「狐金」和「金发·璨月蝶心」两行互相覆盖
      quotes.value[item.name] = quote;
      quotes.value[compactAppearanceName(item.name)] = quote;
    } catch (err) {
      failed += 1;
      stampFailure(item.name, hit, goodsId, err instanceof Error ? err.message : String(err));
    }
  }
  saveQuotes();
  return failed;
}

export async function pickJx3ExteriorSuggest(hit = jx3ExteriorSuggests.value[jx3ExteriorSuggestIndex.value]) {
  if (!hit) return addJx3ExteriorManual();
  jx3ExteriorSuggests.value = [];
  await addJx3ExteriorManual(`${hit.name} https://www.jiansanzs.cn/goods/detail/?id=${hit.goodsId}`);
}

export async function addJx3ExteriorManual(raw = jx3ExteriorAddText.value) {
  const characterId = activeCharacter.value;
  if (!characterId) {
    showToast('先选一个剑网3角色', '手记挂在当前角色上');
    return;
  }
  const parsed = parseExteriorAddInput(raw);
  if (!parsed.name && !parsed.goodsId) return;
  jx3ExteriorBusy.value = true;
  jx3ExteriorError.value = '';
  try {
    let name = parsed.name;
    let goodsId = parsed.goodsId;
    if (!name && goodsId) {
      const quote = await fetchJx3GoodsQuote(goodsId);
      name = quote.name || goodsId;
      goodsId = quote.id || goodsId;
    }
    const existing = manualByCharacter.value[characterId] || [];
    const compact = compactAppearanceName(name);
    const alreadyOwned = ownedItems.value.some(row => compactAppearanceName(row.name) === compact);
    const stored = existing.find(row => compactAppearanceName(row.name) === compact);
    if (!alreadyOwned) {
      manualByCharacter.value[characterId] = [...existing, { name, goodsId }];
      saveManual();
    } else if (stored && goodsId && stored.goodsId !== goodsId) {
      stored.goodsId = goodsId;
      saveManual();
    }
    const book = await loadPriceBook();
    const failed = await priceOwned(
      ownedItems.value.filter(row => compactAppearanceName(row.name) === compact),
      book,
    );
    jx3ExteriorAddText.value = '';
    jx3ExteriorSuggests.value = [];
    if (alreadyOwned && !goodsId) showToast('这件已经在清单里', name);
    else if (failed) showToast('已记下，价没取到', name);
    else if (alreadyOwned) showToast('已按这条链接对价', name);
    else showToast('已加入外观', name);
  } catch (err) {
    jx3ExteriorError.value = err instanceof Error ? err.message : String(err);
    showToast('这件没加上', jx3ExteriorError.value);
  } finally {
    jx3ExteriorBusy.value = false;
  }
}

export function removeJx3ExteriorManual(name: string) {
  const characterId = activeCharacter.value;
  if (!characterId) return;
  const compact = compactAppearanceName(name);
  manualByCharacter.value[characterId] = (manualByCharacter.value[characterId] || [])
    .filter(row => compactAppearanceName(row.name) !== compact);
  saveManual();
}

export async function refreshJx3Exterior() {
  if (jx3ExteriorBusy.value) return;
  jx3ExteriorBusy.value = true;
  jx3ExteriorError.value = '';
  try {
    jx3ExteriorScan.value = await scanJx3Exterior(null);
    let droppedManual = false;
    for (const shot of jx3ExteriorScan.value.snapshots) {
      const ownerId = characterIdForRole(shot.role, shot.server);
      if (!ownerId) continue;
      const shotNames = new Set(shot.items.map(item => compactAppearanceName(item.name)).filter(Boolean));
      const current = manualByCharacter.value[ownerId] || [];
      const next = current.filter(row => !shotNames.has(compactAppearanceName(row.name)));
      if (next.length !== current.length) {
        manualByCharacter.value[ownerId] = next;
        droppedManual = true;
      }
    }
    if (droppedManual) saveManual();
    const book = await loadPriceBook();
    const items = ownedItems.value;
    if (!items.length) return;
    const failed = await priceOwned(items, book);
    if (failed) showToast('外观价未全部取到', `${failed} 件保留上次结果`);
  } catch (err) {
    jx3ExteriorError.value = err instanceof Error ? err.message : String(err);
    showToast('外观快照没读到', jx3ExteriorError.value);
  } finally {
    jx3ExteriorBusy.value = false;
  }
}
