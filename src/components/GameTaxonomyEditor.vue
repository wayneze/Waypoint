<script setup lang="ts">
import { computed, ref } from 'vue';
import { Plus, RotateCcw, X } from '@lucide/vue';
import type { GameCuration, GameStaff, GameTaxonomy } from '../types';
import { curatedGameTaxonomy, staffRoles, taxonomyKey, uniqueNames } from '../services/gameTaxonomy';
const props = defineProps<{ source: GameTaxonomy }>();
const model = defineModel<GameCuration>({ required: true });
const effective = computed(() => curatedGameTaxonomy(props.source, model.value));
const tagQuery = ref(''), newTags = ref(''), person = ref(''), role = ref('剧本');
const visibleSourceTags = computed(() => props.source.tags.filter(t => taxonomyKey(t).includes(taxonomyKey(tagQuery.value))));
const hidden = (tag: string) => (model.value.hidden_tags || []).some(t => taxonomyKey(t) === taxonomyKey(tag));
function hideTag(tag: string) {
  const isSource = props.source.tags.some(t => taxonomyKey(t) === taxonomyKey(tag));
  model.value = { ...model.value,
    added_tags: (model.value.added_tags || []).filter(t => taxonomyKey(t) !== taxonomyKey(tag)),
    hidden_tags: isSource ? uniqueNames([...(model.value.hidden_tags || []), tag]) : model.value.hidden_tags,
  };
}
function restoreTag(tag: string) {
  model.value = { ...model.value, hidden_tags: (model.value.hidden_tags || []).filter(t => taxonomyKey(t) !== taxonomyKey(tag)) };
}
function addTags() {
  const tags = uniqueNames(newTags.value.split(/[,，;；\n]+/));
  if (!tags.length) return;
  model.value = { ...model.value, added_tags: uniqueNames([...(model.value.added_tags || []), ...tags]),
    hidden_tags: (model.value.hidden_tags || []).filter(t => !tags.some(n => taxonomyKey(t) === taxonomyKey(n))) };
  newTags.value = '';
}
function companyText(key: 'developers' | 'publishers', event: Event) {
  model.value = { ...model.value, [key]: uniqueNames((event.target as HTMLTextAreaElement).value.split('\n')) };
}
function editStaff(index: number, field: 'name' | 'role', event: Event) {
  model.value = { ...model.value, staff: effective.value.staff.map((s, i) => i === index ? { ...s, [field]: (event.target as HTMLInputElement).value, source_id: undefined } : s) };
}
function removeStaff(index: number) {
  model.value = { ...model.value, staff: effective.value.staff.filter((_, i) => i !== index) };
}
function addStaff() {
  if (!person.value.trim()) return;
  model.value = { ...model.value, staff: [...effective.value.staff, { name: person.value.trim(), role: role.value }] };
  person.value = '';
}
function rolesFor(member: GameStaff) { return uniqueNames([...staffRoles, member.role]); }
</script>

<template>
  <div class="taxonomy-editor">
    <p class="help">隐藏来源标签、补充个人标签，或修正制作信息。个人修改会保留，重新匹配资料也不会覆盖。</p>
    <section>
      <header><h4>作品标签 <small>{{ effective.tags.length }}</small></h4><button v-if="model.added_tags?.length || model.hidden_tags?.length" type="button" @click="model = { ...model, added_tags: [], hidden_tags: [] }"><RotateCcw />恢复来源标签</button></header>
      <div class="tag-chips" aria-label="当前作品标签"><button v-for="tag in effective.tags" :key="tag" type="button" :aria-label="'移除标签：' + tag" @click="hideTag(tag)">{{ tag }}<X /></button><p v-if="!effective.tags.length" class="help">还没有标签，输入下方名称即可添加。</p></div>
      <div class="add-row"><input v-model="newTags" aria-label="新增作品标签" placeholder="添加标签，多个用逗号分隔" @keydown.enter.prevent="addTags" /><button type="button" :disabled="!newTags.trim()" @click="addTags"><Plus />添加</button></div>
      <details v-if="source.tags.length"><summary>来源标签 {{ source.tags.length }} · 已隐藏 {{ model.hidden_tags?.length || 0 }}</summary><input v-model="tagQuery" type="search" aria-label="搜索来源标签" placeholder="搜索来源标签" /><div class="source-tags"><button v-for="tag in visibleSourceTags" :key="tag" type="button" :class="{ hidden: hidden(tag) }" :aria-pressed="!hidden(tag)" @click="hidden(tag) ? restoreTag(tag) : hideTag(tag)">{{ tag }}<span>{{ hidden(tag) ? '恢复' : '隐藏' }}</span></button></div></details>
    </section>
    <section>
      <header><h4>公司</h4><span class="help">每行一家公司</span></header>
      <div class="company-fields"><label>开发 / 制作公司 <span>{{ model.developers == null ? '跟随资料' : '个人修订' }}<button v-if="model.developers != null" type="button" @click="model = { ...model, developers: null }">恢复自动</button></span><textarea aria-label="开发公司" :value="effective.developers.join('\n')" rows="3" placeholder="填写开发公司" @change="companyText('developers', $event)" /></label><label>发行商 <span>{{ model.publishers == null ? '跟随资料' : '个人修订' }}<button v-if="model.publishers != null" type="button" @click="model = { ...model, publishers: null }">恢复自动</button></span><textarea aria-label="发行商" :value="effective.publishers.join('\n')" rows="3" placeholder="填写发行商" @change="companyText('publishers', $event)" /></label></div>
    </section>
    <section>
      <header><h4>制作人员 <small>{{ effective.staff.length }}</small></h4><button v-if="model.staff != null" type="button" @click="model = { ...model, staff: null }"><RotateCcw />恢复来源人员</button></header>
      <p class="help">{{ model.staff == null ? '当前跟随资料，修改后将保留你的人员名单。' : '当前使用个人修订名单。' }}</p>
      <div class="staff-list"><div v-for="(member, index) in effective.staff" :key="index" class="staff-row"><input :aria-label="'制作人员姓名 ' + (index + 1)" :value="member.name" @change="editStaff(index, 'name', $event)" /><select :aria-label="'制作人员职位 ' + (index + 1)" :value="member.role" @change="editStaff(index, 'role', $event)"><option v-for="r in rolesFor(member)" :key="r">{{ r }}</option></select><button type="button" :aria-label="'移除制作人员：' + member.name" @click="removeStaff(index)"><X /></button></div></div>
      <div class="staff-row add-row"><input v-model="person" aria-label="新增制作人员姓名" placeholder="姓名" @keydown.enter.prevent="addStaff" /><select v-model="role" aria-label="新增制作人员职位"><option v-for="r in staffRoles" :key="r">{{ r }}</option></select><button type="button" aria-label="添加制作人员" :disabled="!person.trim()" @click="addStaff"><Plus /></button></div>
    </section>
  </div>
