<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import type { TaskItem } from '../../types';
import {
  activeCharacter,
  activeCharObj,
  allAccountTasks,
  handleDeleteTask,
  handleRenameTask,
  handleToggleTask,
  handleSwitchTaskCategory
} from '../../workspace/core';
import {
  attachJx3CalendarSlot,
  jx3CalendarSlotIconUrl,
  jx3TaskVisual,
  syncJx3DailyCalendar,
  jx3MapRefreshTick
} from '../../workspace/jx3/calendar';
import { JX3_CALENDAR_SLOTS } from '../../services/jx3Schedule';
import { Check, ChevronDown, Clock, Pencil, Plus, Trash2 } from '@lucide/vue';
import Jx3DailyAdd from './Jx3DailyAdd.vue';
import Jx3Agenda from './Jx3Agenda.vue';
import Jx3BoxIcon from '../Jx3BoxIcon.vue';
import {
  extractJx3TaskScheduleTime,
  getJx3TaskScheduleStatus
} from '../../services/jx3Schedule';
import { updateTaskScheduleTime } from '../../services/api';

type Jx3TaskScope = 'all' | 'daily' | 'weekly';

const jx3TaskScope = ref<Jx3TaskScope>('all');

const showTaskAddBox = ref(false);

function toggleTaskAddBox() {
  showTaskAddBox.value = !showTaskAddBox.value;
}

const taskGroups = computed(() => {
  const tasks = allAccountTasks.value[activeCharacter.value] || [];
  return (
    [
      { id: 'daily', label: '今日日常' },
      { id: 'weekly', label: '本周周常' }
    ] as const
  ).map((group) => {
    const rows = tasks.filter((task) =>
      group.id === 'weekly'
        ? task.category === 'weekly'
        : task.category !== 'weekly'
    );
    const pending = rows.filter((task) => !task.done);
    const completed = rows.filter((task) => task.done);
    return {
      ...group,
      rows,
      pending,
      completed,
      sections: [
        { done: false, rows: pending },
        { done: true, rows: completed }
      ].filter((section) => section.rows.length)
    };
  });
});
const visibleTaskGroups = computed(() =>
  taskGroups.value.filter(
    (group) => jx3TaskScope.value === 'all' || group.id === jx3TaskScope.value
  )
);
const pendingCount = computed(() =>
  taskGroups.value.reduce((count, group) => count + group.pending.length, 0)
);
const totalCount = computed(() =>
  taskGroups.value.reduce((count, group) => count + group.rows.length, 0)
);
watch(activeCharacter, () => {
  jx3EditId.value = '';
  showTaskAddBox.value = false;
});

const jx3EditId = ref('');

const jx3EditTitle = ref('');

async function startJx3TaskEdit(task: TaskItem) {
  jx3EditId.value = task.id;
  jx3EditTitle.value = task.title;
  await nextTick();
  const el = document.querySelector<HTMLInputElement>('[data-jx3-edit-title]');
  el?.focus();
  el?.select();
}

async function commitJx3TaskEdit(task: TaskItem) {
  if (jx3EditId.value !== task.id) return;
  const rawTitle = jx3EditTitle.value.trim();
  jx3EditId.value = '';
  if (!rawTitle) return;

  const extracted = extractJx3TaskScheduleTime(rawTitle);
  if (extracted.scheduleTime) {
    task.schedule_time = extracted.scheduleTime;
    await updateTaskScheduleTime(
      task.id,
      extracted.scheduleTime,
      activeCharacter.value
    );
  }
  const cleanTitle = extracted.scheduleTime ? extracted.title : rawTitle;
  if (cleanTitle !== task.title) {
    await handleRenameTask(task, cleanTitle);
  }
}

function cancelJx3TaskEdit() {
  jx3EditId.value = '';
}

function getTaskScheduleBadge(task: TaskItem) {
  if (!task.schedule_time) return null;
  jx3MapRefreshTick.value;
  return getJx3TaskScheduleStatus(task.schedule_time);
}

const CATALOG_KIND_LABEL: Record<string, string> = {
  pendant: '挂件',
  reputation: '声望',
  adventure: '奇遇',
  token: '牌子',
  achievement: '成就',
  special: '特殊',
  appearance: '外观',
  boss: '首领'
};

async function pickCalendarSlot(
  key: (typeof JX3_CALENDAR_SLOTS)[number]['key']
) {
  const charId = activeCharacter.value;
  if (!charId) return;
  const attached = await attachJx3CalendarSlot(charId, key);
  if (attached) await syncJx3DailyCalendar();
}

function catalogLabel(kind?: string) {
  return kind ? CATALOG_KIND_LABEL[kind] || kind : '';
}
</script>

