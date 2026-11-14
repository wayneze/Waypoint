import type { Directive } from 'vue';
import { resolveWearViews } from '../../services/imageViews';

export function seasonIcon(guide: { icon?: string }) {
  return guide.icon || '';
}

// 每个请求拥有自己的回调、超时和取消函数，换图/卸载立即释放名额。
function imageDirective(limit: number, timeout: number, wear = false): Directive<HTMLImageElement, string> {
  interface Request {
    el: HTMLImageElement;
    src: string;
    dispose: () => void;
  }
  const requests = new WeakMap<HTMLImageElement, Request>();
  const queue = new Set<Request>();
  let inflight = 0;
  let observer: IntersectionObserver | undefined;

  function pump() {
    for (const job of queue) {
      if (inflight >= limit) break;
      queue.delete(job);
      const { el, src } = job;
      if (!el.isConnected) { job.dispose(); continue; }
      inflight++;
      let settled = false;
      const finish = () => {
        if (settled) return;
        settled = true;
        window.clearTimeout(timer);
        el.removeEventListener('load', loaded);
        el.removeEventListener('error', failed);
        inflight--;
      };
      const failed = () => {
        finish();
        el.removeAttribute('src');
        el.style.visibility = 'hidden';
        el.dispatchEvent(new Event('wikifail'));
        pump();
      };
      const loaded = () => {
        if (wear && (el.naturalWidth < 48 || el.naturalHeight < 48)) { failed(); return; }
        finish();
        el.style.visibility = '';
        el.dispatchEvent(new CustomEvent('wikiload', {
          detail: wear ? resolveWearViews(src, el.naturalWidth, el.naturalHeight) : undefined,
        }));
        pump();
      };
      const timer = window.setTimeout(failed, timeout);
      job.dispose = () => { finish(); el.removeAttribute('src'); requests.delete(el); pump(); };
      el.addEventListener('load', loaded);
      el.addEventListener('error', failed);
      el.referrerPolicy = 'no-referrer';
      el.src = hotlinkImageSrc(src);
    }
  }

  function enqueue(el: HTMLImageElement, src: string) {
    requests.get(el)?.dispose();
    el.style.visibility = 'hidden';
    if (!src) { el.dispatchEvent(new Event('wikifail')); return; }
    observer ??= new IntersectionObserver(entries => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        const image = entry.target as HTMLImageElement;
        observer!.unobserve(image);
        const job = requests.get(image);
        if (job) queue.add(job);
      }
      pump();
    }, { rootMargin: '200px' });
    const job: Request = { el, src, dispose: () => {
      observer!.unobserve(el);
      queue.delete(job);
      requests.delete(el);
    } };
    requests.set(el, job);
    observer.observe(el);
  }

  return {
    mounted(el, binding) { enqueue(el, binding.value); },
    updated(el, binding) {
      if (binding.value !== binding.oldValue) enqueue(el, binding.value);
    },
    beforeUnmount(el) { requests.get(el)?.dispose(); },
  };
}

// 图片代理最多等待 25 秒，前端不能在代理返回前先隐藏图片。
export const vWikiImg = imageDirective(6, 30000);
export const vWikiWear = imageDirective(2, 30000, true);

/** 书库海报：桌面端走 img 协议的本地盘缓存，离线也能看。 */
export function posterSrc(url: string | undefined): string {
  const u = (url || '').trim();
  if (!u) return '';
  if (!/^https?:\/\//i.test(u)) return u;
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return u;
  return `http://img.localhost/?poster=1&u=${encodeURIComponent(u)}`;
}

const WIKI_HOSTS = ['wiki.biligame.com', 'patchwiki.biligame.com', 'static.wikia.nocookie.net'];

export function hotlinkImageSrc(url: string) {
  const u = (url || '').trim();
  if (!u) return '';
  try {
    const host = new URL(u).hostname.toLowerCase();
    if (host === 'dp.mz997.com' && new URL(u).pathname.startsWith('/exported_icons/')) {
      return '__TAURI_INTERNALS__' in window ? `http://img.localhost/?u=${encodeURIComponent(u)}` : u;
    }
    // 光遇 wiki / Fandom：桌面端存盘缓存，Fandom 还要带来源页；浏览器调试直接加载
    if (WIKI_HOSTS.includes(host)) {
      return '__TAURI_INTERNALS__' in window ? `http://img.localhost/?u=${encodeURIComponent(u)}` : u;
    }
    if (
      host.endsWith('sinaimg.cn')
      || host.endsWith('qpic.cn')
      || host.endsWith('qlogo.cn')
      || host === 'nie.res.netease.com'
      || host.endsWith('.166.net')
    ) {
      return `http://img.localhost/?u=${encodeURIComponent(u)}`;
    }
  } catch {
    return u;
  }
  return u;
}
