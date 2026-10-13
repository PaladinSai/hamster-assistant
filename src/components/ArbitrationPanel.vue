<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import {
  getArbitrationBrief,
  type ArbitrationBriefDto,
  type ArbitrationSlotDto,
} from "../lib/api";
import LiveRemain from "./routine/LiveRemain.vue";
import { splitNode } from "./routine/liveUtils";

const props = defineProps<{
  refreshToken?: number;
}>();

const brief = ref<ArbitrationBriefDto | null>(null);
const error = ref("");

async function load() {
  try {
    brief.value = await getArbitrationBrief();
    error.value = "";
  } catch (e) {
    error.value = String(e);
  }
}

onMounted(() => {
  void load();
});

watch(
  () => props.refreshToken,
  (t, prev) => {
    if (t == null || t === prev || t === 0) return;
    void load();
  },
);

function formatWhen(iso: string) {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const mon = d.getMonth() + 1;
  const day = d.getDate();
  return `${mon}/${day} ${hh}:${mm}`;
}

/** 节点为「地点 · 星球」；标题用「星球 任务类型」。 */
function slotTitle(s: ArbitrationSlotDto) {
  const { planet, place } = splitNode(s.node);
  const mission = (s.missionType || "任务").trim();
  const where = planet || place;
  if (where && mission) return `${where} ${mission}`;
  return where || mission || "仲裁";
}

function slotMeta(s: ArbitrationSlotDto) {
  const { planet, place } = splitNode(s.node);
  const parts: string[] = [];
  if (planet && place) parts.push(place);
  if (s.faction) parts.push(s.faction);
  return parts.join(" · ");
}

const current = computed(() => brief.value?.current ?? null);
const upcoming = computed(() => brief.value?.upcoming ?? []);

function rowKey(s: ArbitrationSlotDto, i: number) {
  return `${s.nodeCode}-${s.activation}-${i}`;
}
</script>

<template>
  <div class="arbi-stack">
    <p v-if="error" class="arbi-error">{{ error }}</p>
    <p v-else-if="brief" class="arbi-lead">
      日程来源 {{ brief.source }}
    </p>

    <article class="card arbi-card current">
      <h2 class="arbi-section">当前仲裁</h2>
      <template v-if="current">
        <div class="arbi-headline">
          <span class="arbi-slot-title">{{ slotTitle(current) }}</span>
          <LiveRemain
            v-if="current.expiry"
            class="arbi-remain"
            :expiry="current.expiry"
            @ended="load"
          />
        </div>
        <p v-if="slotMeta(current)" class="arbi-meta">{{
          slotMeta(current)
        }}</p>
      </template>
      <p v-else class="arbi-empty">暂无进行中的仲裁（可点「更新数据」同步日程）</p>
    </article>

    <article class="card arbi-card">
      <h2 class="arbi-section">
        <span>后续预测</span>
        <span class="arbi-count">{{ upcoming.length }}</span>
      </h2>
      <ul v-if="upcoming.length" class="arbi-list">
        <li
          v-for="(s, i) in upcoming"
          :key="rowKey(s, i)"
          class="arbi-item"
        >
          <div class="arbi-headline">
            <span class="arbi-slot-title">{{ slotTitle(s) }}</span>
            <span class="arbi-when">{{ formatWhen(s.activation) }}</span>
          </div>
          <p v-if="slotMeta(s)" class="arbi-meta">{{ slotMeta(s) }}</p>
        </li>
      </ul>
      <p v-else class="arbi-empty">暂无后续日程</p>
    </article>
  </div>
</template>

<style scoped>
.arbi-stack {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.arbi-lead {
  margin: 0;
  font-size: 12px;
  color: var(--text-mute);
}
.arbi-error {
  margin: 0;
  color: #e88;
  font-size: 13px;
}
.arbi-card {
  padding: 14px 16px 12px;
}
.arbi-card.current {
  border-color: color-mix(in srgb, var(--accent) 35%, var(--line));
}
.arbi-section {
  margin: 0 0 10px;
  display: flex;
  align-items: baseline;
  gap: 10px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-mute);
}
.arbi-count {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-mute);
}
.arbi-headline {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 8px 12px;
}
.arbi-slot-title {
  font-size: 16px;
  font-weight: 650;
  color: var(--text);
  letter-spacing: 0.02em;
}
.arbi-remain {
  margin-left: auto;
  font-size: 12px;
  font-weight: 500;
  color: var(--cyan);
  font-variant-numeric: tabular-nums;
}
.arbi-meta {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-dim);
}
.arbi-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.arbi-item {
  padding: 10px 12px;
  border-radius: 8px;
  background: rgba(11, 15, 20, 0.45);
  border: 1px solid var(--line-soft);
}
.arbi-when {
  margin-left: auto;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  color: var(--cyan);
}
.arbi-empty {
  margin: 0;
  color: var(--text-mute);
  font-size: 13px;
}
</style>
