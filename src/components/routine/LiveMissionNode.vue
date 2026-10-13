<script setup lang="ts">
import { computed } from "vue";
import { splitNode } from "./liveUtils";

const props = defineProps<{ node: string }>();
const parts = computed(() => splitNode(props.node));
</script>

<template>
  <div class="live-mission-node">{{ parts.place }}</div>
  <div v-if="parts.planet" class="live-mission-planet">{{ parts.planet }}</div>
</template>
