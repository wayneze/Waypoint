<script setup lang="ts">
import type { GameStamp } from '../services/games';

defineProps<{
  game: GameStamp;
}>();
</script>

<template>
  <span class="inline-flex shrink-0" aria-hidden="true">
    <img
      v-if="game === 'jx3'"
      src="/jx3/game-icon.webp"
      alt=""
      class="w-full h-full object-contain"
    />
    <svg
      v-else-if="game === 'sky'"
      class="w-full h-full"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
    >
      <path d="M12 3c-1.8 2.2-4.5 3.8-7.5 4.5 1.8 4.2 5 7.5 7.5 13.5 2.5-6 5.7-9.3 7.5-13.5-3-.7-5.7-2.3-7.5-4.5z" stroke-linejoin="round"/>
      <circle cx="12" cy="9.5" r="1.5" fill="currentColor"/>
    </svg>
    <svg
      v-else-if="game === 'nsh'"
      class="w-full h-full"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
    >
      <circle cx="12" cy="12" r="9" stroke-dasharray="3 3"/>
      <path d="M12 5v14M8.5 8.5L12 5l3.5 3.5" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
    <svg
      v-else
      class="w-full h-full"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
    >
      <rect x="2" y="6" width="20" height="12" rx="6"/>
      <path d="M6 12h4m-2-2v4m7-2h.01m3-2h.01"/>
    </svg>
  </span>
</template>
