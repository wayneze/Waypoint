<script setup lang="ts">
import { computed, ref } from 'vue';
import { ExternalLink, Link, Trash2 } from '@lucide/vue';
import { linksForGames, removeGameLink } from '../workspace/guides/extras';
import { EXPLORE_ROLES } from '../services/exploreSites';
import { openResourceWeb } from '../services/systemApi';
import { singleAndGalgameGuides } from '../workspace/guides/library';
import { handleOpenResourceReader } from '../workspace/guides/reader';
import type { GameLink } from '../workspace/guides/extras';
import { showToast } from '../workspace/core';
import { describeError } from '../services/errors';
const props = defineProps<{ gameId: string; recordId?: string }>();
const sourceIds = computed(() => [...new Set([props.gameId, props.recordId].filter((id): id is string => !!id))]);
const links = computed(() => linksForGames(sourceIds.value));
const busy = ref('');
async function remove(url: string) {
  if (busy.value) return;
  busy.value = url;
  try { await removeGameLink(sourceIds.value, url); }
  catch (error) { showToast('资料未移除', describeError(error).message); }
  finally { busy.value = ''; }
}
function open(link: GameLink) {
  const game = singleAndGalgameGuides.value.find(g => sourceIds.value.includes(g.id) || (!!g.record_id && sourceIds.value.includes(g.record_id)));
  if (link.article && game) { void handleOpenResourceReader(game, link); return; }
  openResourceWeb(link.url, game?.title || link.title, game?.record_id || game?.id || props.gameId).catch(error => showToast('没打开', String(error)));
}
</script>

<template>
  <details v-if="links.length" class="game-links">
    <summary><Link />收藏资料 <span>{{ links.length }}</span></summary>
    <div v-for="link in links" :key="link.url" class="game-link">
      <button type="button" class="game-link-open" @click="open(link)"><span>{{ link.title }}<small>{{ EXPLORE_ROLES.find(role => role.id === link.role)?.label }} · {{ link.site }} · {{ link.article ? '正文已保存' : '在线来源' }}</small></span><ExternalLink v-if="!link.article" /></button>
      <button type="button" class="game-link-remove" :disabled="!!busy" :aria-label="'移除资料：' + link.title" @click="remove(link.url)"><Trash2 /></button>
    </div>
  </details>
</template>

<style scoped>
.game-links { padding:10px 0; border-top:1px solid var(--color-hairline); color:var(--color-muted); font-size:11px; }
summary { display:flex; align-items:center; gap:7px; cursor:pointer; list-style:none; min-height:28px; }
summary::-webkit-details-marker { display:none; }
summary > span { margin-left:auto; color:var(--color-faint); font-family:var(--font-mono); }
.game-links[open] summary { color:var(--color-beacon); }
svg { width:13px; height:13px; flex-shrink:0; }
.game-link { display:flex; gap:8px; border-top:1px solid var(--color-hairline); }
button { cursor:pointer; }
.game-link-open { display:flex; align-items:center; flex:1; min-width:0; gap:8px; text-align:left; padding:10px 0; line-height:1.6; }
.game-link-open > span { flex:1; overflow-wrap:anywhere; }
small { display:block; font-size:10px; color:var(--color-faint); margin-top:4px; }
.game-link-remove { align-self:start; padding:12px 2px; color:var(--color-faint); }
button:hover { color:var(--color-beacon); }
:where(button, summary):focus-visible { outline:2px solid var(--color-beacon); outline-offset:3px; }
</style>
