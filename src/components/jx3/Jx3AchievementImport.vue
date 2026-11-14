<script setup lang="ts">
import {
  applyAllMingyiRoles,
  applyMingyiRole,
  handleImportJx3Achievements,
  handlePickMingyiFolder,
  matchedMingyiCharacter,
  mingyiDataRoot,
  mingyiPasteOpen,
  mingyiScan,
  mingyiScanBusy,
  mingyiScanError,
  rawAchievementInput,
  scanMingyiLocal,
} from '../../workspace/jx3/progress';
import { showAchievementImportModal } from '../../workspace/core';
</script>

<template>
<div v-if="showAchievementImportModal" class="fixed inset-0 z-50 bg-black/80 flex items-center justify-center p-4">
      <div class="w-full max-w-2xl bg-paper border border-hairline rounded-2xl p-6 shadow-2xl space-y-4">
        <div class="flex items-center justify-between border-b border-hairline pb-3">
          <div>
            <h3 class="text-sm font-semibold text-white">茗伊本地成就</h3>
            <p class="text-xs text-zinc-400 mt-0.5">读本机 my#data 里的 achievement_acquire_shot.jx3dat，不经过魔盒</p>
          </div>
          <button @click="showAchievementImportModal = false" class="text-zinc-400 hover:text-white p-1 transition cursor-pointer" title="关闭">
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="M6 18L18 6M6 6l12 12"/></svg>
          </button>
        </div>

        <div class="flex items-center gap-2">
          <input
            type="text"
            v-model="mingyiDataRoot"
            placeholder="自动搜索，或选择剑网3 安装目录 / my#data"
            class="flex-1 min-w-0 bg-canvas border border-hairline rounded-md px-3 py-1.5 text-[12px] text-ink font-mono focus:outline-none focus:border-amber-500/50 placeholder:text-faint">
          <button type="button" @click="handlePickMingyiFolder"
                  class="px-3 py-1.5 bg-paper hover:bg-raised border border-hairline text-[12px] text-ink rounded-md transition cursor-pointer shrink-0">
            选择目录
          </button>
          <button type="button" @click="scanMingyiLocal()" :disabled="mingyiScanBusy"
                  class="px-3 py-1.5 bg-amber-500/15 hover:bg-amber-500/25 text-amber-300 border border-amber-500/30 rounded-md text-[12px] font-medium transition cursor-pointer disabled:opacity-50 shrink-0">
            {{ mingyiScanBusy ? '扫描中' : '扫描' }}
          </button>
        </div>

        <p v-if="mingyiScanError" class="text-[12px] text-alert">{{ mingyiScanError }}</p>
        <p v-else-if="mingyiScan" class="text-[11px] text-zinc-500 font-mono truncate">{{ mingyiScan.data_root }}</p>

        <div v-if="mingyiScan?.warnings?.length" class="text-[11px] text-zinc-500 space-y-0.5">
          <div v-for="w in mingyiScan.warnings" :key="w">{{ w }}</div>
        </div>

        <div v-if="mingyiScan?.roles?.length" class="space-y-2 max-h-72 overflow-y-auto">
          <div v-for="role in mingyiScan.roles" :key="role.uid"
               class="flex items-center justify-between gap-3 px-3 py-2 rounded-xl bg-canvas border border-hairline">
            <div class="min-w-0">
              <div class="text-[13px] text-ink font-medium truncate">{{ role.name || role.uid }}</div>
              <div class="text-[11px] text-zinc-500 truncate">
                {{ role.server }}{{ role.region ? ' · ' + role.region : '' }}
                · {{ role.shot_error ? role.shot_error : role.acquired_count + ' 项已完成' }}
                <template v-if="matchedMingyiCharacter(role)"> · 对上 {{ matchedMingyiCharacter(role)?.name }}</template>
                <template v-else> · 金库未对上</template>
              </div>
            </div>
            <button type="button" @click="applyMingyiRole(role)"
                    :disabled="!!role.shot_error"
                    class="px-2.5 py-1 text-[12px] bg-amber-500 hover:bg-amber-400 disabled:opacity-40 text-stone-950 rounded-md font-semibold transition cursor-pointer shrink-0">
              写入
            </button>
          </div>
        </div>
        <div v-else-if="!mingyiScanBusy && mingyiScan" class="px-3 py-4 rounded-md bg-canvas border border-hairline text-[13px] text-muted">
          没扫到角色目录。确认选的是 interface/my#data，并且进过游戏。
        </div>

        <div class="border-t border-hairline pt-3 space-y-2">
          <button type="button" @click="mingyiPasteOpen = !mingyiPasteOpen"
                  class="text-[11px] text-amber-400/80 hover:text-amber-300 cursor-pointer">
            {{ mingyiPasteOpen ? '收起粘贴' : '没有本机目录时，粘贴 return {0,1,2,...}' }}
          </button>
          <div v-if="mingyiPasteOpen" class="space-y-2">
            <textarea
              v-model="rawAchievementInput"
              rows="5"
              class="w-full bg-canvas border border-hairline rounded-xl p-3 text-xs text-zinc-100 font-mono focus:outline-none focus:border-amber-500/50 resize-none"
              placeholder="return {0,1,2,3,...}"></textarea>
            <div class="flex justify-end">
              <button type="button" @click="handleImportJx3Achievements"
                      class="px-3 py-1.5 text-[12px] bg-paper hover:bg-raised border border-hairline text-ink rounded-md transition cursor-pointer">
                写入当前角色
              </button>
            </div>
          </div>
        </div>

        <div class="flex items-center justify-end space-x-3 pt-2 border-t border-hairline">
          <button @click="showAchievementImportModal = false" class="px-4 py-1.5 text-xs text-zinc-400 hover:text-white rounded-lg transition cursor-pointer">
            关闭
          </button>
          <button type="button" @click="applyAllMingyiRoles"
                  :disabled="!mingyiScan?.roles?.length"
                  class="px-4 py-1.5 text-xs bg-amber-500 hover:bg-amber-400 disabled:opacity-40 text-stone-950 rounded-lg transition font-semibold cursor-pointer">
            按角色名全部写入
          </button>
        </div>
      </div>
    </div>
</template>
