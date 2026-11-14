import { invoke } from '@tauri-apps/api/core';
import { isTauri } from './api';
import { kvGetJson, kvSetConfirmed } from './kv';
import { completeSkyDailySelection, type SkyDailyGuidePayload, type SkyDailySelection, type SkyEventDaily } from './skyDailyService';
import type { SkyBoostAssets } from './skyInventory';
import { progressGet, setProgressEntries } from './progress';

export interface SkyBoostBinding {
  link: string;
  included: SkyDailySelection;
  autoSync: boolean;
}
export interface SkyBoostFeedback {
  day: string;
  fetched_at: string;
  status: string;
  completed: boolean;
  order_state: string;
  assets?: SkyBoostAssets;
}
export interface SkyBoostCompletion {
  link: string;
  included: SkyDailySelection;
  feedback: Omit<SkyBoostFeedback, 'assets'>;
  message: string;
}
export const SKY_BOOST_OPTIONS = [
  { key: 'quests', label: '每日四任务' },
  { key: 'candles', label: '季节蜡烛' },
  { key: 'event', label: '活动日常' },
] as const;

const bindingKey = (id: string) => `waypoint_sky_boost_${id}`;
export function loadSkyBoostBinding(characterId: string): SkyBoostBinding {
  return kvGetJson(bindingKey(characterId), {
    link: '', included: { quests: true, candles: true, event: false }, autoSync: true,
  });
}

export function validateSkyBoostBinding(binding: SkyBoostBinding): SkyBoostBinding {
  const link = binding.link.trim();
  if (link) {
    let url: URL;
    try { url = new URL(link); } catch { throw new Error('请填写完整的反馈链接'); }
    if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) {
      throw new Error('反馈链接只支持 HTTP 或 HTTPS 页面');
    }
    if (!Object.values(binding.included).some(Boolean)) throw new Error('至少选择一项订单包含的日常');
  }
  return { link, autoSync: !!binding.autoSync && !!link, included: { ...binding.included } };
}

export function saveSkyBoostBinding(characterId: string, binding: SkyBoostBinding) {
  if (!characterId) throw new Error('请先选择光遇角色');
  return kvSetConfirmed(bindingKey(characterId), JSON.stringify(validateSkyBoostBinding(binding)));
}

export function supportsSkyBoostLink(link: string): boolean {
  try {
    const url = new URL(link);
    return ['http:', 'https:'].includes(url.protocol) && url.hostname === 'dp.mz997.com' && url.port === '92'
      && !url.username && !url.password && url.hash.startsWith('#/order-search?');
  } catch { return false; }
}

export async function fetchSkyBoostFeedback(link: string): Promise<SkyBoostFeedback> {
  if (!isTauri()) throw new Error('自动读取反馈需在 Waypoint 桌面版使用；可先打开原页查看');
  return invoke<SkyBoostFeedback>('fetch_sky_boost_feedback', { link });
}

export function skyBoostDay(now = new Date()): string {
  return new Intl.DateTimeFormat('en-CA', { timeZone: 'Asia/Shanghai', year: 'numeric', month: '2-digit', day: '2-digit' }).format(now);
}

/** 完成记录按角色、订单、套餐和国服日期隔离；重开应用后仍可停止当天轮询。 */
export function loadSkyBoostCompletion(characterId: string, binding: SkyBoostBinding, day = skyBoostDay()) {
  const record = progressGet<SkyBoostCompletion>('sky.boost_sync', 'completed', characterId);
  return record?.feedback.day === day && record.link === binding.link
    && SKY_BOOST_OPTIONS.every(({ key }) => record.included[key] === binding.included[key])
    ? record : undefined;
}

export function saveSkyBoostCompletion(characterId: string, binding: SkyBoostBinding, feedback: SkyBoostFeedback, message: string) {
  const { assets: _assets, ...status } = feedback;
  return setProgressEntries('sky.boost_sync', [['completed', {
    link: binding.link, included: binding.included, feedback: status, message,
  } satisfies SkyBoostCompletion]], characterId);
}

/** 仅接受本次查询的当天明确完成状态；范围由用户保存的订单服务项目决定。 */
export async function applySkyBoostFeedback(
  payload: SkyDailyGuidePayload, characterId: string, binding: SkyBoostBinding,
  feedback: SkyBoostFeedback, now = new Date(),
  events?: SkyEventDaily[],
): Promise<boolean> {
  const day = skyBoostDay(now);
  const age = now.getTime() - Date.parse(feedback.fetched_at);
  if (!feedback.completed || feedback.order_state !== '正常') return false;
  if (feedback.day !== day || payload.dateStr !== day || !Number.isFinite(age) || age < -60_000 || age > 120_000) {
    throw new Error('反馈或日常日期不匹配，请刷新后重试');
  }
  if (payload.isFallback
    || (binding.included.quests && payload.quests.length !== 4)
    || (binding.included.candles && payload.seasonalCandles.spots.length !== 4)) {
    throw new Error('今日日常条目尚未完整加载，未自动勾选');
  }
  await completeSkyDailySelection(payload, characterId, binding.included, events);
  return true;
}
