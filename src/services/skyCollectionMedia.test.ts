import { beforeEach, expect, test, vi } from 'vitest';
import type { SkyCollectionEntry } from './skyCollection';

const { invoke, cacheGet, cacheSet } = vi.hoisted(() => ({
  invoke: vi.fn(), cacheGet: vi.fn(), cacheSet: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('./fetchCache', () => ({ cacheGet, cacheSet }));

beforeEach(() => {
  vi.resetModules();
  vi.clearAllMocks();
  vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });
});

test('partial media resolution does not repeat automatically and explicit retry can recover', async () => {
  const entries = [
    { icon: 'known.png', wear: '', kind: 'hair' },
    { icon: 'pending.png', wear: '', kind: 'body' },
  ] as SkyCollectionEntry[];
  invoke.mockResolvedValueOnce({ images: { 'known.png': 'hair-photo.png' }, checked: ['known.png'], unavailable: 1 });
  const { ensureCollectionMedia, collectionWear, collectionMediaLoading, collectionMediaError } = await import('./skyCollectionMedia');
  await ensureCollectionMedia(entries);
  // 卡片收到部分图片后会重新计算可见列表，不能因此无限重试失败的来源。
  await ensureCollectionMedia(entries.map(e => ({ ...e, wear: collectionWear(e) })));
  expect(invoke).toHaveBeenCalledTimes(1);
  expect(collectionMediaLoading.value).toBe(false);
  expect(collectionMediaError.value).toBeTruthy();
  expect(cacheSet.mock.lastCall?.[1].checked['pending.png']).toBeUndefined();
  invoke.mockResolvedValueOnce({ images: { 'pending.png': 'outfit-photo.png' }, checked: ['pending.png'], unavailable: 0 });
  await ensureCollectionMedia(entries, true);
  expect(invoke.mock.lastCall?.[1]).toEqual({ items: [{ icon: 'pending.png', kind: 'body' }], retry: true });
  expect(collectionWear(entries[1]!)).toBe('outfit-photo.png');
  expect(collectionMediaError.value).toBe('');

  const saved = cacheSet.mock.lastCall?.[1];
  vi.resetModules();
  cacheGet.mockReturnValue({ data: saved });
  const reopened = await import('./skyCollectionMedia');
  await reopened.ensureCollectionMedia(entries);
  expect(reopened.collectionWear(entries[0]!)).toBe('hair-photo.png');
  expect(reopened.collectionWear(entries[1]!)).toBe('outfit-photo.png');
  expect(invoke).toHaveBeenCalledTimes(2);
});
