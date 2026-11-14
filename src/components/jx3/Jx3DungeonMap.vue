<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { ChevronDown, Map, ZoomIn } from '@lucide/vue';
import { jx3BoxDungeons } from '../../workspace/jx3/collect';
import floors from '../../content/jx3/dungeon-map-floors.json';
import { openLightbox } from '../../workspace/guides/lightbox';
const props = defineProps<{ dungeonKey: string; complete: boolean }>();
const floor = ref(0);
const failed = ref(false);
const dungeon = computed(() => jx3BoxDungeons.value.find(row => row.key === props.dungeonKey));
const mapId = computed(() => dungeon.value?.maps.find(map => (floors as Record<string, number[]>)[map.mapId]?.length)?.mapId);
const levels = computed(() => mapId.value ? (floors as Record<string, number[]>)[mapId.value] || [] : []);
const source = computed(() => mapId.value ? `https://img.jx3box.com/map/maps/map_${mapId.value}_${levels.value[floor.value] ?? 0}.png` : '');
watch(() => props.dungeonKey, () => { floor.value = 0; failed.value = false; });
watch(source, () => { failed.value = false; });
</script>

<template>
  <details v-if="source" :key="`${dungeonKey}/${complete}`" class="jx3-map-guide" :open="!complete">
    <summary><Map :size="15" /><span>秘境地图参考</span><ChevronDown :size="14" /></summary>
    <nav v-if="levels.length > 1" class="jx3-segments" aria-label="地图楼层">
      <button v-for="(_, index) in levels" :key="index" type="button" :aria-pressed="floor === index" @click="floor = index">第 {{ index + 1 }} 层</button>
    </nav>
    <figure v-if="!failed">
      <button type="button" :aria-label="'放大地图：' + dungeonKey" @click="openLightbox(source, dungeonKey + ' · 地图参考', { theme: 'dark' })">
        <img :key="source" :src="source" :alt="dungeonKey + '游戏地图'" loading="lazy" referrerpolicy="no-referrer" @error="failed = true" />
      </button>
      <figcaption>游戏地图 · 魔盒资料<button type="button" class="wp-text-action" @click="openLightbox(source, dungeonKey + ' · 地图参考', { theme: 'dark' })"><ZoomIn :size="13" />放大</button></figcaption>
    </figure>
    <p v-else class="jx3-empty-line">地图暂未加载。<button type="button" class="wp-text-action" @click="failed = false">重试</button></p>
  </details>
</template>
