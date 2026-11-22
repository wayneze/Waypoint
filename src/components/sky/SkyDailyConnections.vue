<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { ArrowUpRight, Gift, Link2, RefreshCw, ChevronDown, X } from '@lucide/vue';
import { activeSkyCharacter, skyCharacters } from '../../workspace/vault';
import { claimDailyLegacy, dailyCanEdit, dailyCharacterId, dailyLegacyCount, isSavingDaily, skyShard } from '../../workspace/sky/daily';
import { boostResult, boostSyncing, syncSkyBoost } from '../../workspace/sky/boost';
import {
  loadSkyBoostBinding, loadSkyBoostCompletion, loadSkyBoostLatest,
  saveSkyBoostBinding, SKY_BOOST_OPTIONS, skyBoostExpired, supportsSkyBoostLink,
} from '../../services/skyBoost';
import { openExternalUrl } from '../../services/systemApi';
import { showSkySettings, skyPlanDay } from '../../workspace/sky/plan';
import { showToast } from '../../workspace/core';
import { loadSkyInventory, SKY_CURRENCIES } from '../../services/skyInventory';
import { setSkySubTab } from '../../workspace/sky/tabs';
import SkyCurrencyIcon from './SkyCurrencyIcon.vue';

const saved = ref(loadSkyBoostBinding(dailyCharacterId.value));
const draft = ref(JSON.parse(JSON.stringify(saved.value)) as typeof saved.value);
const expanded = showSkySettings;
const saving = ref(false);
const error = ref('');
const completedSync = computed(() => loadSkyBoostCompletion(dailyCharacterId.value, saved.value, skyPlanDay.value));
const latestSync = computed(() => loadSkyBoostLatest(dailyCharacterId.value, saved.value));
const result = computed(() => boostResult.value?.characterId === dailyCharacterId.value ? boostResult.value : latestSync.value || completedSync.value);
const latestFeedback = computed(() => result.value?.feedback || latestSync.value?.feedback);
const orderExpired = computed(() => {
  void skyShard.value;
  return !!latestFeedback.value && skyBoostExpired(latestFeedback.value);
});
const expiryLabel = computed(() => {
  const feedback = latestFeedback.value;
  if (!feedback?.expiration_date) return '';
  return `${orderExpired.value ? '已到期' : '到期'} · ${feedback.expiration_date}`;
});
const includedLabel = computed(() => SKY_BOOST_OPTIONS.filter(o => saved.value.included[o.key]).map(o => o.label).join(' + '));
const dirty = computed(() => JSON.stringify(draft.value) !== JSON.stringify(saved.value));
const inventory = computed(() => loadSkyInventory(dailyCharacterId.value));
const syncLabel = computed(() => {
  if (boostSyncing.value) return '同步中';
  if (!saved.value.link) return '连接代练';
  if (orderExpired.value) return '订单已到期';
  if (result.value && !result.value.feedback) return '同步失败';
  if (!supportsSkyBoostLink(saved.value.link)) return '查看反馈';
  return completedSync.value ? '今日已同步' : saved.value.autoSync ? '自动同步' : '手动同步';
});
const syncTime = computed(() => {
  const time = latestFeedback.value?.fetched_at || inventory.value?.outfitsAt || inventory.value?.balancesAt;
  return time ? new Date(time).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }) : '';
});
watch(dailyCharacterId, () => {
  saved.value = loadSkyBoostBinding(dailyCharacterId.value);
  draft.value = JSON.parse(JSON.stringify(saved.value));
  error.value = '';
  expanded.value = false;
});

async function save() {
  const id = dailyCharacterId.value;
  if (!id || saving.value || boostSyncing.value) return;
  saving.value = true;
  error.value = '';
  try {
    await saveSkyBoostBinding(id, draft.value);
    if (dailyCharacterId.value === id) {
      saved.value = loadSkyBoostBinding(id);
      draft.value = JSON.parse(JSON.stringify(saved.value));
      boostResult.value = null;
      expanded.value = false;
      void syncSkyBoost();
    }
    showToast('代练设置已保存', '完成反馈只勾选订单包含的日常');
  } catch (e) { error.value = String(e); }
  finally { saving.value = false; }
}

</script>

