<script setup lang="ts">
import { defineAsyncComponent } from 'vue';
import { showAchievementImportModal, state } from '../workspace/core';
import { showGuideReader } from '../workspace/guides/reader';
import { showEditVaultModal } from '../workspace/vault';
import {
  applyImportTasks,
  characterDisplayName,
  importRawText,
  parsedImportTasks,
  showImportModal,
  toast,
  tooltip,
} from '../workspace/core';
const GameRecordModal = defineAsyncComponent(() => import('./GameRecordModal.vue'));
const VaultDraftModal = defineAsyncComponent(() => import('./VaultDraftModal.vue'));
const GuideReader = defineAsyncComponent(() => import('./GuideReader.vue'));
const Jx3AchievementImport = defineAsyncComponent(() => import('./jx3/Jx3AchievementImport.vue'));
import ImageLightbox from './ImageLightbox.vue';
import ConfirmationDialog from './ConfirmationDialog.vue';
</script>

<template>
    <!-- 浮动游戏装备/成就 Tooltip 卡片 (Studio Graphite 极简质感) -->
    <div v-if="tooltip.show" 
         :style="{ top: `${tooltip.y}px`, left: `${tooltip.x}px` }"
         class="fixed z-50 w-72 bg-paper border border-hairline rounded-xl p-3.5 shadow-2xl pointer-events-none transition-opacity duration-150">
      <div class="flex items-start justify-between border-b border-hairline pb-2">
        <div>
          <div class="text-xs font-bold text-zinc-100 tracking-wide">{{ tooltip.title }}</div>
          <div class="text-[10px] text-zinc-400 mt-0.5">{{ tooltip.category }}</div>
        </div>
        <span class="text-[10px] px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-300 border border-zinc-700 font-mono">
          {{ tooltip.qualityBadge }}
        </span>
      </div>

      <div class="py-2.5 text-[11px] text-zinc-300 leading-relaxed whitespace-pre-line">
        {{ tooltip.desc }}
      </div>

      <div class="pt-2 border-t border-hairline space-y-1 text-[10px] font-mono text-zinc-400">
        <div v-for="s in tooltip.stats" :key="s.label" class="flex items-center justify-between">
          <span>{{ s.label }}:</span>
          <span class="text-zinc-200">{{ s.val }}</span>
        </div>
      </div>
    </div>

    <!-- 批量 Markdown 导入模态框 -->
    <div v-if="showImportModal" class="fixed inset-0 z-50 bg-black/80 flex items-center justify-center p-4">
      <div class="w-full max-w-2xl bg-paper border border-hairline rounded-2xl p-6 shadow-2xl space-y-4">
        <div class="flex items-center justify-between border-b border-hairline pb-3">
          <div>
            <h3 class="text-sm font-semibold text-white">智能批量导入任务</h3>
            <p class="text-xs text-zinc-400 mt-0.5">支持从 Obsidian 复制的表格、待办清单 (- [ ]) 或纯文本，自动识别周期并入库</p>
          </div>
          <button @click="showImportModal = false" class="text-zinc-400 hover:text-white p-1 transition cursor-pointer" title="关闭">
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="M6 18L18 6M6 6l12 12"/></svg>
          </button>
        </div>

        <div class="space-y-2">
          <label class="text-xs text-zinc-300 font-medium">粘贴文本 / 表格内容:</label>
          <textarea v-model="importRawText" rows="6" class="w-full bg-canvas border border-hairline rounded-xl p-3 text-xs text-zinc-100 font-mono focus:outline-none focus:border-sky-500/50 resize-none" placeholder="| 任务名 | 类别 | 标签 |"></textarea>
        </div>

        <!-- 实时解析预览 -->
        <div class="space-y-2">
          <div class="flex items-center justify-between text-xs">
            <span class="text-zinc-400">解析预览 (共识别出 {{ parsedImportTasks.length }} 项):</span>
            <span class="text-zinc-300 font-mono">写入目标: {{ characterDisplayName }}</span>
          </div>

          <div class="max-h-40 overflow-y-auto space-y-1.5 bg-canvas p-2.5 rounded-xl border border-hairline">
            <div v-for="(t, idx) in parsedImportTasks" :key="idx" class="flex items-center justify-between text-xs p-1.5 bg-paper rounded border border-hairline">
              <span class="text-white font-medium truncate">{{ t.title }}</span>
              <div class="flex items-center space-x-2 shrink-0">
                <span class="text-[10px] font-mono px-1.5 py-0.2 rounded border"
                      :class="t.category === 'weekly' ? 'bg-purple-500/15 text-purple-300 border-purple-500/30' : 'bg-sky-500/15 text-sky-300 border-sky-500/30'">
                  {{ t.category === 'weekly' ? '每周循环' : '每日循环' }}
                </span>
                <span class="text-[10px] text-zinc-500 font-mono">{{ t.tag }}</span>
              </div>
            </div>
            <div v-if="parsedImportTasks.length === 0" class="text-center py-4 text-xs text-zinc-500">
              请在上方粘贴内容，系统将自动识别格式并在此展示卡片预览
            </div>
          </div>
        </div>

        <div class="flex items-center justify-end space-x-3 pt-2 border-t border-hairline">
          <button @click="showImportModal = false" class="px-4 py-1.5 text-xs text-zinc-400 hover:text-white rounded-lg transition cursor-pointer">
            取消
          </button>
          <button @click="applyImportTasks" :disabled="parsedImportTasks.length === 0" class="px-4 py-1.5 text-xs bg-sky-500 hover:bg-sky-400 text-stone-950 disabled:opacity-50 rounded-lg transition font-semibold cursor-pointer shadow-sm">
            确认批量入库 ({{ parsedImportTasks.length }})
          </button>
        </div>
      </div>
    </div>

    <VaultDraftModal v-if="state.showAddVaultModal || showEditVaultModal" />

    <GameRecordModal v-if="state.showAddGameModal" />

    <GuideReader v-if="showGuideReader" />
    <Jx3AchievementImport v-if="showAchievementImportModal" />

    <div v-if="toast.show" :role="toast.persistent ? 'alert' : 'status'" class="fixed bottom-6 right-6 w-80 bg-paper border border-hairline text-ink rounded-md shadow-2xl p-4 z-50">
      <div class="flex items-start justify-between">
        <div class="flex items-center gap-2">
          <div class="w-4 h-4 rounded-md bg-raised border border-hairline flex items-center justify-center text-[10px] text-beacon font-bold">W</div>
          <span class="text-[12px] font-medium text-muted">Waypoint</span>
        </div>
        <button type="button" @click="toast.show = false" class="text-faint hover:text-ink p-1 transition cursor-pointer" title="关闭" aria-label="关闭通知">
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="M6 18L18 6M6 6l12 12"/></svg>
        </button>
      </div>
      <div class="mt-2 space-y-1">
        <div class="text-[13px] font-semibold text-ink">{{ toast.title }}</div>
        <div class="text-[12px] text-muted leading-snug">{{ toast.body }}</div>
      </div>
    </div>

    <ImageLightbox />
    <ConfirmationDialog />
</template>
