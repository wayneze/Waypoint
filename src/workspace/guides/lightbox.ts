// 攻略阅读器灯箱。
import { nextTick, ref, reactive, computed } from 'vue';
import type { ImageView } from '../../services/imageViews';
import { normalizeSvgUrl } from '../../services/guideParser';

// 高清交互看图灯箱 (Lightbox) 状态与接口
export interface OpenLightboxOptions {
  theme?: 'paper' | 'dark' | 'blueprint';
  sectionIndex?: number;
  views?: ImageView[];
}

export const lightboxViewportRef = ref<HTMLDivElement | null>(null);

export function bindLightboxViewport(el: unknown) {
  lightboxViewportRef.value = el instanceof HTMLDivElement ? el : null;
}

export const lightboxState = reactive({
  isOpen: false,
  imageUrl: '',
  imageAlt: '',
  zoom: 1.0,
  panX: 0,
  panY: 0,
  isDragging: false,
  dragStartX: 0,
  dragStartY: 0,
  theme: 'dark' as 'paper' | 'dark' | 'blueprint',
  invert: false,
  // 多格/分段裁切与导览支持
  get totalSections() { return this.sections.length; },
  currentSection: -1, // -1 表示完整全图，0..N 表示聚焦单格
  sections: [] as ImageView[],
  wheelMode: 'zoom' as 'zoom' | 'scroll' // 滚轮中心缩放 vs 纵向滚动浏览
});

// 各图片卡片的内联背景主题偏好 (默认 'paper' 或按类型自适应)
export const inlineImageThemes = reactive<Record<string, 'paper' | 'dark'>>({});

export function getInlineImageTheme(url: string, alt: string = ''): 'paper' | 'dark' {
  if (inlineImageThemes[url]) return inlineImageThemes[url];
  if (url.includes('sinaimg.cn') || alt.includes('光遇') || alt.includes('实景') || alt.includes('带圈') || alt.includes('季蜡') || alt.includes('任务') || alt.includes('大蜡') || alt.includes('代币')) {
    return 'dark';
  }
  const isDiagram = url.startsWith('data:image/svg') || url.includes('.svg') || alt.includes('平面') || alt.includes('示意');
  return isDiagram ? 'paper' : 'dark';
}

export function toggleInlineImageTheme(url: string, alt: string = '') {
  const current = getInlineImageTheme(url, alt);
  inlineImageThemes[url] = current === 'paper' ? 'dark' : 'paper';
}

export function openLightbox(url: string, alt: string = '', options?: OpenLightboxOptions) {
  lightboxState.isOpen = true;
  lightboxState.imageUrl = normalizeSvgUrl(url);
  lightboxState.imageAlt = alt || '示意图';
  lightboxState.panX = 0;
  lightboxState.panY = 0;
  lightboxState.isDragging = false;
  lightboxState.theme = options?.theme || getInlineImageTheme(url, alt);
  lightboxState.invert = false;
  lightboxState.sections = options?.views || [];
  lightboxState.currentSection = typeof options?.sectionIndex === 'number' ? options.sectionIndex : -1;
  lightboxState.wheelMode = 'zoom';
  lightboxState.zoom = 1.0;
  void nextTick(() => fitLightboxWhenReady());
}

function fitLightboxWhenReady() {
  const img = lightboxViewportRef.value?.querySelector('img');
  if (!img) return;
  if (img.complete && img.naturalWidth) fitLightboxToScreen();
  else img.addEventListener('load', () => fitLightboxToScreen(), { once: true });
}

export function closeLightbox() {
  lightboxState.isOpen = false;
  lightboxState.isDragging = false;
}

export function setLightboxSection(idx: number) {
  if (idx < -1 || idx >= lightboxState.sections.length) return;
  lightboxState.currentSection = idx;
  lightboxState.panX = 0;
  lightboxState.panY = 0;
  lightboxState.zoom = 1.0;
}

export const currentLightboxView = computed(() => lightboxState.sections[lightboxState.currentSection]);

export function nextLightboxSection() {
  if (lightboxState.totalSections <= 0) return;
  if (lightboxState.currentSection < lightboxState.totalSections - 1) {
    setLightboxSection(lightboxState.currentSection + 1);
  } else {
    setLightboxSection(0);
  }
}

export function prevLightboxSection() {
  if (lightboxState.totalSections <= 0) return;
  if (lightboxState.currentSection > 0) {
    setLightboxSection(lightboxState.currentSection - 1);
  } else {
    setLightboxSection(lightboxState.totalSections - 1);
  }
}

