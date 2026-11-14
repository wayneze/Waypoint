<script setup lang="ts">
import { ref } from 'vue';
import { openExternalUrl } from '../services/systemApi';
import { handleClearSteamWebApiKey, handleSaveSteamWebApiKey, steamWebApiKeyConfigured, steamWebApiKeyDraft } from '../workspace/vault';
const busy = ref(false);
const error = ref('');
async function save(clear = false) {
  if (busy.value) return;
  error.value = '';
  if (!clear && !/^[a-f\d]{32}$/i.test(steamWebApiKeyDraft.value.trim())) { error.value = '请输入 Steam 提供的 32 位 API key。'; return; }
  busy.value = true;
  try { if (clear) await handleClearSteamWebApiKey(); else await handleSaveSteamWebApiKey(); }
  catch (e) { error.value = String(e); }
  finally { busy.value = false; }
}
</script>
<template>
  <details class="steam-connection" :open="!steamWebApiKeyConfigured">
    <summary>Steam 连接 <span>{{ steamWebApiKeyConfigured ? '密钥已配置' : '需要配置密钥' }}</span></summary>
    <div class="body">
      <p>账号卡填写资料页地址；这里配置本机查询用的 API key，可查询已公开的 Steam 账号。之前在设置中保存的 key 会继续使用。</p>
      <form @submit.prevent="save()"><label>Web API key<input v-model="steamWebApiKeyDraft" type="password" autocomplete="off" :placeholder="steamWebApiKeyConfigured ? '输入新 key 替换已保存的 key' : '粘贴 Steam Web API key'" :disabled="busy" /></label><button :disabled="busy || !steamWebApiKeyDraft.trim()">{{ busy ? '正在保存…' : '保存密钥' }}</button><button v-if="steamWebApiKeyConfigured" type="button" :disabled="busy" @click="save(true)">清除密钥</button><button type="button" @click="openExternalUrl('https://steamcommunity.com/dev/apikey')">Steam 官方申请页</button></form>
      <p>密钥保存在 Windows 凭据管理器，不写入导出文件。Steam「我的个人资料」和「游戏详情」需公开，成就才能同步。</p>
      <p v-if="error" role="alert">{{ error }}</p>
    </div>
  </details>
</template>
<style scoped>
.steam-connection{flex-shrink:0;border:1px solid var(--color-hairline);border-radius:5px;margin:8px 0 12px;font-size:12px}.steam-connection summary{cursor:pointer;padding:11px 14px}.steam-connection summary span{margin-left:10px;color:var(--color-beacon);font-size:11px}.body{padding:0 14px 10px}.body p{color:var(--color-muted);line-height:1.8;margin:7px 0}form{display:flex;align-items:end;gap:10px;flex-wrap:wrap}label{display:grid;gap:5px;flex:1;min-width:220px}input,button{font:inherit;color:inherit;border:1px solid var(--color-hairline);border-radius:4px;background:var(--color-paper);padding:8px 10px}button{cursor:pointer}button:disabled{opacity:.45}input:focus-visible,button:focus-visible,summary:focus-visible{outline:2px solid var(--color-beacon);outline-offset:2px}p[role=alert]{color:var(--color-alert,#d68e7b)}
</style>
