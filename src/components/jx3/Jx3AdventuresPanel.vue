<script setup lang="ts">
import { ref, watch } from 'vue';
import { activeCharacter } from '../../workspace/core';
import {
  addAdventureRequirementToTask,
  adventureEvaluations,
  jx3AdventureFilter,
  jx3AdventureFocusList,
  jx3AdventureEmptyList,
  jx3AdventurePetList,
  jx3AdventureDoneList,
  jx3AdventureVisibleList,
  jx3AdventureQuery,
  jx3AdventureManualDone,
  toggleAdventureManual,
  jx3Adventures,
  jx3AdventureStatus,
  jx3CompletedAchieves
} from '../../workspace/jx3/progress';
import { Check, ChevronDown, Plus, Search, X } from '@lucide/vue';
import adventureImages from '../../content/jx3/adventure-images.json';

const ADVENTURE_FILTERS = [
  { id: 'focus' as const, label: '有前置' },
  { id: 'empty' as const, label: '没前置' },
  { id: 'pet' as const, label: '宠物' },
  { id: 'done' as const, label: '已完成' }
];

function adventureTypeLabel(type: string) {
  if (type === '宠物奇遇') return '宠物';
  return type.replace(/奇遇$/, '');
}

const failedAdventureCovers = ref<Record<string, boolean>>({});
function coverUrl(source: string) {
  return (adventureImages as Record<string, string>)[source] || source;
}

function adventureFilterCount(id: (typeof ADVENTURE_FILTERS)[number]['id']) {
  if (id === 'empty') return jx3AdventureEmptyList.value.length;
  if (id === 'pet') return jx3AdventurePetList.value.length;
  if (id === 'done') return jx3AdventureDoneList.value.length;
  return jx3AdventureFocusList.value.length;
}

function adventureReqState(
  req: (typeof adventureEvaluations.value)[number]['adv']['requirements'][number]
) {
  if (req.completed) return 'done';
  if (req.match === 'unlinked') return 'unlinked';
  return 'open';
}

function adventureReqLabel(name: string) {
  const all = name.match(/^完成所有[^\[\n]*\[([^\]]+)\]成就/);
  if (all) return `全部「${all[1]}」`;
  const count = name.match(
    /^完成\s*(\d+|[一二两三四五六七八九十]+)\s*个[^\[\n]*\[([^\]]+)\]成就/
  );
  if (count) return `${count[1]}个「${count[2]}」`;
  const one = name.match(/^完成\[([^\]]+)\]成就$/);
  if (one) return one[1];
  const task = name.match(/^完成(?:任务\[([^\]]+)\]|\[([^\]]+)\]任务)$/);
  if (task) return task[1] || task[2] || name;
  return name.replace(/[\[\]]/g, '');
}

function adventureRemainLine(
  item: (typeof adventureEvaluations.value)[number]
) {
  if (item.completeMatch === 'done') return '以角色快照中的完成记录为准';
  const open = item.adv.requirements.filter(r => !r.completed && r.match === 'missing');
  const pending = item.unlinkedCount ? `另 ${item.unlinkedCount} 项待核对` : '';
  if (!open.length) return pending || '已确认的前置均已达成';
  const first = adventureReqLabel(open[0].name);
  const missing = open.length === 1 ? `待补：${first}` : `待补：${first} 等 ${open.length} 项`;
  return [missing, pending].filter(Boolean).join(' · ');
}

const openAdventureId = ref('');

watch([jx3AdventureFilter, activeCharacter], () => {
  openAdventureId.value = '';
});

function toggleAdventureOpen(id: string) {
  openAdventureId.value = openAdventureId.value === id ? '' : id;
}
function requirementGroups(item: (typeof adventureEvaluations.value)[number]) {
  return [false, true]
    .map((done) => ({
      done,
      rows: item.adv.requirements.filter((req) => req.completed === done)
    }))
    .filter((group) => group.rows.length);
}
</script>

