<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { duration, timeLabel } from '../services/gameActivity';
import { activityError, localPlaytime, runSteamBatch, steamActivity, steamBatch } from '../workspace/gameActivity';
import { steamLibrary, steamLibraryRefreshingId } from '../workspace/vault';
import { vaultList } from '../workspace/core';
import { posterSrc } from './sky/wikiImage';
const props = defineProps<{ recordId: string }>();
const emit = defineEmits<{ install: [] }>();
const link = computed(() => steamActivity.value.links.find(l => l.record_id === props.recordId));
const local = computed(() => localPlaytime.value.find(p => p.record_id === props.recordId));
const accounts = computed(() => steamActivity.value.accounts.filter(a => a.updated_at && steamLibrary.value.some(g => g.vault_item_id === a.account_id && g.appid === link.value?.appid)));
const accountId = ref('');
watch(accounts, items => { if (!items.some(a => a.account_id === accountId.value)) accountId.value = items[0]?.account_id || ''; }, { immediate: true });
const game = computed(() => steamLibrary.value.find(g => g.vault_item_id === accountId.value && g.appid === link.value?.appid));
const account = computed(() => accounts.value.find(a => a.account_id === accountId.value));
const achievements = computed(() => link.value ? account.value?.achievements[link.value.appid] : undefined);
const installed = computed(() => steamActivity.value.installed.find(i => i.appid === link.value?.appid));
const filter = ref('all');
const revealed = ref<string[]>([]);
const error = ref('');
const ownJob = computed(() => steamBatch.value.accountId === accountId.value && steamBatch.value.appids.includes(link.value?.appid || 0));
const attempted = new Set<string>();
const visible = computed(() => (achievements.value?.items || []).filter(a => filter.value === 'all' || (filter.value === 'unlocked' ? a.unlocked : !a.unlocked)));
const unlocked = computed(() => achievements.value?.items.filter(a => a.unlocked).length || 0);
watch([() => props.recordId, accountId], () => { revealed.value = []; error.value = ''; filter.value = 'all'; });
watch([link, accountId, achievements, () => steamBatch.value.running, steamLibraryRefreshingId], () => {
  if (!link.value || !accountId.value || achievements.value || steamBatch.value.running || steamLibraryRefreshingId.value) return;
  const key = `${accountId.value}/${link.value.appid}`;
  if (attempted.has(key)) return;
  attempted.add(key);
  void refresh();
}, { immediate: true });
async function refresh() {
  if (!link.value || !accountId.value || steamBatch.value.running) return;
  error.value = '';
  try { await runSteamBatch(accountId.value, [{ appid: link.value.appid }], 'sync'); }
  catch (e) { error.value = String(e); }
}
</script>

<template>
  <section class="game-activity" aria-label="游玩与成就">
    <p v-if="error || activityError" class="error" role="alert">{{ error || activityError }}</p>
    <p v-if="ownJob" :class="steamBatch.errors.length ? 'error' : 'notice'" role="status">{{ steamBatch.running ? `正在同步：${steamBatch.current}。离开页面后仍会继续。` : steamBatch.stopped ? '后续同步已停止，可重新同步。' : steamBatch.errors.length ? steamBatch.errors.join('；') : 'Steam 时长与成就已同步。' }}</p>
    <section v-if="link" class="steam-section">
      <div class="toolbar"><h3>Steam 游玩记录</h3><label v-if="accounts.length > 1">账号<select v-model="accountId" :disabled="steamBatch.running"><option v-for="a in accounts" :key="a.account_id" :value="a.account_id">{{ vaultList.find(v => v.id === a.account_id)?.account_name || 'Steam 账号' }}</option></select></label><button :disabled="steamBatch.running || !!steamLibraryRefreshingId || !accountId" @click="refresh">{{ ownJob && steamBatch.running ? '正在同步…' : '同步时长与成就' }}</button></div>
      <div class="stats"><div><span>累计游玩</span><strong>{{ game ? duration(game.playtime_forever * 60) : '待同步' }}</strong></div><div><span>近两周</span><strong>{{ game ? duration((game.playtime_2weeks || 0) * 60) : '待同步' }}</strong></div><div><span>最后游玩</span><strong class="date">{{ game?.rtime_last_played ? timeLabel(game.rtime_last_played) : '暂无记录' }}</strong></div><div><span>本机安装</span><strong class="date">{{ installed ? '已安装' : '未检测到安装' }}</strong></div></div>
      <p class="muted">AppID {{ link.appid }} · 时长同步 {{ timeLabel(account?.updated_at) }}<template v-if="installed"> · {{ installed.directory }}</template></p>
      <p v-if="account?.error" class="error">时长更新失败，显示上次缓存：{{ account.error }}</p>
      <div class="toolbar achievement-heading"><h3>成就 <span v-if="achievements?.updated_at && achievements.status !== 'unsupported'">{{ unlocked }} / {{ achievements.items.length }}</span></h3><label>筛选<select v-model="filter" aria-label="筛选成就"><option value="all">全部成就</option><option value="unlocked">已解锁</option><option value="locked">未解锁</option></select></label></div>
      <progress v-if="achievements?.items.length" :value="unlocked" :max="achievements.items.length" :aria-label="`成就已解锁 ${unlocked} / ${achievements.items.length}`" />
      <p v-if="!achievements" class="muted">{{ ownJob && steamBatch.running ? '正在获取首次成就记录…' : '尚无成就缓存，可点击“同步时长与成就”重试。' }}</p>
      <p v-else-if="achievements.status === 'unsupported'" class="muted">这款游戏没有 Steam 成就。</p>
      <p v-else-if="achievements.error" class="error" role="alert">{{ achievements.status === 'private' ? '账号访问 / 隐私受限' : '成就同步失败' }}：{{ achievements.error }}<template v-if="achievements.updated_at">。保留 {{ timeLabel(achievements.updated_at) }} 的结果。</template></p>
      <p v-else class="muted">成就同步 {{ timeLabel(achievements.updated_at) }} · 隐藏成就默认遮蔽，揭示后可能包含剧透。</p>
      <div class="achievements">
        <article v-for="a in visible" :key="a.id" :class="{unlocked:a.unlocked}">
          <template v-if="a.hidden && !a.unlocked && !revealed.includes(a.id)"><span class="hidden-icon" aria-hidden="true">?</span><div><strong>隐藏成就</strong><p>说明和图标可能包含剧透。</p><button @click="revealed.push(a.id)">揭示此成就</button></div></template>
          <template v-else><img v-if="a.unlocked ? a.icon : a.icon_gray" :src="posterSrc(a.unlocked ? a.icon : a.icon_gray)" alt="" loading="lazy" referrerpolicy="no-referrer" /><div><strong>{{ a.name }}</strong><p>{{ a.description || '暂无成就说明' }}</p><small>{{ a.unlocked ? `已解锁${a.unlocked_at ? ' · ' + timeLabel(a.unlocked_at) : ''}` : '尚未解锁' }}</small></div></template>
        </article>
      </div>
      <p v-if="achievements?.status === 'ready' && !visible.length" class="muted">没有符合筛选的成就。</p>
    </section>
    <p v-else class="muted">关联 Steam 后可查看官方时长与成就。<button @click="emit('install')">关联 Steam</button></p>
    <section class="local-section"><div class="toolbar"><h3>本地游玩记录</h3><span v-if="local?.active" class="recording" role="status">正在计时 · 本次 {{ duration(local.active.session.seconds) }}</span></div>
      <p v-if="local?.active?.tracking_error" class="error" role="alert">{{ local.active.tracking_error }}</p>
      <div class="stats"><div><span>本地累计</span><strong>{{ duration((local?.total_seconds || 0) + (local?.active?.session.seconds || 0)) }}</strong></div><div><span>上次启动</span><strong class="date">{{ local?.last_played ? timeLabel(local.last_played) : '暂无记录' }}</strong></div></div>
      <p class="muted">选择本地程序启动时由 Waypoint 计时，退出后约 10 秒结算，休眠不计入。Steam 时长与本地时长可能重叠，分别显示、不相加；Steam 成就只反映所选账号的官方记录，不会从本地存档推断或上传。</p>
      <ol v-if="local?.sessions.length" class="sessions"><li v-for="(s,index) in [...local.sessions].reverse()" :key="index"><span>{{ timeLabel(s.started_at) }}</span><strong>{{ duration(s.seconds) }}</strong><small>{{ s.end_reason === 'interrupted' ? '异常退出，仅保留已确认时长' : s.end_reason === 'backup_restored' ? '从备份恢复' : '已结束' }}{{ s.recovered ? ' · 恢复计时' : '' }}</small></li></ol>
    </section>
  </section>
