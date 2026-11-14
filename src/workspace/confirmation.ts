import { shallowRef } from 'vue';

export interface Confirmation {
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel?: string;
  details?: { label: string; value: string }[];
}

export const confirmation = shallowRef<Confirmation | null>(null);
let answer: ((accepted: boolean) => void) | null = null;

export function askConfirmation(options: Confirmation): Promise<boolean> {
  // A second operation cannot replace a decision the user is already making.
  if (answer) return Promise.resolve(false);
  confirmation.value = options;
  return new Promise(resolve => { answer = resolve; });
}

export function finishConfirmation(accepted: boolean) {
  const resolve = answer;
  answer = null;
  confirmation.value = null;
  resolve?.(accepted);
}

export function formatConfirmationTime(value: string | null | undefined, empty: string): string {
  if (!value) return empty;
  const local = value.match(/^(\d{4}-\d{2}-\d{2})[ T](\d{2}:\d{2})(?::\d{2})?$/);
  if (local) return `${local[1]} ${local[2]}`;
  const date = new Date(value);
  if (!Number.isFinite(date.getTime())) return '时间未知';
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}
