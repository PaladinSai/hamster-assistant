<script setup lang="ts">
import type { SortieLiveDetail } from "./liveTypes";
import LiveMissionNode from "./LiveMissionNode.vue";
import LiveRemain from "./LiveRemain.vue";

defineProps<{ detail: SortieLiveDetail }>();
const emit = defineEmits<{ ended: [] }>();
</script>

<template>
  <div class="live-sortie">
    <div class="live-hero">
      <div class="live-hero-text">
        <span class="live-kicker">今日突击</span>
        <span class="live-hero-title">{{ detail.boss }}</span>
        <span v-if="detail.faction" class="live-tag">{{ detail.faction }}</span>
      </div>
      <div class="live-hero-meta">
        <LiveRemain :expiry="detail.expiry" @ended="emit('ended')" />
        <span v-if="detail.expiryLabel" class="live-expiry">{{ detail.expiryLabel }}</span>
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
          <div v-if="m.modifier" class="live-mission-mod">{{ m.modifier }}</div>
        </div>
      </li>
    </ol>
  </div>
</template>