export function toggleLightboxWheelMode() {
  lightboxState.wheelMode = lightboxState.wheelMode === 'zoom' ? 'scroll' : 'zoom';
}

export function handleLightboxWheel(e: WheelEvent) {
  e.preventDefault();

  // 如果是纵向滚动模式且未按住 Ctrl 键，则平滑上下漫游
  if (lightboxState.wheelMode === 'scroll' && !e.ctrlKey) {
    lightboxState.panY -= e.deltaY * 0.9;
    return;
  }

  // 鼠标光标为中心的几何平滑缩放
  const container = lightboxViewportRef.value;
  if (!container) return;

  const rect = container.getBoundingClientRect();
  const mouseX = e.clientX - rect.left - rect.width / 2;
  const mouseY = e.clientY - rect.top - rect.height / 2;

  const zoomFactor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
  const oldZoom = lightboxState.zoom;
  const newZoom = Math.min(Math.max(0.15, oldZoom * zoomFactor), 8.0);

  // 保持光标所在点在缩放后世界坐标不动
  lightboxState.panX = mouseX - (mouseX - lightboxState.panX) * (newZoom / oldZoom);
  lightboxState.panY = mouseY - (mouseY - lightboxState.panY) * (newZoom / oldZoom);
  lightboxState.zoom = Number(newZoom.toFixed(2));
}

export function zoomInLightbox() {
  const oldZoom = lightboxState.zoom;
  const newZoom = Math.min(8.0, oldZoom * 1.25);
  const ratio = newZoom / oldZoom;
  lightboxState.panX = lightboxState.panX * ratio;
  lightboxState.panY = lightboxState.panY * ratio;
  lightboxState.zoom = Number(newZoom.toFixed(2));
}

export function zoomOutLightbox() {
  const oldZoom = lightboxState.zoom;
  const newZoom = Math.max(0.15, oldZoom / 1.25);
  const ratio = newZoom / oldZoom;
  lightboxState.panX = lightboxState.panX * ratio;
  lightboxState.panY = lightboxState.panY * ratio;
  lightboxState.zoom = Number(newZoom.toFixed(2));
}

export function resetLightboxZoom() {
  lightboxState.zoom = 1.0;
  lightboxState.panX = 0;
  lightboxState.panY = 0;
}

export function fitLightboxToScreen() {
  const container = lightboxViewportRef.value;
  lightboxState.panX = 0;
  lightboxState.panY = 0;
  if (!container || lightboxState.currentSection >= 0) {
    lightboxState.zoom = 1.0;
    return;
  }
  const img = container.querySelector('img');
  const nw = img?.naturalWidth || 0;
  const nh = img?.naturalHeight || 0;
  if (!nw || !nh) {
    lightboxState.zoom = 1.0;
    return;
  }
  const vw = Math.max(320, container.clientWidth - 96);
  const vh = Math.max(200, container.clientHeight - 140);
  lightboxState.zoom = Number(Math.max(0.2, Math.min(vw / nw, vh / nh, 4)).toFixed(2));
}

export function startLightboxPan(e: MouseEvent) {
  if (e.button !== 0) return; // 仅限鼠标左键
  lightboxState.isDragging = true;
  lightboxState.dragStartX = e.clientX - lightboxState.panX;
  lightboxState.dragStartY = e.clientY - lightboxState.panY;

  const onGlobalMouseMove = (moveEv: MouseEvent) => {
    if (!lightboxState.isDragging) return;
    lightboxState.panX = moveEv.clientX - lightboxState.dragStartX;
    lightboxState.panY = moveEv.clientY - lightboxState.dragStartY;
  };

  const onGlobalMouseUp = () => {
    lightboxState.isDragging = false;
    window.removeEventListener('mousemove', onGlobalMouseMove);
    window.removeEventListener('mouseup', onGlobalMouseUp);
  };

  window.addEventListener('mousemove', onGlobalMouseMove);
  window.addEventListener('mouseup', onGlobalMouseUp);
}

export function handleLightboxDblClick(e: MouseEvent) {
  if (lightboxState.zoom > 1.2) {
    resetLightboxZoom();
  } else {
    const container = lightboxViewportRef.value;
    if (!container) return;
    const rect = container.getBoundingClientRect();
    const mouseX = e.clientX - rect.left - rect.width / 2;
    const mouseY = e.clientY - rect.top - rect.height / 2;
    const newZoom = 2.0;
    lightboxState.panX = mouseX - (mouseX - lightboxState.panX) * (newZoom / lightboxState.zoom);
    lightboxState.panY = mouseY - (mouseY - lightboxState.panY) * (newZoom / lightboxState.zoom);
    lightboxState.zoom = newZoom;
  }
}
