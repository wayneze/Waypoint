<script setup lang="ts">
import { ref } from 'vue';
import { ArrowUpRight } from '@lucide/vue';
import { connectMarket } from '../services/discovery';
import { openExternalUrl } from '../services/systemApi';
import { marketIsConnected } from '../workspace/discovery';
const key = ref(''), busy = ref(false), error = ref(''), notice = ref('');
async function save(disconnect = false) {
  if (busy.value || !disconnect && !key.value.trim()) return;
  busy.value = true; error.value = ''; notice.value = '';
  try {
    await connectMarket(disconnect ? '' : key.value);
    marketIsConnected.value = !disconnect;
    key.value = '';
    notice.value = disconnect ? '已断开跨店价格源。' : '已连接，可读取所选地区的商店报价。';
  } catch (e) { error.value = String(e); }
  finally { busy.value = false; }
}
async function open(url: string) { try { await openExternalUrl(url); } catch (e) { error.value = String(e); } }
</script>
<template>
  <section class="market-connection" aria-label="价格来源设置">
    <header><h3>跨店报价 · IsThereAnyDeal</h3><span>{{ marketIsConnected ? '已连接' : '尚未连接' }}</span></header>
    <p>连接后查询 Steam、Epic、GOG 的所选地区报价及该店历史最低价。密钥仅保存在本机凭据管理器。</p>
    <form @submit.prevent="save()"><label>API key<input v-model="key" type="password" autocomplete="off" placeholder="输入自己申请的 API key" /></label><button type="submit" :disabled="busy || !key.trim()">{{ busy ? '验证中…' : '验证并保存' }}</button><button v-if="marketIsConnected" type="button" :disabled="busy" @click="save(true)">断开连接</button></form>
    <div class="market-links"><button type="button" @click="open('https://isthereanydeal.com/apps/my/')">申请 API key<ArrowUpRight /></button><button type="button" @click="open('https://docs.isthereanydeal.com/')">来源与使用条款<ArrowUpRight /></button></div>
    <p v-if="notice" role="status">{{ notice }}</p><p v-if="error" class="market-error" role="alert">{{ error }}</p>
  </section>
</template>
<style scoped>
.market-connection{background:var(--color-paper);border:1px solid var(--color-hairline);border-radius:5px;padding:20px;margin-bottom:24px}.market-connection header{display:flex;align-items:center;justify-content:space-between;gap:12px}.market-connection h3{font-weight:500;font-size:16px}.market-connection header span{font-size:12px;color:var(--color-beacon)}.market-connection p{font-size:12px;line-height:1.8;color:var(--color-muted);margin:10px 0}.market-connection form{display:flex;gap:12px;align-items:end;flex-wrap:wrap}.market-connection label{display:flex;flex:1;min-width:160px;flex-direction:column;gap:8px;font-size:12px}.market-connection input{padding:8px;background:var(--color-chrome);border:1px solid var(--color-hairline);border-radius:4px;color:var(--color-ink)}.market-connection button{display:inline-flex;align-items:center;gap:5px;min-height:34px;font-size:12px;color:var(--color-beacon);cursor:pointer}.market-connection button:disabled{opacity:.45;cursor:default}.market-connection form button{border:1px solid var(--color-hairline);padding:7px 12px;border-radius:4px}.market-links{display:flex;gap:18px;flex-wrap:wrap;margin-top:14px}.market-links svg{width:15px;height:15px}.market-error{color:var(--color-alert)!important;overflow-wrap:anywhere}:where(button,input):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}
</style>
