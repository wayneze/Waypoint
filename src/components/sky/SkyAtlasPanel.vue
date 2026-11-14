<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { ArrowLeft, ArrowRight, ArrowUpRight, Check, ChevronDown, Map, RefreshCw, Search, SlidersHorizontal, X, ZoomIn } from '@lucide/vue';
import {
  ATLAS_REALM_TABS, atlasCharacter, atlasCanEdit, atlasLegacyCount, atlasSyncingName,
  claimAtlasLegacy, isClaimingAtlasLegacy, atlasDashenHint, atlasDashenSummary, atlasQueryId,
  atlasHideCollected, atlasMode, atlasProgress, atlasQuery, atlasRealmFilter, atlasRealmGroups,
  atlasActionKind, atlasActionTitle, currentRealmOverviewMaps,
  isSyncingAtlasDashen, openAtlasAction, syncAtlasFromDashen,
  toggleAtlasLight, toggleAtlasShrine, toggleAtlasAction, type AtlasMode,
} from '../../workspace/sky/atlas';
import { atlasRealmName, atlasAreaImage } from '../../services/skyAtlas';
import { openExternalUrl } from '../../services/systemApi';
import { openAddVaultModal } from '../../workspace/vault';
import { openLightbox } from '../../workspace/guides/lightbox';
import SkyGuideImage from './SkyGuideImage.vue';

const modes: { id: AtlasMode; label: string }[] = [
  { id: 'flow', label: '全部' }, { id: 'lights', label: '小金人' },
  { id: 'shrines', label: '神殿' }, { id: 'actions', label: '动作' },
];
const actionKinds = [{ id: 'all', label: '全部动作' }, { id: 'regular', label: '常驻' }, { id: 'season', label: '季节' }] as const;
const selectedAreaId = ref('');
const areaNavigation = ref<HTMLElement | null>(null);
const routeAreas = computed(() => atlasRealmGroups.value.flatMap(group => group.cards));
watch(routeAreas, areas => {
  if (!areas.some(area => area.id === selectedAreaId.value)) selectedAreaId.value = areas[0]?.id || '';
}, { immediate: true, flush: 'sync' });
const selectedArea = computed(() => routeAreas.value.find(area => area.id === selectedAreaId.value));
const selectedIndex = computed(() => routeAreas.value.findIndex(area => area.id === selectedAreaId.value));
const areaLinks = computed(() => {
  const group = atlasRealmGroups.value.find(group => group.id === selectedArea.value?.realmId);
  return (group?.cards || []).map(area => {
    const items = [...area.lights, ...area.shrines, ...area.actions];
    return { id: area.id, name: area.name, done: items.filter(item => item.checked).length, total: items.length };
  });
});
const scene = computed(() => selectedArea.value ? atlasAreaImage(selectedArea.value.id) : '');
const sceneFailed = ref(false);
watch(scene, () => { sceneFailed.value = false; });
watch(selectedAreaId, async () => {
  await nextTick();
  const nav = areaNavigation.value;
  if (nav && nav.scrollWidth > nav.clientWidth) {
    nav.querySelector('[aria-current="location"]')?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }
});

function moveArea(offset: number) {
  const area = routeAreas.value[selectedIndex.value + offset];
  if (area) selectedAreaId.value = area.id;
}
function clearFilters() {
  atlasQuery.value = '';
  atlasHideCollected.value = false;
  atlasRealmFilter.value = 'all';
  atlasMode.value = 'flow';
  atlasActionKind.value = 'all';
}
</script>

