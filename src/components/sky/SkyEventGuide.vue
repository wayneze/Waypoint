<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import { Images, ChevronDown, ChevronLeft, ChevronRight, ArrowUpRight, Maximize2 } from '@lucide/vue';
import type { SkyCalendarItem, SkyEventGuideImages } from '../../services/skyApi';
import { dailyEventImages, eventImageSources, getSkyEventGuideImages, matchEventGuide } from '../../services/skyEventService';
import { openExternalUrl } from '../../services/systemApi';
import { state } from '../../workspace/core';
import { skyDailyData } from '../../workspace/sky/tabs';
import { skyPlanDay } from '../../workspace/sky/plan';
import { openLightbox } from '../../workspace/guides/lightbox';
import { hotlinkImageSrc, vWikiImg } from './wikiImage';

const props = defineProps<{ item: SkyCalendarItem; compact?: boolean; reference?: boolean; description?: string }>();
const guide = computed(() => matchEventGuide(props.item, state.eventGuides));
const sources = computed(() => eventImageSources(props.item, state.eventGuides));
const result = ref<SkyEventGuideImages | null>(null);
const loading = ref(false);
const failed = ref(false);
const failedImages = ref<string[]>([]);
const imageVersion = ref(0);
const mounted = ref(false);
const expanded = ref(false);
const selectedIndex = ref(0);
let request = 0;
const images = computed(() => [...(props.reference ? [] : dailyEventImages(props.item, skyDailyData.value, skyPlanDay.value)), ...(result.value?.images || [])]
  .filter((image, index, all) => all.findIndex(other => other.url === image.url) === index));
const paragraphs = computed(() => (props.description || guide.value?.quests || '').split(/\n+/).filter(Boolean));
const selectedImage = computed(() => images.value[Math.min(selectedIndex.value, images.value.length - 1)]);
function moveImage(direction: number) {
  selectedIndex.value = (selectedIndex.value + direction + images.value.length) % images.value.length;
}
watch(() => props.item.id, () => { expanded.value = false; selectedIndex.value = 0; failedImages.value = []; });
async function load(force = false) {
  const version = ++request;
  result.value = null;
  failed.value = false;
  if (!sources.value.length) { loading.value = false; return; }
  loading.value = true;
  const accept = (data: SkyEventGuideImages) => { if (version === request) result.value = data; };
  const data = await getSkyEventGuideImages(props.item, sources.value, force, accept);
  if (version !== request) return;
  if (data) accept(data);
  else failed.value = true;
  loading.value = false;
}
function retry() {
  failedImages.value = [];
  imageVersion.value++;
  void load(true);
}
watch(() => [mounted.value, props.item.id, props.item.start, sources.value.join('|')], () => {
  if (mounted.value) void load();
});
onMounted(() => { mounted.value = true; });
onBeforeUnmount(() => { request++; });
</script>

