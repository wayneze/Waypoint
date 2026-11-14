import { test } from 'vitest';
import { charactersFromVault, normalizeVaultItem, pruneVaultItem, resolveTrackDaily } from './vault';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

function item(raw: Record<string, unknown>) {
  return normalizeVaultItem(raw, () => String(raw.platform || ''));
}

test('vault', () => {
  assert(resolveTrackDaily({}) === true, 'missing field on board');
  assert(resolveTrackDaily({ track_daily: true }) === true, 'explicit on');
  assert(resolveTrackDaily({ track_daily: false }) === false, 'explicit off');

  const nsh = item({
    id: 'nsh1',
    platform: 'nsh',
    account_name: 'player@example.com',
    characters: [{ id: 'nsh1', name: '测试号', sect: '龙吟', server: '挑灯看剑', assets: [] }]
  });
  assert(nsh.characters[0].track_daily === true, 'normalize missing field on');
  assert(charactersFromVault(nsh, 'nsh')[0].is_active === true, 'nsh tracked by default');

  const jx3 = item({
    id: 'jx31',
    platform: 'jx3',
    account_name: 'player@example.com',
    characters: [{ id: 'jx31', name: '测试剑侠', assets: [], track_daily: false, starred: true }]
  });
  assert(jx3.characters[0].track_daily === false, 'explicit off kept');
  assert(charactersFromVault(jx3, 'jx3')[0].is_active === false, 'jx3 explicit off');
  assert(normalizeVaultItem(pruneVaultItem(jx3), () => 'jx3').characters[0].starred === true, 'star survives save and reload');

  const pruned = pruneVaultItem({
    ...nsh,
    characters: [{ ...nsh.characters[0], track_daily: false }]
  });
  assert(pruned.characters[0].track_daily === false, 'prune keeps off');

  const masked = item({
    id: 'acct',
    platform: 'jx3',
    account_name: 'wa***@163.com',
    characters: []
  });
  assert(masked.account_name === 'wa***@163.com', 'masked account stays as stored');

  const locked = item({
    id: 'locked',
    platform: 'sky',
    locked: '这台电脑解不开金库密文',
    character_name: '旧角色名',
    characters: [],
  });
  assert(locked.locked === '这台电脑解不开金库密文', 'decryption error remains visible');
  assert(locked.characters.length === 0, 'locked account does not invent a legacy character');
});
