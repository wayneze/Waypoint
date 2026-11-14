<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { Maximize2, Minimize2, Pause, Play, Volume2, VolumeX, ExternalLink, SlidersHorizontal } from '@lucide/vue';
import type { GameExtraMedia, GameMetadata } from '../types';
import { atmosphereSources, playableMedia } from '../services/gameMedia';
import { openResourceWeb } from '../services/systemApi';
import GameMediaPlayer from './GameMediaPlayer.vue';
import { mediaAutoplay, mediaMuted } from '../workspace/settings';
const props = defineProps<{ metadata?: GameMetadata | null; extra?: GameExtraMedia[]; title: string }>();
const emit = defineEmits<{ focus: [value: boolean] }>();
const sources = computed(() => atmosphereSources(props.metadata, props.extra));
const mode = ref<'video' | 'audio' | 'still'>('still');
const selectedUrl = ref('');
const current = computed(() => mode.value === 'still' ? undefined : sources.value.find(item => item.url === selectedUrl.value && item.kind === mode.value) || sources.value.find(item => item.kind === mode.value));
const playing = ref(false), muted = ref(mediaMuted.value), volume = ref(.35), focused = ref(false), activated = ref(false);
const error = ref(''), time = ref(0), duration = ref(0), player = ref<InstanceType<typeof GameMediaPlayer>>();
const retry = ref(0);
const expanded = ref(false);
let chosen = false;
watch(sources, items => {
  if (!chosen) {
    mode.value = items[0]?.kind || 'still'; selectedUrl.value = items[0]?.url || '';
    activated.value = playing.value = !!items.length && mediaAutoplay.value && !document.hidden;
  }
}, { immediate: true });
function focus(value: boolean) { focused.value = value; emit('focus', value); }
function choose(value: typeof mode.value) { chosen = true; pause(); mode.value = value; selectedUrl.value = ''; activated.value = false; error.value = ''; time.value = 0; duration.value = 0; focus(false); }
function pause() { playing.value = false; }
function play(item: GameExtraMedia) {
  if (!playableMedia(item.kind, item.url)) { pause(); void openResourceWeb(item.url, props.title).catch(e => error.value = String(e)); return; }
  chosen = true; mode.value = item.kind; selectedUrl.value = item.url; activated.value = true; error.value = ''; playing.value = true;
}
function toggle() { chosen = true; if (error.value) { retry.value++; error.value = ''; } activated.value = true; playing.value = !playing.value; }
function visible() { if (document.hidden) pause(); }
function clock(value: number) { return `${Math.floor(value / 60)}:${String(Math.floor(value % 60)).padStart(2, '0')}`; }
async function source() {
  if (!current.value) return;
  try { await openResourceWeb(props.metadata?.videos?.find(v => v.url === current.value!.url)?.page_url || current.value.url, props.title); }
  catch (e) { error.value = String(e); }
}
onMounted(() => document.addEventListener('visibilitychange', visible));
onBeforeUnmount(() => document.removeEventListener('visibilitychange', visible));
defineExpose({ play, pause, still: () => choose('still') });
</script>
<template>
  <div class="atmosphere" :class="{ focused }">
    <div v-if="current && activated" class="atmosphere-picture" :class="{ 'is-audio': current.kind === 'audio' }"><GameMediaPlayer :key="current.url + retry" ref="player" :url="current.url" :kind="current.kind" :playing="playing" :muted="muted" :volume="volume" @playing="playing = $event" @time="(at, total) => { time = at; duration = total; }" @error="error = $event" /></div>
    <div v-if="current?.kind === 'video' && activated" class="atmosphere-wash" />
    <div v-if="sources.length" class="atmosphere-controls">
      <button :disabled="!current" :aria-label="playing ? '暂停氛围播放' : '开始氛围播放'" @click="toggle"><Pause v-if="playing" /><Play v-else /></button>
      <div class="atmosphere-label"><strong>{{ current?.title || '静态作品背景' }}</strong><small v-if="error" role="alert">{{ error }}</small></div>
      <button v-if="current" :aria-label="muted ? '开启声音' : '静音'" @click="muted = !muted"><VolumeX v-if="muted" /><Volume2 v-else /></button>
      <button v-if="current?.kind === 'video'" :aria-label="focused ? '退出专注观看' : '专注观看'" @click="focus(!focused)"><Minimize2 v-if="focused" /><Maximize2 v-else /></button>
      <button :aria-expanded="expanded" aria-label="播放选项" @click="expanded = !expanded"><SlidersHorizontal /></button>
      <button v-if="error" aria-label="打开媒体来源" @click="source"><ExternalLink /></button>
      <div v-if="expanded" class="atmosphere-options"><div class="atmosphere-modes"><button v-for="item in [{id:'video' as const,label:'宣传片'},{id:'audio' as const,label:'原声'},{id:'still' as const,label:'静态'}].filter(item => item.id === 'still' || sources.some(source => source.kind === item.id))" :key="item.id" :aria-pressed="mode === item.id" @click="choose(item.id)">{{ item.label }}</button></div><input v-if="current && !muted" v-model.number="volume" class="atmosphere-volume" type="range" min="0" max="1" step=".05" aria-label="音量" /></div>
      <label v-if="expanded && activated && duration" class="atmosphere-timeline"><span>{{ clock(time) }}</span><input type="range" :value="time" min="0" :max="duration" step=".1" aria-label="播放进度" @input="player?.seek(Number(($event.target as HTMLInputElement).value))" /><span>{{ clock(duration) }}</span></label>
    </div>
  </div>