<template>
  <div class="jx3-panel jx3-daily">
    <header class="jx3-lead">
      <div>
        <h2>
          {{
            pendingCount
              ? '今日日程'
              : totalCount
                ? '今日事，已收好'
                : '从一件想做的事开始'
          }}
        </h2>
      </div>
      <div class="jx3-number">
        <strong>{{ pendingCount }}</strong
        ><span>项待办 / 共 {{ totalCount }} 项</span>
      </div>
    </header>
    <p v-if="activeCharObj?.game_id !== 'jx3'" class="jx3-notice">
      先在金库登记并选择剑网3角色，即可记录自己的日常。
    </p>
    <div class="jx3-daily-layout">
      <div class="jx3-daily-main">
        <div class="jx3-toolbar">
          <nav class="jx3-segments" aria-label="待办周期">
            <button
              type="button"
              :aria-pressed="jx3TaskScope === 'all'"
              @click="jx3TaskScope = 'all'"
            >
              全部 <small>{{ totalCount }}</small>
            </button>
            <button
              v-for="group in taskGroups"
              :key="group.id"
              type="button"
              :aria-pressed="jx3TaskScope === group.id"
              @click="jx3TaskScope = group.id"
            >
              {{ group.id === 'daily' ? '日常' : '周常' }}
              <small>{{ group.pending.length }}</small>
            </button>
          </nav>
          <button
            type="button"
            class="jx3-button is-primary"
            :aria-expanded="showTaskAddBox"
            :disabled="activeCharObj?.game_id !== 'jx3'"
            @click="toggleTaskAddBox"
          >
            <Plus :size="15" />添加待办
          </button>
        </div>
        <div class="jx3-quick-add">
          <span>快捷挂入</span
          ><button
            v-for="slot in JX3_CALENDAR_SLOTS.filter((s) => s.defaultOn)"
            :key="slot.key"
            type="button"
            :disabled="activeCharObj?.game_id !== 'jx3'"
            @click="pickCalendarSlot(slot.key)"
          >
            <Jx3BoxIcon
              :src="jx3CalendarSlotIconUrl(slot.key)"
              size="w-4 h-4"
            />{{ slot.label.replace('今日', '') }}<Plus :size="12" />
          </button>
        </div>
        <Jx3DailyAdd v-if="showTaskAddBox" @close="showTaskAddBox = false" />
        <section
          v-for="group in visibleTaskGroups"
          :key="group.id"
          class="jx3-task-section"
          :aria-label="group.label"
        >
          <header class="jx3-section-heading">
            <h3>{{ group.label }}</h3>
            <span
              >{{ group.pending.length }} 待办 ·
              {{ group.completed.length }} 已完成</span
            >
          </header>
          <component
            v-for="section in group.sections"
            :key="`${activeCharacter}/${group.id}/${section.done}/${section.done ? section.rows.map((row) => row.id).join(',') : ''}`"
            :is="section.done ? 'details' : 'div'"
            :class="section.done ? 'jx3-completed' : 'jx3-task-list'"
          >
            <summary v-if="section.done">
              <Check :size="14" /><span
                >已完成 {{ section.rows.length }} 项</span
              ><ChevronDown :size="14" />
            </summary>
            <div
              v-for="task in section.rows"
              :key="task.id"
              class="jx3-task"
              :class="{
                'is-done': task.done,
                'is-active':
                  !task.done && getTaskScheduleBadge(task)?.status === 'active'
              }"
            >
              <button
                type="button"
                class="jx3-check"
                :aria-pressed="task.done"
                :aria-label="(task.done ? '取消完成：' : '完成：') + task.title"
                @click="handleToggleTask(task)"
              >
                <span><Check v-if="task.done" :size="13" /></span>
              </button>
              <Jx3BoxIcon :src="jx3TaskVisual(task).sceneUrl || jx3TaskVisual(task).iconUrl"
                :size="jx3TaskVisual(task).sceneUrl ? 'jx3-task-scene' : 'jx3-task-art'" />
              <div class="jx3-task-copy">
                <input
                  v-if="jx3EditId === task.id"
                  data-jx3-edit-title
                  v-model="jx3EditTitle"
                  :aria-label="'修改待办：' + task.title"
                  @keydown.enter.prevent="commitJx3TaskEdit(task)"
                  @keydown.esc.prevent="cancelJx3TaskEdit"
                  @blur="commitJx3TaskEdit(task)"
                />
                <p v-else @dblclick="startJx3TaskEdit(task)">
                  {{ task.title }}
                </p>
                <small
                  >{{
                    task.source_type === 'api_jx3'
                      ? '日历'
                      : catalogLabel(task.catalog_kind) ||
                        task.tag ||
                        group.label
                  }}<template v-if="task.schedule_time">
                    · {{ task.schedule_time }}</template
                  ></small
                >
              </div>
              <span
                v-if="!task.done && getTaskScheduleBadge(task)"
                class="jx3-status"
                :class="{
                  'is-live': getTaskScheduleBadge(task)?.status === 'active'
                }"
                ><Clock :size="12" />{{
                  getTaskScheduleBadge(task)?.statusText
                }}</span
              >
              <details class="jx3-row-menu">
                <summary :aria-label="'操作待办：' + task.title">···</summary>
                <div>
                  <button type="button" @click="startJx3TaskEdit(task)">
                    <Pencil :size="13" />修改标题
                  </button>
                  <button type="button" @click="handleSwitchTaskCategory(task)">
                    转为{{ group.id === 'daily' ? '周常' : '日常' }}
                  </button>
                  <button
                    type="button"
                    @click="handleDeleteTask(task.id, group.id)"
                  >
                    <Trash2 :size="13" />删除待办
                  </button>
                </div>
              </details>
            </div>
          </component>
          <p v-if="!group.rows.length" class="jx3-empty-line">
            {{
              group.id === 'daily'
                ? '还没有日常待办，可以从快捷挂入或添加待办开始。'
                : '本周暂无安排，长线目标也可以加入这里。'
            }}
          </p>
          <p v-else-if="!group.pending.length" class="jx3-empty-line">
            <Check :size="15" />这一栏已清空，完成记录收在下方。
          </p>
        </section>
      </div>
      <Jx3Agenda />
    </div>
  </div>
</template>
