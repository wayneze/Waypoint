import { expect, test, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: vi.fn() }));
import { invoke } from '@tauri-apps/api/core';
import { setSkyBenefitsView } from './skyBenefits';

test('switching away during creation hides the completed view before another mount can show it', async () => {
  let finishCreation!: () => void;
  vi.mocked(invoke).mockImplementationOnce(() => new Promise<void>(resolve => { finishCreation = resolve; }));
  const bounds = { x: 200, y: 180, width: 800, height: 500 };
  const first = setSkyBenefitsView('checkin', bounds);
  await Promise.resolve();
  const hide = setSkyBenefitsView('checkin', null);
  const returned = setSkyBenefitsView('recharge', { ...bounds, width: 600 });
  expect(invoke).toHaveBeenCalledTimes(1);
  finishCreation();
  await Promise.all([first, hide, returned]);
  expect(vi.mocked(invoke).mock.calls.map(call => call[1])).toEqual([
    { page: 'checkin', bounds }, { page: 'checkin', bounds: null }, { page: 'recharge', bounds: { ...bounds, width: 600 } },
  ]);

  vi.mocked(invoke).mockRejectedValueOnce(new Error('creation failed'));
  await expect(setSkyBenefitsView('checkin', bounds)).rejects.toThrow('creation failed');
  await expect(setSkyBenefitsView('checkin', null)).resolves.toBeUndefined();
});
