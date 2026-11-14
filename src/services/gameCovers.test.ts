import { test } from 'vitest';
import { sanitizeAndEnrichGuide } from './gameCovers.ts';
import type { ObsidianGameGuide } from '../types/index.ts';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

function guide(title: string, poster: string): ObsidianGameGuide {
  return {
    id: `guide_${title}`, title, aliases: [], category: 'galgame', status: '已收藏', rating: 0,
    tags: [], genres: [], file_path: '', relative_path: '', file_name: `${title}.md`, modified: '-', poster,
  };
}

test('gameCovers', () => {
  const listed = sanitizeAndEnrichGuide(guide('2002-腐り姫 ～euthanasia～', 'https://s2.vndb.org/cv/04/40104.jpg'));
  assert(listed.poster === 'https://t.vndb.org/cv/91/88791.jpg', `table poster must beat the note poster, got ${listed.poster}`);
  const unlisted = sanitizeAndEnrichGuide(guide('某部表外作品', 'https://example.com/p.jpg'));
  assert(unlisted.poster === 'https://example.com/p.jpg', 'unlisted works keep the note poster');
  assert(sanitizeAndEnrichGuide(guide('妻中蜜3', '')).links?.vndb === 'https://vndb.org/v18486', 'canonical parenthesized names still match');
  assert(sanitizeAndEnrichGuide(guide('沙耶之歌（Steam 重制版）', '')).links?.vndb === 'https://vndb.org/v97', 'platform labels do not change the work identity');
});
