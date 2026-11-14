import type { Character, VaultAsset, VaultCharacter, VaultItem } from '../types';
import { gameHasRoster, resolveVaultGameId } from './games';

type LooseVault = Partial<VaultItem> & {
  id?: string;
  platform?: string;
  account_name?: string;
  character_name?: string;
  steamid64?: unknown;
  credential?: unknown;
  has_credential?: unknown;
  assets?: unknown;
  characters?: unknown;
};

export function isSteamVaultPlatform(platform: string): boolean {
  return /steam/i.test(platform || '');
}

export function normalizeSteamid64(raw: unknown): string | undefined {
  const t = asText(raw);
  if (!t) return undefined;
  const fromUrl = t.match(/7656119\d{10}/);
  if (fromUrl) return fromUrl[0];
  const digits = t.replace(/\D/g, '');
  if (digits.length === 17) return digits;
  return t;
}

export function parseIdentity(raw: string): { name: string; sect?: string } {
  const text = (raw || '').trim();
  const m = text.match(/^(.*?)[（(]([^）)]+)[）)]\s*$/);
  if (m) {
    const name = m[1].trim();
    const sect = m[2].trim();
    return { name: name || text, sect: sect || undefined };
  }
  return { name: text };
}

function asText(v: unknown): string {
  return typeof v === 'string' ? v.trim() : '';
}

function normalizeAsset(raw: unknown, fallbackId: string): VaultAsset | null {
  if (!raw || typeof raw !== 'object') return null;
  const o = raw as Record<string, unknown>;
  const name = asText(o.name);
  if (!name) return null;
  return {
    id: asText(o.id) || fallbackId,
    name,
    qty: asText(o.qty) || undefined,
    note: asText(o.note) || undefined
  };
}

function normalizeAssetList(raw: unknown, prefix: string): VaultAsset[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .map((item, i) => normalizeAsset(item, `${prefix}_${i}`))
    .filter((item): item is VaultAsset => !!item);
}

function hasOwn(o: Record<string, unknown>, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(o, key);
}

/** 缺字段时默认挂到日常板。 */
export function resolveTrackDaily(raw: Record<string, unknown>): boolean {
  if (hasOwn(raw, 'track_daily')) return raw.track_daily === true;
  if (hasOwn(raw, 'trackDaily')) return raw.trackDaily === true;
  return true;
}

function normalizeCharacter(raw: unknown, fallbackId: string): VaultCharacter | null {
  if (!raw || typeof raw !== 'object') return null;
  const o = raw as Record<string, unknown>;
  const parsed = parseIdentity(asText(o.name));
  if (!parsed.name) return null;
  const id = asText(o.id) || fallbackId;
  return {
    id,
    name: parsed.name,
    sect: asText(o.sect) || parsed.sect,
    server: asText(o.server) || undefined,
    assets: normalizeAssetList(o.assets, `${id}_a`),
    track_daily: resolveTrackDaily(o),
    starred: o.starred === true,
    query_id: asText(o.query_id) || undefined,
  };
}

export function normalizeVaultItem(raw: LooseVault, resolveGameId: (platform: string) => string): VaultItem {
  const id = asText(raw.id) || `v_${Date.now()}`;
  const platform = asText(raw.platform);
  const account_name = asText(raw.account_name);
  const server = asText(raw.server) || undefined;
  const notes = asText(raw.notes) || undefined;
  const assets = normalizeAssetList(raw.assets, `${id}_aa`);
  const locked = asText(raw.locked) || undefined;

  let characters = Array.isArray(raw.characters)
    ? raw.characters
        .map((item, i) => normalizeCharacter(item, `${id}_c${i}`))
        .filter((item): item is VaultCharacter => !!item)
    : [];

  if (!locked && characters.length === 0) {
    const legacy = asText(raw.character_name);
    if (legacy && gameHasRoster(resolveGameId(platform))) {
      const parsed = parseIdentity(legacy);
      characters = [{
        id,
        name: parsed.name,
        sect: parsed.sect,
        server,
        assets: [],
        track_daily: resolveTrackDaily(raw as Record<string, unknown>)
      }];
    }
  }

  characters = characters.map(ch => ({
    ...ch,
    server: ch.server || server
  }));

  return {
    id,
    platform,
    account_name,
    server,
    notes,
    locked,
    steamid64: isSteamVaultPlatform(platform) ? normalizeSteamid64(raw.steamid64) : undefined,
    assets,
    characters
  };
}

export function charactersFromVault(item: VaultItem, gameId: string): Character[] {
  return (item.characters || []).map(ch => ({
    id: ch.id,
    name: ch.name,
    game_id: gameId,
    server: ch.server || item.server,
    sect: ch.sect,
    is_active: ch.track_daily,
    starred: !!ch.starred,
  }));
}

export function newVaultAsset(): VaultAsset {
  return { id: `a_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`, name: '' };
}

export function newVaultCharacter(): VaultCharacter {
  return {
    id: `c_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
    name: '',
    sect: '',
    server: '',
    assets: [],
    track_daily: true,
    query_id: '',
  };
}

export function formatSteamPlaytime(minutes: number): string {
  const m = Math.max(0, Math.floor(minutes || 0));
  if (m < 60) return `${m} 分钟`;
  const h = Math.floor(m / 60);
  const rest = m % 60;
  if (h >= 100 || rest === 0) return `${h} 小时`;
  return `${h} 小时 ${rest} 分`;
}

export function pruneVaultItem(item: VaultItem): VaultItem {
  const steamid64 = isSteamVaultPlatform(item.platform)
    ? normalizeSteamid64(item.steamid64)
    : undefined;
  const skyRoster = resolveVaultGameId(item.platform) === 'sky';
  return {
    ...item,
    account_name: item.account_name.trim(),
    server: item.server?.trim() || undefined,
    notes: item.notes?.trim() || undefined,
    steamid64,
    assets: (item.assets || []).filter(a => a.name.trim()).map(a => ({
      ...a,
      name: a.name.trim(),
      qty: a.qty?.trim() || undefined,
      note: a.note?.trim() || undefined
    })),
    characters: (item.characters || [])
      .filter(ch => ch.name.trim())
      .map(ch => ({
        ...ch,
        name: ch.name.trim(),
        sect: ch.sect?.trim() || undefined,
        server: ch.server?.trim() || item.server?.trim() || undefined,
        track_daily: ch.track_daily !== false,
        query_id: skyRoster ? (ch.query_id?.trim() || undefined) : undefined,
        starred: ch.starred || undefined,
        assets: (ch.assets || []).filter(a => a.name.trim()).map(a => ({
          ...a,
          name: a.name.trim(),
          qty: a.qty?.trim() || undefined,
          note: a.note?.trim() || undefined
        }))
      }))
  };
}
