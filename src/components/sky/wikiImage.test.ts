import { afterEach, expect, test, vi } from 'vitest';
import type { ObjectDirective } from 'vue';

class ImageStub extends EventTarget {
  style = { visibility: '' };
  src = '';
  isConnected = true;
  naturalWidth = 1280;
  naturalHeight = 256;
  removeAttribute(name: string) { if (name === 'src') this.src = ''; }
}

afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); });

test('本地图标保持原路径，只有网络海报进入桌面缓存代理', async () => {
  vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });
  const { posterSrc } = await import('./wikiImage');
  expect(posterSrc('/jx3/art/school-3.webp')).toBe('/jx3/art/school-3.webp');
  expect(posterSrc('data:image/png;base64,AA==')).toBe('data:image/png;base64,AA==');
  expect(posterSrc('https://example.org/poster.jpg')).toBe('http://img.localhost/?poster=1&u=https%3A%2F%2Fexample.org%2Fposter.jpg');
});

test('visible image requests release slots on removal, source change, failure and timeout', async () => {
  vi.resetModules();
  vi.useFakeTimers();
  vi.stubGlobal('window', { setTimeout, clearTimeout });
  const reveal = new Map<Element, () => void>();
  vi.stubGlobal('IntersectionObserver', class {
    constructor(private callback: (entries: { target: Element; isIntersecting: boolean }[]) => void) {}
    observe(el: Element) { reveal.set(el, () => this.callback([{ target: el, isIntersecting: true }])); }
    unobserve(el: Element) { reveal.delete(el); }
  });
  const { vWikiWear } = await import('./wikiImage');
  const directive = vWikiWear as ObjectDirective<HTMLImageElement, string>;
  const run = (hook: 'mounted' | 'updated' | 'beforeUnmount', el: ImageStub, value = '', oldValue = '') =>
    (directive[hook] as Function)(el, { value, oldValue });
  const url = (i: number) => `https://patchwiki.biligame.com/images/sky/${i}.png`;
  const images = [new ImageStub(), new ImageStub(), new ImageStub()];
  for (const [i, image] of images.entries()) run('mounted', image, url(i));
  expect(images.every(image => !image.src)).toBe(true);
  for (const image of images) reveal.get(image as unknown as Element)!();
  expect(images.map(image => !!image.src)).toEqual([true, true, false]);
  run('beforeUnmount', images[0]);
  expect(images[0].src).toBe('');
  expect(images[2].src).toBe(url(2));

  let loaded: CustomEvent | undefined;
  images[1].addEventListener('wikiload', event => { loaded = event as CustomEvent; });
  run('updated', images[1], url(3), url(1));
  expect(images[1].src).toBe('');
  images[1].dispatchEvent(new Event('load')); // 取消的旧请求没有回调。
  expect(loaded).toBeUndefined();
  reveal.get(images[1] as unknown as Element)!();
  images[1].dispatchEvent(new Event('load'));
  expect(loaded!.detail.map((v: { title: string }) => v.title)).toEqual(['正面', '视角 2', '视角 3', '视角 4']);

  const failed = vi.fn();
  images[2].addEventListener('wikifail', failed);
  vi.advanceTimersByTime(25000);
  expect(failed).not.toHaveBeenCalled();
  vi.advanceTimersByTime(5000);
  expect(failed).toHaveBeenCalledTimes(1);
  expect(images[2].src).toBe('');
  images[2].dispatchEvent(new Event('load'));
  expect(failed).toHaveBeenCalledTimes(1);
  run('updated', images[2], url(4), url(2));
  reveal.get(images[2] as unknown as Element)!();
  images[2].dispatchEvent(new Event('error'));
  expect(failed).toHaveBeenCalledTimes(2);
  expect(images[2].src).toBe('');
  expect(vi.getTimerCount()).toBe(0);
});
