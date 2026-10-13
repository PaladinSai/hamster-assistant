<script setup lang="ts">
import { ref, watch } from "vue";
import type { CircuitLiveDetail } from "./liveTypes";
import LiveRemain from "./LiveRemain.vue";

const props = defineProps<{ detail: CircuitLiveDetail }>();
const emit = defineEmits<{ ended: [] }>();

/** 只在换回廊（钢/无尽）时收起轮换表。 */
const showRotation = ref(false);
watch(
  () => props.detail.label,
  () => {
    showRotation.value = false;
  },
);
</script>

<template>
  <div class="live-circuit">
    <div class="live-hero">
      <div class="live-hero-text">
        <span class="live-kicker">{{ detail.label }}</span>
        <span class="live-hero-title">本周轮换</span>
      </div>
      <div class="live-hero-meta">
        <LiveRemain :expiry="detail.expiry" @ended="emit('ended')" />
      </div>
    </div>
    <ul class="live-chip-list">
      <li v-for="(n, i) in detail.thisWeek" :key="'t' + i" class="live-chip">{{ n }}</li>
    </ul>
    <template v-if="detail.nextWeek.length">
      <span class="live-kicker next">下周预测</span>
      <ul class="live-chip-list muted">
        <li v-for="(n, i) in detail.nextWeek" :key="'n' + i" class="live-chip">{{ n }}</li>
      </ul>
    </template>
    <button
      v-if="detail.rotationWeeks.length"
      type="button"
      class="live-rot-toggle"
      @click="showRotation = !showRotation"
    >{{ showRotation ? "收起轮换表" : "查看轮换表" }}</button>
    <ol v-if="showRotation && detail.rotationWeeks.length" class="live-rot-table">
      <li
        v-for="(week, wi) in detail.rotationWeeks"
        :key="wi"
        class="live-rot-row"
        :class="{ current: detail.currentWeekIndex === wi }"
      >
        <span class="live-rot-idx">第 {{ wi + 1 }} 周</span>
        <span class="live-rot-items">{{ week.join(" · ") }}</span>
        <span v-if="detail.currentWeekIndex === wi" class="live-rot-now">本周</span>
      </li>
    </ol>
  </div>
</template>
