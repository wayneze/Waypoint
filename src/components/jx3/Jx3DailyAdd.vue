<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Plus, Search, X } from '@lucide/vue';
import { activeCharacter } from '../../workspace/core';
import {
  attachJx3CalendarSlot,
  attachJx3CatalogHit,
  jx3AddDraft,
  jx3CalendarSlotHits,
  jx3CatalogHits,
  jx3CalendarSlotIconUrl,
  moveJx3AddHighlight,
  submitJx3Add,
  submitJx3CustomTask,
  syncJx3DailyCalendar
} from '../../workspace/jx3/calendar';
import {
  extractJx3TaskScheduleTime,
  jx3CatalogHitIconUrl,
  type JX3_CALENDAR_SLOTS
} from '../../services/jx3Schedule';
import Jx3BoxIcon from '../Jx3BoxIcon.vue';
const emit = defineEmits<{ close: [] }>();
const input = ref<HTMLInputElement | null>(null);
onMounted(() => input.value?.focus());
const detectedTime = computed(
  () =>
    jx3AddDraft.schedule_time.trim() ||
    extractJx3TaskScheduleTime(jx3AddDraft.text).scheduleTime ||
    ''
);
function clearTime() {
  jx3AddDraft.schedule_time = '';
  const parsed = extractJx3TaskScheduleTime(jx3AddDraft.text);
  if (parsed.scheduleTime) jx3AddDraft.text = parsed.title;
}
async function pickSlot(key: (typeof JX3_CALENDAR_SLOTS)[number]['key']) {
  if (!activeCharacter.value) return;
  if (await attachJx3CalendarSlot(activeCharacter.value, key))
    await syncJx3DailyCalendar();
  jx3AddDraft.text = '';
}
</script>
<template>
  <div class="jx3-add-panel grid gap-3" @keydown.esc="emit('close')">
    <div class="jx3-toolbar">
      <nav class="jx3-segments" aria-label="新待办周期">
        <button
          type="button"
          :aria-pressed="jx3AddDraft.category === 'daily'"
          @click="jx3AddDraft.category = 'daily'"
        >
          日常</button
        ><button
          type="button"
          :aria-pressed="jx3AddDraft.category === 'weekly'"
          @click="jx3AddDraft.category = 'weekly'"
        >
          周常
        </button>
      </nav>
      <button
        type="button"
        class="jx3-remove"
        aria-label="收起添加待办"
        @click="emit('close')"
      >
        <X :size="14" />
      </button>
    </div>
    <label class="jx3-search max-w-none"
      ><Search :size="14" /><input
        ref="input"
        v-model="jx3AddDraft.text"
        aria-label="新待办名称"
        placeholder="手写待办，或搜首领、成就、挂件、声望、奇遇"
        @keydown.enter.prevent="submitJx3Add(false)"
        @keydown.down.prevent="moveJx3AddHighlight(1)"
        @keydown.up.prevent="moveJx3AddHighlight(-1)"
        @keydown.tab.prevent="submitJx3Add(true)"
    /></label>
    <div class="jx3-toolbar">
      <label class="flex items-center gap-2 text-faint text-[12px]"
        >时段<input
          v-model="jx3AddDraft.schedule_time"
          aria-label="待办时段（可选）"
          placeholder="如 20:00 - 21:00"
          class="jx3-select" /></label
      ><span v-if="detectedTime" class="jx3-status"
        >识别到 {{ detectedTime
        }}<button type="button" aria-label="清除待办时段" @click="clearTime">
          <X :size="13" /></button></span
      ><button
        type="button"
        class="jx3-button is-primary"
        @click="submitJx3CustomTask()"
      >
        <Plus :size="13" />保存待办
      </button>
    </div>
    <div class="jx3-options">
      <span>常用时段</span
      ><button
        v-for="time in [
          '12:00',
          '20:00',
          '20:00 - 21:00',
          '每周三、周五 20:00'
        ]"
        :key="time"
        type="button"
        @click="jx3AddDraft.schedule_time = time"
      >
        {{ time }}
      </button>
    </div>
    <div v-if="jx3AddDraft.text.trim()">
      <button
        v-for="(slot, index) in jx3CalendarSlotHits"
        :key="slot.key"
        type="button"
        class="jx3-record-head"
        :class="{ 'bg-raised': jx3AddDraft.highlight === index }"
        @click="pickSlot(slot.key)"
      >
        <Jx3BoxIcon
          :src="jx3CalendarSlotIconUrl(slot.key)"
          size="w-6 h-6"
        /><span class="jx3-record-copy">日历 · {{ slot.label }}</span
        ><small>挂入日历</small>
      </button>
      <button
        v-for="(hit, index) in jx3CatalogHits"
        :key="hit.kind + hit.id"
        type="button"
        class="jx3-record-head"
        :class="{
          'bg-raised':
            jx3AddDraft.highlight === jx3CalendarSlotHits.length + index
        }"
        @click="attachJx3CatalogHit(hit)"
      >
        <Jx3BoxIcon :src="jx3CatalogHitIconUrl(hit)" size="w-6 h-6" /><span
          class="jx3-record-copy"
          >{{ hit.tag }} · {{ hit.title
          }}<small v-if="hit.schedule_time">
            · {{ hit.schedule_time }}</small
          ></span
        ><small>{{ hit.hint }}</small>
      </button>
      <p class="text-faint text-[12px]">
        Enter 保存原文；点选或 Tab 引用目录条目。
      </p>
    </div>
  </div>
</template>
