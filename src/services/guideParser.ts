// Obsidian 游戏攻略结构化解析器
// 专为 Obsidian Galgame 攻略排版规范定制 (支持 Callout 折叠、SAVE 点徽章、结局标签、表格解析)

export interface GuideCallout {
  type: 'note' | 'tip' | 'abstract' | 'warning' | 'info' | 'danger';
  title: string;
  foldDefault: boolean;
  content: string;
  lines: string[];
  parsedLines?: FormattedCalloutLine[];
  sectionHeading?: string;
}

export interface GuideTable {
  headers: string[];
  rows: string[][];
}

export interface GuideBlock {
  type: 'heading' | 'callout' | 'table' | 'paragraph' | 'list' | 'lines';
  id?: string;
  level?: number;
  text?: string;
  callout?: GuideCallout;
  table?: GuideTable;
  items?: string[];
  lines?: FormattedCalloutLine[];
}

export interface ParsedGuide {
  title: string;
  overview: string;
  quickIndex: Array<{ title: string; targetText: string }>;
  blocks: GuideBlock[];
}

// 标准化 SVG 数据 URI：若缺少 viewBox 则自动注入；并将 utf-8 纯文本 SVG 安全转码为 base64，根治 URL Hash (#) 截断与转义失败问题
export function normalizeSvgUrl(url: string): string {
  if (!url) return '';
  if (url.startsWith('data:image/svg+xml')) {
    let res = url;
    // 如果是 utf-8 纯文本数据 URI (非 base64)
    if (!res.includes(';base64,')) {
      const commaIdx = res.indexOf(',');
      if (commaIdx !== -1) {
        let svgContent = res.slice(commaIdx + 1);
        // 若缺少 viewBox 则自动从 width/height 注入 viewBox
        if (!svgContent.includes('viewBox')) {
          const wMatch = svgContent.match(/width=['"](\d+)['"]/i);
          const hMatch = svgContent.match(/height=['"](\d+)['"]/i);
          if (wMatch && hMatch) {
            const w = wMatch[1];
            const h = hMatch[1];
            svgContent = svgContent.replace(/<svg\b([^>]*)>/i, `<svg$1 viewBox="0 0 ${w} ${h}">`);
          }
        }
        try {
          // 安全地将 utf-8 SVG 转为 Base64 Data URI，彻底规避浏览器 URL Hash 截断与特殊字符问题
          const base64Data = btoa(unescape(encodeURIComponent(svgContent)));
          return `data:image/svg+xml;base64,${base64Data}`;
        } catch (e) {
          // 兜底：将 '#' 转义为 '%23'
          return `data:image/svg+xml;utf8,${svgContent.replace(/#/g, '%23')}`;
        }
      }
    }
    return res;
  }
  return url;
}

// 行内高亮与排版美化 (保留双空格换行，高亮 SAVE 点、BAD END、时段与剧情 Flag)
function escapeGuideAttr(raw: string): string {
  return raw.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function splitGuideTableRow(line: string): string[] {
  const body = line.trim().slice(1, -1);
  const cells: string[] = [];
  let cell = '';
  for (let i = 0; i < body.length; i++) {
    if (body[i] === '\\' && (body[i + 1] === '|' || body[i + 1] === '\\')) cell += body[++i];
    else if (body[i] === '|') { cells.push(cell.trim()); cell = ''; }
    else cell += body[i];
  }
  return [...cells, cell.trim()];
}

export function formatGuideInline(text: string): string {
  if (!text) return '';

  // 移除导入文本中的引用标记。
  let cleanText = text.replace(/cite.*?/g, '').replace(/\[cite.*?\]/g, '');

  // 预先提取并保护图片/SVG标签 (支持含有括号的复杂 SVG 矢量图)
  const images: string[] = [];
  // [![](图)](链接)章节名：Steam 成就横幅，跟在文字旁边，不是整页大图。
  let s = cleanText.replace(/\[!\[([^\]]*)\]\((https?:\/\/[^)\s]+)\)\]\((https?:\/\/[^)\s]+)\)/g, (_match, alt, img, href) => {
    const label = escapeGuideAttr(String(alt || '章节图标'));
    images.push(`<span class="wp-guide-zoom-clip"><img src="${escapeGuideAttr(img)}" alt="${label}" data-guide-zoom="${escapeGuideAttr(href)}" referrerpolicy="no-referrer" class="wp-guide-zoom" /></span>`);
    return `__WAYPOINT_IMG_${images.length - 1}__`;
  });
  s = s.replace(/!\[([^\]]*)\]\(([\s\S]+?)\)(?=\s|$)/g, (_match, alt, url) => {
    const cleanUrl = escapeGuideAttr(normalizeSvgUrl(url));
    const label = escapeGuideAttr(alt);
    const caption = label ? `<span class="wp-guide-figure-cap">${label}</span>` : '';
    images.push(`<span class="wp-guide-figure"><img src="${cleanUrl}" alt="${label}" data-guide-zoom="${cleanUrl}" referrerpolicy="no-referrer" loading="lazy" />${caption}</span>`);
    return `__WAYPOINT_IMG_${images.length - 1}__`;
  });
  
  s = s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/&lt;br\s*\/?&gt;/gi, '<br/>');

  // 选项分支斜杠 / 分隔符美化 (如 暂时别去打扰她 / 敲门) - 必须在注入任何 HTML/Tailwind 类之前执行，且仅匹配带空格的斜杠
  s = s.replace(/\s{1,}\/\s{1,}/g, ' <span class="text-faint font-bold px-1 select-none">/</span> ');

  // 加粗 **...**
  s = s.replace(/\*\*(.*?)\*\*/g, '<strong class="text-ink font-semibold">$1</strong>');

  // Obsidian 高亮语法 ==...==
  s = s.replace(/==([^=]+)==/g, '<mark class="bg-beacon/15 text-beacon px-1.5 py-0.5 rounded-md border border-beacon/35 font-medium">$1</mark>');

  // 行内代码 `...`
  s = s.replace(/`([^`]+)`/g, '<code class="px-1.5 py-0.5 rounded-md bg-raised text-ink font-mono text-[12px] border border-hairline">$1</code>');

  // 高亮 【SAVE n】 或 【读取 SAVE n】 或 【LOAD n】 以及单独出现的 SAVE 01 / LOAD 01
  s = s.replace(
    /(【?(?:读取\s*|LOAD\s*)?SAVE\s*\d+】?|【(?:LOAD|读取)\s*\d+】)/gi,
    '<span class="inline-block px-1.5 py-0.5 mx-0.5 rounded-md text-[11px] font-mono font-medium bg-beacon/15 text-beacon border border-beacon/35 whitespace-nowrap">$1</span>'
  );

  // 高亮 【Flag：...】 或 【Flag: ...】 (关键剧情线索)
  s = s.replace(
    /【(Flag[：:].*?)】/gi,
    '<span class="inline-block px-1.5 py-0.5 mx-0.5 rounded-md text-[11px] font-mono font-medium bg-done/10 text-done border border-done/35 whitespace-nowrap">【$1】</span>'
  );

  // 高亮 日期与时段标识 (如 ○午前 / ○午後 / ○夜 / ○昼)
  s = s.replace(
    /(○(?:午前|午後|夜|昼))/g,
    '<span class="inline-block px-1.5 py-0.5 mx-0.5 rounded-md text-[11px] font-sans font-medium bg-raised text-muted border border-hairline whitespace-nowrap">$1</span>'
  );

  // 高亮 BAD END / Dead End / BE n (排除半角 ) 与全角 ）)
  s = s.replace(
    /((?:→\s*)?(?:BAD\s*END\s*\d*|Dead\s*End\s*\d*|BE\s*\d+)[^)<>\n\r）]*)/gi,
    '<span class="wp-end wp-end-bad">$1</span>'
  );

  // 高亮 True End / Good End / Normal End，三种结局三种颜色 (排除半角 ) 与全角 ）)
  s = s.replace(
    /((?:→\s*)?(True|Good|Normal)\s*End[^)<>\n\r）]*)/gi,
    (_m, all: string, kind: string) => `<span class="wp-end wp-end-${kind.toLowerCase()}">${all}</span>`
  );

  // 游戏选项对话括号 「...」
  s = s.replace(
    /「([^」]+)」/g,
    '<span class="text-ink font-medium bg-raised px-1 py-0.5 rounded-md border border-hairline">「$1」</span>'
  );

  // （无选项）状态
  s = s.replace(
    /（无选项）/g,
    '<span class="text-faint text-[11px]">（无选项）</span>'
  );

  // 选项箭头引导 (如 → 或 =>)
  s = s.replace(
    /(→|=>|⇒)\s*([^<\n\r]+)/g,
    '<span class="text-faint mx-1">$1</span><span class="text-ink font-medium">$2</span>'
  );

  // Markdown 网页链接 [text](url)
  s = s.replace(/\[([^\]]+)\]\((https?:\/\/[^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer" class="text-beacon hover:text-ink underline underline-offset-2">$1 ↗</a>');

  // 行尾双空格保留单行换行
  s = s.replace(/  $/gm, '<br/>');

  // 图片最后恢复，前面的高亮正则不能改写 img 属性。
  images.forEach((imgHtml, idx) => {
    s = s.replace(`__WAYPOINT_IMG_${idx}__`, () => imgHtml);
  });

  return s;
}

// 结构化 Callout 单行条目解析，驱动卡片化人体工学排版
export interface FormattedCalloutLine {
  raw: string;
  /** day：「8月4日：朝 自宅／昼1 神社」这种按日期走的一步，stepNumber 放日期 */
  type: 'spacer' | 'divider' | 'image' | 'save-header' | 'load-header' | 'end-header' | 'section-header' | 'choice' | 'day' | 'bullet' | 'sub-choice' | 'tip' | 'nav-link' | 'location' | 'table' | 'text';
  stepNumber?: string;
  cleanText: string;
  formattedHtml: string;
  imageUrl?: string;
  imageAlt?: string;
  tableData?: {
    headers: string[];
    rows: string[][];
  };
}

/** 章节横幅两头的 ■■■、━━━、———— 只是装饰 */
function stripBanner(text: string): string {
  return text.replace(/^[■□━─—=\s]+/, '').replace(/[■□━─—=\s]+$/, '').trim() || text.trim();
}

/** 日期后面跟着当天安排：日期做左侧标签，正文里的「／」分隔调淡，一眼能扫出时段 */
function dayLine(line: string, date: string, rest: string): FormattedCalloutLine {
  let html = formatGuideInline(rest);
  if (!html.includes('<img')) html = html.replace(/／/g, '<span class="wp-guide-sep">／</span>');
  return { raw: line, type: 'day', stepNumber: date, cleanText: rest, formattedHtml: html };
}

export function parseCalloutLine(line: string): FormattedCalloutLine {
  const trimmed = line.trim();
  if (!trimmed) {
    return { raw: line, type: 'spacer', cleanText: '', formattedHtml: '' };
  }

  // 分割线：---、***，以及整行的 —————————。横线后面直接粘着配图时，配图单独成行。
  const ruleBody = trimmed.replace(/^[—\-＊*]{8,}(?=\s|$|\[)/, '').trim();
  if (/^[—\-＊*]{8,}$/.test(trimmed) || trimmed === '***' || trimmed.startsWith('---')) {
    return { raw: line, type: 'divider', cleanText: '', formattedHtml: '' };
  }
  if (ruleBody && ruleBody !== trimmed && /^\[!\[/.test(ruleBody)) {
    return parseCalloutLine(ruleBody);
  }

  // 自动剔除导航废弃行：[回到索引](#快速索引) 或 [返回索引] 等 (顶部已有吸顶路线大纲胶囊，无需正文内繁杂链接)
  if (trimmed.includes('[回到索引]') || trimmed.includes('[返回索引]') || trimmed.includes('(#快速索引)') || trimmed === '回到路线大纲' || trimmed === '回到索引') {
    return { raw: line, type: 'spacer', cleanText: '', formattedHtml: '' };
  }

  // 自动剔除模板维护说明与无意义备注行 (对玩家查攻略毫无意义，纯占位)
  if (
    trimmed.includes('折叠文本行尾保留双空格') ||
    trimmed.includes('保留双空格') ||
    trimmed.includes('重排折叠与索引') ||
    trimmed.includes('原始逐章攻略全文已保留') ||
    trimmed.includes('本篇无官方成就；完成度=') ||
    trimmed.includes('原文未删减') ||
    trimmed.includes('请勿清理') ||
    trimmed.includes('注：括号内是移动至的地点') ||
    trimmed.includes('注：括号内是移动地点') ||
    trimmed.includes('折叠内已按编号标注触发')
  ) {
    return { raw: line, type: 'spacer', cleanText: '', formattedHtml: '' };
  }

  // 整行只有 [![](图)](链接)：少女三部曲这类是大图。旁边还有章节名的走下面的行内横幅。
  const linkedOnly = trimmed.match(/^\[!\[([^\]]*)\]\((https?:\/\/[^)\s]+)\)\]\((https?:\/\/[^)\s]+)\)$/);
  if (linkedOnly) {
    return {
      raw: line,
      type: 'image',
      imageAlt: linkedOnly[1] || '示意图',
      imageUrl: linkedOnly[2],
      cleanText: linkedOnly[1] || '示意图',
      formattedHtml: ''
    };
  }

  // Markdown 嵌入图片或 SVG (支持带换行和带括号的复杂 SVG 矢量图)
  const fullImgMatch = trimmed.match(/^!\[([^\]]*)\]\(([\s\S]+)\)$/);
  if (fullImgMatch) {
    const rawUrl = fullImgMatch[2];
    const cleanUrl = normalizeSvgUrl(rawUrl);
    return {
      raw: line,
      type: 'image',
      imageAlt: fullImgMatch[1] || '示意图',
      imageUrl: cleanUrl,
      cleanText: fullImgMatch[1] || '示意图',
      formattedHtml: ''
    };
  }

  // Obsidian 高亮头部语法 ==...== (如 ==读取SAVE1==、==◆2周目 save01==、==成就：Ria-ju==)
  if (trimmed.startsWith('==') && trimmed.endsWith('==') && trimmed.length > 4) {
    const inner = trimmed.slice(2, -2).trim();
    if (inner.includes('SAVE') && (inner.includes('读取') || inner.includes('LOAD'))) {
      return { raw: line, type: 'load-header', cleanText: inner, formattedHtml: formatGuideInline(inner) };
    }
    if (inner.includes('SAVE') || inner.includes('存档') || inner.includes('保存')) {
      return { raw: line, type: 'save-header', cleanText: inner, formattedHtml: formatGuideInline(inner) };
    }
    if (inner.includes('成就') || inner.includes('TIPS') || inner.includes('Tips')) {
      return { raw: line, type: 'tip', cleanText: inner, formattedHtml: formatGuideInline(inner) };
    }
    if (inner.includes('END') || inner.includes('End') || inner.includes('结局')) {
      return { raw: line, type: 'end-header', cleanText: inner, formattedHtml: formatGuideInline(inner) };
    }
    return { raw: line, type: 'section-header', cleanText: inner, formattedHtml: formatGuideInline(inner) };
  }

  // **强调小节标题与关键存档/结局横幅**
  if (trimmed.startsWith('**') && (trimmed.endsWith('**') || trimmed.endsWith('**  '))) {
    const inner = stripBanner(trimmed.replace(/\*\*/g, ''));
    if (inner.startsWith('注：')) {
      return { raw: line, type: 'tip', cleanText: inner, formattedHtml: formatGuideInline(inner) };
    }
    if (inner.includes('SAVE') && (inner.includes('读取') || inner.includes('LOAD'))) {
      return { raw: line, type: 'load-header', cleanText: inner, formattedHtml: formatGuideInline(inner) };
    }
    if (inner.includes('SAVE') || inner.includes('存档') || inner.includes('保存')) {
      return { raw: line, type: 'save-header', cleanText: inner, formattedHtml: formatGuideInline(inner) };
    }
    if (inner.includes('END') || inner.includes('End') || inner.includes('结局') || inner.includes('Normal') || inner.includes('True')) {
      return { raw: line, type: 'end-header', cleanText: inner, formattedHtml: formatGuideInline(inner) };
    }
    return { raw: line, type: 'section-header', cleanText: inner, formattedHtml: formatGuideInline(inner) };
  }

  // 地点移动导航 (如 （玄关）、（乙叶房间）、（1楼） 等移动指引)
  const locMatch = trimmed.match(/^[（\(]([^\)）]+)[）\)]$/);
  if (locMatch) {
    const locName = locMatch[1].trim();
    if (locName !== '无选项' && !locName.startsWith('探索模式') && locName.length <= 15) {
      return {
        raw: line,
        type: 'location',
        cleanText: locName,
        formattedHtml: `<span class="inline-block px-2 py-0.5 rounded-md text-[12px] font-sans font-semibold bg-raised text-muted border border-hairline whitespace-nowrap">移动至：${locName}</span>`
      };
    }
  }

  // 1. 圈号选择支 ①②③④⑤⑥⑦⑧⑨⑩...
  const circledDigits: Record<string, string> = {
    '①': '1', '②': '2', '③': '3', '④': '4', '⑤': '5',
    '⑥': '6', '⑦': '7', '⑧': '8', '⑨': '9', '⑩': '10',
    '⑪': '11', '⑫': '12', '⑬': '13', '⑭': '14', '⑮': '15',
    '⑯': '16', '⑰': '17', '⑱': '18', '⑲': '19', '⑳': '20'
  };
  if (trimmed.length >= 2 && circledDigits[trimmed[0]]) {
    const stepNum = circledDigits[trimmed[0]];
    const content = trimmed.slice(1).trim();
    return {
      raw: line,
      type: 'choice',
      stepNumber: stepNum,
      cleanText: content,
      formattedHtml: formatGuideInline(content)
    };
  }

  // 2. 括号数字选择支 【1】、[1]、(1)
  const bracketMatch = trimmed.match(/^[【\[\(](\d+)[】\]\)]\s*(.*)$/);
  if (bracketMatch) {
    return {
      raw: line,
      type: 'choice',
      stepNumber: bracketMatch[1],
      cleanText: bracketMatch[2],
      formattedHtml: formatGuideInline(bracketMatch[2])
    };
  }

  // 3. 中文数字选择支 一、 二、 三、
  const cnNumMatch = trimmed.match(/^([一二三四五六七八九十]+)[、\.]\s*(.*)$/);
  if (cnNumMatch) {
    return {
      raw: line,
      type: 'choice',
      stepNumber: cnNumMatch[1],
      cleanText: cnNumMatch[2],
      formattedHtml: formatGuideInline(cnNumMatch[2])
    };
  }

  // 3月5日、8月1日—5日、7月26日凌晨。必须在数字选项之前，否则「3月5日」会被当成第 3 项。
  // 只有日期是小节头；日期后面跟着当天安排（妻中蜜、天神乱漫按天走）是一步，不能整行当标题加粗
  const dated = trimmed.match(/^(\d{1,2}月\d{1,2}日(?:[—\-~～至]\d{1,2}日)?(?:凌晨|上午|中午|下午|晚上|午前|午后)?)\s*[:：]?\s*(.*)$/);
  if (dated) {
    const rest = dated[2].trim();
    if (!rest) return { raw: line, type: 'section-header', cleanText: dated[1], formattedHtml: formatGuideInline(dated[1]) };
    // 「10月8日 ○午前」只是日期加时段，仍是小节头（Fate hollow ataraxia）
    if (/^[○●◎]?\s*(?:午前|午後|午后|夜|朝|昼|夕方|深夜)$/.test(rest)) {
      return { raw: line, type: 'section-header', cleanText: trimmed, formattedHtml: formatGuideInline(trimmed) };
    }
    return dayLine(line, dated[1], rest);
  }

  // 7/28 14:53 这种短信时间戳：小节头，不是第 7 项。后面还有正文的同上按一天的一步
  const stamp = trimmed.match(/^(\d{1,2}\/\d{1,2}(?:\s+\d{1,2}:\d{2})?)(?:\s+(.*))?$/);
  if (stamp) {
    const rest = (stamp[2] || '').trim();
    if (!rest) return { raw: line, type: 'section-header', cleanText: trimmed, formattedHtml: formatGuideInline(trimmed) };
    return dayLine(line, stamp[1], rest);
  }

  // 4. 数字编号选择支 (1. 2. 10. 1★ 7 文字)。编号后必须是分隔符或空白，1.5倍、120#、2004年 都不算
  const choiceMatch = trimmed.match(/^(\d{1,3}(?:\.(?!\d)|[、★]|(?=\s)))\s*(.*)$/);
  if (choiceMatch) {
    const stepNum = choiceMatch[1].replace(/[\.、]/, '');
    const content = choiceMatch[2];
    return {
      raw: line,
      type: 'choice',
      stepNumber: stepNum,
      cleanText: content,
      formattedHtml: formatGuideInline(content)
    };
  }

  // 列表要点 (- 要点 / * 要点)
  if (trimmed.startsWith('- ') || trimmed.startsWith('* ')) {
    const content = trimmed.slice(2).trim();
    return {
      raw: line,
      type: 'bullet',
      cleanText: content,
      formattedHtml: formatGuideInline(content)
    };
  }

  // 二级分支条目 (└ 进入 / ├ / - 选择:)
  if (trimmed.startsWith('└') || trimmed.startsWith('├') || trimmed.startsWith('- 选择')) {
    return {
      raw: line,
      type: 'sub-choice',
      cleanText: trimmed,
      formattedHtml: formatGuideInline(trimmed)
    };
  }

  // 提示注解 (※ / 注：)
  if (trimmed.startsWith('※') || trimmed.startsWith('注：') || trimmed.startsWith('*注：*') || trimmed.startsWith('提示：')) {
    return {
      raw: line,
      type: 'tip',
      cleanText: trimmed,
      formattedHtml: formatGuideInline(trimmed)
    };
  }

  // 【3月5日】、【SAVE 1】、【自本编开始】。横线装饰不算正文。
  if (trimmed.startsWith('【') && trimmed.includes('】')) {
    const inner = trimmed.slice(1, trimmed.indexOf('】')).trim();
    if (/SAVE|存档|保存/i.test(inner)) {
      return { raw: line, type: 'save-header', cleanText: inner, formattedHtml: formatGuideInline(inner) };
    }
    if (/LOAD|LAOD|读取|承接/i.test(inner)) {
      return { raw: line, type: 'load-header', cleanText: inner, formattedHtml: formatGuideInline(inner) };
    }
    if (/Flag/i.test(inner)) {
      return { raw: line, type: 'tip', cleanText: trimmed, formattedHtml: formatGuideInline(trimmed) };
    }
    // 括号后面还有正文（【手机触发器】更改过壁纸）时整行保留，不能只剩括号里的字
    const rest = trimmed.slice(trimmed.indexOf('】') + 1).replace(/^[━─—\s]+$/, '').trim();
    const text = rest ? trimmed : inner;
    return { raw: line, type: 'section-header', cleanText: text, formattedHtml: formatGuideInline(text) };
  }

  // 全角地点 ［窗帘］。半角括号地点走上面的 location。
  const wideLoc = trimmed.match(/^［([^］]{1,16})］$/);
  if (wideLoc) {
    return {
      raw: line,
      type: 'location',
      cleanText: wideLoc[1],
      formattedHtml: `<span class="inline-block px-2 py-0.5 rounded-md text-[12px] font-sans font-semibold bg-raised text-muted border border-hairline whitespace-nowrap">移动至：${wideLoc[1]}</span>`
    };
  }

  // ◆调查 / ☆妄想 / ■事件 / ▼选项 / ●检查。仓库里这四种前缀反复出现。
  // 短的（◆人物选择、●检查表、■第六冠「暴食」■■■）是小节头；
  // 带说明或箭头的（▼007│吊坠⇒决定、■川岛家响子1：想被……）是要做的一步，按要点列
  const markLead = trimmed.match(/^[◆☆■▼●]\s*(.*)$/);
  if (markLead) {
    const body = stripBanner(markLead[1]);
    if (/SAVE|存档|保存/i.test(body)) {
      return { raw: line, type: 'save-header', cleanText: body, formattedHtml: formatGuideInline(body) };
    }
    if (body.length > 10 || /[⇒→│：:]/.test(body)) {
      return { raw: line, type: 'bullet', cleanText: body, formattedHtml: formatGuideInline(body) };
    }
    return {
      raw: line,
      type: 'section-header',
      cleanText: body || trimmed,
      formattedHtml: formatGuideInline(body || trimmed)
    };
  }

  // 「SAVE 5开始」「从SAVE3开始」：从这个存档读起
  if (/^(?:从|自)?\s*SAVE\s*\d+\s*(?:处)?(?:开始|继续)/i.test(trimmed)) {
    return { raw: line, type: 'load-header', cleanText: trimmed, formattedHtml: formatGuideInline(trimmed) };
  }

  return {
    raw: line,
    type: 'text',
    cleanText: trimmed,
    formattedHtml: formatGuideInline(trimmed)
  };
}

// 批量解析 Callout 内部行，支持将连续的 Markdown 表格行聚合为结构化表格
/** 表格一格里用 <br> 把一天的调查、证据、地点串起来。拆开后走同一套行分类。 */
export function splitPackedGuideCell(cell: string): string[] {
  return cell
    .replace(/<br\s*\/?>/gi, '\n')
    .split('\n')
    .map(part => part.replace(/^[\s\u3000]+/, '').trim())
    .filter(Boolean);
}

function boldMarks(line: string): number {
  return (line.match(/\*\*/g) || []).length;
}

export function parseCalloutLines(lines: string[]): FormattedCalloutLine[] {
  const result: FormattedCalloutLine[] = [];
  let idx = 0;

  while (idx < lines.length) {
    const l = lines[idx];
    const trimmed = l.trim();

    // 检查是否是 Markdown 表格开始行 (| col1 | col2 | 且下一行包含 ---)
    if (
      trimmed.startsWith('|') && 
      trimmed.endsWith('|') && 
      idx + 1 < lines.length && 
      lines[idx + 1].trim().startsWith('|') && 
      lines[idx + 1].trim().includes('---')
    ) {
      const tableLines: string[] = [];
      while (idx < lines.length && lines[idx].trim().startsWith('|') && lines[idx].trim().endsWith('|')) {
        tableLines.push(lines[idx].trim());
        idx++;
      }
      if (tableLines.length >= 2) {
        const rawHeaders = splitGuideTableRow(tableLines[0]);
        const rows: string[][] = [];
        const packed: string[] = [];
        for (let r = 2; r < tableLines.length; r++) {
          const cells = splitGuideTableRow(tableLines[r]);
          if (rawHeaders.length === 1 && cells.some(cell => /<br\s*\/?>/i.test(cell))) packed.push(...cells);
          else rows.push(cells);
        }
        if (rows.length) {
          result.push({
            raw: tableLines.join('\n'),
            type: 'table',
            cleanText: '数据表格',
            formattedHtml: '',
            tableData: { headers: rawHeaders, rows }
          });
        }
        for (const cell of packed) {
          for (const part of splitPackedGuideCell(cell)) result.push(parseCalloutLine(part));
        }
        continue;
      }
    }

    // Steam 搬来的攻略把粗体拆成两行（**Vision Projection / 投影的视觉**），合成一行再分类
    if (trimmed.startsWith('**') && boldMarks(trimmed) % 2 === 1) {
      const close = lines.slice(idx + 1, idx + 4).findIndex(next => boldMarks(next) % 2 === 1);
      if (close !== -1) {
        const parts = lines.slice(idx, idx + close + 2).map(part => part.trim()).filter(Boolean);
        result.push(parseCalloutLine(parts.join(' ')));
        idx += close + 2;
        continue;
      }
    }

    if (/<br\s*\/?>/i.test(trimmed)) {
      for (const part of splitPackedGuideCell(trimmed)) {
        result.push(parseCalloutLine(part));
      }
      idx++;
      continue;
    }

    const parsed = parseCalloutLine(l);
    // 如果是过滤掉的废弃行（返回了 cleanText 为空的 spacer，且原文并不是纯空行），则不压入结果
    if (parsed.type === 'spacer' && !parsed.cleanText && trimmed) {
      idx++;
      continue;
    }
    // 连续横线只留一条；空行不叠加，开头和横线旁边的空行不要（行尾双空格的笔记每两行就夹一个空行）
    const prev = result[result.length - 1];
    if (parsed.type === 'divider' && prev?.type === 'divider') {
      idx++;
      continue;
    }
    if (parsed.type === 'spacer' && (!prev || prev.type === 'spacer' || prev.type === 'divider')) {
      idx++;
      continue;
    }
    if (parsed.type === 'divider' && prev?.type === 'spacer') result.pop();

    // 过滤掉孤立的“地图”或“攻略”空标题与多余分割线 (如紧接着 ---)
    if ((parsed.cleanText === '地图' || parsed.cleanText === '攻略') && idx + 1 < lines.length && lines[idx + 1].trim().startsWith('---')) {
      idx += 2;
      continue;
    }

    result.push(parsed);
    idx++;
  }

  return result;
}

/** 路线折叠是 `> [!note]` 这种带类型的 callout。配图 `> [![](url)](link)` 不是。 */
export function isGuideCalloutStart(line: string): boolean {
  return /^>\s*\[!\w+\]/.test(line.trim());
}

// 解析完整的 Obsidian 攻略文本为结构化区块
export function parseObsidianGuide(markdown: string): ParsedGuide {
  const result: ParsedGuide = {
    title: '',
    overview: '',
    quickIndex: [],
    blocks: []
  };

  if (!markdown) return result;

  // 1. 分离并去除 YAML Frontmatter
  let body = markdown;
  if (body.startsWith('---')) {
    const endFm = body.indexOf('\n---', 3);
    if (endFm !== -1) {
      body = body.slice(endFm + 4).trim();
    }
  }

  // 移除正文中的引用标记。
  body = body.replace(/cite.*?/g, '').replace(/\[cite.*?\]/g, '');

  const lines = body.split(/\r?\n/);
  let idx = 0;
  let skippingSection = false;
  let currentH2Title = '';

  while (idx < lines.length) {
    const line = lines[idx];
    const trimmed = line.trim();

    if (!trimmed) {
      idx++;
      continue;
    }

    // 2. 识别 Markdown 标题 (#, ##, ###)
    if (trimmed.startsWith('#')) {
      const match = trimmed.match(/^(#{1,4})\s+(.*)$/);
      if (match) {
        const level = match[1].length;
        const titleText = match[2].trim();

        // 遇到 # 快速索引 或 # 快速导航：进入跳过模式，因为抽屉顶部已有吸顶大纲胶囊栏
        if (titleText.includes('快速索引') || titleText.includes('快速导航')) {
          skippingSection = true;
          idx++;
          continue;
        }

        // 遇到纯占位的 # 攻略 或 # 攻略正文：直接跳过
        if (titleText === '攻略' || titleText === '攻略正文' || titleText === '全线攻略') {
          skippingSection = false;
          idx++;
          continue;
        }

        // 遇到 # 游戏概述 或 # 游戏简介：提取文本到 overview，不作为突兀的一级标题占用攻略主屏
        if (titleText === '游戏概述' || titleText === '游戏简介') {
          idx++;
          const overviewLines: string[] = [];
          while (idx < lines.length && !lines[idx].trim().startsWith('#') && !isGuideCalloutStart(lines[idx])) {
            const ovTrim = lines[idx].trim();
            if (ovTrim && !ovTrim.startsWith('|') && !ovTrim.includes('---')) {
              overviewLines.push(ovTrim);
            }
            idx++;
          }
          result.overview = overviewLines
            .join(' ')
            .replace(/[\ue000-\uf8ff]|cite.*?|\[cite.*?\]/g, '')
            .replace(/原文未删减.*?。/g, '')
            .replace(/下方保留全流程原文.*?。/g, '')
            .replace(/按阶段折叠.*?。/g, '')
            .trim();
          continue;
        }

        // 其他正规标题：解除跳过状态
        skippingSection = false;

        const blockId = `heading-${result.blocks.length}-${encodeURIComponent(titleText.slice(0, 15))}`;

        if (!result.title && level === 1) {
          result.title = titleText;
        }

        if (level === 2) {
          const hClean = titleText.replace(/（.*?）|\(.*?\)/g, '').trim();
          if (!hClean.includes('快速索引') && !hClean.includes('游戏概述') && !hClean.includes('路线与结局') && hClean !== '攻略') {
            currentH2Title = hClean;
          }
        }

        result.blocks.push({
          type: 'heading',
          id: blockId,
          level,
          text: titleText
        });

        idx++;
        continue;
      }
    }

    // 如果处于跳过模式（如快速索引的列表、静态总结表格等）：跳过内容
    if (skippingSection) {
      idx++;
      continue;
    }

    // 3. 识别 Obsidian Callout 折叠块 (> [!type]- Title)
    if (isGuideCalloutStart(trimmed)) {
      skippingSection = false;
      const calloutMatch = trimmed.match(/^>\s*\[!(\w+)\]([+-]?)\s*(.*)$/);
      if (calloutMatch) {
        const rawType = calloutMatch[1].toLowerCase();
        let calloutType: GuideCallout['type'] = 'note';
        if (['tip', 'success', 'check'].includes(rawType)) calloutType = 'tip';
        else if (['abstract', 'summary', 'tldr'].includes(rawType)) calloutType = 'abstract';
        else if (['warning', 'caution', 'attention'].includes(rawType)) calloutType = 'warning';
        else if (['danger', 'error', 'bug'].includes(rawType)) calloutType = 'danger';
        else if (['info', 'todo'].includes(rawType)) calloutType = 'info';

        const foldSymbol = calloutMatch[2];
        const foldDefault = foldSymbol !== '+'; // 规范中默认为 '-' 折叠
        const rawCalloutTitle = calloutMatch[3].trim() || '攻略内容';
        const calloutTitle = rawCalloutTitle
          .replace(/[｜|]\s*折叠.*$/i, '')
          .replace(/[（\(]\s*折叠[^\)）]*[）\)]/gi, '')
          .replace(/[（\(]\s*文本行尾[^\)）]*[）\)]/gi, '')
          .replace(/[（\(]\s*内嵌\s*SVG[^\)）]*[）\)]/gi, '')
          .trim() || '攻略内容';

        const calloutLines: string[] = [];
        idx++;

        // 收集该 callout 块内的所有引用行
        while (idx < lines.length) {
          const cLine = lines[idx];
          const cTrim = cLine.trim();
          // 如果遇到了新的 Callout 开头，立即截断当前块，交由外层解析
          if (isGuideCalloutStart(cTrim)) {
            break;
          }
          if (cTrim.startsWith('>')) {
            // 清理一至两层引用标记 (> 或 > >)
            let cleanContent = cTrim.replace(/^>\s?/, '');
            if (cleanContent.startsWith('> ')) {
              cleanContent = cleanContent.slice(2);
            } else if (cleanContent.startsWith('>')) {
              cleanContent = cleanContent.slice(1);
            }
            calloutLines.push(cleanContent);
            idx++;
          } else if (!cTrim) {
            // 空行：向后探测，如果后续非空行仍属于本 Callout 的子引用内容（且不是新 Callout），则保留空行，否则断开
            let nextNonEmptyIdx = idx + 1;
            while (nextNonEmptyIdx < lines.length && !lines[nextNonEmptyIdx].trim()) {
              nextNonEmptyIdx++;
            }
            if (
              nextNonEmptyIdx < lines.length &&
              lines[nextNonEmptyIdx].trim().startsWith('>') &&
              !isGuideCalloutStart(lines[nextNonEmptyIdx])
            ) {
              calloutLines.push('');
              idx++;
            } else {
              break;
            }
          } else {
            break;
          }
        }

        const parsedLines = parseCalloutLines(calloutLines);

        result.blocks.push({
          type: 'callout',
          callout: {
            type: calloutType,
            title: calloutTitle,
            foldDefault,
            content: calloutLines.join('\n'),
            lines: calloutLines,
            parsedLines,
            sectionHeading: currentH2Title || undefined
          }
        });

        continue;
      }
    }

    // 4. 识别 Markdown 表格 (| ... |)
    if (trimmed.startsWith('|') && trimmed.endsWith('|') && trimmed.includes('|')) {
      const headers = splitGuideTableRow(trimmed);

      idx++;
      if (idx < lines.length && lines[idx].trim().startsWith('|') && lines[idx].includes('---')) {
        idx++;
      }

      const rows: string[][] = [];
      while (idx < lines.length) {
        const rowLine = lines[idx].trim();
        if (rowLine.startsWith('|') && rowLine.endsWith('|')) {
          const cells = splitGuideTableRow(rowLine);
          rows.push(cells);
          idx++;
        } else {
          break;
        }
      }

      result.blocks.push({
        type: 'table',
        table: { headers, rows }
      });
      continue;
    }

    // 5. 章节正文走和路线卡同一套行分类，不再落成纯段落。
    const prose: string[] = [];
    while (idx < lines.length) {
      const proseLine = lines[idx];
      const proseTrim = proseLine.trim();
      if (!proseTrim) {
        if (prose.length && prose[prose.length - 1] !== '') prose.push('');
        idx++;
        continue;
      }
      if (proseTrim.startsWith('#') || isGuideCalloutStart(proseTrim) || (proseTrim.startsWith('|') && proseTrim.endsWith('|'))) break;
      prose.push(proseLine);
      idx++;
    }
    if (!prose.length) {
      idx++;
      continue;
    }
    const parsedLines = parseCalloutLines(prose);
    if (parsedLines.length) {
      result.blocks.push({ type: 'lines', lines: parsedLines });
    }
  }

  // 6. 智能收敛提炼路线大纲索引 (消除 H2 与随后紧邻 Callout 产生的双重冗余胶囊)
  const dedupedQuickIndex: Array<{ title: string; targetText: string }> = [];
  for (let bIdx = 0; bIdx < result.blocks.length; bIdx++) {
    const b = result.blocks[bIdx];
    if (b.type === 'heading' && b.level === 2 && b.text) {
      const hText = b.text.replace(/（.*?）|\(.*?\)/g, '').trim();
      if (!hText.includes('快速索引') && !hText.includes('游戏概述') && !hText.includes('路线与结局') && hText !== '攻略') {
        dedupedQuickIndex.push({
          title: hText,
          targetText: b.text
        });
      }
    } else if (b.type === 'callout' && b.callout) {
      const prevBlock = bIdx > 0 ? result.blocks[bIdx - 1] : null;
      const prevIsH2 = prevBlock && prevBlock.type === 'heading' && prevBlock.level === 2;
      // 只有当该 Callout 前面没有紧邻的 H2 标题时，才将 Callout 自身作为大纲胶囊
      if (!prevIsH2) {
        const cTitle = b.callout.title
          .replace(/[｜|].*$/, '')
          .replace(/（.*?）|\(.*?\)/g, '')
          .trim();
        if (cTitle && !cTitle.includes('回到索引') && !dedupedQuickIndex.some(q => q.title === cTitle || q.targetText === b.callout!.title)) {
          dedupedQuickIndex.push({
            title: cTitle,
            targetText: b.callout.title
          });
        }
      }
    }
  }
  result.quickIndex = dedupedQuickIndex;

  return result;
}
