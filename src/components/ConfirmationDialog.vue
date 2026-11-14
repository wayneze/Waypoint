<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { CircleHelp, X } from '@lucide/vue';
import { confirmation, finishConfirmation } from '../workspace/confirmation';

const dialog = ref<HTMLDialogElement | null>(null);
watch(confirmation, async value => {
  await nextTick();
  if (value && confirmation.value === value) dialog.value?.showModal();
  else if (!confirmation.value) dialog.value?.close();
});
onBeforeUnmount(() => finishConfirmation(false));
</script>

<template>
  <dialog
    ref="dialog"
    class="wp-confirmation"
    aria-labelledby="confirmation-title"
    aria-describedby="confirmation-message"
    @cancel.prevent="finishConfirmation(false)"
    @close="finishConfirmation(false)"
    @click.self="finishConfirmation(false)">
    <template v-if="confirmation">
      <div class="flex items-start gap-3">
        <span class="shrink-0 grid place-items-center w-10 h-10 rounded-lg bg-beacon/10 text-beacon border border-beacon/25">
          <CircleHelp class="w-5 h-5" aria-hidden="true" />
        </span>
        <div class="min-w-0 flex-1">
          <p class="text-[12px] text-faint">Waypoint · 操作确认</p>
          <h2 id="confirmation-title" class="mt-0.5 text-[18px] font-semibold text-ink">{{ confirmation.title }}</h2>
        </div>
        <button type="button" aria-label="关闭确认" class="p-2 rounded-md text-muted hover:bg-raised hover:text-ink cursor-pointer" @click="finishConfirmation(false)">
          <X class="w-4 h-4" aria-hidden="true" />
        </button>
      </div>
      <p id="confirmation-message" class="mt-4 text-[14px] leading-relaxed text-muted">{{ confirmation.message }}</p>
      <dl v-if="confirmation.details?.length" class="mt-4 rounded-md border border-hairline bg-canvas p-3 space-y-2 text-[13px]">
        <div v-for="detail in confirmation.details" :key="detail.label" class="flex flex-wrap justify-between gap-x-4 gap-y-1">
          <dt class="text-faint">{{ detail.label }}</dt>
          <dd class="font-mono text-ink tabular-nums">{{ detail.value }}</dd>
        </div>
      </dl>
      <div class="mt-6 flex flex-wrap justify-end gap-2">
        <button type="button" autofocus class="min-h-10 px-4 rounded-md border border-hairline bg-canvas text-muted hover:bg-raised cursor-pointer" @click="finishConfirmation(false)">
          {{ confirmation.cancelLabel || '取消' }}
        </button>
        <button type="button" class="min-h-10 px-4 rounded-md border border-beacon/40 bg-beacon/15 text-ink hover:bg-beacon/25 cursor-pointer" @click="finishConfirmation(true)">
          {{ confirmation.confirmLabel }}
        </button>
      </div>
    </template>
  </dialog>
</template>

<style scoped>
.wp-confirmation {
  position: fixed;
  inset: 0;
  margin: auto;
  width: 30rem;
  max-width: calc(100% - 2rem);
  max-height: calc(100% - 2rem);
  overflow-y: auto;
  padding: 1.5rem;
  border: 1px solid var(--color-hairline);
  border-radius: 12px;
  background: var(--color-paper);
  color: var(--color-ink);
  box-shadow: 0 24px 80px #0008;
}
.wp-confirmation::backdrop { background: #0e0b08b8; }
.wp-confirmation button:focus-visible { outline: 2px solid var(--color-beacon); outline-offset: 3px; }
</style>
