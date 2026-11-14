<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { Check, Compass } from '@lucide/vue';
import { discoveryInterests, marketRegions, type DiscoveryPreferences } from '../services/discovery';
import { discoveryPreferences, hydrateDiscovery, saveDiscoveryPreferences } from '../workspace/discovery';
const props=defineProps<{initial?:boolean;cancellable?:boolean}>();
const emit=defineEmits<{saved:[];cancel:[]}>();
const draft=ref<DiscoveryPreferences>({interests:[],all:false,country:'JP'});
const ready=ref(false),busy=ref(false),error=ref(''),notice=ref('');
watch(discoveryPreferences,value=>{
  if(value)draft.value={...value,interests:[...value.interests]};
  notice.value='';
});
onMounted(async()=>{
  try {
    await hydrateDiscovery();
    if(discoveryPreferences.value)draft.value={...discoveryPreferences.value,interests:[...discoveryPreferences.value.interests]};
    ready.value=true;
  }catch(e){error.value=String(e);}
});
const canSave=computed(()=>ready.value && (draft.value.all || draft.value.interests.length>0));
function toggle(id:string) {
  draft.value.all=false;
  draft.value.interests=draft.value.interests.includes(id)?draft.value.interests.filter(i=>i!==id):[...draft.value.interests,id];
}
async function save() {
  if(busy.value || !canSave.value)return;
  busy.value=true;error.value='';
  try {await saveDiscoveryPreferences(draft.value);notice.value='偏好已保存';emit('saved');}
  catch(e){error.value=String(e);}
  finally {busy.value=false;}
}
</script>
<template>
  <form class="discovery-preferences" aria-label="发现偏好设置" @submit.prevent="save">
    <header><Compass /><div><h3>{{ props.initial?'先选你想遇见的作品':'发现偏好' }}</h3><p>可多选，匹配任一偏好即可出现。发现与优惠共用，之后可在这里或设置中调整。</p></div></header>
    <div class="interest-options" role="group" aria-label="感兴趣的游戏类型">
      <button v-for="interest in discoveryInterests" :key="interest.id" type="button" :disabled="!ready || busy" :aria-pressed="!draft.all && draft.interests.includes(interest.id)" @click="toggle(interest.id)">
        <span><strong>{{ interest.label }}</strong><Check v-if="!draft.all && draft.interests.includes(interest.id)" /></span><small>{{ interest.description }}</small>
      </button>
    </div>
    <label class="preference-all"><input v-model="draft.all" type="checkbox" :disabled="!ready || busy" />浏览全部类型，不按类型筛选</label>
    <div class="preference-region"><label>商店地区<select v-model="draft.country" :disabled="!ready || busy"><option v-for="region in marketRegions" :key="region.id" :value="region.id">{{ region.label }} · {{ region.currency }}</option></select></label><p>决定商店商品与报价。中文资料照常显示；已有提醒保留原地区和币种，可逐项调整。</p></div>
    <p v-if="error" class="preference-error" role="alert">{{ error }}</p><p v-if="notice" role="status">{{ notice }}</p>
    <footer><span v-if="!draft.all">{{ draft.interests.length?`已选 ${draft.interests.length} 类`:'尚未选择类型' }}</span><button v-if="!initial && cancellable" type="button" :disabled="busy" @click="emit('cancel')">取消</button><button type="submit" :disabled="busy || !canSave">{{ busy?'保存中…':initial?'开始发现':'保存偏好' }}</button></footer>
  </form>
</template>
<style scoped>
.discovery-preferences{padding:24px;border:1px solid var(--color-hairline);border-radius:6px;background:var(--color-paper);margin:20px 0}.discovery-preferences header{display:flex;gap:12px;align-items:start;margin-bottom:24px}.discovery-preferences svg{width:18px;height:18px;color:var(--color-beacon);flex:none}.discovery-preferences header>svg{margin-top:5px}.discovery-preferences h3{font:24px var(--font-display);margin:0 0 8px}.discovery-preferences p{font-size:12px;color:var(--color-muted);line-height:1.8}.interest-options{display:grid;grid-template-columns:repeat(auto-fit,minmax(205px,1fr));gap:12px}.interest-options button{padding:16px;text-align:left;background:var(--color-canvas);border:1px solid var(--color-hairline);border-radius:4px;cursor:pointer}.interest-options button[aria-pressed=true]{border-color:var(--color-beacon);background:color-mix(in srgb,var(--color-beacon) 8%,var(--color-canvas))}.interest-options span{display:flex;justify-content:space-between;gap:10px}.interest-options strong{font-size:14px;font-weight:500}.interest-options small{display:block;font-size:12px;color:var(--color-muted);margin-top:8px;line-height:1.6}.preference-all{display:flex;gap:8px;font-size:12px;margin:20px 0}.preference-region{display:flex;gap:16px;align-items:center;flex-wrap:wrap;padding-top:16px;border-top:1px solid var(--color-hairline)}.preference-region label{display:flex;gap:10px;align-items:center;font-size:12px;flex:none}.preference-region p{flex:1;min-width:200px}.preference-region select{padding:8px;background:var(--color-canvas);border:1px solid var(--color-hairline);color:var(--color-ink);border-radius:4px}.discovery-preferences footer{display:flex;justify-content:end;align-items:center;gap:16px;margin-top:22px;font-size:12px}.discovery-preferences footer>span{margin-right:auto;color:var(--color-muted)}.discovery-preferences footer button{cursor:pointer;padding:9px 16px;border:1px solid var(--color-hairline);border-radius:4px}.discovery-preferences footer button[type=submit]{background:var(--color-beacon);color:var(--color-canvas);border-color:var(--color-beacon)}button:disabled{opacity:.45;cursor:default}.discovery-preferences .preference-error{color:var(--color-alert)}:where(button,input,select):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}
</style>
