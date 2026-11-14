import type { ObsidianGameGuide } from '../types';
import { gameTitleKeys } from './gameIdentity';

export interface AuthoritativeGameMeta {
  matchKeys: string[];
  canonicalTitle: string;
  poster: string;
  vndbId: string;
  rating: number;
}

/**
 * 旧攻略笔记的封面补充表；拆分后的系列作品由各自笔记提供元数据。
 * 彻底消除由于 Obsidian Markdown 模板复制粘贴导致的张冠李戴（海猫、天神乱漫等脏图残留）
 * 所有封面 URL 均经网络连通性测试 (HTTP 200 OK)
 */
export const AUTHORITATIVE_GALGAMES: AuthoritativeGameMeta[] = [
  {
    matchKeys: ['腐り姫', 'euthanasia', 'kusarihime'],
    canonicalTitle: '腐り姫 ～euthanasia～',
    poster: 'https://t.vndb.org/cv/91/88791.jpg',
    vndbId: 'v37',
    rating: 7.9,
  },
  {
    matchKeys: ['fate stay night', 'fate/stay night', 'fsn'],
    canonicalTitle: 'Fate/stay night',
    poster: 'https://t.vndb.org/cv/71/89071.jpg',
    vndbId: 'v11',
    rating: 8.6,
  },
  {
    matchKeys: ['fate hollow ataraxia', 'fate/hollow ataraxia', 'fha'],
    canonicalTitle: 'Fate/hollow ataraxia',
    poster: 'https://t.vndb.org/cv/13/89213.jpg',
    vndbId: 'v50',
    rating: 8.1,
  },
  {
    matchKeys: ['媚肉之香', '媚肉の香', 'biniku no kaori'],
    canonicalTitle: '媚肉之香',
    poster: 'https://t.vndb.org/cv/59/7959.jpg',
    vndbId: 'v470',
    rating: 7.9,
  },
  // 悠之空是缘之空的 FanDisc，笔记别名里带“缘之空”，必须排在前面
  {
    matchKeys: ['悠之空', 'ハルカナソラ', 'haruka na sora'],
    canonicalTitle: '悠之空（Haruka na Sora）',
    poster: 'https://t.vndb.org/cv/85/96185.jpg',
    vndbId: 'v1899',
    rating: 7.4,
  },
  {
    matchKeys: ['缘之空', 'ヨスガノソラ', 'yosuga no sora', '穹妹'],
    canonicalTitle: '缘之空（Yosuga no Sora）',
    poster: 'https://t.vndb.org/cv/84/96184.jpg',
    vndbId: 'v1194',
    rating: 7.4,
  },
  {
    matchKeys: ['chaos;head', 'chaos head', '混沌头'],
    canonicalTitle: 'CHAOS;HEAD NOAH',
    poster: 'https://t.vndb.org/cv/05/75805.jpg',
    vndbId: 'v22505',
    rating: 7.8,
  },
  {
    matchKeys: ['phantom integration', 'phantom of inferno', 'phantom'],
    canonicalTitle: 'Phantom INTEGRATION',
    poster: 'https://t.vndb.org/cv/74/85574.jpg',
    vndbId: 'v45',
    rating: 7.9,
  },
  {
    matchKeys: ['命运石之门0', 'steins;gate 0', 'steins gate 0', '石头门0'],
    canonicalTitle: '命运石之门 0',
    poster: 'https://t.vndb.org/cv/34/75834.jpg',
    vndbId: 'v17102',
    rating: 8.1,
  },
  {
    matchKeys: ['命运石之门', 'steins;gate', 'steins gate', '命运石', '石头门'],
    canonicalTitle: '命运石之门',
    poster: 'https://t.vndb.org/cv/19/77819.jpg',
    vndbId: 'v2002',
    rating: 9.0,
  },
  {
    matchKeys: ['天神乱漫', 'tenshin ranman'],
    canonicalTitle: '天神乱漫 LUCKY or UNLUCKY!?',
    poster: 'https://t.vndb.org/cv/11/27811.jpg',
    vndbId: 'v1322',
    rating: 7.4,
  },
  {
    matchKeys: ['美好的每一天', '素晴らしき日々', 'subarashiki hibi', '不连续存在'],
    canonicalTitle: '美好的每一天 不连续存在',
    poster: 'https://t.vndb.org/cv/17/90017.jpg',
    vndbId: 'v3144',
    rating: 8.7,
  },
  {
    matchKeys: ['你与我和伊甸园的苹果', 'キミとボクとエデンの林檎', 'eden no ringo'],
    canonicalTitle: '你与我和伊甸园的苹果',
    poster: 'https://t.vndb.org/cv/46/145046.jpg',
    vndbId: 'v5635',
    rating: 7.0,
  },
  {
    matchKeys: ['恋骑士', '恋騎士', 'koikishi'],
    canonicalTitle: '恋骑士 Purely☆Kiss',
    poster: 'https://t.vndb.org/cv/55/89655.jpg',
    vndbId: 'v7260',
    rating: 7.1,
  },
  {
    matchKeys: ['鬼的捉迷藏', '鬼ごっこ', 'onigokko'],
    canonicalTitle: '鬼的捉迷藏（鬼ごっこ！）',
    poster: 'https://t.vndb.org/cv/99/79799.jpg',
    vndbId: 'v5549',
    rating: 7.1,
  },
  {
    matchKeys: ['君と彼女と彼女の恋', '君与她与她的恋', 'totono'],
    canonicalTitle: '君与她与她的恋（君と彼女と彼女の恋。）',
    poster: 'https://t.vndb.org/cv/07/77907.jpg',
    vndbId: 'v7738',
    rating: 7.9,
  },
  {
    matchKeys: ['chaos;child', 'chaos child', '混沌之子'],
    canonicalTitle: 'CHAOS;CHILD 混沌之子',
    poster: 'https://t.vndb.org/cv/35/75835.jpg',
    vndbId: 'v14018',
    rating: 8.4,
  },
  {
    matchKeys: ['妻中蜜', '妻みぐい', 'tsumamigui'],
    canonicalTitle: '妻中蜜3（妻みぐい3）',
    poster: 'https://t.vndb.org/cv/89/95789.jpg',
    vndbId: 'v18486',
    rating: 7.9,
  },
  {
    matchKeys: ['ままごと', 'mamagoto', '过家家'],
    canonicalTitle: 'ままごと ～ままとないしょのえっちしましょ～',
    poster: 'https://t.vndb.org/cv/03/116003.jpg',
    vndbId: 'v26530',
    rating: 8.1,
  },
  {
    matchKeys: ['沙耶之歌', '沙耶の唄', 'saya no uta', 'song of saya'],
    canonicalTitle: '沙耶之歌（Saya no Uta）',
    poster: 'https://t.vndb.org/cv/68/93368.jpg',
    vndbId: 'v97',
    rating: 7.9,
  },
  {
    matchKeys: ['恋狱', '月狂病', 'カルタグラ', 'cartagra'],
    canonicalTitle: '恋狱～月狂病～FHD',
    poster: 'https://t.vndb.org/cv/74/77274.jpg',
    vndbId: 'v515',
    rating: 7.2,
  },
  {
    matchKeys: ['群星', 'stellaris'],
    canonicalTitle: '群星（Stellaris）',
    poster: 'https://cdn.cloudflare.steamstatic.com/steam/apps/281990/header.jpg',
    vndbId: '',
    rating: 9.2,
  },
];

