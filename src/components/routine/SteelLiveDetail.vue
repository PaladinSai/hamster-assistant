<script setup lang="ts">
import { computed } from "vue";
import type { SteelLiveDetail } from "./liveTypes";
import LiveMissionNode from "./LiveMissionNode.vue";
import LiveRemain from "./LiveRemain.vue";

const props = defineProps<{ detail: SteelLiveDetail }>();
const emit = defineEmits<{ ended: [] }>();

const days = computed(() =>
  [
    { label: "今天", missions: props.detail.today, key: "t" },
    { label: "明天", missions: props.detail.tomorrow, key: "n" },
  ].filter((d) => d.missions.length > 0),
);
</script>

<template>
  <div class="live-steel">
    <div class="live-hero">
      <div class="live-hero-text">
        <span class="live-kicker">钢铁之路侵袭</span>
      </div>
      <div class="live-hero-meta">
        <LiveRemain :expiry="detail.expiry" @ended="emit('ended')" />
        <span v-if="detail.expiryLabel" class="live-expiry">{{ detail.expiryLabel }}</span>
      </div>
    </div>
    <template v-for="day in days" :key="day.key">
      <h4 class="live-steel-day">{{ day.label }}</h4>
      <ol class="live-missions">
        <li
          v-for="(m, i) in day.missions"
          :key="day.key + i"
          class="live-mission"
        >
          <span class="live-mission-idx">{{ i + 1 }}</span>
          <div class="live-mission-body">
            <div class="live-mission-type">
              {{ m.missionType || "任务" }}
              <span v-if="m.faction" class="live-steel-faction">{{ m.faction }}</span>
            </div>
            <div
              v-if="m.minLevel || m.maxLevel"
              class="live-steel-levels"
            >{{ m.minLevel }}-{{ m.maxLevel }}</div>
            <LiveMissionNode v-if="m.node" :node="m.node" />
          </div>
        </li>
      </ol>
    </template>
    <p
      v-if="!days.length"
      class="live-steel-note"
    >暂无节点日程（仅 UTC 日窗）</p>
  </div>
</template>
