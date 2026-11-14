import artwork from '../content/jx3/artwork.json';
import type { Jx3ReputationItem } from '../types';
import { reactive } from 'vue';

export const jx3AchievementIconIds = reactive<Record<string, number>>({ ...artwork.achievementIcons });

export function jx3AchievementIconUrl(id?: string): string {
  const iconId = id ? jx3AchievementIconIds[id] : undefined;
  return iconId ? jx3ArtworkUrl(`https://icon.jx3box.com/icon/${iconId}.png`) : '';
}

export function jx3ArtworkUrl(source: string): string {
  return (artwork.images as Record<string, string>)[source] || source;
}

export function jx3CategoryImage(name: string): string {
  const source = (artwork.categories as Record<string, string>)[name];
  return source ? jx3ArtworkUrl(source) : '';
}

export function jx3ReputationIconUrl(rep?: Pick<Jx3ReputationItem, 'name' | 'iconPath'>): string {
  if (!rep) return '';
  const key = rep.iconPath?.replace(/\\/g, '/').split('/').pop()?.replace(/\.tga$/i, '').toLowerCase();
  const source = key
    ? `https://img.jx3box.com/reputation/reputation/std/icon/${key}.png`
    : (artwork.reputations as Record<string, string>)[rep.name];
  return source ? jx3ArtworkUrl(source) : '';
}

export function jx3PlaceSceneUrl(name: string): string {
  const source = (artwork.places as Record<string, string>)[name.trim()];
  return source ? jx3ArtworkUrl(source) : '';
}

export function jx3AchievementCategoryImage(name: string): string {
  const category = ({ 角色: '修为', 江湖: '杂闻', 竞技: '阵营', 生活: '技艺', 其它: '杂闻' } as Record<string, string>)[name] || name;
  return jx3CategoryImage(category);
}
