import type { TaskItem } from '../types';

/**
 * 智能 Markdown 与纯文本解析器
 * 可自动识别：
 * 1. Markdown 标准表格 (| 任务 | 类别 | ...)
 * 2. Markdown 任务清单 (- [ ] 任务名)
 * 3. 常见分行纯文本 (每行一项)
 */
export function parseMarkdownTasks(
  rawText: string,
  characterId: string,
  resetRules: { daily?: string; weekly?: string } = {}
): TaskItem[] {
  const dailyReset = resetRules.daily || '00:00';
  const weeklyReset = resetRules.weekly || '周一 00:00';
  const lines = rawText.split(/\r?\n/).map(l => l.trim()).filter(l => l.length > 0);
  const tasks: TaskItem[] = [];

  if (lines.length === 0) return tasks;

  // 检测是否为 Markdown 表格
  const isTable = lines.some(l => l.startsWith('|') && l.endsWith('|')) && lines.some(l => l.includes('---'));

  if (isTable) {
    // 过滤掉分割线 |---|---|
    const tableLines = lines.filter(l => l.startsWith('|') && !l.includes('---'));
    if (tableLines.length >= 1) {
      // 首行为表头，忽略第一行，从数据行开始解析

      // 其余行为数据
      for (let i = 1; i < tableLines.length; i++) {
        const cells = tableLines[i]
          .split('|')
          .map(c => c.trim())
          .filter(c => c.length > 0);
        
        if (cells.length > 0) {
          const title = cells[0];
          const secondCol = (cells[1] || '').toLowerCase();
          const category: 'daily' | 'weekly' | 'custom' = secondCol.includes('week') || secondCol.includes('周') ? 'weekly' : 'daily';
          const tag = cells[2] || (category === 'daily' ? '日常' : '周常');

          tasks.push({
            id: `import_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
            character_id: characterId,
            title,
            category,
            reset_rule: category === 'daily' ? dailyReset : weeklyReset,
            source_type: 'custom',
            tag,
            done: false
          });
        }
      }
      return tasks;
    }
  }

  // 检测是否为 Markdown 待办清单 (- [ ] / * [ ])
  for (const line of lines) {
    const todoMatch = line.match(/^[-*]\s*\[([ xX])\]\s*(.+)$/);
    if (todoMatch) {
      const isDone = todoMatch[1].toLowerCase() === 'x';
      const text = todoMatch[2].trim();
      const isWeekly = text.includes('周常') || text.includes('每周') || text.includes('CD');

      tasks.push({
        id: `import_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
        character_id: characterId,
        title: text,
        category: isWeekly ? 'weekly' : 'daily',
        reset_rule: isWeekly ? weeklyReset : dailyReset,
        source_type: 'custom',
        tag: isWeekly ? '周常' : '日常',
        done: isDone
      });
      continue;
    }

    // 普通列表行 (- 任务 或 1. 任务)
    const listMatch = line.match(/^([-*]|\d+\.)\s*(.+)$/);
    const content = listMatch ? listMatch[2].trim() : line;

    if (content.length > 0 && !content.startsWith('#')) {
      const isWeekly = content.includes('周常') || content.includes('每周') || content.includes('CD');
      tasks.push({
        id: `import_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
        character_id: characterId,
        title: content,
        category: isWeekly ? 'weekly' : 'daily',
        reset_rule: isWeekly ? weeklyReset : dailyReset,
        source_type: 'custom',
        tag: isWeekly ? '周常' : '自建',
        done: false
      });
    }
  }

  return tasks;
}
