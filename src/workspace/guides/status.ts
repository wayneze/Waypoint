// 单机 / Galgame 游玩状态。
import { ref } from 'vue';
import { type ObsidianGameGuide } from '../../types';
import { updateGuideFrontmatter } from '../../services/systemApi';
import { progressMap, progressSet } from '../../services/progress';
import { showToast } from '../core';

export const customGameStatusMap = ref<Record<string, string>>({});

const STATUS_SCOPE = 'guide.status';

export function loadCustomGameStatus() {
  customGameStatusMap.value = progressMap<string>(STATUS_SCOPE);
}

export function getEffectiveGameStatus(guide: ObsidianGameGuide | null | undefined): string {
  if (!guide) return '未开始';
  return (guide.record_id && customGameStatusMap.value[guide.record_id])
    || customGameStatusMap.value[guide.id] || guide.status || '未开始';
}

export async function setGameStatus(guide: ObsidianGameGuide, newStatus: string) {
  if (!guide) return;
  const statusId = guide.record_id || guide.id;
  customGameStatusMap.value[statusId] = newStatus;
  guide.status = newStatus;

  progressSet(STATUS_SCOPE, statusId, newStatus);

  if (guide.file_path) {
    updateGuideFrontmatter(guide.file_path, newStatus, undefined);
  }

  showToast('状态已更新', `已将《${guide.title}》标记为【${newStatus}】`);
}

// 游戏状态胶囊视觉配置表：发光呼吸圆点 + 现代半透微光底色，彻底杜绝粗糙 Emoji 与浏览器原生杂乱样式
export function getGameStatusConfig(statusStr: string) {
  switch (statusStr) {
    case '已通关':
      return {
        label: '已通关',
        stamp: 'done',
        dotClass: 'bg-done',
        textClass: 'text-done',
        bgClass: 'bg-done/10 border-done/35 hover:bg-done/15',
        badgeBg: 'bg-done/10 text-done border-done/35'
      };
    case '在玩':
    case '游玩中':
      return {
        label: '在玩中',
        stamp: 'beacon',
        dotClass: 'bg-beacon',
        textClass: 'text-beacon',
        bgClass: 'bg-beacon/15 border-beacon/35 hover:bg-beacon/25',
        badgeBg: 'bg-beacon/15 text-beacon border-beacon/35'
      };
    case '已收藏':
      return {
        label: '已收藏',
        stamp: 'gal',
        dotClass: 'bg-stamp-gal',
        textClass: 'text-stamp-gal',
        bgClass: 'bg-raised border-hairline hover:bg-hairline/40',
        badgeBg: 'bg-raised text-stamp-gal border-hairline'
      };
    default:
      return {
        label: '未开始',
        stamp: 'gal',
        dotClass: 'bg-faint',
        textClass: 'text-muted',
        bgClass: 'bg-canvas border-hairline hover:bg-raised',
        badgeBg: 'bg-canvas text-muted border-hairline'
      };
  }
}
