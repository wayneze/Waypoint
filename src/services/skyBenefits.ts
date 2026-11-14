import { invoke, isTauri } from '@tauri-apps/api/core';

export interface BenefitsBounds { x: number; y: number; width: number; height: number }
export type BenefitsPage = 'checkin' | 'recharge';
export type BenefitsAction = 'back' | 'home';

// Serialize creation, resizing and hiding, including quick tab changes and remounts.
let viewUpdates: Promise<void> = Promise.resolve();
export function setSkyBenefitsView(page: BenefitsPage, bounds: BenefitsBounds | null): Promise<void> {
  const update = viewUpdates.then(() => {
    if (!isTauri()) {
      if (bounds) throw new Error('官方福利需要在 Waypoint 桌面版中打开。');
      return;
    }
    return invoke<void>('set_sky_benefits_view', { page, bounds });
  });
  viewUpdates = update.catch(() => {});
  return update;
}

export function navigateSkyBenefits(page: BenefitsPage, action: BenefitsAction): Promise<void> {
  return invoke('navigate_sky_benefits', { page, action });
}
