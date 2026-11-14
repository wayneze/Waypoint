<script setup lang="ts">
import { computed, ref } from 'vue';
import { BookOpen, ChevronDown, Pencil, Pin } from '@lucide/vue';
import type { ObsidianGameGuide } from '../types';
import { gameRecords } from '../workspace/core';
import { steamActivity } from '../workspace/gameActivity';
import { gameLaunchOptions } from '../services/localGames';
import { libraryCardTitle } from '../services/gameLibrary';
import { guideProgressMap, handleOpenGuideReader } from '../workspace/guides/reader';
import { recentGames } from '../workspace/guides/extras';
import { isGamePinned, openGameRecordEditor, togglePinGame } from '../workspace/guides/library';
import GameLaunch from './GameLaunch.vue';

const props = defineProps<{ games: ObsidianGameGuide[] }>();
const emit = defineEmits<{ setup: [game: ObsidianGameGuide] }>();
const launchId = ref(''), readId = ref('');
const editor = ref<HTMLDetailsElement>();
const launchGames = computed(() => props.games.filter(game => {
  const record = gameRecords.value.find(record => record.id === game.record_id);
  const link = steamActivity.value.links.find(link => link.record_id === game.record_id);
  return gameLaunchOptions(record?.library?.installations || [], !!link,
    steamActivity.value.installed.some(item => item.appid === link?.appid), {}, false).length;
}));
const readingGames = computed(() => props.games.filter(game => game.file_path));
const launchGame = computed(() => launchGames.value.find(game => game.id === launchId.value) || launchGames.value[0]);
const readGame = computed(() => readingGames.value.find(game => game.id === readId.value) || readingGames.value[0]);
function edit(game: ObsidianGameGuide) {
  if (editor.value) editor.value.open = false;
  openGameRecordEditor(game);
}
</script>

<template>
  <div class="library-card-actions">
    <div v-if="launchGame" class="card-action">
      <GameLaunch compact :record-id="launchGame.record_id" :title="launchGame.title" @setup="emit('setup', launchGame)" />
      <label v-if="launchGames.length > 1" class="card-picker" :title="'启动作品：' + launchGame.title"><ChevronDown /><select aria-label="选择要启动的合集作品" :value="launchGame.id" @change="launchId = ($event.target as HTMLSelectElement).value"><option v-for="game in launchGames" :key="game.id" :value="game.id">{{ libraryCardTitle(game, true) }}</option></select></label>
    </div>
    <div v-if="readGame" class="card-action">
      <button class="library-read" :title="'阅读：' + readGame.title" @click="handleOpenGuideReader(readGame)"><BookOpen />{{ guideProgressMap[readGame.id] || recentGames[readGame.id] ? '继续阅读' : '阅读攻略' }}</button>
      <label v-if="readingGames.length > 1" class="card-picker" :title="'阅读作品：' + readGame.title"><ChevronDown /><select aria-label="选择要阅读的合集作品" :value="readGame.id" @change="readId = ($event.target as HTMLSelectElement).value"><option v-for="game in readingGames" :key="game.id" :value="game.id">{{ libraryCardTitle(game, true) }}</option></select></label>
    </div>
    <button v-if="games.length === 1" class="library-pin" :aria-label="(isGamePinned(games[0]!) ? '取消置顶作品：' : '置顶作品：') + games[0]!.title" :aria-pressed="isGamePinned(games[0]!)" :title="isGamePinned(games[0]!) ? '取消置顶' : '置顶作品'" @click="togglePinGame(games[0]!)"><Pin :fill="isGamePinned(games[0]!) ? 'currentColor' : 'none'" /></button>
    <button v-if="games.length === 1" class="library-edit" :aria-label="'编辑作品：' + games[0]!.title" title="编辑作品与合集" @click="edit(games[0]!)"><Pencil /></button>
    <details v-else ref="editor" class="card-edit-menu" @keydown.esc.stop="editor!.open = false">
      <summary class="library-edit" :aria-label="'编辑合集作品：' + games[0]?.series" title="编辑合集作品"><Pencil /></summary>
      <div><div v-for="game in games" :key="game.id" class="member-actions"><button :aria-label="'编辑作品：' + game.title" :title="game.title" @click="edit(game)">{{ libraryCardTitle(game, true) }}</button><button :aria-label="(isGamePinned(game) ? '取消置顶作品：' : '置顶作品：') + game.title" :aria-pressed="isGamePinned(game)" @click="togglePinGame(game)"><Pin :fill="isGamePinned(game) ? 'currentColor' : 'none'" /></button></div></div>
    </details>
  </div>
</template>

<style scoped>
.library-card-actions{position:relative;display:flex;align-items:center;gap:2px 10px;flex-wrap:wrap;min-height:30px;padding:2px 54px 0 0}
.card-action{display:flex;align-items:center;gap:1px;min-width:0}
button,summary{display:inline-flex;align-items:center;justify-content:center;gap:6px;min-height:28px;padding:4px 0;border:0;background:transparent;color:var(--color-muted);font-size:12px;cursor:pointer}
button:hover,summary:hover{background:var(--color-paper);color:var(--color-beacon)}
svg{width:13px;height:13px;flex:none}
.card-picker{position:relative;display:grid;place-items:center;width:18px;height:28px;color:var(--color-faint)}
.card-picker select{position:absolute;inset:0;width:100%;height:100%;opacity:0;cursor:pointer}
.library-edit{width:24px;padding:5px;color:var(--color-faint);opacity:.65}
.library-card-actions>button.library-edit{position:absolute;right:0;bottom:0}
.library-pin{position:absolute;right:28px;bottom:0;width:24px;padding:5px;color:var(--color-faint)}.library-pin[aria-pressed=true],.member-actions [aria-pressed=true]{color:var(--color-beacon)}
.library-edit:hover,.library-edit:focus-visible{opacity:1}
.card-edit-menu{position:absolute;right:0;bottom:0}
summary{list-style:none}summary::-webkit-details-marker{display:none}
.card-edit-menu[open]{z-index:10}
.card-edit-menu>div{position:absolute;bottom:100%;right:0;width:220px;max-width:70vw;padding:6px;border:1px solid var(--color-hairline);border-radius:4px;background:var(--color-chrome);box-shadow:0 10px 28px #0006}
.card-edit-menu>div button{display:block;width:100%;text-align:left;padding:7px 9px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;font-size:11px}
.member-actions{display:flex;align-items:center;min-width:0}.member-actions>button:first-child{flex:1;min-width:0}.card-edit-menu .member-actions>button:last-child{width:32px;flex:none;display:grid;place-items:center}
:where(button,summary):focus-visible,.card-picker:focus-within{outline:2px solid var(--color-beacon);outline-offset:2px}
</style>