</template>

<style scoped>
header,.toolbar{display:flex;align-items:center;gap:14px;flex-wrap:wrap}header{justify-content:space-between;margin-bottom:26px}.eyebrow{font:10px var(--font-mono);color:var(--color-beacon);letter-spacing:.14em}h2{font:28px var(--font-display);margin:8px 0}h3{font:19px var(--font-display);margin-right:auto}h3 span{color:var(--color-beacon);font:13px var(--font-mono);margin-left:8px}button,select{border:1px solid var(--color-hairline);padding:8px 11px;border-radius:4px;background:var(--color-paper);font-size:12px;color:inherit}button{cursor:pointer}button:disabled{opacity:.45;cursor:default}label{font-size:11px;color:var(--color-muted)}label select{margin-left:8px}.stats{display:grid;grid-template-columns:repeat(auto-fit,minmax(120px,1fr));gap:16px;margin:18px 0}.stats span{display:block;color:var(--color-muted);font-size:11px;margin-bottom:10px}.stats strong{font:22px var(--font-display)}.stats .date{font:13px var(--font-sans);line-height:1.8}.muted,.error,.notice{font-size:12px;line-height:1.8;overflow-wrap:anywhere;margin:14px 0}.muted{color:var(--color-muted)}.error{color:#e99b8b}.notice,.recording{color:var(--color-beacon)}.recording{font-size:12px}.achievement-heading{border-top:1px solid var(--color-hairline);padding-top:26px;margin-top:22px}progress{width:100%;height:5px;margin-top:20px;accent-color:var(--color-beacon)}.achievements{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,240px),1fr));gap:1px;background:var(--color-hairline);margin-block:18px}.achievements:empty{display:none}.achievements article{background:var(--color-canvas);padding:14px;display:flex;align-items:start;gap:14px}.achievements img,.hidden-icon{width:48px;height:48px;border-radius:5px;flex-shrink:0}.hidden-icon{display:grid;place-items:center;background:var(--color-paper);color:var(--color-faint);font:25px var(--font-display)}.achievements strong{font-size:13px;font-weight:500}.achievements p{font-size:12px;color:var(--color-muted);line-height:1.8;margin:5px 0}.achievements small{font-size:10px;color:var(--color-faint)}.achievements .unlocked small{color:var(--color-beacon)}.achievements button{padding:4px 8px;font-size:11px;margin-top:5px}.local-section{border-top:1px solid var(--color-hairline);padding-top:22px;margin-top:22px}.sessions{list-style:none;padding:0}.sessions li{display:flex;gap:18px;flex-wrap:wrap;align-items:center;padding:12px 0;border-bottom:1px solid var(--color-hairline);font-size:12px}.sessions small{color:var(--color-muted);margin-left:auto}.sessions strong{font-weight:500}
</style>
