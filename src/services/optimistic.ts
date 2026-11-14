/**
 * 乐观写入：先改界面，再保存；保存失败就撤回界面改动并把错误抛给全局提示。
 * `apply` 和 `revert` 只动内存状态，`save` 负责落库。
 */
export async function optimistic(apply: () => void, revert: () => void, save: () => Promise<unknown>): Promise<void> {
  apply();
  try {
    await save();
  } catch (e) {
    revert();
    throw e;
  }
}
