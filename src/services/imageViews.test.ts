import { expect, test } from 'vitest';
import { imageViewStyle, resolveWearViews } from './imageViews';

test('wear layout uses the source format, preserving unknown images whole', () => {
  const views = resolveWearViews('https://patchwiki.biligame.com/images/sky/example.png', 1280, 256);
  expect(views.map(v => v.title)).toEqual(['正面', '视角 2', '视角 3', '视角 4']);
  expect(views.map(v => imageViewStyle(v).left)).toEqual(['-100%', '-200%', '-300%', '-400%']);
  for (const [src, width, height] of [
    ['https://example.com/panorama.png', 1280, 256],
    ['https://patchwiki.biligame.com/images/other/banner.png', 1280, 256],
    ['https://wiki.biligame.com/sky/Special:FilePath/unknown.png', 1024, 256],
    ['https://wiki.biligame.com/sky/Special:FilePath/portrait.png', 180, 240],
  ] as const) {
    expect(resolveWearViews(src, width, height)).toEqual([
      { title: '原图', x: 0, y: 0, width: 1, height: height / width },
    ]);
  }
});