<template>
  <section v-if="item.kind === 'event'" class="sky-event-guide" :class="{ 'is-compact': compact }">
    <button type="button" class="sky-event-guide-toggle" :aria-expanded="expanded" :aria-label="`${expanded ? '收起' : '展开'}${item.title}图文攻略`" @click="expanded = !expanded">
      <Images class="w-3.5 h-3.5" /><span>{{ reference ? '往届图文攻略' : '图文攻略' }}</span><small v-if="images.length">{{ images.length }} 张</small><ChevronDown class="w-3.5 h-3.5 sky-event-guide-chevron" :class="{ 'is-open': expanded }" />
    </button>
    <div v-if="expanded" class="sky-event-guide-content">
      <p v-if="loading" role="status" class="text-faint">正在匹配{{ reference ? '往届' : '本届' }}攻略图片…</p>
      <div v-if="selectedImage" class="sky-event-images">
        <figure :key="selectedImage.url">
          <button v-if="!failedImages.includes(selectedImage.url)" :key="imageVersion" type="button" class="sky-event-image"
                  :aria-label="`放大攻略：${selectedImage.title}`" @click="openLightbox(hotlinkImageSrc(selectedImage.url), `${item.title} · ${selectedImage.title}`)">
            <img v-wiki-img="selectedImage.url" :alt="selectedImage.title" @wikifail="failedImages.push(selectedImage!.url)" />
            <span><Maximize2 class="w-3.5 h-3.5" />点击查看完整长图</span>
          </button>
          <div v-else class="sky-event-image-error">图片暂时未加载 <button class="wp-text-action" type="button" @click="retry">重试</button></div>
          <figcaption><span>{{ selectedImage.title }}</span><button type="button" class="wp-text-action shrink-0" :aria-label="`${selectedImage.title}来源`" @click="openExternalUrl(selectedImage.source_url)">来源 <ArrowUpRight class="w-3 h-3" /></button></figcaption>
        </figure>
        <nav v-if="images.length > 1" class="sky-image-pager" :aria-label="`${item.title}攻略图片切换`">
          <button type="button" aria-label="上一张攻略" @click="moveImage(-1)"><ChevronLeft class="w-4 h-4" /></button>
          <span aria-live="polite">{{ Math.min(selectedIndex + 1, images.length) }} / {{ images.length }}</span>
          <button type="button" aria-label="下一张攻略" @click="moveImage(1)"><ChevronRight class="w-4 h-4" /></button>
        </nav>
      </div>
      <p v-else-if="!loading" class="text-faint">{{ failed ? '攻略图片暂时获取失败。' : `${reference ? '往届' : '本届'}来源暂未发布可匹配的攻略图片。` }}</p>
      <button v-if="!loading && (failed || result?.unavailable_sources.length || !images.length)" type="button" class="wp-text-action justify-self-start" @click="retry">重新匹配图片</button>
      <details v-if="paragraphs.length" class="sky-event-text">
        <summary>玩法文字说明</summary>
        <p v-for="paragraph in paragraphs" :key="paragraph">{{ paragraph }}</p>
      </details>
    </div>
  </section>
</template>

<style scoped>
.sky-event-guide { margin-top: 12px; }
.sky-event-guide-toggle { display: flex; align-items: center; gap: 7px; width: 100%; min-height: 36px; padding: 8px 0; cursor: pointer; font-size: 13px; color: var(--color-beacon); text-align: left; }
.sky-event-guide-chevron { margin-left: auto; transition: transform .15s; }
.sky-event-guide-chevron.is-open { transform: rotate(180deg); }
.sky-event-guide-content { display: grid; gap: 12px; padding: 8px 0 12px; font-size: 13px; line-height: 1.85; color: var(--color-muted); }
.sky-event-images { max-width: 480px; }
.sky-event-image { position: relative; display: block; width: 100%; height: 180px; overflow: hidden; border-radius: 7px; border: 1px solid var(--color-hairline); background: var(--color-surface); cursor: zoom-in; }
.is-compact .sky-event-image { height: 160px; }
.sky-event-image img { width: 100%; height: 100%; object-fit: cover; object-position: top; }
.sky-event-image span { position: absolute; bottom: 0; left: 0; right: 0; display: flex; justify-content: center; align-items: center; gap: 6px; padding: 18px 8px 10px; background: linear-gradient(transparent, #000b); color: white; font-size: 12px; }
.sky-event-images figcaption { display: flex; justify-content: space-between; gap: 8px; font-size: 12px; margin-top: 7px; }
.sky-event-image-error { display: flex; align-items: center; justify-content: center; gap: 8px; min-height: 90px; border: 1px solid var(--color-hairline); border-radius: 7px; }
.sky-event-text summary { color: var(--color-muted); font-size: 12px; cursor: pointer; }
.sky-image-pager { display: flex; align-items: center; justify-content: center; gap: 18px; margin-top: 7px; }
.sky-image-pager button { display: grid; place-items: center; width: 36px; height: 32px; border-radius: 5px; cursor: pointer; }
.sky-image-pager button:hover { color: var(--color-beacon); background: var(--color-raised); }
.sky-image-pager span { font-size: 12px; font-variant-numeric: tabular-nums; }
</style>