<template>
  <div class="jx3-panel jx3-adventures">
    <header class="jx3-lead">
      <div>
        <h2>奇遇寻踪</h2>
        <p>奖励与前置条件</p>
      </div>
      <div class="jx3-number">
        <strong>{{ jx3AdventureDoneList.length }}</strong
        ><span>奇遇已完成</span>
      </div>
    </header>
    <div class="jx3-toolbar">
      <nav class="jx3-segments" aria-label="奇遇分组">
        <button
          v-for="f in ADVENTURE_FILTERS"
          :key="f.id"
          type="button"
          :aria-pressed="jx3AdventureFilter === f.id"
          @click="jx3AdventureFilter = f.id"
        >
          {{ f.label }}<small>{{ adventureFilterCount(f.id) }}</small>
        </button>
      </nav>
      <label class="jx3-search"
        ><Search :size="14" /><input
          v-model="jx3AdventureQuery"
          aria-label="搜索奇遇"
          placeholder="奇遇名称或前置条件" /><button
          v-if="jx3AdventureQuery"
          type="button"
          aria-label="清空奇遇搜索"
          @click="jx3AdventureQuery = ''"
        >
          <X :size="14" /></button
      ></label>
    </div>
    <p v-if="jx3AdventureStatus === 'fail'" class="jx3-notice">
      奇遇目录暂时无法更新，当前显示已有目录。
    </p>
    <p
      v-else-if="jx3AdventureStatus === 'idle' && !jx3Adventures.length"
      class="jx3-notice"
      role="status"
    >
      正在读取奇遇目录…
    </p>
    <p v-else-if="jx3CompletedAchieves.size === 0" class="jx3-notice">
      导入茗伊成就后，奇遇及前置的完成情况会自动更新。
    </p>
    <p v-if="!jx3AdventureVisibleList.length" class="jx3-empty-line">
      {{
        jx3AdventureFilter === 'done'
          ? '还没有记录已完成的奇遇。'
          : '没有符合筛选的奇遇，试试其他分组或关键词。'
      }}
    </p>
    <div v-else class="jx3-record-list">
      <article
        v-for="advItem in jx3AdventureVisibleList"
        :key="advItem.adv.id"
        class="jx3-record"
        :data-adventure-id="advItem.adv.id"
      >
        <button
          type="button"
          class="jx3-record-head"
          :aria-expanded="openAdventureId === advItem.adv.id"
          @click="toggleAdventureOpen(advItem.adv.id)"
        >
          <img
            v-if="
              advItem.adv.coverUrl && !failedAdventureCovers[advItem.adv.id]
            "
            class="jx3-cover"
            :src="coverUrl(advItem.adv.coverUrl)"
            alt=""
            loading="lazy"
            referrerpolicy="no-referrer"
            @error="failedAdventureCovers[advItem.adv.id] = true"
          />
          <div class="jx3-record-copy">
            <small>{{ adventureTypeLabel(advItem.adv.type) }}</small>
            <h3>{{ advItem.adv.name }}</h3>
            <p v-if="advItem.adv.reward">奖励 · {{ advItem.adv.reward }}</p>
            <p v-if="openAdventureId !== advItem.adv.id">
              {{ adventureRemainLine(advItem) }}
            </p>
          </div>
          <div class="jx3-record-count">
            <strong v-if="advItem.total && advItem.completeMatch !== 'done'"
              >{{ advItem.completedCount }} / {{ advItem.total }}</strong
            ><small>{{
              jx3AdventureFilter === 'done'
                ? '已完成'
                : advItem.total
                  ? '前置已确认'
                  : '暂无前置记录'
            }}</small
            ><progress
              v-if="advItem.total && advItem.completeMatch !== 'done'"
              class="jx3-progress"
              :max="advItem.total"
              :value="advItem.completedCount"
              :aria-label="advItem.adv.name + '前置进度'"
            />
          </div>
          <ChevronDown :size="15" />
        </button>
        <div
          v-if="openAdventureId === advItem.adv.id"
          class="jx3-record-details"
        >
          <p v-if="!advItem.adv.requirements.length" class="text-faint">
            目录未记录前置条件，完成情况以茗伊快照为准。
          </p>
          <p v-if="advItem.completeMatch === 'done'" class="jx3-notice">已在角色成就快照中确认完成，不再把当前前置条件列为缺口。</p>
          <component
            v-if="advItem.completeMatch !== 'done'"
            v-for="group in requirementGroups(advItem)"
            :is="group.done ? 'details' : 'div'"
            :key="`${activeCharacter}/${advItem.adv.id}/${group.done}/${group.done ? group.rows.map((r) => r.id).join(',') : ''}`"
            :class="{ 'jx3-completed': group.done }"
          >
            <summary v-if="group.done">
              <Check :size="14" />已达成 {{ group.rows.length }} 项<ChevronDown
                :size="14"
              />
            </summary>
            <div
              v-for="req in group.rows"
              :key="req.id"
              class="jx3-requirement"
              :class="{ 'is-done': group.done }"
            >
              <p>{{ adventureReqLabel(req.name) }}</p>
              <button
                v-if="adventureReqState(req) === 'open'"
                type="button"
                class="jx3-button"
                @click="addAdventureRequirementToTask(req, advItem.adv.name)"
              >
                <Plus :size="13" />加入待办
              </button>
              <button
                v-else-if="
                  adventureReqState(req) === 'unlinked' ||
                  jx3AdventureManualDone[req.id]
                "
                type="button"
                class="jx3-button"
                :aria-pressed="!!jx3AdventureManualDone[req.id]"
                @click="toggleAdventureManual(req.id)"
              >
                {{ req.completed ? '撤销手记' : '待核对 · 记为达成' }}
              </button>
              <span v-else class="jx3-status is-done"
                ><Check :size="13" />已达成</span
              >
            </div>
          </component>
        </div>
      </article>
    </div>
  </div>
</template>
