<script setup lang="ts">
import type { SkyTodoGroup } from '../../services/skyPlan';
import { toggleSkyGroup } from '../../workspace/sky/plan';
import { dailyCharacterId, isSavingDaily } from '../../workspace/sky/daily';
import { openLightbox } from '../../workspace/guides/lightbox';
import { Check, ChevronDown, Maximize2, Flame, ListChecks } from '@lucide/vue';
import { hotlinkImageSrc } from './wikiImage';
import SkyEventGuide from './SkyEventGuide.vue';
import SkyGuideImage from './SkyGuideImage.vue';
import { computed } from 'vue';

const props = defineProps<{ groups: SkyTodoGroup[]; characterId?: string; compact?: boolean }>();
const displayedGroups = computed(() => props.groups.map(group => ({
  ...group,
  sections: group.done
    ? [{ completed: false, items: group.items }]
    : [
      { completed: false, items: group.items.filter(item => !item.done) },
      { completed: true, items: group.items.filter(item => item.done) },
    ].filter(section => section.items.length),
})));
</script>

<template>
  <div class="sky-todo-list">
    <article v-for="group in displayedGroups" :key="group.id" class="sky-todo" :class="{ 'is-done': group.done, 'is-compact': compact, 'is-candles': group.id === 'candles', 'is-event': group.id.startsWith('event/') }">
      <div v-if="!compact && group.id.startsWith('event/')" class="sky-event-task">
        <SkyGuideImage class="sky-event-art" :src="group.event?.icon" :alt="`${group.title}活动封面`">
          <span class="sky-event-kicker">活动日常</span>
        </SkyGuideImage>
        <div class="sky-event-task-body">
          <div class="sky-event-task-title">
            <div><small>{{ group.done ? '今日已完成' : '今日待收集' }}</small><h4>{{ group.title }}</h4></div>
            <button type="button" class="sky-task-complete" :class="{ 'is-done': group.done }"
                    :disabled="!(characterId || dailyCharacterId) || !group.ready || isSavingDaily"
                    :aria-label="`${group.done ? '撤销' : '完成'}：${group.title}`" :aria-pressed="group.done"
                    @click="toggleSkyGroup(group, characterId)">
              <Check class="w-4 h-4" />{{ group.done ? '已完成' : '标记完成' }}
            </button>
          </div>
          <details class="sky-event-instructions">
            <summary>收集方式 <ChevronDown class="w-3.5 h-3.5" /></summary>
            <p v-for="item in group.items" :key="item.id">{{ item.title }}</p>
          </details>
          <SkyEventGuide v-if="group.event" :item="group.event" compact />
          <button v-else-if="group.image" type="button" class="wp-text-action text-sm" @click="openLightbox(hotlinkImageSrc(group.image), group.title)">查看活动攻略 <Maximize2 class="w-3.5 h-3.5" /></button>
        </div>
      </div>
      <template v-else>
      <div class="sky-todo-heading">
        <span class="sky-todo-icon"><Flame v-if="group.id === 'candles'" class="w-4 h-4" /><ListChecks v-else class="w-4 h-4" /></span>
        <div class="min-w-0 flex-1">
          <h4 class="text-[15px] font-semibold text-ink">{{ group.title }}</h4>
          <p v-if="compact || !group.ready" class="text-[12px] text-muted mt-0.5">{{ group.done ? '今日已完成' : group.hint }}</p>
        </div>
        <span v-if="!compact && group.ready" class="sky-group-progress" :class="{ 'is-done': group.done }">
          {{ group.done ? '已完成' : `${group.items.filter(i => i.done).length} / ${group.items.length}` }}
        </span>
        <button type="button" class="sky-check" :class="{ 'is-done': group.done }"
                :disabled="!(characterId || dailyCharacterId) || !group.ready || isSavingDaily"
                :aria-label="`${group.done ? '撤销' : '完成'}：${group.title}`" :aria-pressed="group.done"
                @click="toggleSkyGroup(group, characterId)">
          <Check class="w-4 h-4" />
        </button>
      </div>
      <div v-if="!compact && group.ready" class="sky-todo-details sky-task-layout">
        <div class="sky-task-guide">
          <SkyGuideImage :src="group.image" :alt="`${group.title}完整图解`" />
          <button v-if="group.image" type="button" class="sky-quest-guide-open"
                  :aria-label="`查看${group.title}完整攻略`" @click="openLightbox(hotlinkImageSrc(group.image), `${group.title} · 完整攻略`)">
            <Maximize2 class="w-4 h-4" />查看完整图解
          </button>
        </div>
        <div class="sky-task-checklist">
        <component v-for="section in group.sections" :key="`${section.completed}/${section.items.map(item => item.id).join(',')}`"
                   :is="section.completed ? 'details' : 'div'" :class="{ 'sky-completed-items': section.completed }">
        <summary v-if="section.completed" class="sky-completed-items-summary">
          已完成 {{ section.items.length }} 项 <ChevronDown class="w-3 h-3" />
        </summary>
        <div class="sky-guide-grid">
          <div v-for="item in section.items" :key="item.id" class="sky-guide-row">
            <label class="sky-guide-copy" :class="{ 'is-done': item.done }">
              <input type="checkbox" :checked="item.done"
                     :disabled="!(characterId || dailyCharacterId) || isSavingDaily"
                     @change="toggleSkyGroup(group, characterId, item)" />
              <span class="sky-guide-text">
                <span class="sky-guide-title">{{ item.title }}</span>
                <small v-if="group.id !== 'quests' && item.detail && item.detail !== '未标明地图'" :title="item.detail">{{ item.detail }}</small>
              </span>
            </label>
          </div>
        </div>
        </component>
        </div>
      </div>
      </template>
    </article>
  </div>