<template>
  <section class="sky-atlas" aria-label="探索手记">
    <div v-if="!atlasCharacter" class="atlas-notice">
      <p>可以先浏览地图，登记光遇角色后即可记录自己的探索进度。</p>
      <button type="button" class="wp-text-action" @click="openAddVaultModal('光遇')">登记角色 <ArrowRight :size="14" /></button>
    </div>
    <div v-if="atlasLegacyCount" class="atlas-notice" role="status">
      <p>{{ atlasLegacyCount }} 条旧共享记录待归属，归入后会与当前角色的进度合并。</p>
      <button type="button" class="wp-text-action" :disabled="!atlasCanEdit || isSyncingAtlasDashen" @click="claimAtlasLegacy">
        {{ isClaimingAtlasLegacy ? '正在归入…' : atlasCharacter ? `归入「${atlasCharacter.name}」` : '请先登记角色' }}
      </button>
    </div>

    <div class="atlas-toolbar">
      <nav class="atlas-kinds" aria-label="探索类型">
        <button v-for="mode in modes" :key="mode.id" type="button"
                :aria-pressed="atlasMode === mode.id" @click="atlasMode = mode.id">{{ mode.label }}</button>
      </nav>
      <div class="atlas-tools">
        <label class="atlas-search">
          <Search :size="15" aria-hidden="true" />
          <input v-model="atlasQuery" type="search" aria-label="搜索区域、动作或位置" placeholder="搜索区域、动作、位置" />
          <button v-if="atlasQuery" type="button" aria-label="清空探索搜索" @click="atlasQuery = ''"><X :size="14" /></button>
        </label>
        <button type="button" class="atlas-pending" :aria-pressed="atlasHideCollected" @click="atlasHideCollected = !atlasHideCollected">
          <SlidersHorizontal :size="14" />只看未收
        </button>
      </div>
    </div>
    <nav v-if="atlasMode === 'actions'" class="atlas-action-kinds" aria-label="动作来源">
      <button v-for="kind in actionKinds" :key="kind.id" type="button" :aria-pressed="atlasActionKind === kind.id"
              @click="atlasActionKind = kind.id">{{ kind.label }}</button>
    </nav>

    <div class="atlas-journey">
      <nav class="atlas-map-nav" aria-label="探索地图">
        <button v-for="realm in ATLAS_REALM_TABS" :key="realm.id" type="button"
                :aria-pressed="atlasRealmFilter === realm.id" @click="atlasRealmFilter = realm.id">{{ realm.name }}</button>
      </nav>
      <div class="atlas-route">
        <p>{{ selectedArea?.realmName || atlasRealmName(atlasRealmFilter) }}<span>{{ areaLinks.length }} 个地区</span></p>
        <div class="atlas-route-actions">
          <span>{{ selectedArea ? selectedIndex + 1 : 0 }} / {{ routeAreas.length }}</span>
          <button type="button" aria-label="上一个地区" :disabled="selectedIndex <= 0" @click="moveArea(-1)"><ArrowLeft :size="16" /></button>
          <button type="button" aria-label="下一个地区" :disabled="!selectedArea || selectedIndex >= routeAreas.length - 1" @click="moveArea(1)"><ArrowRight :size="16" /></button>
        </div>
      </div>
      <nav v-if="areaLinks.length" ref="areaNavigation" class="atlas-area-nav" aria-label="探索地区导航">
        <button v-for="area in areaLinks" :key="area.id" type="button"
                :aria-current="selectedAreaId === area.id ? 'location' : undefined"
                :aria-label="`${area.name}，已收集 ${area.done}/${area.total}`" @click="selectedAreaId = area.id">
          <Check v-if="area.done === area.total" :size="12" class="atlas-area-complete" aria-hidden="true" />
          <span>{{ area.name }}</span><small>{{ area.done }}/{{ area.total }}</small>
        </button>
      </nav>

      <template v-if="selectedArea">
          <header class="atlas-scene" :class="{ 'has-image': scene && !sceneFailed }">
            <button v-if="scene && !sceneFailed" type="button" class="atlas-scene-image"
                    :aria-label="'放大查看' + selectedArea.name + '场景图'" @click="openLightbox(scene, selectedArea.name + ' · 场景图', { theme: 'dark' })">
              <img :key="scene" :src="scene" :alt="selectedArea.name + '场景图，不标注收集点位'" referrerpolicy="no-referrer" @error="sceneFailed = true" />
            </button>
            <div class="atlas-scene-copy">
              <p>{{ selectedArea.realmName }}<span>沿途探索</span></p>
              <h2>{{ selectedArea.name }}</h2>
            </div>
            <span v-if="scene && !sceneFailed" class="atlas-scene-hint"><ZoomIn :size="13" />地区场景 · 非点位图</span>
            <span v-else class="atlas-scene-hint">{{ sceneFailed ? '场景图暂未加载' : '场景图待收录' }}</span>
          </header>

          <div class="atlas-checklist">
            <section v-if="selectedArea.shrines.length" class="atlas-section" :style="{ order: selectedArea.sections.indexOf('shrines') }" aria-label="神殿清单">
              <h3>神殿<small>{{ selectedArea.shrineDone }}/{{ selectedArea.shrineTotal }}</small></h3>
              <div class="atlas-entries">
              <article v-for="shrine in selectedArea.shrines" :key="shrine.id" class="atlas-entry">
                <div class="atlas-check-row">
                <button type="button" class="atlas-check" :aria-pressed="shrine.checked" :disabled="!atlasCanEdit"
                        :aria-label="(shrine.checked ? '取消点亮神殿：' : '点亮神殿：') + shrine.hint" @click="toggleAtlasShrine(shrine.id)">
                  <span><Check v-if="shrine.checked" :size="14" /></span>
                </button>
                <p :class="{ 'is-checked': shrine.checked }">{{ shrine.hint }}</p>
                </div>
                <details v-if="shrine.image" :key="`${atlasCharacter?.id}/${shrine.id}/${shrine.checked}`"
                         class="atlas-location" :open="!shrine.checked">
                  <summary :aria-label="shrine.hint + ' · 神殿位置参考'">
                    <span>神殿位置参考</span><ChevronDown :size="14" aria-hidden="true" />
                  </summary>
                  <figure class="atlas-guide">
                    <SkyGuideImage :src="shrine.image" :alt="selectedArea.name + ' · 神殿位置参考'" />
                    <figcaption><span>神殿位置参考</span><button type="button" class="wp-text-action"
                      :aria-label="'放大神殿位置参考：' + shrine.hint"
                      @click="openLightbox(shrine.image, selectedArea.name + ' · 神殿位置参考', { theme: 'dark' })"><ZoomIn :size="13" />放大</button></figcaption>
                  </figure>
                </details>
              </article>
              </div>
            </section>
            <section v-if="selectedArea.lights.length" class="atlas-section" :style="{ order: selectedArea.sections.indexOf('lights') }" aria-label="小金人清单">
              <h3>小金人<small>{{ selectedArea.lightDone }}/{{ selectedArea.lightTotal }}</small></h3>
              <div class="atlas-entries atlas-light-entries">
              <div v-for="light in selectedArea.lights" :key="light.id" class="atlas-check-row">
                <button type="button" class="atlas-check" :aria-pressed="light.checked" :disabled="!atlasCanEdit"
                        :aria-label="(light.checked ? '取消收集光翼：' : '收集光翼：') + light.hint" @click="toggleAtlasLight(light.id)">
                  <span><Check v-if="light.checked" :size="14" /></span>
                </button>
                <p :class="{ 'is-checked': light.checked }">{{ light.hint }}</p>
              </div>
              </div>
            </section>
            <section v-if="selectedArea.actions.length" class="atlas-section" :style="{ order: selectedArea.sections.indexOf('actions') }" aria-label="动作清单">
              <h3>动作<small>{{ selectedArea.actionDone }}/{{ selectedArea.actionTotal }}</small></h3>
              <p class="atlas-guide-hint">未收集的位置图默认展开，已获得后自动收起；点击位置参考可展开或收起。</p>
              <div class="atlas-entries">
              <article v-for="action in selectedArea.actions" :key="action.id" class="atlas-entry">
                <div class="atlas-check-row">
                <button type="button" class="atlas-check" :aria-pressed="action.checked" :disabled="!atlasCanEdit"
                        :aria-label="(action.checked ? '取消收集动作：' : '收集动作：') + atlasActionTitle(action)" @click="toggleAtlasAction(action.id)">
                  <span><Check v-if="action.checked" :size="14" /></span>
                </button>
                <div class="atlas-action-copy"><p :class="{ 'is-checked': action.checked }">{{ atlasActionTitle(action) }}</p>
                  <small>{{ action.season }}<template v-if="action.spiritName !== atlasActionTitle(action)"> · {{ action.spiritName }}</template></small>
                </div>
                </div>
                <details :key="`${atlasCharacter?.id}/${action.id}/${action.checked}`"
                         class="atlas-location" :open="!action.checked">
                  <summary :aria-label="atlasActionTitle(action) + ' · 位置参考'">
                    <span>位置参考</span><ChevronDown :size="14" aria-hidden="true" />
                  </summary>
                  <p class="atlas-location-clue">{{ action.hint }}</p>
                  <figure v-if="action.locationImage" class="atlas-guide">
                    <SkyGuideImage :src="action.locationImage.preview" :alt="atlasActionTitle(action) + ' · 位置参考'" />
                    <figcaption><span>位置参考</span><button type="button" class="wp-text-action"
                      :aria-label="'放大动作位置参考：' + atlasActionTitle(action)"
                      @click="openLightbox(action.locationImage.full, atlasActionTitle(action) + ' · 位置参考', { theme: 'dark' })"><ZoomIn :size="13" />放大</button></figcaption>
                  </figure>
                  <div class="atlas-entry-links">
                    <button v-if="action.sourceUrl" type="button" class="wp-text-action"
                      :aria-label="'查看位置资料来源：' + atlasActionTitle(action)" @click="openExternalUrl(action.sourceUrl)">资料来源 <ArrowUpRight :size="12" /></button>
                    <button v-if="action.constellationId" type="button" class="wp-text-action" @click="openAtlasAction(action)">星盘 <ArrowRight :size="12" /></button>
                  </div>
                </details>
              </article>
              </div>
            </section>
          </div>
      </template>
      <div v-else class="atlas-empty">
          <Map :size="30" /><h2>{{ atlasHideCollected ? '当前筛选下没有待收集内容' : '没有找到匹配的地点' }}</h2>
          <p>试试其他地图、收集类型或关键词。</p>
          <button type="button" class="wp-text-action" @click="clearFilters">清除筛选 <ArrowRight :size="14" /></button>
      </div>
      <footer class="atlas-footer">
        <div class="atlas-overview-links">
          <button v-for="map in currentRealmOverviewMaps" :key="map.url" type="button" class="wp-text-action"
                  @click="openLightbox(map.url, (atlasRealmFilter === 'all' ? '全图' : atlasRealmName(atlasRealmFilter)) + ' · ' + map.name, { theme: 'dark' })">
            <Map :size="14" />{{ atlasRealmFilter === 'all' ? map.name : '查看' + map.name }}
          </button>
        </div>
        <p v-if="selectedArea && atlasDashenHint(selectedArea.realmName)" class="atlas-sync-hint">{{ atlasDashenHint(selectedArea.realmName) }}</p>
        <button type="button" class="wp-text-action" :disabled="isSyncingAtlasDashen || !atlasQueryId || !atlasCanEdit"
                :title="atlasQueryId ? atlasDashenSummary || '同步当前角色的小金人' : '在金库的光遇角色资料中填写查询ID后可同步'"
                @click="syncAtlasFromDashen">
          <RefreshCw :size="13" :class="{ 'animate-spin': isSyncingAtlasDashen }" />
          {{ isSyncingAtlasDashen ? `${atlasSyncingName} · 同步中` : '大神同步光翼' }}
        </button>
      </footer>
    </div>
    <p class="atlas-total">角色总进度<span>光翼 {{ atlasProgress.lights }}/{{ atlasProgress.lightTotal }}</span><span>神殿 {{ atlasProgress.shrines }}/{{ atlasProgress.shrineTotal }}</span><span>动作 {{ atlasProgress.actions }}/{{ atlasProgress.actionTotal }}</span></p>
  </section>
