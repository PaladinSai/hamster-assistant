<script setup lang="ts">
import type { NightwaveLiveDetail } from "./liveTypes";
import LiveRemain from "./LiveRemain.vue";

defineProps<{
  detail: NightwaveLiveDetail;
}>();

const emit = defineEmits<{ ended: [] }>();
</script>

<template>
  <ul class="live-challenges">
    <li
      v-for="(c, i) in detail.challenges"
      :key="i"
      class="live-challenge"
      :class="{ elite: c.isElite }"
    >
      <div class="live-challenge-top">
        <span v-if="c.isElite" class="live-badge">精英</span>
        <span class="live-challenge-title">{{ c.title }}</span>
      </div>
      <p v-if="c.description" class="live-challenge-desc">{{ c.description }}</p>
      <div class="live-challenge-meta">
        <LiveRemain :expiry="c.expiry" @ended="emit('ended')" />
        <span v-if="c.reputation != null">声望 {{ c.reputation }}</span>
      </div>
    </li>
  </ul>
</template>
