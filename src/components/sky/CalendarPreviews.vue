<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { calendarCollectionEntries } from '../../services/skyCollection';
import { collectionEntries, collectionCanEdit, toggleCollectionOwned, toggleCollectionWanted } from '../../workspace/sky/collection';
import { state } from '../../workspace/core';
import { calendarBriefPreviews, calendarItemGroups, calendarPreviews, type SkyCalendarItem } from '../../services/skyCalendarService';
import { openLightbox } from '../../workspace/guides/lightbox';
import { hotlinkImageSrc, vWikiImg } from './wikiImage';
import ItemTile from './ItemTile.vue';

const props = defineProps<{ item: SkyCalendarItem }>();
const groups = computed(() => calendarItemGroups(props.item).map(group => ({ ...group,
  items: group.items.map(item => ({ ...item, collection: matchPreview(item.name, item.icon) })),
})));
const collection = computed(() => calendarCollectionEntries(props.item, collectionEntries.value, state.seasonalGuides, state.eventGuides));
function matchPreview(name: string, icon: string) {
  const matches = collection.value.filter(e => e.name === name || (!!icon && e.icon === icon));
  return matches.length === 1 ? matches[0] : undefined;
}
const expanded = ref<Record<string, boolean>>({});
const previewCount = 12;
watch(() => props.item.id, () => { expanded.value = {}; });
</script>

<template>
  <div class="space-y-3">
    <div v-if="calendarBriefPreviews(item).length" class="space-y-2">
      <button v-for="p in calendarBriefPreviews(item)" :key="p.icon" type="button" class="block w-full cursor-zoom-in"
              :aria-label="'放大查看' + p.name" @click="openLightbox(hotlinkImageSrc(p.icon), p.name)">
        <img v-wiki-img="p.icon" :alt="p.name" class="w-full h-auto rounded-md bg-canvas" />
      </button>
    </div>
    <div v-if="calendarPreviews(item, '先祖').length" class="space-y-1">
      <p class="text-[12px] text-muted">先祖</p>
      <div class="flex flex-wrap gap-1.5">
        <div v-for="p in calendarPreviews(item, '先祖')" :key="p.name" class="flex flex-col items-center gap-0.5 w-14">
          <img v-if="p.icon" v-wiki-img="p.icon" alt="" class="w-8 h-8 rounded-md bg-canvas object-contain" />
          <span class="text-[11px] text-muted text-center leading-tight">{{ p.name }}</span>
        </div>
      </div>
    </div>
    <section v-for="group in groups" :key="group.slot" class="space-y-1.5" :aria-label="group.slot">
      <p class="text-[12px] text-muted">{{ group.slot }} · {{ group.items.length }}</p>
      <div class="wp-tiles">
        <ItemTile v-for="p in (expanded[group.slot] ? group.items : group.items.slice(0, previewCount))"
                  :key="p.name + '|' + p.icon" :name="p.name"
                  :wear="p.wear" :icon="p.icon" :source-url="item.url"
                  :toggle="!!p.collection" :done="p.collection?.owned"
                  :wanted="p.collection?.wanted" :disabled="!collectionCanEdit"
                  @toggle="p.collection && toggleCollectionOwned(p.collection!)"
                  @wish="p.collection && toggleCollectionWanted(p.collection!)" />
      </div>
      <button v-if="group.items.length > previewCount" type="button" class="text-[12px] text-beacon py-1 cursor-pointer"
              :aria-expanded="!!expanded[group.slot]" @click="expanded[group.slot] = !expanded[group.slot]">
        {{ expanded[group.slot] ? '收起' : `查看全部 ${group.items.length} 件` }}
      </button>
    </section>
  </div>
</template>
