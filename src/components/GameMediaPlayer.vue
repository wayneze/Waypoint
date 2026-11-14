<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue';
import type Hls from 'hls.js';
const props = defineProps<{ url: string; kind: 'audio' | 'video'; playing: boolean; muted: boolean; volume: number }>();
const emit = defineEmits<{ error: [message: string]; playing: [value: boolean]; time: [current: number, duration: number] }>();
const element = ref<HTMLMediaElement>();
let stream: Hls | undefined;
let generation = 0;
function stop() { generation++; stream?.destroy(); stream = undefined; element.value?.pause(); }
async function syncPlayback() {
  const media = element.value, ticket = generation;
  if (!media) return;
  media.muted = props.muted; media.volume = props.volume;
  if (!props.playing) { media.pause(); return; }
  try { await media.play(); }
  catch (error) {
    if (ticket !== generation || (error as DOMException).name === 'AbortError') return;
    emit('playing', false); emit('error', '无法开始播放，请重试或打开媒体来源。');
  }
}
watch([() => props.url, element], async () => {
  stop();
  const video = element.value, ticket = generation;
  if (!video) return;
  if (!/\.m3u8(?:[?#]|$)/i.test(props.url) || video.canPlayType('application/vnd.apple.mpegurl')) {
    video.src = props.url; void syncPlayback();
    return;
  }
  try {
  const { default: Hls } = await import('hls.js');
  if (ticket !== generation) return;
  if (!Hls.isSupported()) { emit('playing', false); emit('error','当前播放器不支持此视频流，请打开来源观看。'); return; }
  stream = new Hls({ enableWorker:false, maxBufferLength:20 });
  stream.on(Hls.Events.ERROR, (_event, data) => {
    if (data.fatal && ticket === generation) { stop(); emit('playing', false); emit('error','视频流暂时无法加载，请重试或打开来源。'); }
  });
  stream.loadSource(props.url);
  stream.on(Hls.Events.MANIFEST_PARSED, () => { if (ticket === generation) void syncPlayback(); });
  stream.attachMedia(video as HTMLVideoElement);
  } catch { if (ticket === generation) { emit('playing', false); emit('error', '媒体加载失败，请重试或打开来源。'); } }
}, { flush:'post' });
watch(() => [props.playing, props.muted, props.volume], syncPlayback);
function time() { if (element.value) emit('time', element.value.currentTime, Number.isFinite(element.value.duration) ? element.value.duration : 0); }
function seek(value: number) { if (element.value && Number.isFinite(value)) element.value.currentTime = value; }
function failed() { emit('playing', false); emit('error', '媒体暂时无法加载，可以重试或打开来源。'); }
defineExpose({ seek });
onBeforeUnmount(() => { stop(); element.value?.removeAttribute('src'); element.value?.load(); });
</script>
<template>
  <component :is="kind === 'video' ? 'video' : 'audio'" ref="element" playsinline preload="metadata" @error="failed" @ended="emit('playing', false)" @timeupdate="time" @loadedmetadata="time" />
</template>
<style scoped>
video{width:100%;height:100%;object-fit:cover;object-position:center}audio{display:none}
</style>