</template>
<style scoped>
.atmosphere{position:static}.atmosphere-picture,.atmosphere-wash{position:absolute;inset:0;pointer-events:none;z-index:-1}.atmosphere-picture{overflow:hidden}.atmosphere-picture.is-audio{visibility:hidden}.atmosphere-wash{background:linear-gradient(90deg,color-mix(in srgb,var(--color-canvas) 94%,transparent),transparent 85%),linear-gradient(0deg,var(--color-canvas),transparent 80%)}.atmosphere.focused .atmosphere-wash{background:linear-gradient(0deg,#101410e8,transparent 35%)}.atmosphere.focused .atmosphere-picture{background:#111}.atmosphere-controls{display:flex;align-items:center;gap:12px;flex-wrap:wrap;margin:0 32px;padding:8px 0;border-top:1px solid var(--color-hairline)}button{display:inline-flex;align-items:center;justify-content:center;gap:6px;min-height:34px;padding:7px 10px;border-radius:4px;font-size:12px;cursor:pointer}button:disabled{opacity:.35;cursor:default}button:hover:not(:disabled){background:var(--color-paper)}svg{width:16px;height:16px}.atmosphere-label{flex:1;min-width:120px}.atmosphere-label strong{display:block;font-size:12px;font-weight:500}.atmosphere-label small{display:block;font-size:11px;color:var(--color-muted);margin-top:4px;overflow-wrap:anywhere}.atmosphere-modes{display:flex;gap:2px}.atmosphere-modes button{font-size:11px;color:var(--color-muted)}.atmosphere-modes [aria-pressed=true]{color:var(--color-beacon);background:color-mix(in srgb,var(--color-beacon) 9%,transparent)}.atmosphere-volume{width:60px;accent-color:var(--color-beacon)}.atmosphere-timeline{display:flex;align-items:center;gap:12px;flex-basis:100%;font:10px var(--font-mono);color:var(--color-muted)}.atmosphere-timeline input{flex:1;min-width:0;accent-color:var(--color-beacon)}:where(button,input):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}@media(max-width:800px){.atmosphere-controls{margin:0 24px;gap:6px}.atmosphere-label{flex-basis:60%}}
.atmosphere-options{display:flex;align-items:center;gap:12px;flex-basis:100%}.atmosphere-label strong{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.atmosphere-label{min-width:0}
</style>
