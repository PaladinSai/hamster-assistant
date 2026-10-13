<script setup lang="ts">
import { computed, onMounted, ref, toRef, watch } from "vue";
import type { LiveNightwaveChallengeDto } from "../lib/api";
import { useLiveDailyCache } from "../composables/useLiveCache";
import LiveRemain from "./routine/LiveRemain.vue";
import { formatShortTime } from "./routine/liveUtils";

const props = defineProps<{
  refreshToken?: number;
}>();

const refreshTokenRef = toRef(props, "refreshToken");
const { brief, ensureLoaded, reload } = useLiveDailyCache(refreshTokenRef);

const challenges = computed<LiveNightwaveChallengeDto[]>(() => {
  const b = brief.value;
  if (!b) return [];
  return [...b.nightwaveDaily, ...b.nightwaveWeekly];
});
const seasonActivation = computed(
  () => brief.value?.nightwaveSeason?.activation ?? null,
);
const seasonExpiry = computed(() => brief.value?.nightwaveSeason?.expiry ?? null);
const error = ref("");

onMounted(() => {
  void ensureLoaded().catch((e) => {
    error.value = String(e);
  });
});

watch(brief, () => {
  error.value = "";
});
</script>

<template>
  <div class="card nightwave-card">
    <h2 class="nw-title">
      <span>午夜电波</span>
      <span v-if="seasonActivation || seasonExpiry" class="nw-season">
        <template v-if="seasonActivation">
          自 {{ formatShortTime(seasonActivation) }}
        </template>
        <template v-if="seasonActivation && seasonExpiry"> · </template>
        <LiveRemain
          v-if="seasonExpiry"
          :expiry="seasonExpiry"
          @ended="reload"
        />
      </span>
    </h2>
    <p v-if="error" class="nw-error">{{ error }}</p>
    <ul v-else class="nw-list">
      <li
        v-for="(c, i) in challenges"
        :key="i"
        class="nw-item"
        :class="{ elite: c.isElite }"
      >
        <div class="nw-item-top">
          <span v-if="c.isElite" class="nw-badge">精英</span>
          <span class="nw-item-title">{{ c.title }}</span>
          <LiveRemain
            v-if="c.expiry"
            class="nw-item-remain"
            :expiry="c.expiry"
            @ended="reload"
          />
        </div>
        <p v-if="c.description" class="nw-item-desc">{{ c.description }}</p>
        <div v-if="c.reputation != null" class="nw-item-meta">声望 {{ c.reputation }}</div>
      </li>
      <li v-if="challenges.length === 0" class="nw-empty">暂无电波挑战</li>
    </ul>
  </div>
</template>

<style scoped>
.nw-title {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 8px 12px;
}
.nw-season {
  color: var(--text-mute);
  font-weight: 500;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}
.nw-error {
  margin: 0;
  color: #e88;
  font-size: 13px;
}
.nw-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.nw-item {
  padding: 12px 14px;
  border-radius: 8px;
  background: rgba(11, 15, 20, 0.45);
  border: 1px solid var(--line-soft);
}
.nw-item.elite {
  border-color: rgba(212, 163, 90, 0.45);
  background: linear-gradient(
    120deg,
    rgba(212, 163, 90, 0.1),
    rgba(11, 15, 20, 0.4)
  );
}
.nw-item-top {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}
.nw-badge {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.04em;
  padding: 2px 7px;
  border-radius: 4px;
  color: #1a1208;
  background: var(--accent);
}
.nw-item-title {
  font-size: 15px;
  font-weight: 650;
  color: var(--text);
}
.nw-item-remain {
  margin-left: auto;
  font-size: 12px;
  color: var(--cyan);
  font-variant-numeric: tabular-nums;
}
.nw-item-desc {
  margin: 8px 0 0;
  font-size: 14px;
  line-height: 1.5;
  color: var(--text-dim);
}
.nw-item-meta {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-mute);
}
.nw-empty {
  margin: 0;
  padding: 4px 0;
  color: var(--text-mute);
  font-size: 13px;
}
</style>
