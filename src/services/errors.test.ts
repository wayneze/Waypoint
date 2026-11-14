import { test } from 'vitest';
import { describeError } from './errors';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

test('英文原始错误归类成中文，后端中文原样保留', () => {
  const t = describeError(new Error('error sending request for url (https://x): operation timed out'));
  assert(t.code === 'timeout' && !/timed/.test(t.message), JSON.stringify(t));
  assert(describeError('database is locked').code === 'busy', 'busy');
  const zh = describeError('WebDAV 返回 401，账号或密码不对');
  assert(zh.code === 'auth' && zh.message === 'WebDAV 返回 401，账号或密码不对', JSON.stringify(zh));
  assert(describeError('奇怪的错误').message === '奇怪的错误', 'cjk passthrough');
});
