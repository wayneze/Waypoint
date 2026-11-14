<script setup lang="ts">
import type { FormattedCalloutLine } from '../services/guideParser';
import { formatGuideInline } from '../services/guideParser';
import { openLightbox } from '../workspace/guides/lightbox';

function onInlineClick(event: MouseEvent) {
  const img = (event.target as HTMLElement).closest('img[data-guide-zoom]') as HTMLImageElement | null;
  if (!img) return;
  event.preventDefault();
  event.stopPropagation();
  openLightbox(img.dataset.guideZoom || img.src, img.alt);
}

const props = defineProps<{
  line: FormattedCalloutLine;
  here?: boolean;
  markable?: boolean;
  imageTheme?: 'paper' | 'dark';
  imageFailed?: boolean;
}>();

const emit = defineEmits<{
  mark: [];
  openImage: [url: string, alt: string];
  toggleTheme: [url: string, alt: string];
  imageError: [];
}>();
</script>

<template>
  <div v-if="line.type === 'choice'"
       class="wp-guide-choice"
       :class="[here ? 'is-here' : '', markable ? 'cursor-pointer' : '']"
       @click="markable && emit('mark')">
    <span class="wp-guide-num">{{ line.stepNumber }}</span>
    <div class="min-w-0" v-html="line.formattedHtml" @click="onInlineClick"></div>
  </div>

  <div v-else-if="line.type === 'day'"
       class="wp-guide-day wp-guide-mark"
       :class="[here ? 'is-here' : '', markable ? 'cursor-pointer' : '']"
       @click="markable && emit('mark')">
    <span class="wp-guide-date">{{ line.stepNumber }}</span>
    <div class="min-w-0" v-html="line.formattedHtml" @click="onInlineClick"></div>
  </div>

  <div v-else-if="line.type === 'save-header'" class="wp-guide-cue wp-guide-cue-save">
    <span class="wp-guide-kicker">存档</span>
    <span class="wp-guide-cue-text">{{ line.cleanText }}</span>
  </div>

  <div v-else-if="line.type === 'load-header'" class="wp-guide-cue wp-guide-cue-load">
    <span class="wp-guide-kicker">读档</span>
    <span class="wp-guide-cue-text">{{ line.cleanText }}</span>
  </div>

  <div v-else-if="line.type === 'end-header'" class="wp-guide-cue wp-guide-cue-end">
    <span class="wp-guide-kicker">结局</span>
    <span class="wp-guide-cue-text">{{ line.cleanText }}</span>
  </div>

  <h5 v-else-if="line.type === 'section-header'" class="wp-guide-section">
    {{ line.cleanText }}
  </h5>

  <div v-else-if="line.type === 'bullet'"
       class="wp-guide-bullet wp-guide-mark"
       :class="[here ? 'is-here' : '', markable ? 'cursor-pointer' : '']"
       @click="markable && emit('mark')">
    <span class="wp-guide-bullet-mark"></span>
    <div class="flex-1 min-w-0" v-html="line.formattedHtml" @click="onInlineClick"></div>
  </div>

  <p v-else-if="line.type === 'tip'" class="wp-guide-tip" v-html="line.formattedHtml" @click="onInlineClick"></p>

  <div v-else-if="line.type === 'sub-choice'"
       class="wp-guide-sub wp-guide-mark"
       :class="[here ? 'is-here' : '', markable ? 'cursor-pointer' : '']"
       @click="markable && emit('mark')">
    <div class="flex-1 min-w-0" v-html="line.formattedHtml" @click="onInlineClick"></div>
  </div>

  <div v-else-if="line.type === 'divider'" class="wp-guide-rule"></div>

  <figure v-else-if="line.type === 'image'" class="my-5">
    <div
      @click="emit('openImage', line.imageUrl || '', line.imageAlt || '')"
      class="cursor-zoom-in w-fit max-w-full overflow-hidden rounded-md border border-hairline"
      :class="props.imageTheme === 'paper' ? 'bg-white' : 'bg-canvas'"
    >
      <div v-if="props.imageFailed" class="p-8 text-center text-muted text-[13px]">图像未能加载</div>
      <img
        v-else
        :src="line.imageUrl"
        :alt="line.imageAlt"
        class="max-w-full max-h-[420px] object-contain select-none pointer-events-none"
        loading="lazy"
        @error="emit('imageError')"
      />
    </div>
    <figcaption class="mt-2 flex items-center justify-between gap-2 text-[12px] text-faint">
      <span class="truncate">{{ line.imageAlt || '示意图' }}</span>
      <button type="button"
              @click.stop="emit('toggleTheme', line.imageUrl || '', line.imageAlt || '')"
              class="hover:text-ink cursor-pointer shrink-0">
        {{ props.imageTheme === 'paper' ? '白底' : '暗底' }}
      </button>
    </figcaption>
  </figure>

  <p v-else-if="line.type === 'location'" class="my-2 text-[13px] text-muted" v-html="line.formattedHtml"></p>

  <div v-else-if="line.type === 'table' && line.tableData" class="my-3 overflow-x-auto rounded-md border border-hairline bg-paper">
    <table class="w-full text-left text-[13px] border-collapse">
      <thead>
        <tr class="bg-raised border-b border-hairline text-muted">
          <th v-for="(th, thIdx) in line.tableData.headers" :key="thIdx" class="px-3 py-2 font-medium">
            <span v-html="formatGuideInline(th)"></span>
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(row, rIdx) in line.tableData.rows" :key="rIdx" class="border-b border-hairline">
          <td v-for="(cell, cellIdx) in row" :key="cellIdx" class="px-3 py-2 font-mono">
            <span v-if="cell === 'YES'" class="text-done">YES</span>
            <span v-else-if="cell === 'NO'" class="text-alert">NO</span>
            <span v-else v-html="formatGuideInline(cell)"></span>
          </td>
        </tr>
      </tbody>
    </table>
  </div>

  <div v-else-if="line.type === 'spacer'" class="h-3"></div>

  <div v-else
       class="py-1.5 wp-guide-mark"
       :class="[here ? 'is-here text-beacon' : '', markable && line.cleanText.trim() ? 'cursor-pointer' : '']"
       @click="markable && line.cleanText.trim() && emit('mark')">
    <div class="min-w-0" v-html="line.formattedHtml" @click="onInlineClick"></div>
  </div>
</template>
