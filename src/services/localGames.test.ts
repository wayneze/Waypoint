import { expect, test, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { gameLaunchOptions, launchGameOption, preferredLaunchOption } from './localGames';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => {}) }));

test('launch selection keeps the selected directory across reorder and dispatches only the selected client', async () => {
  const installations = ['C:/Games/edition-a', 'D:/Games/edition-b'].map(directory => ({ root: directory, directory, executable: `${directory}/game.exe`, candidates: [] }));
  const available = Object.fromEntries(installations.map(i => [i.directory, true]));
  const options = gameLaunchOptions(installations, true, true, available, true);
  expect(preferredLaunchOption(options)).toBe('steam');
  const preferred = options[2]!.key;
  const reordered = gameLaunchOptions([...installations].reverse(), true, true, available, true);
  expect(preferredLaunchOption(reordered, preferred)).toBe(preferred);
  await launchGameOption('record', reordered.find(o => o.key === preferred)!);
  expect(invoke).toHaveBeenLastCalledWith('launch_library_game', { recordId: 'record', installationIndex: 0 });
  await launchGameOption('record', options[0]!);
  expect(invoke).toHaveBeenLastCalledWith('launch_steam_game', { recordId: 'record' });
  expect(preferredLaunchOption(options.slice(0, 2), preferred)).toBe('');
  const offline = gameLaunchOptions(installations, true, true, {}, true);
  expect(preferredLaunchOption(offline, preferred)).toBe(preferred);
  await expect(launchGameOption('record', offline[2]!)).rejects.toThrow('不可用');
  expect(invoke).toHaveBeenCalledTimes(2);
});
