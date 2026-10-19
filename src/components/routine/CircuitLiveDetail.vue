<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { WarframeListItem, WeaponFamilyListItem } from "../../lib/api";
import { statusPillKind } from "../../lib/status";
import { circuitItemStatus } from "./circuitProgress";
import type { CircuitLiveDetail } from "./liveTypes";
import LiveRemain from "./LiveRemain.vue";

const props = withDefaults(
  defineProps<{
    detail: CircuitLiveDetail;
    families?: WarframeListItem[];
    weaponFamilies?: WeaponFamilyListItem[];
  }>(),
  { families: () => [], weaponFamilies: () => [] },
);
const emit = defineEmits<{ ended: [] }>();

/** 只在换回廊（钢/无尽）时收起轮换表。 */
const showRotation = ref(false);
watch(
  () => props.detail.label,
  () => {
    showRotation.value = false;
  },
);

function itemStatus(name: string) {
  return circuitItemStatus(props.detail.label, name, {
    weaponFamilies: props.weaponFamilies,
    families: props.families,
  });
}

function statusKind(name: string) {
  const label = itemStatus(name);
  return label ? statusPillKind(label) : "mute";
}

const progressHint = computed(() =>
  props.detail.label === "钢铁回廊"
    ? "灵化进度"
    : props.detail.label === "无尽回廊"
      ? "普通甲"
      : "",
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
      <li v-for="(n, i) in detail.thisWeek" :key="'t' + i" class="live-chip">
        <span class="live-chip-name">{{ n }}</span>
        <span
          v-if="itemStatus(n)"
          class="live-chip-status"
          :class="statusKind(n)"
          :title="progressHint"
        >{{ itemStatus(n) }}</span>
      </li>
    </ul>
    <template v-if="detail.nextWeek.length">
      <span class="live-kicker next">下周预测</span>
      <ul class="live-chip-list muted">
        <li v-for="(n, i) in detail.nextWeek" :key="'n' + i" class="live-chip">
          <span class="live-chip-name">{{ n }}</span>
          <span
            v-if="itemStatus(n)"
            class="live-chip-status"
            :class="statusKind(n)"
            :title="progressHint"
          >{{ itemStatus(n) }}</span>
        </li>
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
        <span class="live-rot-items">
          <span
            v-for="(n, ni) in week"
            :key="ni"
            class="live-rot-item"
          >
            {{ n }}
            <span
              v-if="itemStatus(n)"
              class="live-chip-status inline"
              :class="statusKind(n)"
            >{{ itemStatus(n) }}</span>
            <template v-if="ni < week.length - 1"> · </template>
          </span>
        </span>
        <span v-if="detail.currentWeekIndex === wi" class="live-rot-now">本周</span>
      </li>
    </ol>
  </div>
</template>