</template>

<style>
.sky-todo-list { display: grid; gap: 22px; container-type: inline-size; }
.sky-todo { min-width: 0; }
.sky-todo-heading { display: flex; align-items: center; gap: 10px; margin-bottom: 12px; }
.sky-todo-icon { display: grid; place-items: center; color: var(--game-color); width: 30px; height: 30px; background: color-mix(in srgb, var(--game-color) 9%, transparent); border-radius: 9px; }
.sky-check { width: 23px; height: 23px; border: 1px solid var(--color-hairline); border-radius: 7px; display: grid; place-items: center; color: transparent; flex-shrink: 0; cursor: pointer; }
.sky-check:hover:not(:disabled) { color: var(--color-beacon); border-color: var(--color-beacon); }
.sky-check.is-done { background: var(--color-done); border-color: var(--color-done); color: var(--color-canvas); }
.sky-check:disabled { opacity: .45; cursor: default; }
.sky-group-progress { color: var(--color-faint); font-size: 11px; }
.sky-group-progress.is-done { color: var(--color-done); }
.sky-todo-details { margin-top: 10px; }
.sky-task-layout { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1.15fr); gap: 18px; align-items: start; }
.sky-task-guide { position: relative; height: 250px; border-radius: 11px; overflow: hidden; }
.sky-task-guide .sky-guide-art { height: 100%; }
.sky-quest-guide-open { position: absolute; inset: auto 0 0; display: flex; justify-content: center; align-items: center; gap: 8px; padding: 28px 12px 14px; background: linear-gradient(transparent, #101917ed); color: #fff; font-size: 12px; cursor: zoom-in; }
.sky-task-checklist .sky-guide-grid { grid-template-columns: 1fr; gap: 8px; }
.sky-task-checklist .sky-guide-copy { align-items: center; padding: 14px; min-height: 56px; }
.sky-task-checklist .sky-guide-copy input { margin-top: 0; }
.sky-completed-items { margin-top: 12px; }
.sky-completed-items-summary { display: flex; align-items: center; gap: 7px; padding: 6px 0; color: var(--color-muted); font-size: 12px; cursor: pointer; list-style: none; }
.sky-completed-items-summary::-webkit-details-marker { display: none; }
.sky-completed-items[open] .sky-completed-items-summary svg { transform: rotate(180deg); }
.sky-guide-grid { display: grid; }
.sky-guide-row { display: flex; flex-direction: column; min-width: 0; overflow: hidden; border: 1px solid var(--color-hairline); border-radius: 11px; background: var(--color-surface); }
.sky-guide-copy { display: flex; order: 2; gap: 10px; align-items: start; cursor: pointer; min-width: 0; padding: 12px; flex: 1; }
.sky-guide-copy input { width: 15px; height: 15px; margin-top: 4px; flex-shrink: 0; accent-color: var(--color-done); }
.sky-guide-text { min-width: 0; }
.sky-guide-title { color: var(--color-ink); font-size: 14px; line-height: 1.6; }
.sky-guide-text small { display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; margin-top: 4px; color: var(--color-muted); font-size: 12px; line-height: 1.6; }
.sky-guide-copy.is-done .sky-guide-title { color: var(--color-muted); }
.sky-todo.is-compact { padding: 10px 0; }
.sky-todo.is-compact .sky-todo-heading { margin: 0; }
.sky-event-task { border: 1px solid var(--color-hairline); border-radius: 14px; overflow: hidden; display: grid; grid-template-columns: minmax(180px, .85fr) minmax(0, 1fr); background: var(--color-surface); }
.sky-event-art { position: relative; min-height: 210px; overflow: hidden; background: color-mix(in srgb, var(--game-color) 8%, var(--color-canvas)); }
.sky-event-kicker { position: absolute; top: 12px; left: 12px; border-radius: 5px; padding: 4px 8px; background: #101917d9; color: #fff; font-size: 11px; }
.sky-event-task-body { padding: 20px; min-width: 0; }
.sky-event-task-title { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
.sky-event-task-title small { font-size: 11px; color: var(--color-beacon); }
.sky-event-task-title h4 { font-size: 20px; font-weight: 600; color: var(--color-ink); margin-top: 4px; }
.sky-task-complete { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; padding: 8px 11px; border: 1px solid var(--color-hairline); border-radius: 6px; cursor: pointer; color: var(--color-ink); }
.sky-task-complete:hover { border-color: var(--color-done); color: var(--color-done); }
.sky-task-complete.is-done { color: var(--color-done); background: color-mix(in srgb, var(--color-done) 9%, transparent); }
.sky-task-complete:disabled { opacity: .5; cursor: default; }
.sky-event-instructions { margin-top: 20px; font-size: 13px; color: var(--color-muted); }
.sky-event-instructions summary { display: flex; align-items: center; justify-content: space-between; gap: 8px; cursor: pointer; list-style: none; padding: 8px 0; }
.sky-event-instructions summary::-webkit-details-marker { display: none; }
.sky-event-instructions[open] summary svg { transform: rotate(180deg); }
.sky-event-instructions p { line-height: 1.85; padding: 4px 0; }
.sky-event-task-body .sky-event-guide { margin-top: 4px; border-top: 1px solid var(--color-hairline); }
@container (max-width: 600px) {
  .sky-task-layout { grid-template-columns: 1fr; gap: 12px; }
  .sky-task-guide { height: 180px; }
  .sky-event-task { grid-template-columns: 1fr; }
  .sky-event-art { min-height: 150px; height: 150px; }
}
</style>
