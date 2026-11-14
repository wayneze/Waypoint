import type { CSSProperties } from 'vue';

/** 所有裁切坐标均以原图宽度为 1；纵向长图尾部的额外内容不影响单格大小。 */
export interface ImageView {
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export function imageViewAspectRatio(view: ImageView): number {
  return view.width / view.height;
}

export function imageViewStyle(view: ImageView): CSSProperties {
  return {
    width: `${100 / view.width}%`,
    height: 'auto',
    left: `${-100 * view.x / view.width}%`,
    top: `${-100 * view.y / view.height}%`,
  };
}

export function resolveWearViews(src: string, width: number, height: number): ImageView[] {
  let bili = false;
  try {
    const url = new URL(src);
    bili = (url.hostname === 'wiki.biligame.com' && url.pathname.startsWith('/sky/'))
      || (url.hostname === 'patchwiki.biligame.com' && url.pathname.startsWith('/images/sky/'));
  } catch { /* 非来源地址按单图展示。 */ }

  // B 站光遇展示图：首格图标，第二格正面，共 5 个正方格；其余朝向顺序因图而异。
  // 不把任意来源的长图、其他比例或未知拼图推断成此格式。
  if (bili && height > 0 && width === height * 5) {
    return ['正面', '视角 2', '视角 3', '视角 4'].map((title, index) => ({
      title, x: (index + 1) / 5, y: 0, width: 1 / 5, height: 1 / 5,
    }));
  }
  return [{ title: '原图', x: 0, y: 0, width: 1, height: height / width }];
}
