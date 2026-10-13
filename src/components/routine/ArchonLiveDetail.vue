<script setup lang="ts">
import type { ArchonLiveDetail } from "./liveTypes";
import LiveMissionNode from "./LiveMissionNode.vue";
import LiveRemain from "./LiveRemain.vue";

defineProps<{ detail: ArchonLiveDetail }>();
const emit = defineEmits<{ ended: [] }>();
</script>

<template>
  <div class="live-archon">
    <div class="live-hero">
      <div class="live-hero-text">
        <span class="live-kicker">执政官猎杀</span>
        <span class="live-hero-title">{{ detail.boss }}</span>
        <span v-if="detail.shard" class="live-tag">{{ detail.shard }}</span>
      </div>
      <div class="live-hero-meta">
        <LiveRemain :expiry="detail.expiry" @ended="emit('ended')" />
      </div>
    </div>
    <ol class="live-missions">
      <li
        v-for="(m, i) in detail.missions"
        :key="i"
        class="live-mission"
      >
        <span class="live-mission-idx">{{ i + 1 }}</span>
        <div class="live-mission-body">
          <div class="live-mission-type">{{ m.missionType || "任务" }}</div>
          <LiveMissionNode v-if="m.node" :node="m.node" />
        </div>
      </li>
    </ol>
  </div>
</template>
