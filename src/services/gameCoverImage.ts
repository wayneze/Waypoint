import type { CoverPosition, GameLibraryDetails } from '../types';
import { isHttp } from './exploreSites';

export function isCoverSource(value: string): boolean {
  return isHttp(value) || (value.length <= 2_000_000 && /^data:image\/jpeg;base64,\/9j\/[A-Za-z0-9+/]*={0,2}$/.test(value));
}

export function coverPosition(position?: CoverPosition) {
  return `${position?.x ?? 50}% ${position?.y ?? 50}%`;
}
export function detailBackground(library?: GameLibraryDetails, poster?: string) {
  return library?.background ?? (library?.metadata?.background || poster || '');
}

/** 封面和背景共用的图片导入入口，缩小后随档案一起保存。 */
export async function importCoverImage(file: File): Promise<string> {
  if (!['image/jpeg', 'image/png', 'image/webp', 'image/gif'].includes(file.type)) {
    throw new Error('请选择 JPG、PNG、WebP 或 GIF 图片');
  }
  if (file.size > 20 * 1024 * 1024) throw new Error('图片超过 20 MB，请先缩小后再导入');
  const bitmap = await createImageBitmap(file);
  try {
    const ratio = Math.min(1, 1600 / Math.max(bitmap.width, bitmap.height));
    const canvas = document.createElement('canvas');
    canvas.width = Math.max(1, Math.round(bitmap.width * ratio));
    canvas.height = Math.max(1, Math.round(bitmap.height * ratio));
    const context = canvas.getContext('2d');
    if (!context) throw new Error('无法处理图片，请重新选择');
    context.fillStyle = '#29281f';
    context.fillRect(0, 0, canvas.width, canvas.height);
    context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
    const data = canvas.toDataURL('image/jpeg', 0.86);
    if (!isCoverSource(data)) throw new Error('处理后的图片仍过大，请选择尺寸更小的图片');
    return data;
  } finally { bitmap.close(); }
}
