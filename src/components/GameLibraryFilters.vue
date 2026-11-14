<script setup lang="ts">
import { computed, ref } from 'vue';
import { Check, ChevronDown, Minus, SlidersHorizontal, X } from '@lucide/vue';
import { gameTaxonomyFilter as filter, singleAndGalgameGuides, taxonomyFilterCount } from '../workspace/guides/library';
import { emptyTaxonomyFilter, taxonomyKey } from '../services/gameTaxonomy';
const expanded = ref(false), group = ref<'tags' | 'companies' | 'people'>('tags'), query = ref(''), excluding = ref(false);
const includes = (names: string[], name: string) => names.some(n => taxonomyKey(n) === taxonomyKey(name));
const roles = computed(() => [...new Set(singleAndGalgameGuides.value.flatMap(g => g.taxonomy?.staff.map(s => s.role) || []))].sort());
const options = computed(() => {
  const counts = new Map<string, { name: string; count: number }>();
  for (const game of singleAndGalgameGuides.value) {
    const t = game.taxonomy!;
    const names = group.value === 'tags' ? t.tags : group.value === 'companies'
      ? filter.value.company_role === 'developer' ? t.developers : filter.value.company_role === 'publisher' ? t.publishers : [...t.developers, ...t.publishers]
      : t.staff.filter(s => !filter.value.staff_role || s.role === filter.value.staff_role).map(s => s.name);
    for (const key of new Set(names.map(taxonomyKey))) {
      const item = counts.get(key);
      if (item) item.count++;
      else counts.set(key, { name: names.find(n => taxonomyKey(n) === key)!, count: 1 });
    }
  }
  return [...counts.values()].filter(o => taxonomyKey(o.name).includes(taxonomyKey(query.value)))
    .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name, 'zh-Hans-CN'));
});
const selections = computed(() => [
  ...filter.value.tags.map(name => ({ key: 'tags' as const, name, label: '标签：' + name })),
  ...filter.value.excluded.map(name => ({ key: 'excluded' as const, name, label: '排除：' + name })),
  ...filter.value.companies.map(name => ({ key: 'companies' as const, name, label: (filter.value.company_role === 'developer' ? '开发：' : filter.value.company_role === 'publisher' ? '发行：' : '公司：') + name })),
  ...filter.value.people.map(name => ({ key: 'people' as const, name, label: '人员：' + name })),
]);
function remove(key: 'tags' | 'excluded' | 'companies' | 'people', name: string) {
  filter.value = { ...filter.value, [key]: filter.value[key].filter(n => taxonomyKey(n) !== taxonomyKey(name)) };
}
function selected(name: string) {
  return group.value === 'tags' ? includes(filter.value.tags, name) || includes(filter.value.excluded, name) : includes(filter.value[group.value], name);
}
function toggle(name: string) {
  const key = group.value === 'tags' ? excluding.value ? 'excluded' : 'tags' : group.value;
  if (includes(filter.value[key], name)) remove(key, name);
  else {
    const opposite = key === 'tags' ? 'excluded' : key === 'excluded' ? 'tags' : undefined;
    filter.value = { ...filter.value, [key]: [...filter.value[key], name],
      ...(opposite ? { [opposite]: filter.value[opposite].filter(n => taxonomyKey(n) !== taxonomyKey(name)) } : {}) };
  }
}
function selectGroup(value: typeof group.value) { group.value = value; query.value = ''; }
</script>