<template>
  <section class="sky-connections" aria-label="角色与日常同步" @keydown.esc="expanded = false">
    <div class="sky-connection-bar">
      <select id="sky-daily-character" v-model="dailyCharacterId" :disabled="saving"
              aria-label="记录角色" class="sky-connection-character">
        <option disabled value="">选择光遇角色</option>
        <option v-for="ch in skyCharacters" :key="ch.id" :value="ch.id">{{ ch.name }}{{ ch.server ? ` · ${ch.server}` : '' }}</option>
      </select>
      <div v-if="inventory" class="sky-connection-balances" aria-label="同步资产">
        <span v-for="(label, key) in SKY_CURRENCIES" :key="key" :title="label"
              role="img" :aria-label="`${label} ${inventory.balances[key] ?? '未同步'}`">
          <SkyCurrencyIcon :currency="key" /><strong aria-hidden="true">{{ inventory.balances[key] ?? '—' }}</strong>
        </span>
      </div>
      <div class="sky-connection-actions">
      <button type="button" class="wp-text-action" title="前往光遇福利页" @click="setSkySubTab('benefits')">
        <Gift class="w-3 h-3" />福利 · 签到
      </button>
      <button type="button" class="sky-sync-trigger" :class="{ 'has-error': result && !result.feedback }"
              :disabled="!dailyCharacterId || saving" :aria-expanded="expanded"
              aria-controls="sky-sync-settings" :title="result?.message || '查看同步记录与订单设置'" @click="expanded = !expanded">
        <RefreshCw v-if="boostSyncing" class="w-3 h-3 animate-spin" /><Link2 v-else class="w-3 h-3" />
        {{ syncLabel }}<time v-if="syncTime && saved.link">{{ syncTime }}</time>
        <ChevronDown class="w-3 h-3 transition-transform" :class="{ 'rotate-180': expanded }" />
      </button>
      <span v-if="expiryLabel && saved.link" class="sky-order-expiry" :class="{ 'is-expired': orderExpired }">
        {{ expiryLabel }}
      </span>
      </div>
    </div>
    <p v-if="!skyCharacters.length" class="py-2 text-sm text-muted">先在账号金库添加光遇角色，再记录日常或绑定代练。</p>
    <div v-if="expanded" id="sky-sync-settings" class="sky-connection-panel">
      <div class="flex items-center justify-between gap-3 mb-3">
        <h3 class="font-semibold text-sm">代练同步</h3>
        <button type="button" class="wp-text-action" aria-label="关闭同步设置" @click="expanded = false"><X class="w-4 h-4" /></button>
      </div>
      <div class="sky-sync-layout">
      <div class="sky-sync-summary">
      <p class="sky-sync-state" :class="{ 'is-complete': completedSync }">{{ completedSync ? '今日同步已完成' : '同步记录' }}</p>
      <p v-if="orderExpired" class="text-xs text-muted mt-1.5">自动查询已停止。续期后可手动刷新。</p>
      <p v-else-if="completedSync && saved.autoSync" class="text-xs text-muted mt-1.5">今日不再自动请求，明日恢复检查。</p>
      <dl v-if="latestFeedback" class="mt-2 space-y-1 text-xs text-muted">
        <div v-if="latestFeedback.days != null && latestFeedback.day === skyPlanDay && !orderExpired" class="flex gap-2">
          <dt>剩余</dt><dd>{{ latestFeedback.days }} 天</dd>
        </div>
        <div v-if="latestFeedback.start_time" class="flex gap-2">
          <dt>开始时间</dt><dd>{{ latestFeedback.start_time }}</dd>
        </div>
        <div v-if="latestFeedback.done_time && latestFeedback.day === skyPlanDay && !orderExpired" class="flex gap-2">
          <dt>预计查收</dt><dd>{{ latestFeedback.done_time }}</dd>
        </div>
      </dl>
      <div v-if="dailyLegacyCount" class="py-3 text-sm flex flex-wrap items-center gap-2">
      <span class="text-muted">有旧的共享日常记录，选择所属角色后可合并。</span>
      <button type="button" class="wp-text-action" :disabled="!dailyCanEdit" @click="claimDailyLegacy">
        归入{{ activeSkyCharacter ? `「${activeSkyCharacter.name}」` : '所选角色' }}
      </button>
      </div>
      <div v-if="saved.link" class="flex items-center flex-wrap gap-x-3 gap-y-2 mt-3">
        <span class="text-sm text-muted flex-1">{{ includedLabel }}</span>
        <button v-if="saved.link" type="button" class="wp-text-action text-sm" @click="openExternalUrl(saved.link)">打开原页</button>
        <button v-if="saved.link" type="button" class="wp-text-action text-sm" :disabled="boostSyncing || isSavingDaily || saving" @click="syncSkyBoost(true)">
          <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': boostSyncing }" />手动刷新
        </button>
      </div>
      <div v-if="result" class="mt-2 text-sm text-muted" role="status">
        <span v-if="result.feedback" class="text-ink">{{ result.feedback.status }} · </span>{{ result.message }}
        <span v-if="result.feedback" class="ml-2 text-xs text-faint">{{ new Date(result.feedback.fetched_at).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }) }} 更新</span>
      </div>
      <p v-if="saved.link && !supportsSkyBoostLink(saved.link)" class="mt-2 text-sm text-muted">已保存链接，此平台尚未适配自动读取，可打开原页查看。</p>
        <button v-if="inventory" type="button" class="wp-text-action text-xs mt-3" @click="setSkySubTab('outfits'); expanded = false">
          已拥有外观 {{ inventory.outfits.length }} 件 <ArrowUpRight class="w-3.5 h-3.5" />
        </button>
      </div>
      <form class="sky-sync-form space-y-4" @submit.prevent="save">
        <div>
          <label for="sky-feedback-link" class="text-sm text-muted block mb-1.5">订单反馈链接</label>
          <input id="sky-feedback-link" v-model="draft.link" type="url" autocomplete="off" placeholder="粘贴代练商家提供的查单链接"
                 :disabled="saving || boostSyncing" class="w-full rounded-lg border border-hairline bg-canvas px-3 py-2 text-sm" />
          <p class="mt-1.5 text-xs text-faint">换商家时替换链接；清空并保存可解除绑定，已完成记录会保留。</p>
        </div>
        <fieldset :disabled="saving || boostSyncing" class="space-y-2">
          <legend class="text-sm font-medium mb-2">订单包含哪些日常</legend>
          <div class="flex flex-wrap gap-2">
            <label v-for="option in SKY_BOOST_OPTIONS" :key="option.key"
                   class="flex items-center gap-2 rounded-lg border px-3 py-2 text-sm cursor-pointer"
                   :class="draft.included[option.key] ? 'border-beacon/40 bg-beacon/10 text-ink' : 'border-hairline text-muted'">
              <input v-model="draft.included[option.key]" type="checkbox" class="accent-beacon" />{{ option.label }}
            </label>
          </div>
          <p class="text-xs text-muted">只自动勾选订单包含的日常；活动服务不包含一次性奖励和兑换。</p>
          <label class="flex items-center gap-2 text-sm text-muted pt-2">
            <input v-model="draft.autoSync" type="checkbox" class="accent-beacon" />自动同步完成反馈
          </label>
          <p class="text-xs text-faint">未完成时每 5 分钟检查；今日完成或订单到期后停止自动查询。</p>
        </fieldset>
        <p v-if="error" class="text-sm text-alert" role="alert">{{ error }}</p>
        <button type="submit" class="wp-game-action is-primary" :disabled="saving || boostSyncing || !dirty">{{ saving ? '保存中…' : '保存设置' }}</button>
      </form>
      </div>
    </div>
  </section>
