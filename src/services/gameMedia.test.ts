import { expect, test } from 'vitest';
import { atmosphereSources, playableMedia } from './gameMedia';
import type { GameMetadata } from '../types';
import { detailBackground } from './gameCoverImage';
test('atmosphere uses playable trailers, then audio, and does not mistake album pages for audio', () => {
  const metadata = { videos: [{ title: 'Trailer', url: 'https://example.org/movie.m3u8?token=x' }], music: [{ url: 'https://bgm.tv/subject/1' }] } as unknown as GameMetadata;
  const audio = { title: 'OST', kind: 'audio' as const, url: 'https://example.org/ost.mp3' };
  expect(atmosphereSources(metadata, [audio]).map(item => item.kind)).toEqual(['video', 'audio']);
  expect(atmosphereSources(undefined, [audio])[0]).toBe(audio);
  expect(atmosphereSources(undefined, [{ ...audio, url: 'https://bgm.tv/subject/1' }])).toEqual([]);
  expect(playableMedia('audio', 'javascript:evil()')).toBe(false);
  expect(playableMedia('video','https://official.test/pv.m4v')).toBe(true);
  expect(playableMedia('video','https://www.bilibili.com/video/BV15x411A7xL/')).toBe(false);
});
test('background keeps explicit images and plain color; automatic never selects an arbitrary screenshot', () => {
  const library = { installations:[],metadata:{screenshots:['https://example.org/scene.jpg']} as GameMetadata };
  expect(detailBackground(library, 'poster')).toBe('poster');
  expect(detailBackground({...library,background:''}, 'poster')).toBe('');
  expect(detailBackground({...library,background:'custom'}, 'poster')).toBe('custom');
  expect(detailBackground({...library,metadata:{...library.metadata,background:'official'}}, 'poster')).toBe('official');
});
