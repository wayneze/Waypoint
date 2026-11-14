import type { GameCuration, GameMetadata, GameStaff, GameTaxonomy, ObsidianGameGuide } from '../types';

export const taxonomyKey = (name: string) => name.normalize('NFKC').trim().replace(/\s+/g, ' ').toLocaleLowerCase();
export function uniqueNames(names: string[]): string[] {
  return [...new Map(names.filter(n => n.trim()).map(n => [taxonomyKey(n), n.trim()])).values()];
}
// 来源 infobox 用显式分隔符；不按 &、+、・ 等公司/人名的组成字符拆分。
export const splitCredits = (value?: string | null) => uniqueNames((value ?? '').split(/\s+\/\s+|[、；;\n]+/));
const developerLabels = new Set(['开发', '开发商', '游戏开发商', '开发公司', '制作公司', 'developers', 'developer']);
const publisherLabels = new Set(['发行', '发行商', '发行公司', 'publishers', 'publisher']);
const roleAliases: Record<string, string> = {
  '脚本': '剧本', 'シナリオ': '剧本', 'scenario': '剧本', '导演': '导演', '监督': '导演', '監督': '导演', 'director': '导演',
  '制作人': '制作人', 'プロデューサー': '制作人', 'producer': '制作人', '企画': '企画', '策划': '企画',
  '原画': '原画', 'art': '原画', '角色设计': '角色设计', '人物设定': '角色设计', 'chardesign': '角色设计',
  '音乐': '音乐', '音楽': '音乐', 'music': '音乐', 'songs': '歌曲', '声优': '配音', '声優': '配音', '配音': '配音',
  '剧本': '剧本', '剧本协力': '剧本协力', '原作': '原作', 'staff': '其他',
  'translator': '翻译', 'editor': '编辑', 'qa': '测试', 'engineer': '程序', 'programmer': '程序',
};
export const staffRoles = ['制作人', '导演', '企画', '剧本', '剧本协力', '原画', '角色设计', '音乐', '歌曲', '配音', '原作', '程序', '翻译', '编辑', '测试', '其他'];
export const creditRole = (role: string) => roleAliases[taxonomyKey(role)] || role.trim() || '其他';
export const isCreditField = (label: string) => developerLabels.has(taxonomyKey(label)) || publisherLabels.has(taxonomyKey(label)) || !!roleAliases[taxonomyKey(label)];
function uniqueStaff(staff: GameStaff[]) {
  return [...new Map(staff.filter(s => s.name.trim()).map(s => {
    const member = { ...s, name: s.name.trim(), role: creditRole(s.role) };
    return [taxonomyKey(member.name) + '\0' + taxonomyKey(member.role), member];
  })).values()];
}
export function sourceGameTaxonomy(guide?: ObsidianGameGuide, metadata?: GameMetadata | null): GameTaxonomy {
  const fields = metadata?.info || [];
  const fieldNames = (labels: Set<string>) => fields.filter(f => labels.has(taxonomyKey(f.label))).flatMap(f => splitCredits(f.value));
  return {
    tags: uniqueNames([...(guide?.tags || []), ...(metadata?.tags || [])]),
    developers: uniqueNames([...(metadata?.developers?.length ? metadata.developers : splitCredits(metadata?.developer)), ...splitCredits(guide?.developer), ...fieldNames(developerLabels)]),
    publishers: uniqueNames([...(metadata?.publishers || []), ...splitCredits(guide?.publisher), ...fieldNames(publisherLabels)]),
    staff: uniqueStaff([...(metadata?.staff || []), ...fields.filter(f => roleAliases[taxonomyKey(f.label)])
      .flatMap(f => splitCredits(f.value).map(name => ({ name, role: creditRole(f.label) })))]),
  };
}
export function curatedGameTaxonomy(source: GameTaxonomy, curation?: GameCuration | null): GameTaxonomy {
  const hidden = new Set((curation?.hidden_tags || []).map(taxonomyKey));
  return {
    tags: uniqueNames([...source.tags.filter(t => !hidden.has(taxonomyKey(t))), ...(curation?.added_tags || [])]),
    developers: uniqueNames(curation?.developers ?? source.developers),
    publishers: uniqueNames(curation?.publishers ?? source.publishers),
    staff: uniqueStaff(curation?.staff ?? source.staff),
  };
}
export interface GameTaxonomyFilter {
  tags: string[]; excluded: string[]; tag_mode: 'all' | 'any';
  companies: string[]; company_role: 'all' | 'developer' | 'publisher';
  people: string[]; staff_role: string;
}
export const emptyTaxonomyFilter = (): GameTaxonomyFilter => ({
  tags: [], excluded: [], tag_mode: 'all', companies: [], company_role: 'all', people: [], staff_role: '',
});
export type GameTaxonomySelection = { kind: 'tag' | 'developer' | 'publisher' | 'person'; name: string; role?: string };
export function matchesGameTaxonomy(taxonomy: GameTaxonomy, filter: GameTaxonomyFilter): boolean {
  const contains = (values: string[], term: string) => values.some(v => taxonomyKey(v) === taxonomyKey(term));
  if (filter.excluded.some(t => contains(taxonomy.tags, t))) return false;
  if (filter.tags.length && !(filter.tag_mode === 'all' ? filter.tags.every(t => contains(taxonomy.tags, t)) : filter.tags.some(t => contains(taxonomy.tags, t)))) return false;
  const companies = filter.company_role === 'developer' ? taxonomy.developers : filter.company_role === 'publisher' ? taxonomy.publishers : [...taxonomy.developers, ...taxonomy.publishers];
  if (filter.companies.length && !filter.companies.some(c => contains(companies, c))) return false;
  const staff = taxonomy.staff.filter(s => !filter.staff_role || s.role === filter.staff_role);
  return (!filter.staff_role || staff.length > 0) && (!filter.people.length || filter.people.some(p => contains(staff.map(s => s.name), p)));
}
