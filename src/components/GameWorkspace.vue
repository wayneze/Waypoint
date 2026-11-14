<script setup lang="ts" generic="T extends string">
import { nextTick, ref, watch } from 'vue';
import { Ellipsis } from '@lucide/vue';
import GameStampIcon from './GameStampIcon.vue';
import type { GameStamp } from '../services/games';

const props = defineProps<{
  title: string;
  subtitle: string;
  game: GameStamp;
  active: T;
  tabs: { id: T; label: string; count?: string | number; notice?: string }[];
}>();
const emit = defineEmits<{ change: [tab: T] }>();
const content = ref<HTMLElement | null>(null);
const navigation = ref<HTMLElement | null>(null);
const actions = ref<HTMLDetailsElement | null>(null);
const positions = new Map<T, number>();
let changing = false;

watch(() => props.active, async (tab, previous) => {
  if (content.value) positions.set(previous, content.value.scrollTop);
  changing = true;
  if (actions.value) actions.value.open = false;
  await nextTick();
  if (content.value) content.value.scrollTop = positions.get(tab) || 0;
  navigation.value?.querySelector('[aria-current="page"]')?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  changing = false;
}, { flush: 'pre' });

function rememberPosition() {
  if (!changing && content.value) positions.set(props.active, content.value.scrollTop);
}

function closeActions(event: Event) {
  if (!actions.value) return;
  actions.value.open = false;
  if (event instanceof KeyboardEvent) actions.value.querySelector('summary')?.focus();
}
</script>

<template>
  <section class="wp-game-workspace" :data-game="game">
    <header class="wp-game-heading">
      <div class="wp-game-identity">
        <span class="wp-game-emblem"><GameStampIcon :game="game" class="w-6 h-6" /></span>
        <div class="min-w-0">
          <h1>{{ title }}</h1>
          <p>{{ subtitle }}</p>
        </div>
      </div>
      <div class="wp-game-summary"><slot name="summary" /></div>
    </header>
    <nav ref="navigation" class="wp-game-tabs" :aria-label="title + '分组'">
      <button v-for="tab in tabs" :key="tab.id" type="button"
              :aria-current="active === tab.id ? 'page' : undefined"
              :data-game-tab="tab.id" @click="emit('change', tab.id)">
        <slot name="tab-icon" :tab="tab" />
        <span>{{ tab.label }}</span>
        <span v-if="tab.count !== undefined" class="wp-game-tab-count">{{ tab.count }}</span>
        <span v-if="tab.notice" class="wp-game-tab-notice" :title="tab.notice" :aria-label="tab.notice" />
      </button>
    </nav>
    <div class="wp-game-toolbar">
      <p class="wp-game-context"><slot name="context" /></p>
      <div class="wp-game-tools">
        <slot name="actions" />
        <details v-if="$slots.more" ref="actions" class="wp-game-more" @keydown.esc.prevent.stop="closeActions">
          <summary class="wp-text-action" aria-label="更多页面操作"><Ellipsis class="w-4 h-4" />更多</summary>
          <div class="wp-game-more-menu" @click="closeActions"><slot name="more" /></div>
        </details>
      </div>
    </div>
    <div ref="content" class="wp-game-content" role="region" :aria-label="title + ' · ' + tabs.find(tab => tab.id === active)?.label"
         tabindex="0" @scroll.passive="rememberPosition">
      <slot />
    </div>
  </section>
</template>
