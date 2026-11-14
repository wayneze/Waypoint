<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { activeCharacter } from '../../workspace/core';
import { jx3AchievementSections, addJx3PlaceRowToTask, filteredJx3PlacePapers, jx3CompletedAchieves, jx3DungeonAchieveIds, jx3DungeonDoneCount, jx3PlaceQuery, jx3SnapshotCoverage } from '../../workspace/jx3/progress';
import { Check, ChevronDown, Plus, Search, X } from '@lucide/vue';
import Jx3DungeonImage from './Jx3DungeonImage.vue';
import Jx3BoxIcon from '../Jx3BoxIcon.vue';
import { jx3AchievementCategoryImage, jx3AchievementIconUrl } from '../../services/jx3Artwork';
const view = ref<'places' | 'sections'>('places');
const state = ref<'all' | 'missing' | 'done'>('all');
const category = ref('all');
const openId = ref('');
const limit = ref(60);
const categories = computed(() => [...new Set(jx3AchievementSections.value.map(group => group.category))]);
const groups = computed(() => {
  const q = jx3PlaceQuery.value.trim().toLowerCase();
  const source = view.value === 'places'
    ? filteredJx3PlacePapers.value.map(paper => ({ id: paper.id, title: paper.title, category: '', place: true, rows: paper.rows }))
    : jx3AchievementSections.value.filter(group => category.value === 'all' || group.category === category.value).map(group => ({ id: group.id, title: group.name, category: group.category, place: false, rows: group.rows }));
  return source.map(group => {
    const titleMatches = [group.title, group.category].some(name => name.toLowerCase().includes(q));
    const rows = group.rows.filter(row => (!q || titleMatches || row.name.toLowerCase().includes(q) || row.id === q) && (state.value === 'all' || row.match === state.value));
    return { ...group, rows, done: rows.filter(row => row.match === 'done').length };
  }).filter(group => group.rows.length);
});
watch([activeCharacter, view, state, category, jx3PlaceQuery], () => { openId.value = ''; limit.value = 60; });
function rowGroups(rows: (typeof groups.value)[number]['rows']) {
  return [false, true].map(done => ({ done, rows: rows.filter(row => (row.match === 'done') === done) })).filter(group => group.rows.length);
}
</script>

<template>
  <div class="jx3-panel">
    <header class="jx3-lead">
      <div><h2>成就拾遗</h2><p>完整游戏目录，按地点或分类查看已完成与待补项目。</p></div>
      <div class="jx3-number"><strong>{{ jx3CompletedAchieves.size }}</strong><span>快照记录</span></div>
    </header>
    <div class="jx3-toolbar">
      <nav class="jx3-segments" aria-label="成就视图">
        <button type="button" :aria-pressed="view === 'places'" @click="view = 'places'">按地点</button>
        <button type="button" :aria-pressed="view === 'sections'" @click="view = 'sections'">按分类</button>
      </nav>
      <nav class="jx3-segments" aria-label="成就状态">
        <button type="button" :aria-pressed="state === 'all'" @click="state = 'all'">全部</button>
        <button type="button" :aria-pressed="state === 'missing'" @click="state = 'missing'">未完成</button>
        <button type="button" :aria-pressed="state === 'done'" @click="state = 'done'">已完成</button>
      </nav>
      <select v-if="view === 'sections'" v-model="category" aria-label="成就大类"><option value="all">全部分类</option><option v-for="name in categories" :key="name" :value="name">{{ name }}</option></select>
      <label class="jx3-search"><Search :size="14" /><input v-model="jx3PlaceQuery" aria-label="搜索成就" placeholder="地点、成就名称或 ID" /><button v-if="jx3PlaceQuery" type="button" aria-label="清空成就搜索" @click="jx3PlaceQuery = ''"><X :size="14" /></button></label>
    </div>
    <p v-if="!jx3CompletedAchieves.size" class="jx3-notice">尚未导入角色快照，目录条目显示待同步。</p>
    <p v-else class="text-muted">当前可见目录匹配 {{ jx3SnapshotCoverage.matched }} 项；另有 {{ jx3SnapshotCoverage.historical }} 项历史或隐藏记录。秘境已完成 {{ jx3DungeonDoneCount }} / {{ jx3DungeonAchieveIds.size }}。</p>
    <div v-if="groups.length" class="jx3-record-list">
      <article v-for="group in groups.slice(0, limit)" :key="group.id" class="jx3-record">
        <button type="button" class="jx3-record-head" :aria-expanded="openId === group.id" @click="openId = openId === group.id ? '' : group.id">
          <Jx3DungeonImage v-if="group.place" :name="group.title" />
          <Jx3BoxIcon v-else :src="jx3AchievementCategoryImage(group.category)" size="jx3-category-art" />
          <div class="jx3-record-copy"><small v-if="group.category">{{ group.category }}</small><h3>{{ group.title }}</h3></div>
          <div class="jx3-record-count"><strong>{{ group.done }} / {{ group.rows.length }}</strong><small>{{ jx3CompletedAchieves.size ? '已完成' : '待同步' }}</small></div><ChevronDown :size="15" />
        </button>
        <div v-if="openId === group.id" class="jx3-record-details">
          <component v-for="part in rowGroups(group.rows)" :is="part.done ? 'details' : 'div'" :key="`${activeCharacter}/${group.id}/${part.done}`" :open="part.done && state === 'done' ? true : undefined" :class="{ 'jx3-completed': part.done }">
            <summary v-if="part.done"><Check :size="14" />已完成 {{ part.rows.length }} 项<ChevronDown :size="14" /></summary>
            <div v-for="row in part.rows" :key="row.id" class="jx3-requirement" :class="{ 'is-done': part.done }">
              <Jx3BoxIcon :src="jx3AchievementIconUrl(row.id)" size="jx3-achievement-icon" /><p>{{ row.name }}<small class="ml-2 text-faint">{{ row.point || 0 }} 资历</small></p>
              <button v-if="row.match === 'missing'" type="button" class="jx3-button" @click="addJx3PlaceRowToTask(row, group.title)"><Plus :size="13" />加入待办</button><span v-else class="jx3-status">{{ part.done ? '已完成' : '待同步' }}</span>
            </div>
          </component>
        </div>
      </article>
      <button v-if="groups.length > limit" type="button" class="jx3-button" @click="limit += 60">继续浏览（还有 {{ groups.length - limit }} 组）</button>
    </div>
    <p v-else class="jx3-empty-line">没有符合条件的成就。</p>
  </div>
</template>
