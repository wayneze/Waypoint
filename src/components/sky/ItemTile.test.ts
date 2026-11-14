import { expect, test, vi } from 'vitest';
import { createSSRApp } from 'vue';
import { renderToString } from 'vue/server-renderer';
vi.mock('../../workspace/guides/lightbox', () => ({ openLightbox: vi.fn() }));
vi.mock('./wikiImage', () => ({ vWikiImg: {}, vWikiWear: {}, hotlinkImageSrc: (url: string) => url }));
import ItemTile from './ItemTile.vue';

test('仅预览的图片没有无效心愿按钮，接入收藏的图片提供真实心愿状态', async () => {
  const preview = await renderToString(createSSRApp(ItemTile, { name: '斗篷' }));
  expect(preview).not.toContain('想要');
  const available = await renderToString(createSSRApp(ItemTile, { name: '斗篷', wanted: false }));
  expect(available).toContain('aria-pressed="false"');
  expect(available).toContain('想要');
  const wanted = await renderToString(createSSRApp(ItemTile, { name: '斗篷', wanted: true }));
  expect(wanted).toContain('aria-pressed="true"');
  expect(wanted).toContain('已加入想要');
});

test('图标模式不请求实穿，实穿模式保持统一媒体框和可用详情入口', async () => {
  const props = { name: '斗篷', icon: 'https://example.com/icon.png', wear: 'https://example.com/wear.png', inspect: true };
  const icon = await renderToString(createSSRApp(ItemTile, { ...props, mode: 'icon' }));
  expect(icon).not.toContain('wp-tile-frame');
  expect(icon).toContain('is-icon-mode');
  const wear = await renderToString(createSSRApp(ItemTile, { ...props, mode: 'wear' }));
  expect(wear).toContain('wp-tile-frame');
  expect(wear).toContain('查看外观详情：斗篷');
});
