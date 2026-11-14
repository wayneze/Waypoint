<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { ChevronLeft, ChevronRight, Expand, RefreshCw, Heart, Check } from '@lucide/vue';
import { imageViewAspectRatio, imageViewStyle, type ImageView } from '../../services/imageViews';
import { openLightbox } from '../../workspace/guides/lightbox';
import { openExternalUrl } from '../../services/systemApi';
import { hotlinkImageSrc, vWikiImg, vWikiWear } from './wikiImage';

const props = withDefaults(defineProps<{
  name: string;
  wear?: string;
  icon?: string;
  meta?: string;
  sourceUrl?: string;
  done?: boolean;
  toggle?: boolean;
  wanted?: boolean;
  disabled?: boolean;
  mode?: 'icon' | 'wear';
  inspect?: boolean;
}>(), { wanted: undefined, mode: 'wear' });
const emit = defineEmits<{ toggle: []; wish: []; preview: []; inspect: [] }>();
const state = ref<'loading' | 'ready' | 'failed'>('loading');
const iconState = ref<'loading' | 'ready' | 'failed'>('loading');
const views = ref<ImageView[]>([]);
const viewIndex = ref(0);
const attempt = ref(0);
const iconAttempt = ref(0);
const sourceError = ref('');
const showWear = computed(() => props.mode === 'wear' && !!props.wear);
const wearReady = computed(() => showWear.value && state.value === 'ready');
const iconReady = computed(() => !!props.icon && iconState.value === 'ready');
const previewSrc = computed(() => wearReady.value ? props.wear : iconReady.value ? props.icon : '');
const failed = computed(() => !wearReady.value && (
  (showWear.value && state.value === 'failed') || (!!props.icon && iconState.value === 'failed')
));
const failureText = computed(() => showWear.value && state.value === 'failed' ? '实穿加载失败' : '图标加载失败');
const placeholder = computed(() => {
  if (previewSrc.value) return '';
  if (showWear.value && state.value === 'loading') return '加载实穿…';
  if (props.icon && iconState.value === 'loading') return '加载图标…';
  return props.wear || props.icon ? '图片加载失败' : '暂未匹配到图片';
});
const currentView = computed(() => views.value[viewIndex.value]);
const frameStyle = computed(() => {
  const view = currentView.value;
  if (!view) return undefined;
  // 保持原始比例，适配 3:4 卡片；裁切坐标只来自来源层。
  const aspectRatio = imageViewAspectRatio(view);
  return { aspectRatio, width: Math.min(1, aspectRatio / (3 / 4)) * 100 + '%' };
});
watch(() => props.wear, reset);
watch(() => props.icon, () => { iconState.value = 'loading'; });
watch(() => props.sourceUrl, () => { sourceError.value = ''; });

function reset() {
  state.value = 'loading';
  views.value = [];
  viewIndex.value = 0;
}

function loaded(event: Event) {
  views.value = (event as CustomEvent<ImageView[]>).detail;
  state.value = 'ready';
}

function retry() {
  if (state.value === 'failed') {
    reset();
    attempt.value++;
  }
  if (iconState.value === 'failed') {
    iconState.value = 'loading';
    iconAttempt.value++;
  }
}

function step(delta: number) {
  if (views.value.length < 2) return;
  viewIndex.value = (viewIndex.value + delta + views.value.length) % views.value.length;
}

function preview() {
  if (props.inspect) { emit('inspect'); return; }
  const src = previewSrc.value;
  if (!src) return;
  emit('preview');
  openLightbox(hotlinkImageSrc(src), props.name, {
    theme: 'dark',
    views: wearReady.value && views.value.length > 1 ? views.value : [],
    sectionIndex: wearReady.value && views.value.length > 1 ? viewIndex.value : -1,
  });
}

async function openSource() {
  if (!props.sourceUrl) return;
  sourceError.value = '';
  try {
    await openExternalUrl(props.sourceUrl);
  } catch {
    sourceError.value = '来源页面打开失败，请重试';
  }
}
</script>

