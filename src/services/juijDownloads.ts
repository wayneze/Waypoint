import { invoke } from '@tauri-apps/api/core';

interface JuijResource { url: string; bytes: number[]; destination?: string }
export interface JuijDownload { label: string; page: string }
export interface JuijDestination { url: string; kind: 'share' | 'external' }
export interface JuijFailure { kind: string; message: string; retryAt?: number }

export class JuijAccessError extends Error {
  constructor(readonly failure: JuijFailure) { super(failure.message); }
}

export function juijFailure(cause: unknown): JuijFailure {
  if (cause instanceof JuijAccessError) return cause.failure;
  return { kind: 'unknown', message: cause instanceof Error ? cause.message : '读取站点失败，请稍后再试。' };
}

async function readResource(url: string): Promise<JuijResource> {
  try { return await invoke<JuijResource>('read_juij_resource', { url }); }
  catch (cause) {
    if (cause && typeof cause === 'object' && 'kind' in cause && 'message' in cause
      && typeof cause.kind === 'string' && typeof cause.message === 'string') {
      const retryAt = 'retryAt' in cause && typeof cause.retryAt === 'number' && Number.isFinite(cause.retryAt) ? cause.retryAt : undefined;
      throw new JuijAccessError({ kind: cause.kind, message: cause.message, retryAt });
    }
    throw new JuijAccessError({ kind: 'unknown', message: '读取站点失败，请稍后再试。' });
  }
}

export async function findJuijDownloads(article: string): Promise<JuijDownload[]> {
  const resource = await readResource(article);
  const document = new DOMParser().parseFromString(new TextDecoder().decode(new Uint8Array(resource.bytes)), 'text/html');
  const links: JuijDownload[] = [];
  for (const anchor of document.querySelectorAll<HTMLAnchorElement>('.article-content a[href]')) {
    let url: URL;
    try { url = new URL(anchor.getAttribute('href')!, resource.url); } catch { continue; }
    if (url.hostname !== 'i.juij.fun' || url.protocol !== 'https:' || url.port || url.username || url.password || !url.searchParams.get('jump')?.startsWith('download')) continue;
    if (links.some(link => link.page === url.href)) continue;
    links.push({ label: anchor.textContent?.trim() || '下载入口', page: url.href });
  }
  if (!links.length) throw new Error('这篇文章没有可识别的下载入口，请查看原文。');
  return links;
}

export async function resolveJuijDownload(page: string): Promise<JuijDestination> {
  const resource = await readResource(page);
  if (resource.destination) {
    let url: URL | undefined;
    try { url = new URL(resource.destination); } catch { /* 下方统一显示未解析 */ }
    if (url && ['http:', 'https:'].includes(url.protocol) && !url.username && !url.password) {
      const share = ['pan.quark.cn', 'pan.baidu.com', 'www.alipan.com', 'www.aliyundrive.com', 'www.123pan.com', 'www.123865.com'].includes(url.hostname);
      return { url: url.href, kind: share ? 'share' : 'external' };
    }
  }
  throw new JuijAccessError({ kind: 'verification', message: '未取得外部下载地址。站点可能需要扫码或浏览器验证，请打开下载页查看提示。' });
}