</template>

<style scoped>
.sky-atlas { padding-top: 4px; }
.atlas-toolbar, .atlas-tools, .atlas-kinds, .atlas-pending, .atlas-search { display: flex; align-items: center; gap: 8px; }
.atlas-toolbar { justify-content: space-between; flex-wrap: wrap; margin-bottom: 14px; }
.atlas-kinds { gap: 3px; }
.atlas-kinds button { padding: 8px 13px; border-radius: 6px; font-size: 14px; color: var(--color-muted); cursor: pointer; }
.atlas-kinds button[aria-pressed="true"] { background: var(--color-raised); color: var(--color-ink); font-weight: 600; }
.atlas-search { border-bottom: 1px solid var(--color-hairline); color: var(--color-muted); padding: 6px 2px; }
.atlas-search input { width: 165px; min-width: 0; background: transparent; color: var(--color-ink); font-size: 13px; }
.atlas-search button { display: grid; place-items: center; width: 24px; height: 24px; cursor: pointer; }
.atlas-pending { min-height: 36px; padding: 5px 8px; white-space: nowrap; border-radius: 5px; color: var(--color-muted); font-size: 12px; cursor: pointer; }
.atlas-pending[aria-pressed="true"] { background: var(--color-raised); color: var(--color-stamp-sky); }
.atlas-action-kinds { display: flex; gap: 14px; margin-bottom: 14px; }
.atlas-action-kinds button { padding: 4px 0; border-bottom: 1px solid transparent; font-size: 12px; color: var(--color-muted); cursor: pointer; }
.atlas-action-kinds [aria-pressed="true"] { border-color: var(--color-stamp-sky); color: var(--color-stamp-sky); }
.atlas-journey { overflow: hidden; border: 1px solid var(--color-hairline); border-radius: 8px; background: var(--color-paper); }
.atlas-map-nav { display: flex; flex-wrap: wrap; gap: 4px 6px; padding: 10px 16px; border-bottom: 1px solid var(--color-hairline); }
.atlas-map-nav button { min-height: 36px; padding: 7px 11px; border-radius: 5px; color: var(--color-muted); font-size: 13px; cursor: pointer; }
.atlas-map-nav [aria-pressed="true"] { color: var(--color-stamp-sky); background: color-mix(in srgb, var(--color-stamp-sky) 12%, transparent); font-weight: 600; }
.atlas-route, .atlas-route-actions { display: flex; align-items: center; gap: 10px; }
.atlas-route { justify-content: space-between; padding: 8px 16px 2px 22px; }
.atlas-route p { display: flex; align-items: baseline; gap: 12px; font-size: 13px; color: var(--color-ink); }
.atlas-route p span { color: var(--color-faint); font-size: 12px; }
.atlas-route-actions { flex-shrink: 0; gap: 4px; }
.atlas-route-actions > span { font-size: 11px; font-variant-numeric: tabular-nums; color: var(--color-faint); margin-right: 8px; white-space: nowrap; }
.atlas-route-actions button { display: grid; place-items: center; width: 36px; height: 36px; border-radius: 5px; color: var(--color-muted); cursor: pointer; }
.atlas-area-nav { display: flex; flex-wrap: wrap; gap: 6px; padding: 4px 22px 16px; scrollbar-color: var(--color-hairline) var(--color-paper); }
.atlas-area-nav::-webkit-scrollbar { height: 5px; }.atlas-area-nav::-webkit-scrollbar-track { background: var(--color-paper); }.atlas-area-nav::-webkit-scrollbar-thumb { background: var(--color-hairline); border-radius: 5px; }
.atlas-area-nav button { display: flex; align-items: center; gap: 8px; min-height: 36px; max-width: 100%; padding: 7px 10px; border: 1px solid var(--color-hairline); border-radius: 5px; color: var(--color-muted); font-size: 12px; cursor: pointer; scroll-margin-inline: 16px; }
.atlas-area-nav small { font-size: 11px; color: var(--color-faint); font-variant-numeric: tabular-nums; white-space: nowrap; }
.atlas-area-nav [aria-current="location"] { color: var(--color-stamp-sky); border-color: var(--color-stamp-sky); background: color-mix(in srgb, var(--color-stamp-sky) 8%, transparent); }
.atlas-area-nav [aria-current="location"] small { color: var(--color-stamp-sky); }
.atlas-area-complete { color: var(--color-done); flex-shrink: 0; }
.atlas-scene { min-height: 110px; position: relative; display: flex; align-items: flex-end; padding: 22px; background: var(--color-raised); }
.atlas-scene-image { position: absolute; inset: 0; width: 100%; height: 100%; cursor: zoom-in; }
.atlas-scene-image img { width: 100%; height: 100%; object-fit: cover; object-position: center 48%; }
.atlas-scene-image::after { content: ''; position: absolute; inset: 0; background: linear-gradient(90deg, #141614ed, #141614a6 36%, #14161426 80%); }
.atlas-scene-copy, .atlas-scene-hint { position: relative; pointer-events: none; }
.atlas-scene-copy p { display: flex; align-items: center; gap: 12px; font-size: 12px; color: var(--color-muted); }
.atlas-scene-copy p > span { padding-left: 12px; border-left: 1px solid var(--color-hairline); }
.atlas-scene-copy h2 { margin-top: 7px; font-family: var(--font-display); font-size: 29px; line-height: 1.4; font-weight: 500; color: var(--color-ink); }
.atlas-scene-hint { display: flex; align-items: center; gap: 6px; margin-left: auto; padding: 5px 8px; border-radius: 4px; background: var(--color-canvas); font-size: 11px; color: var(--color-muted); }
.atlas-checklist { display: flex; flex-direction: column; gap: 28px; padding: 20px 22px 22px; }
.atlas-section { min-width: 0; }
.atlas-section h3 { display: flex; align-items: baseline; gap: 10px; font-size: 14px; font-weight: 600; margin-bottom: 10px; }
.atlas-section h3 small { color: var(--color-faint); font-size: 12px; font-weight: 400; }
.atlas-entries { columns: 2; column-gap: 28px; }
.atlas-entry { display: flow-root; break-inside: avoid; min-width: 0; border-top: 1px solid var(--color-hairline); padding-top: 8px; margin-bottom: 20px; }
.atlas-light-entries { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0 28px; }
.atlas-guide-hint { color: var(--color-muted); font-size: 12px; margin: -2px 0 16px; }
.atlas-check-row { display: flex; align-items: flex-start; gap: 6px; min-height: 44px; padding: 8px 0; }
.atlas-check { display: grid; place-items: center; flex: 0 0 44px; height: 44px; margin-top: -5px; cursor: pointer; }
.atlas-check > span { display: grid; place-items: center; width: 18px; height: 18px; border: 1px solid var(--color-faint); border-radius: 4px; }
.atlas-check[aria-pressed="true"] > span { color: var(--color-done); border-color: var(--color-done); background: color-mix(in srgb, var(--color-done) 10%, transparent); }
.atlas-check-row > p, .atlas-action-copy { min-width: 0; flex: 1; padding-top: 3px; font-size: 14px; line-height: 1.65; overflow-wrap: anywhere; }
.atlas-action-copy small { display: block; color: var(--color-muted); font-size: 12px; margin-top: 3px; }
.is-checked { color: var(--color-faint); text-decoration: line-through; }
.atlas-location summary { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 44px; color: var(--color-muted); font-size: 12px; cursor: pointer; list-style: none; }
.atlas-location summary::-webkit-details-marker { display: none; }
.atlas-location[open] > summary svg { transform: rotate(180deg); }
.atlas-location-clue { font-size: 13px; line-height: 1.8; color: var(--color-muted); margin: 4px 0 12px; }
.atlas-guide :deep(.sky-guide-art) { border-radius: 5px; background: transparent; }
.atlas-guide :deep(.sky-guide-art:not(.is-ready)) { min-height: 84px; background: var(--color-canvas); }
.atlas-guide :deep(.sky-guide-art > img) { position: relative; display: block; width: 100%; height: auto; object-fit: contain; }
.atlas-guide figcaption, .atlas-entry-links { display: flex; align-items: center; justify-content: space-between; gap: 12px; color: var(--color-faint); font-size: 12px; }
.atlas-guide figcaption .wp-text-action, .atlas-entry-links .wp-text-action { min-height: 44px; font-size: 12px; }
.atlas-entry-links { justify-content: flex-end; }
.atlas-footer { display: flex; align-items: center; flex-wrap: wrap; gap: 10px 18px; margin-inline: 22px; padding-block: 12px; border-top: 1px solid var(--color-hairline); }
.atlas-overview-links { display: flex; flex-wrap: wrap; gap: 16px; margin-right: auto; }.atlas-footer .wp-text-action { font-size: 12px; min-height: 30px; }
.atlas-sync-hint { font-size: 12px; color: var(--color-muted); }
.atlas-total { display: flex; flex-wrap: wrap; gap: 4px 16px; margin-top: 12px; font-size: 12px; color: var(--color-faint); }
.atlas-empty { display: grid; place-content: center; justify-items: center; gap: 12px; min-height: 240px; padding: 20px; color: var(--color-muted); text-align: center; }.atlas-empty h2 { font-size: 17px; color: var(--color-ink); }.atlas-empty p { font-size: 13px; }
.atlas-notice { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px 20px; padding: 10px 0 16px; color: var(--color-muted); font-size: 12px; }
button:disabled { cursor: not-allowed; opacity: .45; }
button:focus-visible, input:focus-visible, summary:focus-visible { outline: 2px solid var(--color-stamp-sky); outline-offset: -2px; }
@media (hover: hover) { .atlas-map-nav button:hover, .atlas-area-nav button:hover, .atlas-route-actions button:hover, .atlas-kinds button:hover, .atlas-pending:hover { background: var(--color-raised); }.atlas-check:not(:disabled):hover > span { border-color: var(--color-stamp-sky); } }
@container (max-width: 760px) {
  .atlas-tools { flex: 1; justify-content: flex-end; }.atlas-entries { columns: 1; }
  .atlas-entry { margin-bottom: 18px; }.atlas-light-entries { grid-template-columns: minmax(0, 1fr); gap: 0; }
}
@container (max-width: 600px) {
  .atlas-toolbar { gap: 10px; }.atlas-kinds { width: 100%; }.atlas-kinds button { flex: 1; }.atlas-tools { width: 100%; }.atlas-search { flex: 1; }.atlas-search input { width: 100%; }
  .atlas-map-nav { padding: 8px 10px; gap: 3px; }.atlas-map-nav button { min-height: 40px; padding: 7px 10px; }
  .atlas-route { padding-inline: 16px; }.atlas-route p { gap: 8px; }.atlas-route-actions > span { display: none; }
  .atlas-area-nav { flex-wrap: nowrap; overflow-x: auto; padding: 4px 16px 14px; gap: 6px; scrollbar-width: thin; }.atlas-area-nav button { flex-shrink: 0; min-height: 40px; }
  .atlas-scene { min-height: 100px; padding: 18px; }.atlas-scene-copy h2 { font-size: 25px; }.atlas-scene-copy p > span { display: none; }.atlas-scene-hint { font-size: 10px; padding: 4px 6px; }
  .atlas-checklist { padding: 18px; }.atlas-footer { margin-inline: 18px; }
}
</style>
