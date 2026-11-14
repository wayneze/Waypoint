<script setup lang="ts">
import type { SKY_CURRENCIES } from '../../services/skyInventory';
defineProps<{ currency: keyof typeof SKY_CURRENCIES }>();
</script>

<template>
  <svg viewBox="0 0 24 28" fill="none" aria-hidden="true" class="sky-currency-icon" :class="`is-${currency}`">
    <path v-if="currency === 'heart'"
          d="M12 24S2 17.4 2 10.4C2 4.8 8.8 3.3 12 8c3.2-4.7 10-3.2 10 2.4C22 17.4 12 24 12 24Z"
          fill="currentColor" stroke="#bb7d88" stroke-width=".8" />
    <template v-else>
      <path d="M12.1 1.4c.5 3.2 3.2 4 3.2 6.2a3.3 3.3 0 0 1-6.6 0c0-2.1 2.4-3.4 3.4-6.2Z"
            fill="currentColor" />
      <path d="M6.5 11.5c3.3-1 7.7-1 11 0V25c-3.3 1.1-7.7 1.1-11 0V11.5Z"
            fill="currentColor" stroke="var(--candle-edge)" stroke-width=".8" />
      <path d="M7 12c2.8 1 7.2 1 10 0M12 10V8.5" stroke="var(--candle-edge)" stroke-linecap="round" />
      <path v-if="currency === 'season_candle'" d="m12 16 2.5 3-2.5 3-2.5-3 2.5-3Z" fill="#fff2c6" />
      <path v-else-if="currency === 'prestige'" d="m12 14.5 1.4 3.4 3.1 1.1-3.1 1.2-1.4 3.3-1.4-3.3-3.1-1.2 3.1-1.1 1.4-3.4Z" fill="#fff0d9" />
      <path v-else d="M9 16v6" stroke="#fff" stroke-width="1.5" stroke-linecap="round" opacity=".7" />
    </template>
  </svg>
</template>

<style scoped>
.sky-currency-icon { width: 22px; height: 26px; flex: none; overflow: visible; }
.is-candles { color: #f0eee4; --candle-edge: #aaa99e; }
.is-season_candle { color: #e6b367; --candle-edge: #aa773c; }
.is-heart { color: #edb0bd; }
.is-prestige { color: #de8474; --candle-edge: #a95749; }
</style>
