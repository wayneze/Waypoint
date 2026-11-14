<script setup lang="ts">
import {
  closeLightbox,
  fitLightboxToScreen,
  handleLightboxDblClick,
  handleLightboxWheel,
  lightboxState,
  currentLightboxView,
  bindLightboxViewport,
  nextLightboxSection,
  prevLightboxSection,
  resetLightboxZoom,
  setLightboxSection,
  startLightboxPan,
  toggleLightboxWheelMode,
  zoomInLightbox,
  zoomOutLightbox,
} from '../workspace/guides/lightbox';
import { imageViewAspectRatio, imageViewStyle } from '../services/imageViews';
import { nextTick, ref, watch } from 'vue';
import { ChevronLeft, ChevronRight, Compass, Moon, RotateCcw, Sun, X, ZoomIn, ZoomOut } from '@lucide/vue';

const dialog = ref<HTMLElement | null>(null);
let previousFocus: HTMLElement | null = null;
watch(() => lightboxState.isOpen, async open => {
  if (open) {
    previousFocus = document.activeElement as HTMLElement;
    await nextTick();
    dialog.value?.focus();
  } else previousFocus?.focus();
});

function keydown(event: KeyboardEvent) {
  // 灯箱持有全部看图快捷键，关闭后才交还工作台。
  let handled = true;
  switch (event.key) {
    case 'Escape': closeLightbox(); break;
    case 'ArrowLeft':
      if (currentLightboxView.value) prevLightboxSection();
      else lightboxState.panX += 60;
      break;
    case 'ArrowRight':
      if (currentLightboxView.value) nextLightboxSection();
      else lightboxState.panX -= 60;
      break;
    case 'ArrowUp': lightboxState.panY += 60; break;
    case 'ArrowDown': lightboxState.panY -= 60; break;
    case '+': case '=': zoomInLightbox(); break;
    case '-': zoomOutLightbox(); break;
    case '0': resetLightboxZoom(); break;
    default:
      if (/^[1-9]$/.test(event.key) && Number(event.key) <= lightboxState.totalSections) {
        setLightboxSection(Number(event.key) - 1);
      } else handled = false;
  }
  if (handled) { event.preventDefault(); event.stopPropagation(); }
  if (event.key === 'Tab') {
    const buttons = Array.from(dialog.value?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') || []);
    const first = buttons[0], last = buttons[buttons.length - 1];
    if (event.shiftKey && (document.activeElement === first || document.activeElement === dialog.value)) {
      event.preventDefault(); last?.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault(); first?.focus();
    }
  }
}
</script>

<template>
  <div
    v-if="lightboxState.isOpen"
    ref="dialog"
    role="dialog"
    aria-modal="true"
    :aria-label="lightboxState.imageAlt"
    class="fixed inset-0 z-[999] bg-black/90 flex flex-col select-none"
    @keydown="keydown"
    tabindex="0"
  >
    <div class="min-h-13 px-3 py-2 gap-2 flex-wrap bg-chrome border-b border-hairline flex items-center justify-between z-20 shrink-0">
      <div class="flex items-center gap-3 text-[13px] text-ink min-w-0">
        <span class="font-semibold truncate max-w-xs md:max-w-md">{{ lightboxState.imageAlt || '攻略示意图' }}</span>
        <span class="text-[12px] px-2 py-0.5 rounded-md bg-raised text-muted font-mono shrink-0 border border-hairline">
          {{ Math.round(lightboxState.zoom * 100) }}%
        </span>
        <span v-if="lightboxState.currentSection >= 0"
              class="text-[12px] px-2 py-0.5 rounded-md bg-beacon/15 text-beacon border border-beacon/35 shrink-0 font-medium hidden sm:inline-flex items-center gap-1">
          <span>裁切特写 · {{ lightboxState.currentSection + 1 }} / {{ lightboxState.totalSections }}</span>
        </span>
        <span v-else-if="lightboxState.totalSections > 0"
              class="text-[12px] px-2 py-0.5 rounded-md bg-raised text-muted border border-hairline shrink-0 font-medium hidden sm:inline-flex items-center gap-1">
          <span>完整全景</span>
        </span>
      </div>

      <div class="flex items-center gap-1 bg-paper p-1 rounded-md border border-hairline shrink-0">
        <button
          type="button"
          @click="lightboxState.theme = 'paper'"
          :class="['px-2 py-1 rounded-md text-[12px] font-medium transition cursor-pointer flex items-center gap-1', lightboxState.theme === 'paper' ? 'bg-raised text-ink' : 'text-faint hover:text-ink']"
          title="纯白图纸底（推荐黑白平面图与线稿）"
        >
          <Sun class="w-3.5 h-3.5" />
          <span>纯白</span>
        </button>

        <button
          type="button"
          @click="lightboxState.theme = 'blueprint'"
          :class="['px-2 py-1 rounded-md text-[12px] font-medium transition cursor-pointer flex items-center gap-1', lightboxState.theme === 'blueprint' ? 'bg-raised text-ink' : 'text-faint hover:text-ink']"
          title="建筑蓝图风格"
        >
          <Compass class="w-3.5 h-3.5" />
          <span>蓝图</span>
        </button>

        <button
          type="button"
          @click="lightboxState.theme = 'dark'"
          :class="['px-2 py-1 rounded-md text-[12px] font-medium transition cursor-pointer flex items-center gap-1', lightboxState.theme === 'dark' ? 'bg-raised text-ink' : 'text-faint hover:text-ink']"
          title="暗色画布（适合游戏截图与彩色CG）"
        >
          <Moon class="w-3.5 h-3.5" />
          <span>暗色</span>
        </button>

        <div class="w-px h-4 bg-hairline mx-1"></div>

        <button
          type="button"
          @click="toggleLightboxWheelMode"
          class="px-2 py-1 rounded-md text-[12px] font-medium transition cursor-pointer flex items-center gap-1 border border-hairline"
          :class="lightboxState.wheelMode === 'zoom' ? 'bg-beacon/15 text-beacon border-beacon/35' : 'bg-done/10 text-done border-done/35'"
          :title="lightboxState.wheelMode === 'zoom' ? '当前模式：滚轮缩放画布 (点击切换为滚轮纵向翻页)' : '当前模式：滚轮纵向滚动 (点击切换为滚轮缩放)'"
        >
          <span>滚轮: {{ lightboxState.wheelMode === 'zoom' ? '缩放' : '翻页' }}</span>
        </button>

        <div class="w-px h-4 bg-hairline mx-1"></div>

        <button
          type="button"
          @click="zoomOutLightbox"
          class="p-1 rounded-md text-muted hover:text-ink hover:bg-raised transition cursor-pointer"
          title="缩小"
        >
          <ZoomOut class="w-4 h-4" />
        </button>
        <button
          type="button"
          @click="zoomInLightbox"
          class="p-1 rounded-md text-muted hover:text-ink hover:bg-raised transition cursor-pointer"
          title="放大"
        >
          <ZoomIn class="w-4 h-4" />
        </button>
        <button
          type="button"
          @click="resetLightboxZoom"
          class="px-2 py-1 rounded-md text-[12px] text-muted hover:text-ink hover:bg-raised transition cursor-pointer font-mono"
          title="1:1 原始尺寸"
        >
          1:1
        </button>
        <button
          type="button"
          @click="fitLightboxToScreen"
          class="px-2 py-1 rounded-md text-[12px] text-muted hover:text-ink hover:bg-raised transition cursor-pointer"
          title="适应视图"
        >
          <RotateCcw class="w-3.5 h-3.5 mr-1 inline" />
          <span>适应</span>
        </button>

        <div class="w-px h-4 bg-hairline mx-1"></div>

        <button
          type="button"
          @click="lightboxState.invert = !lightboxState.invert"
          :class="['px-2 py-1 rounded-md text-[12px] font-medium transition cursor-pointer', lightboxState.invert ? 'bg-beacon/20 text-beacon' : 'text-faint hover:text-ink']"
          title="反转黑白颜色"
        >
          反相
        </button>
      </div>

      <button
        type="button"
        @click="closeLightbox"
        class="p-2 rounded-md text-faint hover:text-ink hover:bg-raised transition cursor-pointer flex items-center gap-1"
        title="关闭 (Esc)"
      >
        <X class="w-5 h-5" />
        <span class="text-[12px] font-mono">ESC</span>
      </button>
    </div>

    <div v-if="lightboxState.totalSections > 0"
         class="h-10 px-5 bg-canvas border-b border-hairline flex items-center justify-between z-10 shrink-0 overflow-x-auto text-[12px]">
      <div class="flex items-center gap-1.5 min-w-0">
        <span class="text-faint text-[11px] mr-1 shrink-0">分区导览</span>
        <button
          type="button"
          @click="setLightboxSection(-1)"
          class="px-2.5 py-1 rounded-md font-medium transition cursor-pointer flex items-center gap-1 shrink-0"
          :class="lightboxState.currentSection === -1
            ? 'bg-beacon/20 text-beacon border border-beacon/40'
            : 'bg-paper text-muted hover:text-ink hover:bg-raised border border-hairline'"
        >
          <span>完整全景</span>
        </button>

        <button
          v-for="(sec, index) in lightboxState.sections" :key="index"
          type="button"
          @click="setLightboxSection(index)"
          class="px-2.5 py-1 rounded-md font-medium transition cursor-pointer flex items-center gap-1 truncate max-w-[170px] shrink-0"
          :class="lightboxState.currentSection === index
            ? 'bg-beacon/20 text-beacon border border-beacon/40'
            : 'bg-paper text-muted hover:text-ink hover:bg-raised border border-hairline'"
          :title="sec.title"
        >
          <span class="truncate">{{ sec.title }}</span>
        </button>
      </div>

      <div v-if="lightboxState.currentSection >= 0" class="flex items-center gap-2 shrink-0 ml-3">
        <button type="button" @click="prevLightboxSection" class="p-1 rounded-md bg-paper hover:bg-raised text-muted hover:text-ink border border-hairline cursor-pointer" title="上一格 (← 键)">
          <ChevronLeft class="w-3.5 h-3.5" />
        </button>
        <span class="font-mono text-faint text-[11px]">{{ lightboxState.currentSection + 1 }} / {{ lightboxState.totalSections }}</span>
        <button type="button" @click="nextLightboxSection" class="p-1 rounded-md bg-paper hover:bg-raised text-muted hover:text-ink border border-hairline cursor-pointer" title="下一格 (→ 键)">
          <ChevronRight class="w-3.5 h-3.5" />
        </button>
      </div>
    </div>

    <!-- 视口与画布区域 -->
    <div 
      :ref="bindLightboxViewport"
      class="flex-1 w-full overflow-hidden relative flex items-center justify-center select-none"
      :class="[
        lightboxState.isDragging ? 'cursor-grabbing' : 'cursor-grab',
        {
          'bg-[#f1f3f7]': lightboxState.theme === 'paper',
          'bg-[#0a0f1d]': lightboxState.theme === 'blueprint',
          'bg-[#0d0e12]': lightboxState.theme === 'dark'
        }
      ]"
      @wheel="handleLightboxWheel"
      @mousedown="startLightboxPan"
      @dblclick="handleLightboxDblClick"
    >
      <!-- 背景网格（蓝图或纸面辅助线） -->
      <div 
        v-if="lightboxState.theme === 'blueprint'" 
        class="absolute inset-0 opacity-20 pointer-events-none"
        style="background-image: radial-gradient(#38bdf8 1px, transparent 1px); background-size: 24px 24px;"
      ></div>
      <div 
        v-else-if="lightboxState.theme === 'paper'" 
        class="absolute inset-0 opacity-40 pointer-events-none"
        style="background-image: radial-gradient(#cbd5e1 1px, transparent 1px); background-size: 20px 20px;"
      ></div>

      <!-- 左右悬浮翻格箭头 (聚焦单格模式下显示) -->
      <button 
        v-if="lightboxState.totalSections > 0 && lightboxState.currentSection >= 0"
        @click.stop="prevLightboxSection"
        class="absolute left-6 top-1/2 -translate-y-1/2 w-11 h-11 rounded-full bg-black/70 hover:bg-black/90 text-white border border-white/20 backdrop-blur-md flex items-center justify-center transition shadow-2xl z-20 cursor-pointer hover:scale-110"
        title="上一格 (←)"
      >
        <ChevronLeft class="w-6 h-6" />
      </button>
      <button 
        v-if="lightboxState.totalSections > 0 && lightboxState.currentSection >= 0"
        @click.stop="nextLightboxSection"
        class="absolute right-6 top-1/2 -translate-y-1/2 w-11 h-11 rounded-full bg-black/70 hover:bg-black/90 text-white border border-white/20 backdrop-blur-md flex items-center justify-center transition shadow-2xl z-20 cursor-pointer hover:scale-110"
        title="下一格 (→)"
      >
        <ChevronRight class="w-6 h-6" />
      </button>

      <!-- 缩放与平移图元 (注意：拖动中 transition 为 none，实现绝对零延迟丝滑跟随) -->
      <div 
        class="origin-center will-change-transform flex items-center justify-center"
        :class="[
          lightboxState.isDragging ? 'transition-none' : 'transition-transform duration-100 ease-out'
        ]"
        :style="{
          transform: `translate(${lightboxState.panX}px, ${lightboxState.panY}px) scale(${lightboxState.zoom})`,
          filter: lightboxState.invert ? 'invert(1)' : 'none'
        }"
      >
        <!-- 1. 单格聚焦模式 (精确裁切展示第 k 格) -->
        <div 
          v-if="currentLightboxView"
          class="relative overflow-hidden rounded-xl shadow-2xl border border-white/10 bg-black/50"
          :style="{
            width: `min(80vw, ${68 * imageViewAspectRatio(currentLightboxView)}vh, 1180px)`,
            aspectRatio: imageViewAspectRatio(currentLightboxView)
          }"
        >
          <img 
            :src="lightboxState.imageUrl" 
            :alt="lightboxState.imageAlt + ' · ' + currentLightboxView.title"
            class="wp-view-image select-none pointer-events-none block"
            :style="imageViewStyle(currentLightboxView)"
          />
        </div>

        <!-- 2. 全景长图 / 普通单图模式 -->
        <div v-else class="p-8 flex items-center justify-center">
          <img 
            :src="lightboxState.imageUrl" 
            :alt="lightboxState.imageAlt" 
            class="max-w-none shadow-2xl rounded-lg pointer-events-none select-none"
            @load="fitLightboxToScreen"
            :class="{
              'bg-white p-4 border border-hairline': lightboxState.theme === 'paper' && !lightboxState.imageUrl.includes('sinaimg') && !lightboxState.imageUrl.includes('img.localhost'),
              'bg-canvas p-4 border-2 border-beacon/40': lightboxState.theme === 'blueprint'
            }"
          />
        </div>
      </div>

      <!-- 底部操作提示悬浮条 -->
      <div class="absolute bottom-6 left-1/2 -translate-x-1/2 px-4 py-2 rounded-full bg-paper border border-hairline text-[12px] text-muted shadow-2xl hidden md:flex whitespace-nowrap items-center gap-3 pointer-events-none z-10">
        <span>拖拽平移</span>
        <span class="text-faint">|</span>
        <span>{{ lightboxState.wheelMode === 'zoom' ? '滚轮中心缩放' : '滚轮纵向翻页' }}</span>
        <span class="text-faint">|</span>
        <span>双击自适应/复原</span>
        <span v-if="lightboxState.totalSections > 0" class="text-faint">|</span>
        <span v-if="lightboxState.totalSections > 0" class="text-beacon">← / → 切换分段</span>
        <span class="text-faint">|</span>
        <span class="text-beacon font-mono">Esc 关闭</span>
      </div>
    </div>
  </div>
</template>
