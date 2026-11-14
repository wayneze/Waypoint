<script setup lang="ts">
import { ref, watch } from 'vue';

const props = withDefaults(defineProps<{
  src?: string;
  size?: string;
}>(), {
  src: '',
  size: 'w-5 h-5',
});

const failed = ref(false);
watch(() => props.src, () => { failed.value = false; });
</script>

<template>
  <img
    v-if="src && !failed"
    :src="src"
    alt=""
    referrerpolicy="no-referrer"
    class="rounded-sm shrink-0 object-cover bg-canvas"
    :class="size"
    @error="failed = true"
  >
</template>
