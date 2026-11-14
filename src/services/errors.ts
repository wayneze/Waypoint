/**
 * 把后端和网络的原始错误归类成 {code, message}，界面只显示中文说明。
 * 后端自己写的中文错误原样保留；英文的 reqwest / IO / SQLite 错误按特征归类。
 */
export type ErrorCode = 'timeout' | 'network' | 'auth' | 'not_found' | 'busy' | 'disk' | 'unknown';

export interface AppError {
  code: ErrorCode;
  message: string;
  /** 原始错误，放在提示的悬停信息或日志里 */
  raw: string;
}

const RULES: Array<[ErrorCode, RegExp, string]> = [
  ['timeout', /timed? ?out|timeout|deadline/i, '请求超时，网络慢或对方没响应'],
  ['network', /dns|connect|connection|network|unreachable|os error 10061|os error 11001|error sending request/i, '连不上网络或对方服务器'],
  ['auth', /401|403|unauthori[sz]ed|forbidden/i, '账号或密码不对，或没有权限'],
  ['not_found', /404|not found|no such file|cannot find/i, '要找的文件或地址不存在'],
  ['busy', /database is locked|busy|being used by another process|os error 32/i, '文件正被占用，稍后再试'],
  ['disk', /no space|disk full|os error 112|permission denied|os error 5\b/i, '磁盘空间不足或没有写入权限'],
];

const HAS_CJK = /[一-鿿]/;

export function describeError(err: unknown): AppError {
  const raw = err instanceof Error ? err.message : typeof err === 'string' ? err : JSON.stringify(err);
  for (const [code, re, message] of RULES) {
    if (re.test(raw)) {
      // 后端已经写了中文说明的，用它自己的话
      return { code, message: HAS_CJK.test(raw) ? raw : message, raw };
    }
  }
  return { code: 'unknown', message: HAS_CJK.test(raw) ? raw : `出错了：${raw}`, raw };
}
