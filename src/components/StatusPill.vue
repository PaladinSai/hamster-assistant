<script setup lang="ts">
import { computed } from "vue";
import { statusPillKind } from "../lib/status";

const props = defineProps<{ label: string }>();
const kind = computed(() => (props.label ? statusPillKind(props.label) : "mute"));
</script>

<template>
  <span v-if="label" class="pill" :class="kind">{{ label }}</span>
  <span v-else class="pill mute">—</span>
</template>
