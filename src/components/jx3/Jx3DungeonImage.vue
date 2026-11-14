<script setup lang="ts">
import { computed } from 'vue';
import { jx3BoxDungeons } from '../../workspace/jx3/collect';
import { jx3BoxDungeonThumbUrl, resolveJx3BoxDungeon } from '../../services/jx3Collect';
import Jx3BoxIcon from '../Jx3BoxIcon.vue';
import { jx3PlaceSceneUrl } from '../../services/jx3Artwork';
const props = defineProps<{ name: string; size?: string }>();
const source = computed(() => jx3BoxDungeonThumbUrl(resolveJx3BoxDungeon(props.name, jx3BoxDungeons.value)?.icon)
  || jx3PlaceSceneUrl(props.name.split(' · ')[0] || props.name));
</script>

<template>
  <Jx3BoxIcon v-if="source" :src="source" :size="size || 'jx3-place-image'" />
</template>
