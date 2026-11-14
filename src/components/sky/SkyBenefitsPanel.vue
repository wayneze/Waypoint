<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { ArrowLeft, ArrowRight, Gift, ShoppingBag, RotateCw, X } from '@lucide/vue';
import { navigateSkyBenefits, setSkyBenefitsView, type BenefitsAction, type BenefitsPage } from '../../services/skyBenefits';
import { showToast, uiZoomLevel } from '../../workspace/core';

const props = defineProps<{ active: boolean }>();
const page = ref<BenefitsPage | null>(null);
const host = ref<HTMLElement>();
const error = ref('');
const ready = ref(false);
const navigating = ref(false);
let observer: ResizeObserver | undefined;
let frame = 0;
let disposed = false;

async function syncView() {
  if (disposed) return;
  const currentPage = page.value ?? 'checkin';
  const rect = host.value?.getBoundingClientRect();
  const visible = props.active && page.value && rect && rect.width >= 1 && rect.height >= 1;
  try {
    await setSkyBenefitsView(currentPage, visible ? { x: rect.x, y: rect.y, width: rect.width, height: rect.height } : null);
    if (!disposed && visible && currentPage === page.value) { ready.value = true; error.value = ''; }
  } catch (cause) {
    if (!disposed && currentPage === page.value) { ready.value = false; error.value = String(cause); }
  }
}
function scheduleLayout() {
  cancelAnimationFrame(frame);
  frame = requestAnimationFrame(() => { void syncView(); });
}
async function navigate(action: BenefitsAction) {
  if (!page.value) return;
  navigating.value = true;
  try { await navigateSkyBenefits(page.value, action); }
  catch (cause) { showToast('福利页面操作失败', String(cause)); }
  finally { navigating.value = false; }
}
function changePage(next: BenefitsPage | null) {
  if (page.value === next) return;
  ready.value = false;
  error.value = '';
  page.value = next;
}
watch(() => [props.active, uiZoomLevel.value, page.value], async () => {
  if (!props.active) page.value = null;
  await nextTick();
  scheduleLayout();
});
onMounted(() => {
  observer = new ResizeObserver(scheduleLayout);
  if (host.value) observer.observe(host.value);
  window.addEventListener('resize', scheduleLayout);
  scheduleLayout();
});
onBeforeUnmount(() => {
  disposed = true;
  cancelAnimationFrame(frame);
  observer?.disconnect();
  window.removeEventListener('resize', scheduleLayout);
  void setSkyBenefitsView(page.value ?? 'checkin', null).catch(cause => showToast('福利页面收起失败', String(cause)));
});
</script>

<template>
  <section class="sky-benefits-panel" :class="{ 'is-expanded': page }" aria-label="光遇官方福利">
    <div v-if="!page" class="sky-benefits-home">
      <div class="sky-benefits-entries">
        <article class="sky-benefits-entry">
          <div class="sky-benefits-entry-heading"><Gift /><span>每日福利</span></div>
          <h3>签到福利</h3>
          <p>每日签到，领取当期累计奖励。</p>
          <div class="sky-benefits-entry-footer">
            <span>签到 · 累计奖励 · 礼品领取</span>
            <button type="button" class="wp-game-action is-primary" @click="changePage('checkin')">签到 / 领取<ArrowRight class="w-3.5 h-3.5" /></button>
          </div>
        </article>
        <article class="sky-benefits-entry">
          <div class="sky-benefits-entry-heading"><ShoppingBag /><span>按需充值</span></div>
          <h3>优惠充值</h3>
          <p>查看网易官方商品与当前充值优惠。</p>
          <div class="sky-benefits-entry-footer">
            <span>官方商品 · 优惠 · 订单</span>
            <button type="button" class="wp-game-action" @click="changePage('recharge')">查看充值<ArrowRight class="w-3.5 h-3.5" /></button>
          </div>
        </article>
      </div>
      <p class="sky-benefits-login-note">首次登录后，此设备会保留官方登录状态；会话过期时需重新登录。</p>
    </div>
    <div v-show="page" class="sky-benefits-official">
    <div class="sky-benefits-toolbar">
      <nav class="sky-benefits-tabs" aria-label="官方福利分类">
        <button type="button" :aria-current="page === 'checkin' ? 'page' : undefined" @click="changePage('checkin')"><Gift />签到福利</button>
        <button type="button" :aria-current="page === 'recharge' ? 'page' : undefined" @click="changePage('recharge')"><ShoppingBag />优惠充值</button>
      </nav>
      <div class="sky-benefits-tools">
        <button type="button" :disabled="!ready || navigating" aria-label="返回上一页" title="返回上一页" @click="navigate('back')"><ArrowLeft /></button>
        <button type="button" :disabled="!ready || navigating" :aria-label="page === 'checkin' ? '刷新签到福利' : '刷新优惠充值'" title="刷新当前入口" @click="navigate('home')"><RotateCw :class="{ 'animate-spin': navigating }" /></button>
        <button type="button" class="sky-benefits-collapse" @click="changePage(null)"><X />收起官网</button>
      </div>
    </div>
    <div ref="host" class="sky-benefits-web">
      <div v-if="error" role="alert" class="sky-benefits-message">
        <p>{{ error }}</p><button type="button" class="wp-game-action mt-3" @click="syncView">重新打开</button>
      </div>
      <p v-else-if="!ready" role="status" class="sky-benefits-message">正在打开官方福利…</p>
    </div>
    </div>
  </section>
