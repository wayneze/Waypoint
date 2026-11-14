<script setup lang="ts">
import { computed } from 'vue';
import { BookOpen, Check, Gamepad2, Monitor } from '@lucide/vue';
import { gameRecords } from '../workspace/core';
import { steamActivity } from '../workspace/gameActivity';
const props = defineProps<{ recordId?: string; hasGuide?: boolean }>();
const record = computed(() => gameRecords.value.find(r => r.id === props.recordId));
const link = computed(() => steamActivity.value.links.find(l => l.record_id === props.recordId));
const steamInstalled = computed(() => steamActivity.value.installed.some(i => i.appid === link.value?.appid));
</script>
<template>
  <div class="game-badges" aria-label="游戏来源与内容">
    <span v-if="link" :title="steamInstalled ? 'Steam · 本机已安装' : 'Steam · 未检测到本机安装'"><Gamepad2 />Steam<Check v-if="steamInstalled" class="installed" /></span>
    <span v-if="record?.library?.installations.length" title="已关联本地安装，启动时检查目录"><Monitor />本地<small v-if="record.library.installations.length > 1">{{ record.library.installations.length }}</small></span>
    <span v-if="hasGuide"><BookOpen />攻略</span>
  </div>
</template>
<style scoped>
.game-badges{display:flex;align-items:center;gap:10px;flex-wrap:wrap;color:var(--color-muted);font-size:11px;min-height:20px}.game-badges:empty{display:none}span{display:inline-flex;align-items:center;gap:4px}svg{width:13px;height:13px;flex-shrink:0}.installed{width:11px;height:11px;color:var(--color-beacon)}small{font-size:10px}
</style>
