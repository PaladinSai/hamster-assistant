<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  getFissureBrief,
  type FissureBriefDto,
  type LiveFissureDto,
} from "../lib/api";
import LiveRemain from "./routine/LiveRemain.vue";
import { splitNode } from "./routine/liveUtils";

const props = defineProps<{
  refreshToken?: number;
}>();

type FilterKey = "all" | "normal" | "steel" | "storm";

const FILTERS: { key: FilterKey; label: string }[] = [
  { key: "all", label: "全部" },
  { key: "normal", label: "普通" },
  { key: "steel", label: "钢铁" },
  { key: "storm", label: "九重天" },
];

const brief = ref<FissureBriefDto | null>(null);
const error = ref("");
const filter = ref<FilterKey>("all");
let endedReloadTimer: ReturnType<typeof setTimeout> | undefined;

async function load() {
  try {
    brief.value = await getFissureBrief();
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

type Section = { key: FilterKey; title: string; items: LiveFissureDto[] };

const allSections = computed<Section[]>(() => {
  const b = brief.value;
  if (!b) return [];
  return [
    { key: "normal", title: "普通裂隙", items: b.normal },
    { key: "steel", title: "钢铁裂隙", items: b.steel },
    { key: "storm", title: "九重天裂隙", items: b.storm },
  ];
});

const sections = computed(() => {
  if (filter.value === "all") return allSections.value;
  return allSections.value.filter((s) => s.key === filter.value);
});

const totalCount = computed(() =>
  sections.value.reduce((n, s) => n + s.items.length, 0),
);
</script>

<template>
  <div class="fissure-stack list-page">
    <div class="list-filters">
      <div class="toolbar">
        <div class="seg">
          <button
            v-for="f in FILTERS"
            :key="f.key"
            type="button"
            :class="{ on: filter === f.key }"
            @click="filter = f.key"
          >
            {{ f.label }}
          </button>
        </div>
        <span style="color: var(--text-mute); font-size: 12px">
          {{ totalCount }} 条
        </span>
      </div>
    </div>

    <p v-if="error" class="fis-error">{{ error }}</p>
    <div
      v-for="sec in sections"
      :key="sec.key"
      class="card fis-card"
      :class="sec.key"
    >
      <h2 class="fis-title">
        <span>{{ sec.title }}</span>
        <span class="fis-count">{{ sec.items.length }}</span>
      </h2>
      <ul class="fis-list">
        <li
          v-for="(f, i) in sec.items"
          :key="sec.key + i + f.node + f.tier"
          class="fis-item"
        >
          <div class="fis-item-top">
            <span class="fis-tier">{{ f.tierZh || f.tier || "—" }}</span>
            <span v-if="f.missionType" class="fis-mission">{{
              f.missionType
            }}</span>
            <LiveRemain
              v-if="f.expiry"
              class="fis-remain"
              :expiry="f.expiry"
              @ended="onRemainEnded"
            />
          </div>
          <div v-if="f.node" class="fis-node">
            <template v-for="n in [splitNode(f.node)]" :key="f.node">
              <span class="fis-place">{{ n.place }}</span>
              <span v-if="n.planet" class="fis-planet">{{ n.planet }}</span>
            </template>
          </div>
        </li>
        <li v-if="sec.items.length === 0" class="fis-empty">暂无</li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.fissure-stack {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.fis-error {
  margin: 0;
  color: #e88;
  font-size: 13px;
}
.fis-title {
  display: flex;
  align-items: baseline;
  gap: 10px;
}
.fis-count {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-mute);
}
.fis-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.fis-item {
  padding: 10px 12px;
  border-radius: 8px;
  background: rgba(11, 15, 20, 0.45);
  border: 1px solid var(--line-soft);
}
.fis-card.steel .fis-item {
  border-color: color-mix(in srgb, #c4a35a 35%, var(--line-soft));
}
.fis-card.storm .fis-item {
  border-color: color-mix(in srgb, var(--cyan) 30%, var(--line-soft));
}
.fis-item-top {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}
.fis-tier {
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.02em;
  color: var(--accent);
}
.fis-mission {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}
.fis-remain {
  margin-left: auto;
  font-size: 12px;
  color: var(--cyan);
  font-variant-numeric: tabular-nums;
}
.fis-node {
  margin-top: 4px;
  display: flex;
  flex-wrap: wrap;
  gap: 6px 10px;
  font-size: 12px;
}
.fis-place {
  color: var(--text-dim);
}
.fis-planet {
  color: var(--text-mute);
}
.fis-empty {
  margin: 0;
  padding: 4px 0;
  color: var(--text-mute);
  font-size: 13px;
}
</style>
