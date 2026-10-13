<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import {
  getPlainsBrief,
  type PlainsBriefDto,
  type PlainsCycleDto,
} from "../lib/api";
import LiveRemain from "./routine/LiveRemain.vue";

const props = defineProps<{
  refreshToken?: number;
}>();

const brief = ref<PlainsBriefDto | null>(null);
const error = ref("");
let endedReloadTimer: ReturnType<typeof setTimeout> | undefined;

async function load() {
  try {
    brief.value = await getPlainsBrief();
    error.value = "";
  } catch (e) {
    error.value = String(e);
  }
}

function onRemainEnded() {
  if (endedReloadTimer != null) clearTimeout(endedReloadTimer);
  endedReloadTimer = setTimeout(() => {
    endedReloadTimer = undefined;
    void load();
  }, 800);
}

onMounted(() => {
  void load();
});

onUnmounted(() => {
  if (endedReloadTimer != null) clearTimeout(endedReloadTimer);
});

watch(
  () => props.refreshToken,
  (t, prev) => {
    if (t == null || t === prev || t === 0) return;
    void load();
  },
);

function stateClass(c: PlainsCycleDto) {
  return `st-${c.state}`;
}
</script>

<template>
  <div class="plains-stack">
    <p v-if="error" class="plains-error">{{ error }}</p>
    <p
      v-else-if="brief && !brief.cycles.some((c) => c.id === 'cetus')"
      class="plains-hint"
    >
      夜灵 / 魔胎需 Ostron 赏金数据，可点「更新数据」同步。
    </p>

    <div v-if="brief" class="plains-grid">
      <article
        v-for="c in brief.cycles"
        :key="c.id"
        class="card plains-card"
        :class="stateClass(c)"
      >
        <h2 class="plains-name">{{ c.nameZh }}</h2>
        <div class="plains-state-row">
          <span class="plains-badge">{{ c.stateZh }}</span>
          <span v-if="c.noteZh" class="plains-note">{{ c.noteZh }}</span>
        </div>
        <p class="plains-next">
          <LiveRemain
            class="plains-remain"
            :expiry="c.expiry"
            @ended="onRemainEnded"
          />
          <span> · 后变为 {{ c.nextStateZh }}</span>
        </p>
      </article>
    </div>
  </div>
</template>

<style scoped>
.plains-stack {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.plains-error {
  margin: 0;
  color: #e88;
  font-size: 13px;
}
.plains-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-mute);
}
.plains-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 12px;
}
.plains-card {
  padding: 16px 16px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.plains-name {
  margin: 0;
  font-size: 16px;
  font-weight: 650;
}
.plains-state-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}
.plains-badge {
  display: inline-block;
  font-size: 13px;
  font-weight: 650;
  padding: 4px 10px;
  border-radius: 6px;
  background: var(--accent-soft);
  color: var(--accent);
}
.plains-card.st-day .plains-badge,
.plains-card.st-warm .plains-badge {
  background: color-mix(in srgb, #d4a017 22%, transparent);
  color: #e6c35c;
}
.plains-card.st-night .plains-badge,
.plains-card.st-cold .plains-badge {
  background: color-mix(in srgb, #6ab0e0 22%, transparent);
  color: #8ec8f0;
}
.plains-card.st-fass .plains-badge {
  background: color-mix(in srgb, #e07a3a 22%, transparent);
  color: #f0a060;
}
.plains-card.st-vome .plains-badge {
  background: color-mix(in srgb, #7a8cff 22%, transparent);
  color: #a8b4ff;
}
.plains-card.st-joy .plains-badge {
  background: color-mix(in srgb, #e6c35c 22%, transparent);
  color: #f0d78a;
}
.plains-card.st-anger .plains-badge {
  background: color-mix(in srgb, #e06060 22%, transparent);
  color: #f09090;
}
.plains-card.st-envy .plains-badge {
  background: color-mix(in srgb, #5cb86a 22%, transparent);
  color: #8fd49a;
}
.plains-card.st-sorrow .plains-badge {
  background: color-mix(in srgb, #6a9ad4 22%, transparent);
  color: #9abce6;
}
.plains-card.st-fear .plains-badge {
  background: color-mix(in srgb, #9a6ad4 22%, transparent);
  color: #c0a0e6;
}
.plains-note {
  font-size: 11px;
  color: var(--text-mute);
}
.plains-next {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
  line-height: 1.45;
}
.plains-remain {
  color: var(--cyan);
  font-variant-numeric: tabular-nums;
  font-weight: 550;
}
</style>