</template>

<style scoped>
.taxonomy-editor{display:grid;gap:26px}.taxonomy-editor section{display:grid;gap:12px;min-width:0}.taxonomy-editor header{display:flex;align-items:center;justify-content:space-between;gap:12px}h4{font-size:14px;font-weight:500}small{font:11px var(--font-mono);color:var(--color-faint);margin-left:6px}.help{font-size:12px;line-height:1.8;color:var(--color-muted)}button{display:inline-flex;align-items:center;justify-content:center;gap:6px;min-height:32px;font-size:12px;color:var(--color-beacon);cursor:pointer}button:disabled{opacity:.45;cursor:default}svg{width:13px;height:13px;flex:none}.tag-chips,.source-tags{display:flex;flex-wrap:wrap;gap:7px}.tag-chips button,.source-tags button{border:1px solid var(--color-hairline);background:var(--color-paper);border-radius:4px;padding:4px 9px;max-width:100%;overflow-wrap:anywhere;text-align:left}.tag-chips button{color:var(--color-ink)}input,select,textarea{min-width:0;width:100%;padding:8px 10px;min-height:36px;border:1px solid var(--color-hairline);border-radius:4px;background:var(--color-chrome);color:var(--color-ink);font-size:12px}input::placeholder,textarea::placeholder{color:var(--color-faint)}textarea{resize:vertical}.add-row{display:flex;gap:8px}.add-row>input{flex:1}.add-row>button{border:1px solid var(--color-hairline);border-radius:4px;padding:6px 10px;flex:none}details{border-top:1px solid var(--color-hairline);padding-top:12px}summary{font-size:12px;color:var(--color-muted);cursor:pointer;padding:4px 0}details>input{margin-top:10px}.source-tags{max-height:150px;overflow:auto;margin-top:10px}.source-tags button{color:var(--color-muted)}.source-tags span{font-size:10px;color:var(--color-faint)}.source-tags .hidden{border-style:dashed}.source-tags .hidden>span{color:var(--color-beacon)}.company-fields{display:grid;grid-template-columns:1fr 1fr;gap:16px}label{display:grid;gap:7px;font-size:12px;color:var(--color-ink)}label>span{display:flex;align-items:center;justify-content:space-between;gap:8px;font-size:11px;color:var(--color-faint);min-height:24px}label>span button{font-size:11px;min-height:24px}.staff-list{display:grid;gap:8px;max-height:250px;overflow:auto}.staff-row{display:grid;grid-template-columns:minmax(0,1fr) 105px 36px;gap:8px}.staff-row>button{border:1px solid var(--color-hairline);border-radius:4px}.staff-row.add-row{display:grid}:where(button,input,select,textarea,summary):focus-visible{outline:2px solid var(--color-beacon);outline-offset:2px}@media(max-width:650px){.company-fields{grid-template-columns:1fr}.taxonomy-editor header{flex-wrap:wrap}.staff-row{grid-template-columns:minmax(0,1fr) 85px 32px}}
</style>
