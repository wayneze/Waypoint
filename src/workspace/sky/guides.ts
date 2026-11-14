// 自定义季节目录；物品拥有与心愿统一由 collection 管理。
import { ref, reactive } from 'vue';
import type { SeasonalGuide } from '../../types';
import { saveCustomSeasons } from '../../services/skySeasonService';
import { showToast, state } from '../core';

// 动态新增季节向导状态 (折叠至底，消除纯手填误解)
export const showAddSeasonModal = ref(false);
export const newSeasonForm = reactive({
  season: '',
  realm: '云野',
  loc: '',
  quests: '主线任务 1-4',
  benefit: '',
  items: '',
  priority: 'A级实用',
  candles: 100,
  ascended: 0,
  hearts: 10
});

export function handleAddCustomSeason() {
  if (!newSeasonForm.season.trim()) {
    showToast('请输入季节名称', '如：云巢 · 筑巢季 或 联动季');
    return;
  }
  const id = 'custom_' + Date.now();
  const guide: SeasonalGuide = {
    id,
    season: newSeasonForm.season.trim(),
    shortName: newSeasonForm.season.trim(),
    realm: newSeasonForm.realm,
    loc: newSeasonForm.loc.trim() || '大世界常驻向导',
    quests: newSeasonForm.quests.trim() || '主线任务',
    benefit: newSeasonForm.benefit.trim() || '常驻季节兑换',
    items: newSeasonForm.items.trim() || '季节限定兑换道具',
    itemList: [
      { id: `${id}_1`, name: newSeasonForm.items.trim() || '限定道具', costDesc: `${newSeasonForm.candles}白蜡`, candles: Number(newSeasonForm.candles) || 0, ascended: Number(newSeasonForm.ascended) || 0, hearts: Number(newSeasonForm.hearts) || 0, obtained: false }
    ],
    task: newSeasonForm.quests.trim() || '主线任务',
    priority: newSeasonForm.priority,
    candles: Number(newSeasonForm.candles) || 0,
    ascended: Number(newSeasonForm.ascended) || 0,
    hearts: Number(newSeasonForm.hearts) || 0,
    selected: false,
    isCustom: true
  };
  state.seasonalGuides.unshift(guide);
  saveCustomSeasons(state.seasonalGuides);
  showToast('季节向导添加成功', `${guide.season} 已加入收藏目录`);
  showAddSeasonModal.value = false;
  newSeasonForm.season = '';
  newSeasonForm.loc = '';
  newSeasonForm.benefit = '';
  newSeasonForm.items = '';
}

export function handleDeleteCustomSeason(id: string) {
  state.seasonalGuides = state.seasonalGuides.filter(g => g.id !== id);
  saveCustomSeasons(state.seasonalGuides);
  showToast('已删除季节向导', '自定义季节已移出收藏目录');
}
