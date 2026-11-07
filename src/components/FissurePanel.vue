<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  getFissureBrief,
  getFissureWatch,
  setFissureWatch,
  type FissureBriefDto,
  type FissureWatchTarget,
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

const KIND_OPTIONS = FILTERS.filter((f) => f.key !== "all");
const TIER_OPTIONS = ["古纪", "前纪", "中纪", "后纪"];

const brief = ref<FissureBriefDto | null>(null);
const error = ref("");
const filter = ref<FilterKey>("all");
const missionTypes = ref<string[]>([]);
const targets = ref<FissureWatchTarget[]>([]);
const pickKind = ref("");
const pickTier = ref("");
const pickMission = ref("");
const lastNotified = ref<string[]>([]);
const watchOpen = ref(false);
const watchCount = computed(() => targets.value.length);
const draftReady = computed(
  () =>
    pickKind.value !== "" && pickTier.value !== "" && pickMission.value !== "",
);
const draftExists = computed(() =>
  targets.value.some(
    (t) =>
      t.kind === pickKind.value &&
      t.tier === pickTier.value &&
      t.missionType === pickMission.value,
  ),
);
let endedReloadTimer: ReturnType<typeof setTimeout> | undefined;

async function load() {
  try {
    brief.value = await getFissureBrief();
    error.value = "";
  } catch (e) {
    error.value = String(e);
  }
}

async function loadWatch() {
  try {
    const view = await getFissureWatch();
    missionTypes.value = view.missionTypes;
    targets.value = view.prefs.targets ?? [];
    lastNotified.value = view.prefs.lastNotified ?? [];
  } catch {
    /* 预览里没有 Tauri 时忽略关注栏 */
  }
}

function kindLabel(kind: string) {
  return KIND_OPTIONS.find((item) => item.key === kind)?.label ?? kind;
}

function targetKey(target: FissureWatchTarget) {
  return `${target.kind}|${target.tier}|${target.missionType}`;
}

async function persistWatch(next: FissureWatchTarget[]) {
  const previous = targets.value;
  targets.value = next;
  try {
    const saved = await setFissureWatch({
      targets: next,
      lastNotified: lastNotified.value,
    });
    targets.value = saved.targets;
    lastNotified.value = saved.lastNotified;
  } catch (e) {
    targets.value = previous;
    error.value = String(e);
    await loadWatch();
  }
}

function addTarget() {
  if (!draftReady.value || draftExists.value) return;
  void persistWatch([
    ...targets.value,
    {
      kind: pickKind.value,
      tier: pickTier.value,
      missionType: pickMission.value,
    },
  ]);
}

function removeTarget(target: FissureWatchTarget) {
  const key = targetKey(target);
  void persistWatch(targets.value.filter((item) => targetKey(item) !== key));
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
  void loadWatch();
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

    <section class="fis-watch">
      <button
        type="button"
        class="fis-watch-toggle"
        @click="watchOpen = !watchOpen"
      >
        <span>关注设置</span>
        <span v-if="watchCount" class="fis-watch-count">
          已关注 {{ watchCount }}
        </span>
        <span class="fis-watch-caret">{{ watchOpen ? "▾" : "▸" }}</span>
      </button>
      <div v-if="watchOpen" class="fis-watch-body">
        <p class="fis-watch-hint">
          种类、纪元、任务类型各选一个，再添加成一条目标。可以加多条，例如钢铁 · 古纪 · 歼灭，和九重天 · 后纪 · 生存。裂隙和某一条完全一致才提醒，每条单独弹窗，倒计时用它自己的到期时间。
        </p>
        <div class="fis-watch-group">
          <p class="fis-watch-label">种类</p>
          <div class="chip-row">
            <button
              v-for="kind in KIND_OPTIONS"
              :key="kind.key"
              type="button"
              class="watch-chip"
              :class="{ on: pickKind === kind.key }"
              @click="pickKind = kind.key"
            >
              {{ kind.label }}
            </button>
          </div>
        </div>
        <div class="fis-watch-group">
          <p class="fis-watch-label">纪元</p>
          <div class="chip-row">
            <button
              v-for="tier in TIER_OPTIONS"
              :key="tier"
              type="button"
              class="watch-chip"
              :class="{ on: pickTier === tier }"
              @click="pickTier = tier"
            >
              {{ tier }}
            </button>
          </div>
        </div>
        <div class="fis-watch-group">
          <p class="fis-watch-label">任务类型</p>
          <div class="chip-row">
            <button
              v-for="mission in missionTypes"
              :key="mission"
              type="button"
              class="watch-chip"
              :class="{ on: pickMission === mission }"
              @click="pickMission = mission"
            >
              {{ mission }}
            </button>
          </div>
        </div>
        <button
          type="button"
          class="fis-add"
          :disabled="!draftReady || draftExists"
          @click="addTarget"
        >
          {{ draftExists ? "这条已经在列表里" : "添加这条目标" }}
        </button>
        <div v-if="targets.length" class="fis-target-list">
          <div
            v-for="target in targets"
            :key="targetKey(target)"
            class="fis-target"
          >
            <span>
              {{ kindLabel(target.kind) }} · {{ target.tier }} ·
              {{ target.missionType }}
            </span>
            <button
              type="button"
              class="fis-target-x"
              @click="removeTarget(target)"
            >
              移除
            </button>
          </div>
        </div>
      </div>
    </section>

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
.fis-watch {
  border: 1px solid var(--line-soft);
  border-radius: 8px;
  background: rgba(11, 15, 20, 0.35);
}
.fis-watch-toggle {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border: none;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  font-weight: 650;
  cursor: pointer;
  text-align: left;
}
.fis-watch-count {
  font-size: 12px;
  font-weight: 500;
  color: var(--accent);
}
.fis-watch-caret {
  margin-left: auto;
  color: var(--text-mute);
}
.fis-watch-body {
  padding: 0 12px 12px;
}
.fis-watch-hint {
  margin: 0 0 10px;
  font-size: 12px;
  line-height: 1.45;
  color: var(--text-mute);
}
.fis-watch-group + .fis-watch-group {
  margin-top: 10px;
}
.fis-watch-label {
  margin: 0 0 6px;
  font-size: 12px;
  color: var(--text-dim);
}
.fis-add {
  margin-top: 12px;
  border: 1px solid var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
  font: inherit;
  font-size: 12px;
  font-weight: 650;
  padding: 6px 12px;
  border-radius: 6px;
  cursor: pointer;
}
.fis-add:disabled {
  border-color: var(--line);
  background: transparent;
  color: var(--text-mute);
  cursor: default;
}
.fis-target-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 12px;
}
.fis-target {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--line-soft);
  border-radius: 6px;
  background: rgba(11, 15, 20, 0.45);
  font-size: 13px;
}
.fis-target-x {
  margin-left: auto;
  border: none;
  background: transparent;
  color: var(--text-mute);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.fis-target-x:hover {
  color: var(--text);
}
.chip-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.watch-chip {
  appearance: none;
  border: 1px solid var(--line);
  background: var(--bg-row);
  color: var(--text-dim);
  font: inherit;
  font-size: 12px;
  padding: 5px 10px;
  border-radius: 6px;
  cursor: pointer;
}
.watch-chip.on {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
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
