import { test } from 'vitest';
import {
  exteriorPriceStatus,
  exteriorPricedTotal,
  formatExteriorPrice,
  mergeOwnedExteriors,
  parseExteriorAddInput,
  parseJx3PriceBook,
  suggestFromPriceBook,
} from './jx3Exterior.ts';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

test('jx3Exterior', () => {
  const note = `20000+
[### 金发·璨月蝶心 狐金](https://www.jiansanzs.cn/goods/detail/?id=62965f763d0a52386355e4e4)

免费
无风·金发
[红发·鲛人歌 十五红](https://www.jiansanzs.com/goods/detail/?id=66b50d5de02f3112f0261707)
`;

  const book = parseJx3PriceBook(note);
  assert(book.length === 2, `expected 2 priced links, got ${book.length}`);
  assert(book[0].name === '金发·璨月蝶心' && book[0].alias === '狐金' && book[0].band === '20000+', book[0].name);
  assert(book[1].goodsId === '66b50d5de02f3112f0261707' && book[1].band === '免费', book[1].band);
  assert(!book.some(row => row.name === '无风·金发'), 'unlinked line stays out of the price book');

  const priced = { price: 76950, status: exteriorPriceStatus(76950, '') };
  const failed = { price: 12000, status: exteriorPriceStatus(12000, '2026-09-24T00:00:00') };
  const missing = { price: null, status: exteriorPriceStatus(null, '') };
  assert(priced.status === 'priced' && failed.status === 'failed' && missing.status === 'unpriced', 'status split');
  assert(exteriorPricedTotal([priced, failed, missing]) === 76950, String(exteriorPricedTotal([priced, failed, missing])));

  const named = parseExteriorAddInput('金发·璨月蝶心');
  assert(named.name === '金发·璨月蝶心' && named.goodsId === '', 'plain name stays a name');
  const linked = parseExteriorAddInput('[### 金发·璨月蝶心 狐金](https://www.jiansanzs.cn/goods/detail/?id=62965f763d0a52386355e4e4)');
  assert(linked.name === '金发·璨月蝶心' && linked.goodsId === '62965f763d0a52386355e4e4', linked.goodsId);
  const bare = parseExteriorAddInput('https://www.jiansanzs.com/goods/detail/?id=66b50d5de02f3112f0261707');
  assert(bare.goodsId === '66b50d5de02f3112f0261707' && bare.name === '', bare.goodsId);
  const picked = parseExteriorAddInput('金发·璨月蝶心 https://www.jiansanzs.cn/goods/detail/?id=62965f763d0a52386355e4e4');
  assert(picked.name === '金发·璨月蝶心' && picked.goodsId === '62965f763d0a52386355e4e4', picked.name);

  const owned = mergeOwnedExteriors(
    [{ kind: 'hair', id: 1, name: '金发·璨月蝶心' }],
    [{ name: '金发·璨月蝶心', goodsId: '62965f763d0a52386355e4e4' }, { name: '红发·鲛人歌', goodsId: '' }],
  );
  assert(formatExteriorPrice(76950) === '76,950', formatExteriorPrice(76950));
  const hits = suggestFromPriceBook(book, '狐金');
  assert(hits.length === 1 && hits[0].name === '金发·璨月蝶心' && hits[0].goodsId === '62965f763d0a52386355e4e4', hits[0]?.name || 'none');
  assert(suggestFromPriceBook(book, '无风').length === 0, 'unlinked note line is not a suggestion');
  assert(owned.length === 2, `snapshot and manual must not double count, got ${owned.length}`);
  assert(owned[0].source === 'snapshot' && owned[1].source === 'manual' && owned[1].name === '红发·鲛人歌', owned.map(row => row.source).join(','));
});
