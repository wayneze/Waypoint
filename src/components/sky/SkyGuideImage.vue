<script setup lang="ts">
import { ref, watch } from 'vue';
import { ImageOff, LoaderCircle, RotateCw } from '@lucide/vue';
import { vWikiImg } from './wikiImage';

const props = defineProps<{ src?: string; alt: string }>();
const status = ref<'loading' | 'ready' | 'failed'>('loading');
const version = ref(0);
watch(() => props.src, () => { status.value = 'loading'; });
function retry() {
  status.value = 'loading';
  version.value++;
}
</script>

<template>
  <div class="sky-guide-art" :class="{ 'is-ready': status === 'ready' }">
    <img v-if="src" :key="`${src}/${version}`" v-wiki-img="src" :alt="alt"
         @wikiload="status = 'ready'" @wikifail="status = 'failed'" />
    <div v-if="!src || status !== 'ready'" class="sky-guide-art-status" :role="src ? 'status' : undefined">
      <LoaderCircle v-if="src && status === 'loading'" class="sky-guide-art-spinner" aria-label="图片加载中" />
      <template v-else>
        <ImageOff class="w-5 h-5" />
        <button v-if="src" type="button" @click.stop="retry"><RotateCw class="w-3 h-3" />重新加载图片</button>
        <span v-else>图片待更新</span>
      </template>
    </div>
    <slot />
  </div>
</template>

<style scoped>
.sky-guide-art { position: relative; overflow: hidden; background: color-mix(in srgb, var(--game-color) 7%, var(--color-canvas)); }
.sky-guide-art > img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: top; }
.sky-guide-art-status { position: absolute; inset: 0; display: flex; flex-direction: column; justify-content: center; align-items: center; gap: 10px; color: var(--color-muted); font-size: 12px; }
.sky-guide-art-status button { display: flex; align-items: center; gap: 5px; cursor: pointer; padding: 7px; color: var(--game-color); }
.sky-guide-art-spinner { width: 20px; height: 20px; animation: sky-guide-spin 1.5s linear infinite; opacity: .6; }
@keyframes sky-guide-spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) { .sky-guide-art-spinner { animation: none; } }
</style>