<template>
  <div class="taxonomy-filter">
    <div class="filter-summary"><button class="filter-toggle" type="button" :aria-expanded="expanded" aria-controls="taxonomy-filter-panel" @click="expanded = !expanded"><SlidersHorizontal />标签与制作筛选<span v-if="taxonomyFilterCount">{{ taxonomyFilterCount }}</span><ChevronDown :class="{ rotated: expanded }" /></button>
      <div class="selected-filters" aria-label="已选标签与制作筛选"><button v-for="s in selections" :key="s.key + s.name" type="button" :aria-label="'取消筛选：' + s.label" @click="remove(s.key, s.name)">{{ s.label }}<X /></button><button v-if="filter.staff_role" type="button" @click="filter = { ...filter, staff_role: '' }">职位：{{ filter.staff_role }}<X /></button><button v-if="taxonomyFilterCount" type="button" class="clear" @click="filter = emptyTaxonomyFilter()">清除</button></div>
    </div>
    <section v-if="expanded" id="taxonomy-filter-panel" class="filter-panel" aria-label="标签与制作筛选">
      <header><div role="group" aria-label="筛选维度"><button v-for="item in [{id:'tags',name:'标签'},{id:'companies',name:'公司'},{id:'people',name:'制作人员'}] as const" :key="item.id" type="button" :aria-pressed="group === item.id" @click="selectGroup(item.id)">{{ item.name }}</button></div><button type="button" aria-label="收起标签筛选" @click="expanded = false"><X /></button></header>
      <div class="filter-controls"><input v-model="query" type="search" :aria-label="group === 'tags' ? '搜索标签' : group === 'companies' ? '搜索公司' : '搜索制作人员'" placeholder="输入名称查找…" />
        <template v-if="group === 'tags'"><select v-model="excluding" aria-label="标签筛选方式"><option :value="false">包含标签</option><option :value="true">排除标签</option></select><select v-model="filter.tag_mode" aria-label="标签组合规则"><option value="all">包含全部所选</option><option value="any">包含任一所选</option></select></template>
        <select v-else-if="group === 'companies'" v-model="filter.company_role" aria-label="公司类型"><option value="all">开发与发行</option><option value="developer">开发 / 制作公司</option><option value="publisher">发行商</option></select>
        <select v-else v-model="filter.staff_role" aria-label="制作人员职位筛选"><option value="">所有职位</option><option v-for="r in roles" :key="r">{{ r }}</option><option v-if="filter.staff_role && !roles.includes(filter.staff_role)">{{ filter.staff_role }}</option></select>
      </div>
      <div class="filter-options"><button v-for="item in options.slice(0, 60)" :key="item.name" type="button" :aria-pressed="selected(item.name)" :class="{ excluded: group === 'tags' && includes(filter.excluded, item.name) }" @click="toggle(item.name)"><Minus v-if="group === 'tags' && includes(filter.excluded, item.name)" /><Check v-else-if="selected(item.name)" />{{ item.name }}<small>{{ item.count }}</small></button><p v-if="!options.length">没有匹配的名称，可在“编辑作品 → 标签与制作”中补充。</p></div>
      <p class="filter-help">{{ options.length > 60 ? '显示前 60 项，共 ' + options.length + ' 项，输入名称缩小范围。' : '共 ' + options.length + ' 项。' }} 公司、人员各自匹配任一所选；不同维度同时满足。</p>
    </section>
  </div>
</template>

<style scoped>
.taxonomy-filter{flex:none;min-width:0;margin:0 0 12px}.filter-summary{display:flex;align-items:center;gap:10px;flex-wrap:wrap}.filter-toggle{flex:none}.filter-toggle>span{font:11px var(--font-mono);color:var(--color-beacon)}.selected-filters{display:flex;align-items:center;flex-wrap:wrap;gap:6px;min-width:0}.selected-filters button{font-size:11px;border:1px solid var(--color-hairline);background:var(--color-paper);padding:3px 8px;min-height:28px}.selected-filters .clear{border:0;background:transparent;color:var(--color-beacon)}button{display:inline-flex;align-items:center;justify-content:center;gap:6px;min-height:32px;padding:6px 9px;border-radius:4px;font-size:12px;color:var(--color-muted);cursor:pointer;overflow-wrap:anywhere;max-width:100%;text-align:left}button:hover{background:var(--color-paper);color:var(--color-ink)}svg{width:13px;height:13px;flex:none}.rotated{transform:rotate(180deg)}.filter-panel{border:1px solid var(--color-hairline);border-radius:5px;background:var(--color-chrome);padding:12px 14px;margin-top:9px}.filter-panel header{display:flex;justify-content:space-between;align-items:center;margin-bottom:10px}.filter-panel header>div{display:flex;gap:6px}.filter-panel [aria-pressed=true]{background:var(--color-paper);color:var(--color-beacon)}.filter-controls{display:flex;gap:10px;flex-wrap:wrap}input,select{min-width:0;border:1px solid var(--color-hairline);border-radius:4px;background:var(--color-canvas);padding:7px 9px;color:var(--color-ink);font-size:12px;min-height:34px}input{flex:1;min-width:130px}input::placeholder{color:var(--color-faint)}.filter-options{display:flex;align-content:start;flex-wrap:wrap;gap:6px;max-height:160px;overflow:auto;margin-top:12px;padding:2px}.filter-options button{border:1px solid var(--color-hairline);padding:4px 9px;font-size:12px}.filter-options [aria-pressed=true]{border-color:var(--color-beacon)}.filter-options .excluded{border-style:dashed;color:var(--color-alert)}small{font:10px var(--font-mono);color:var(--color-faint);margin-left:4px}.filter-options p,.filter-help{font-size:11px;line-height:1.8;color:var(--color-faint)}.filter-help{margin-top:10px}:where(button,input,select):focus-visible{outline:2px solid var(--color-beacon);outline-offset:2px}
</style>
