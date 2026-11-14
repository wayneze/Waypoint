import { jx3ArtworkUrl } from './jx3Artwork';

// 剑三门派 → JX3BOX 门派图标。school_id 取自 JX3BOX/jx3box-data data/xf/school.json。
const SCHOOL_IDS: Record<string, number> = {
  天策: 1, 万花: 2, 纯阳: 3, 七秀: 4, 少林: 5, 藏剑: 6, 丐帮: 7, 明教: 8, 五毒: 9, 唐门: 10,
  苍云: 18, 长歌: 19, 霸刀: 20, 蓬莱: 21, 凌雪: 22, 衍天: 23, 药宗: 24, 刀宗: 26, 万灵: 29,
  段氏: 38, 无相: 39,
};

/** 门派字段常写成「纯阳·气纯」「万灵山庄」，按包含匹配。 */
export function jx3SchoolId(sect: string | undefined): number | null {
  const s = (sect || '').trim();
  if (!s) return null;
  for (const [name, id] of Object.entries(SCHOOL_IDS)) {
    if (s.includes(name)) return id;
  }
  return null;
}

export function jx3SchoolIconUrl(sect: string | undefined): string {
  const id = jx3SchoolId(sect);
  return id == null ? '' : jx3ArtworkUrl(`https://img.jx3box.com/image/school/${id}.png`);
}
