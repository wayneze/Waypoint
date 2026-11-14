// 全局热键。主面板和记一笔各一条，存在 app_settings。

export const HOTKEY_PANEL_KEY = 'hotkey_panel';
export const HOTKEY_CAPTURE_KEY = 'hotkey_capture';
export const DEFAULT_PANEL_HOTKEY = 'Alt+W';
export const DEFAULT_CAPTURE_HOTKEY = 'Alt+M';

const MODS = ['ctrl', 'alt', 'shift', 'super', 'meta', 'command', 'cmd'] as const;

export function normalizeHotkey(raw: string): string | null {
  const parts = raw
    .trim()
    .toLowerCase()
    .split('+')
    .map(part => part.trim())
    .filter(Boolean);
  if (parts.length < 2) return null;
  const key = parts[parts.length - 1];
  const mods = parts.slice(0, -1);
  if (!key || key.length > 12 || MODS.includes(key as (typeof MODS)[number])) return null;
  if (mods.some(mod => !MODS.includes(mod as (typeof MODS)[number]))) return null;
  if (new Set(mods).size !== mods.length) return null;
  const order = MODS.filter(mod => mods.includes(mod));
  const named = order.map(mod => (mod === 'meta' || mod === 'command' || mod === 'cmd' ? 'Super' : mod[0].toUpperCase() + mod.slice(1)));
  const displayKey = key === 'space' ? 'Space' : key.length === 1 ? key.toUpperCase() : key[0].toUpperCase() + key.slice(1);
  return [...named, displayKey].join('+');
}