</template>

<style scoped>
.sky-connections { border-bottom: 1px solid var(--color-hairline); min-width: 0; container-type: inline-size; }
.sky-connection-bar { display: flex; align-items: center; gap: 22px; min-height: 42px; padding: 5px 0; flex-wrap: wrap; }
.sky-connection-character { background: transparent; color: var(--color-ink); font-size: 12px; max-width: 170px; padding: 5px 18px 5px 0; }
.sky-connection-balances { display: flex; align-items: center; flex-wrap: wrap; gap: 8px 17px; color: var(--color-muted); font-size: 11px; }
.sky-connection-balances > span { display: inline-flex; align-items: center; gap: 6px; white-space: nowrap; }
.sky-connection-balances strong { color: var(--color-ink); font-family: var(--font-mono); font-size: 13px; font-weight: 500; font-variant-numeric: tabular-nums; }
.sky-connection-actions { display: flex; align-items: center; gap: 20px; margin-left: auto; font-size: 12px; }
.sky-sync-trigger { display: flex; align-items: center; gap: 7px; color: var(--color-muted); cursor: pointer; padding: 6px 0; }
.sky-sync-trigger:hover, .sky-sync-trigger[aria-expanded="true"] { color: var(--color-ink); }
.sky-sync-trigger.has-error { color: var(--color-alert); }
.sky-sync-trigger time { color: var(--color-faint); font-size: 10px; font-family: var(--font-mono); }
.sky-connection-panel { width: 100%; min-width: 0; margin: 6px 0 14px; padding: 18px 20px; background: var(--color-surface); border: 1px solid var(--color-hairline); border-radius: 9px; }
.sky-sync-layout { display: grid; grid-template-columns: minmax(0, .85fr) minmax(0, 1.15fr); gap: 28px; }
.sky-sync-summary, .sky-sync-form { min-width: 0; overflow-wrap: anywhere; }
.sky-sync-summary { padding-right: 28px; border-right: 1px solid var(--color-hairline); }
.sky-sync-state { font-size: 17px; font-weight: 600; color: var(--color-ink); }
.sky-sync-state.is-complete { color: var(--color-done); }
.sky-order-expiry { color: var(--color-muted); font-size: 11px; overflow-wrap: anywhere; }
.sky-connection-actions { flex-wrap: wrap; }
.sky-order-expiry.is-expired { color: var(--color-alert); }
.sky-sync-form fieldset { min-width: 0; }
@container (max-width: 720px) {
  .sky-sync-layout { grid-template-columns: minmax(0, 1fr); gap: 18px; }
  .sky-sync-summary { padding: 0 0 18px; border-right: 0; border-bottom: 1px solid var(--color-hairline); }
  .sky-connection-panel { padding: 16px; }
}
@media (max-width: 1100px) {
  .sky-connection-bar { gap: 8px 16px; }
  .sky-connection-balances { gap: 12px; }
  .sky-connection-actions { gap: 14px; }
}
</style>