/**
 * 排除曾在攻略模板中重复用于不同作品的封面地址。
 */
const CONTAMINATED_TEMPLATE_URLS = new Set([
  'https://s2.vndb.org/cv/51/30051.jpg',
  'https://cdn.cloudflare.steamstatic.com/steam/apps/702050/header.jpg',
  'https://lain.bgm.tv/pic/cover/l/31/85/12941_EP1sL.jpg',
]);

function tableMetaFor(guide: ObsidianGameGuide): AuthoritativeGameMeta | undefined {
  // 完整作品名才可采用核验表，续作、副标题及别名中的系列名不能继承本篇身份。
  for (const name of [guide.original_title, guide.title, guide.file_name].filter(Boolean) as string[]) {
    const keys = new Set(gameTitleKeys(name.replace(/\.md$/i, '').replace(/^\d{4}-/, '')));
    const matches = AUTHORITATIVE_GALGAMES.filter(meta =>
      [meta.canonicalTitle, ...meta.matchKeys].flatMap(gameTitleKeys).some(value => keys.has(value)));
    if (matches.length === 1) return matches[0];
  }
}

/**
 * 根据已核验元数据补充攻略封面与标题。
 */
export function sanitizeAndEnrichGuide(guide: ObsidianGameGuide): ObsidianGameGuide {
  // 移除导入摘要中的引用标记和模板导航。
  let cleanSummary = guide.summary;
  if (cleanSummary) {
    cleanSummary = cleanSummary
      .replace(/[\ue000-\uf8ff]|cite.*?|\[cite.*?\]/g, '')
      .replace(/\[\[(?:[^|\]]*\|)?([^\]]+)\]\]/g, '$1')
      .trim();
    if (
      cleanSummary.includes('折叠文本行尾') ||
      cleanSummary.includes('回到索引') ||
      cleanSummary.includes('返回索引') ||
      cleanSummary.includes('未删减，仅用折叠') ||
      cleanSummary.includes('重排折叠与索引') ||
      cleanSummary.startsWith('![') ||
      cleanSummary.length <= 2
    ) {
      cleanSummary = undefined;
    }
  }

  const meta = tableMetaFor(guide);
  // 表外作品才用笔记的 poster；被模板污染或防盗链的地址视为没填
  let cleanPoster = guide.poster;
  if (cleanPoster && (
    CONTAMINATED_TEMPLATE_URLS.has(cleanPoster)
    || cleanPoster.includes('upload.wikimedia.org')
    || cleanPoster.includes('img-getchu.ssl-image.com')
  )) {
    cleanPoster = undefined;
  }

  return {
    ...guide,
    poster: meta?.poster || cleanPoster,
    links: meta?.vndbId ? { ...guide.links, vndb: `https://vndb.org/${meta.vndbId}` } : guide.links,
    rating: guide.rating || meta?.rating || 0,
    rating_vndb: guide.rating_vndb || meta?.rating,
    summary: cleanSummary,
  };
}