</template>

<style scoped>
.sky-benefits-panel { display: flex; flex-direction: column; flex: 1; min-height: 0; margin: 0 !important; }
.sky-benefits-home { overflow: auto; }
.sky-benefits-entries { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 40px; }
.sky-benefits-entry { min-width: 0; border-top: 1px solid var(--color-hairline); border-bottom: 1px solid var(--color-hairline); padding: 24px 0 20px; }
.sky-benefits-entry-heading { display: flex; align-items: center; gap: 8px; color: var(--color-muted); font-size: 12px; }
.sky-benefits-entry-heading svg { width: 18px; height: 18px; color: var(--game-color); }
.sky-benefits-entry h3 { font-size: 22px; font-weight: 500; margin: 18px 0 8px; color: var(--color-ink); }
.sky-benefits-entry > p { font-size: 13px; line-height: 1.7; color: var(--color-muted); }
.sky-benefits-entry-footer { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; margin-top: 28px; }
.sky-benefits-entry-footer > span { font-size: 12px; color: var(--color-faint); }
.sky-benefits-login-note { font-size: 12px; color: var(--color-faint); line-height: 1.8; padding-top: 18px; }
.sky-benefits-official { display: flex; flex-direction: column; flex: 1; min-height: 0; width: min(100%, 540px); align-self: center; }
.sky-benefits-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 0 0 12px; flex-shrink: 0; }
.sky-benefits-tabs { display: flex; gap: 4px; padding: 3px; background: var(--color-paper); border: 1px solid var(--color-hairline); border-radius: 6px; }
.sky-benefits-tabs button { display: flex; align-items: center; gap: 6px; padding: 7px 12px; font-size: 13px; color: var(--color-muted); border-radius: 3px; cursor: pointer; }
.sky-benefits-tabs button[aria-current="page"] { color: var(--game-color); background: color-mix(in srgb, var(--game-color) 10%, transparent); }
.sky-benefits-tools { display: flex; align-items: center; gap: 4px; color: var(--color-muted); }
.sky-benefits-tools button { padding: 6px; cursor: pointer; border-radius: 4px; }
.sky-benefits-collapse { display: flex; align-items: center; gap: 4px; font-size: 12px; white-space: nowrap; }
.sky-benefits-tools button:hover { background: var(--color-paper); color: var(--color-ink); }
.sky-benefits-tools button:disabled { opacity: .4; cursor: default; }
.sky-benefits-toolbar svg { width: 14px; height: 14px; }
.sky-benefits-web { flex: 1; min-height: 0; position: relative; background: var(--color-surface); outline: 1px solid var(--color-hairline); }
.sky-benefits-message { padding: 28px; color: var(--color-muted); font-size: 13px; }
@container (max-width: 680px) {
  .sky-benefits-entries { grid-template-columns: 1fr; gap: 0; }
  .sky-benefits-entry + .sky-benefits-entry { border-top: 0; }
  .sky-benefits-entry { padding-top: 18px; }
  .sky-benefits-entry h3 { margin-top: 12px; }
  .sky-benefits-entry-footer { margin-top: 18px; }
}
</style>
