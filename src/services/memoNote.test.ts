import { test } from 'vitest';
import { isWeekdayActive, noteParts, noteStepProgress, parseWeekdays, toggleNoteStep, weekdayLabel } from './memoNote.ts';
import { memoVisibleOnBoard } from './memoBoard.ts';
import type { CustomMemo } from '../types/index.ts';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

test('memoNote', () => {
  const note = '先看 https://example.com/a.\n- [ ] 扬州对话\n- [x] 准备金条';
  const parts = noteParts(note);
  assert(parts.some(part => part.kind === 'link' && part.url === 'https://example.com/a'), 'link keeps trailing dot out');
  assert(noteStepProgress(note).total === 2 && noteStepProgress(note).done === 1, 'step progress');
  const toggled = toggleNoteStep(note, 0);
  assert(toggled.includes('- [x] 扬州对话'), 'toggle first step');
  assert(toggled.includes('- [x] 准备金条'), 'second step stays');

  assert(weekdayLabel(parseWeekdays('5,6,0')) === '周五六日', weekdayLabel(parseWeekdays('5,6,0')));
  assert(isWeekdayActive('', new Date('2026-09-23T12:00:00')), 'empty weekdays always active');
  assert(isWeekdayActive('3', new Date('2026-09-23T12:00:00')), 'wednesday active');
  assert(!isWeekdayActive('5,6,0', new Date('2026-09-23T12:00:00')), 'weekend inactive on wednesday');

  function memo(partial: Partial<CustomMemo>): CustomMemo {
    return {
      id: 'm1',
      title: '买马草',
      tag: '生活',
      cycle_type: 'once',
      has_progress: false,
      current_progress: 0,
      target_progress: 0,
      done: false,
      created_at: '2026-09-23',
      ...partial,
    };
  }

  const loose = memo({});
  const owned = memo({ character_id: 'c_a' });
  const finished = memo({ done: true });
  assert(memoVisibleOnBoard(loose, 'all') && !memoVisibleOnBoard(loose, 'c_a'), 'unscoped memo stays on overview');
  assert(memoVisibleOnBoard(owned, 'all') && memoVisibleOnBoard(owned, 'c_a') && !memoVisibleOnBoard(owned, 'c_b'), 'owned memo follows its character');
  assert(!memoVisibleOnBoard(finished, 'all'), 'done memo leaves the board');
  assert(memoVisibleOnBoard(finished, 'all', 'other', true), 'completed view includes done memo');
  assert(!memoVisibleOnBoard({ ...owned, done: true }, 'c_b', 'other', true), 'completed view keeps owner isolation');
  assert(memoVisibleOnBoard(memo({ cycle_type: 'weekly' }), 'all', 'weekly'), 'weekly lands in week');
  assert(!memoVisibleOnBoard(memo({ cycle_type: 'daily' }), 'all', 'other'), 'daily does not land in other');
});
