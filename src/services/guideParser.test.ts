import { describe, expect, it } from 'vitest';
import { formatGuideInline, parseCalloutLines, parseObsidianGuide } from './guideParser';

function imgTag(html: string): string {
  return html.match(/<img [^>]*>/)?.[0] || '';
}

describe('formatGuideInline', () => {
  it('reads exported tables without splitting escaped pipes or dropping empty columns', () => {
    const rows = ['| | 选择 |', '| --- | --- |', '| A\\|B | 左<br>右 |'];
    expect(parseCalloutLines(rows)[0]?.tableData).toEqual({ headers: ['', '选择'], rows: [['A|B', '左<br>右']] });
    expect(parseObsidianGuide(rows.join('\n')).blocks.find(block => block.table)?.table)
      .toEqual({ headers: ['', '选择'], rows: [['A|B', '左<br>右']] });
    expect(formatGuideInline('左<br>右')).toBe('左<br/>右');
    expect(formatGuideInline('<br onmouseover="evil()">')).not.toContain('<br ');
  });
  it('keeps image markup intact after inline highlighting', () => {
    const tag = imgTag(formatGuideInline('路线 ![BE 1 → 结局 / 分支](https://example.com/a.png) 之后'));
    expect(tag).toContain('src="https://example.com/a.png"');
    expect(tag).toContain('alt="BE 1 → 结局 / 分支"');
  });

  it('escapes quotes in image alt text', () => {
    const tag = imgTag(formatGuideInline('![x" onerror="alert(1)](https://example.com/a.png) '));
    expect(tag).not.toContain('onerror="alert');
    expect(tag).toContain('alt="x&quot; onerror=&quot;alert(1)"');
  });
});

describe('parseCalloutLines', () => {
  it('classifies Steam-style guide lines without dropping text', () => {
    const lines = parseCalloutLines([
      ' **Vision Projection',
      ' 投影的视觉**',
      '',
      '',
      ' 【手机触发器】更改过待机画面的壁纸',
      ' 7/28 14:53',
      ' 1周目路线',
      ' 19. 相信远坂。',
      ' 1、11日废墟',
      ' 1.5倍速度',
      ' ———————————————',
    ]);
    expect(lines.map(l => [l.type, l.cleanText])).toEqual([
      ['section-header', 'Vision Projection 投影的视觉'],
      ['spacer', ''],
      ['section-header', '【手机触发器】更改过待机画面的壁纸'],
      ['section-header', '7/28 14:53'],
      ['text', '1周目路线'],
      ['choice', '相信远坂。'],
      ['choice', '11日废墟'],
      ['text', '1.5倍速度'],
      ['divider', ''],
    ]);
  });

  // 句子都取自库里的真实攻略
  it('keeps dated steps, banners and long mark lines out of section headers', () => {
    const lines = parseCalloutLines([
      '8月4日：朝 自宅／昼1 神社',
      '8月1日—5日 同海夕里线',
      '4月1日',
      '10月8日 ○午前',
      '7/29 13:49 收到邮件',
      '**■■■第 1 章│【Eyes in eyes】■■■**',
      '**———————————— 注：同上**',
      '◆人物选择',
      '▼007│吊坠⇒决定',
      'SAVE 5开始',
    ]);
    expect(lines.map(l => [l.type, l.stepNumber ?? '', l.cleanText])).toEqual([
      ['day', '8月4日', '朝 自宅／昼1 神社'],
      ['day', '8月1日—5日', '同海夕里线'],
      ['section-header', '', '4月1日'],
      ['section-header', '', '10月8日 ○午前'],
      ['day', '7/29 13:49', '收到邮件'],
      ['section-header', '', '第 1 章│【Eyes in eyes】'],
      ['tip', '', '注：同上'],
      ['section-header', '', '人物选择'],
      ['bullet', '', '007│吊坠⇒决定'],
      ['load-header', '', 'SAVE 5开始'],
    ]);
    expect(lines[0].formattedHtml).toContain('wp-guide-sep');
  });
});