<template>
  <div class="wp-tile text-left" :class="{ 'is-done': done, 'is-icon-mode': mode === 'icon', 'is-collection-tile': toggle }">
    <div class="wp-tile-visual">
    <button type="button" class="wp-tile-media" :aria-label="(inspect ? '查看外观详情：' : '放大查看') + name"
            :disabled="!inspect && !previewSrc" @click="preview"
            @keydown.left.prevent.stop="step(-1)" @keydown.right.prevent.stop="step(1)">
      <span v-if="showWear" class="wp-tile-frame" :style="frameStyle">
        <img :key="wear + ':' + attempt" v-wiki-wear="wear || ''" :alt="currentView ? name + ' · ' + currentView.title : name"
             class="wp-view-image" :style="currentView ? imageViewStyle(currentView) : undefined"
             @wikiload="loaded" @wikifail="state = 'failed'" />
      </span>
      <img v-if="icon && !wearReady" :key="icon + ':' + iconAttempt" v-wiki-img="icon"
           :alt="name + ' · 图标'" class="is-icon"
           @wikiload="iconState = 'ready'" @wikifail="iconState = 'failed'" />
      <span v-if="placeholder" class="wp-tile-placeholder" role="status">{{ placeholder }}</span>
      <span v-else-if="showWear && state === 'loading'" class="wp-tile-status" role="status">加载实穿…</span>
      <Expand v-if="previewSrc" class="wp-tile-expand" aria-hidden="true" />
    </button>
    <div v-if="(mode === 'wear' && views.length > 1 && state === 'ready') || failed || (mode === 'wear' && !wear && iconReady)" class="wp-tile-support">
    <div v-if="mode === 'wear' && views.length > 1 && state === 'ready'" class="wp-tile-views" :aria-label="name + '视角'">
      <button type="button" :aria-label="name + '上一个视角'" @click="step(-1)"><ChevronLeft /></button>
      <span aria-live="polite">{{ currentView?.title }} <small>{{ viewIndex + 1 }}/{{ views.length }}</small></span>
      <button type="button" :aria-label="name + '下一个视角'" @click="step(1)"><ChevronRight /></button>
    </div>
    <button v-if="failed" type="button" class="wp-tile-retry" @click="retry">
      <RefreshCw class="w-3 h-3 shrink-0" />{{ failureText }} · 重试
    </button>
    <span v-else-if="mode === 'wear' && !wear && iconReady" class="wp-tile-fallback">未匹配实穿 · 显示图标</span>
    </div>
    </div>
    <span class="wp-tile-name" :title="name">{{ name }}</span>
    <span v-if="meta" class="wp-tile-meta">{{ meta }}</span>
    <a v-if="sourceUrl && !wearReady && !toggle && !inspect" :href="sourceUrl" class="text-[12px] text-beacon self-start"
       :aria-label="'查看' + name + '的来源'" @click.prevent="openSource">查看来源</a>
    <span v-if="sourceError" class="text-[12px] text-alert" role="status">{{ sourceError }}</span>
    <div v-if="toggle || wanted !== undefined" class="wp-tile-actions">
      <button v-if="toggle" type="button" class="wp-tile-obtained" :aria-pressed="!!done" :disabled="disabled" :aria-label="(done ? '取消已拥有：' : '标记已拥有：') + name" @click="emit('toggle')">
        <Check :size="14" />{{ done ? '已拥有' : '标记拥有' }}
      </button>
      <button v-if="wanted !== undefined" type="button" class="wp-tile-wish" :aria-label="(wanted ? '已加入想要：' : '想要：') + name" :aria-pressed="wanted" :disabled="disabled" @click="emit('wish')"><Heart :size="16" :fill="wanted ? 'currentColor' : 'none'" /></button>
    </div>
    <span v-else-if="done" class="wp-tile-meta text-done">已拥有</span>
  </div>
</template>
