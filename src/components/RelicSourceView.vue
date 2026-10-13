<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import {
  getRelicSourceFilters,
  searchRelicSources,
  type RelicLocationDto,
  type RelicMatchedItemDto,
  type RelicSourceItemDto,
  type RelicSourceFilterOptions,
} from "../lib/api";
import { rarityLabel } from "../lib/status";
import { gradeClass } from "../lib/gradeClass";
import { useExpandedSet } from "../composables/useExpandedSet";
import RelicSourceFilters from "./RelicSourceFilters.vue";

const props = defineProps<{
  initialRelicKey?: string | null;
  initialQuery?: string | null;
  /** 顶栏搜索框文案 */
  externalQuery?: string;
  updating?: boolean;
  refreshToken?: number;
  /** 当前是否在遗物页；首次激活才搜。 */
  active?: boolean;
}>();

const query = ref("");
const eras = ref<number[]>([]);
const relicKeys = ref<string[]>([]);
const rotations = ref<string[]>([]);
const gameModes = ref<string[]>([]);
const planets = ref<string[]>([]);
const filterOpts = ref<RelicSourceFilterOptions | null>(null);
const items = ref<RelicSourceItemDto[]>([]);
const truncated = ref(false);
const loading = ref(false);
const error = ref("");
const started = ref(false);

let debounce: ReturnType<typeof setTimeout> | null = null;

async function loadFilters() {
  try {
    filterOpts.value = await getRelicSourceFilters();
  } catch {
    /* 筛选项失败不挡搜索 */
  }
}

async function runSearch(opts?: { relicKey?: string }) {
  loading.value = true;
  error.value = "";
  try {
    const res = await searchRelicSources({
      query: opts?.relicKey ? undefined : query.value.trim() || undefined,
      relicKey: opts?.relicKey,
      relicKeys: relicKeys.value.length ? relicKeys.value : undefined,
      eras: eras.value.length ? eras.value : undefined,
      gameModes: gameModes.value.length ? gameModes.value : undefined,
      rotations: rotations.value.length ? rotations.value : undefined,
      planets: planets.value.length ? planets.value : undefined,
    });
    items.value = res.items;
    truncated.value = res.truncated;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function scheduleSearch() {
  if (debounce) clearTimeout(debounce);
  debounce = setTimeout(() => void runSearch(), 280);
}

function clearFilters() {
  eras.value = [];
  relicKeys.value = [];
  rotations.value = [];
  gameModes.value = [];
  planets.value = [];
  void runSearch();
}

function show(zh: string | null | undefined, fallback: string) {
  return zh || fallback;
}

function fmtChance(c: number | null) {
  if (c == null) return "—";
  return `${c % 1 === 0 ? c.toFixed(0) : c.toFixed(2)}%`;
}

function translateStagePart(part: string) {
  const s = part.trim();
  if (/^final stage$/i.test(s)) return "最终阶段";
  const of = s.match(/^stage\s+(\d+)\s+of\s+(\d+)$/i);
  if (of) return `第 ${of[1]}/${of[2]} 阶段`;
  const one = s.match(/^stage\s+(\d+)$/i);
  if (one) return `第 ${one[1]} 阶段`;
  return s;
}

function stageLabel(raw: string) {
  return raw
    .replace(/\s+and\s+/gi, ", ")
    .split(/\s*,\s*/)
    .map(translateStagePart)
    .filter((s) => s.length > 0)
    .join("、");
}

function nodeLabel(loc: RelicLocationDto) {
  if (loc.sourceKind === "bounty" && loc.stage) return stageLabel(loc.stage);
  return show(loc.nodeOrBountyZh, loc.nodeOrBounty);
}

function dropLabel(loc: RelicLocationDto) {
  const chance = fmtChance(loc.chance);
  const rot = loc.rotation?.trim();
  if (rot) return `${rot}（${chance}）`;
  return chance;
}

function matchItemLabel(m: RelicMatchedItemDto) {
  return m.itemNameZh || m.itemNameEn;
}

type SortKey = "region" | "mode" | "node" | "drop";
type SortDir = "asc" | "desc";

const COLUMNS: { key: SortKey; label: string }[] = [
  { key: "region", label: "任务区域" },
  { key: "mode", label: "任务类型" },
  { key: "node", label: "任务节点" },
  { key: "drop", label: "掉落信息" },
];

const sortRules = ref<{ key: SortKey; dir: SortDir }[]>([{ key: "drop", dir: "desc" }]);
const { expanded, toggleExpand } = useExpandedSet();

function sortRank(key: SortKey) {
  const i = sortRules.value.findIndex((r) => r.key === key);
  return i < 0 ? null : i + 1;
}

function sortDir(key: SortKey) {
  return sortRules.value.find((r) => r.key === key)?.dir ?? null;
}

function toggleSort(key: SortKey) {
  const list = sortRules.value.slice();
  const i = list.findIndex((r) => r.key === key);
  if (i < 0) list.push({ key, dir: "asc" });
  else if (list[i].dir === "asc") list[i] = { key, dir: "desc" };
  else list.splice(i, 1);
  sortRules.value = list;
}

function textKey(loc: RelicLocationDto, key: SortKey) {
  if (key === "region") return show(loc.planetOrHubZh, loc.planetOrHub);
  if (key === "mode") return show(loc.gameModeZh, loc.gameMode || "");
  if (key === "node") return nodeLabel(loc);
  return "";
}

function compareLoc(a: RelicLocationDto, b: RelicLocationDto) {
  for (const rule of sortRules.value) {
    if (rule.key === "drop") {
      const ca = a.chance;
      const cb = b.chance;
      if (ca == null && cb == null) continue;
      if (ca == null) return 1;
      if (cb == null) return -1;
      if (ca !== cb) return rule.dir === "asc" ? ca - cb : cb - ca;
      continue;
    }
    const cmp = textKey(a, rule.key).localeCompare(textKey(b, rule.key), "zh");
    if (cmp !== 0) return rule.dir === "asc" ? cmp : -cmp;
  }
  return 0;
}

function bestLocation(locs: RelicLocationDto[]) {
  let best: RelicLocationDto | null = null;
  for (const loc of locs) {
    if (loc.chance == null) continue;
    if (!best || best.chance == null || loc.chance > best.chance) best = loc;
  }
  return best ?? locs[0] ?? null;
}

function sortedLocations(it: RelicSourceItemDto) {
  return it.locations.slice().sort(compareLoc);
}

const searching = computed(() => query.value.trim().length > 0);

const visibleRelics = computed(() =>
  items.value
    .filter((it) => searching.value || it.locations.length > 0)
    .map((it) => ({
      it,
      best: bestLocation(it.locations),
      // 仅展开行排序，摘要行不碰全量 locations
      sortedLocs: expanded.value.has(it.relicKey)
        ? sortedLocations(it)
        : ([] as RelicLocationDto[]),
    })),
);

const noLocations = computed(
  () => !searching.value && items.value.length > 0 && visibleRelics.value.length === 0,
);

function emptyDetail(it: RelicSourceItemDto) {
  return it.vaulted
    ? "已入库，当前没有可刷来源。"
    : "没有符合当前筛选的来源。";
}

onMounted(() => {
  if (props.externalQuery) {
    query.value = props.externalQuery;
  }
  started.value = true;
  void loadFilters();
  if (props.initialRelicKey) {
    query.value = props.initialRelicKey;
    void runSearch({ relicKey: props.initialRelicKey });
  } else if (props.initialQuery) {
    query.value = props.initialQuery;
    void runSearch();
  } else {
    void runSearch();
  }
});

watch(
  () => props.externalQuery,
  (q) => {
    if (!started.value) return;
    const next = q ?? "";
    if (next === query.value) return;
    query.value = next;
    scheduleSearch();
  },
);

watch(
  () => props.initialRelicKey,
  (k) => {
    if (!k) return;
    query.value = k;
    void runSearch({ relicKey: k });
  },
);

watch(
  () => props.refreshToken,
  (t, prev) => {
    if (t == null || t === prev || t === 0) return;
    void loadFilters();
    void runSearch(
      props.initialRelicKey ? { relicKey: props.initialRelicKey } : undefined,
    );
  },
);
</script>

<template>
  <div class="list-page relic-src">
    <div class="list-filters">
      <RelicSourceFilters
        v-model:eras="eras"
        v-model:relic-keys="relicKeys"
        v-model:rotations="rotations"
        v-model:game-modes="gameModes"
        v-model:planets="planets"
        :filter-opts="filterOpts"
        :loading="loading"
        @search="runSearch()"
        @schedule-search="scheduleSearch"
        @clear-filters="clearFilters"
      />
    </div>

    <div class="table-wrap">
      <p v-if="error" class="banner"><strong>错误</strong> {{ error }}</p>
      <p v-if="truncated" class="relic-src-note">结果过多，仅显示前 120 条，请收窄筛选。</p>
      <p v-if="!loading && items.length === 0" class="relic-src-empty">
        无匹配。可换关键词，或等后台同步完遗物来源后再试。
      </p>

      <p v-if="!loading && noLocations" class="relic-src-empty">
        已入库，当前没有可刷来源。
      </p>
      <table v-else-if="visibleRelics.length" class="relic-src-table">
        <thead>
          <tr>
            <th>遗物名称</th>
            <th>任务区域</th>
            <th>任务类型</th>
            <th>任务节点</th>
            <th>掉落信息</th>
          </tr>
        </thead>
        <tbody v-for="row in visibleRelics" :key="row.it.relicKey">
          <tr class="relic-src-summary" @click="toggleExpand(row.it.relicKey)">
            <td>
              <div class="relic-src-name">
                <span class="chev">{{ expanded.has(row.it.relicKey) ? "▾" : "▸" }}</span>
                {{ row.it.nameZh || row.it.relicKey }}
                <span class="meta">{{ row.it.locations.length }} 处</span>
              </div>
              <div v-if="row.it.matchedItems.length" class="relic-src-match">
                <span
                  v-for="(m, i) in row.it.matchedItems"
                  :key="`${row.it.relicKey}-${i}-${m.itemNameEn}`"
                  class="relic-src-match-item"
                >
                  <span class="relic-src-match-name">{{ matchItemLabel(m) }}</span>
                  <span
                    v-if="rarityLabel(m.rarity)"
                    class="grade"
                    :class="gradeClass(m.rarity)"
                    :title="rarityLabel(m.rarity)"
                  >{{ rarityLabel(m.rarity)[0] }}</span>
                </span>
              </div>
            </td>
            <template v-if="row.best">
              <td>{{ show(row.best.planetOrHubZh, row.best.planetOrHub) }}</td>
              <td>{{ show(row.best.gameModeZh, row.best.gameMode || "") }}</td>
              <td>{{ nodeLabel(row.best) }}</td>
              <td class="drop">{{ dropLabel(row.best) }}</td>
            </template>
            <template v-else>
              <td></td>
              <td></td>
              <td></td>
              <td></td>
            </template>
          </tr>
          <tr v-if="expanded.has(row.it.relicKey)" class="relic-src-detail">
            <td colspan="5">
              <p v-if="row.it.locations.length === 0" class="relic-src-empty">
                {{ emptyDetail(row.it) }}
              </p>
              <table v-else class="relic-src-table inner">
                <thead>
                  <tr>
                    <th v-for="col in COLUMNS" :key="col.key">
                      <button type="button" class="relic-src-sort" @click="toggleSort(col.key)">
                        {{ col.label }}
                        <span v-if="sortRank(col.key)" class="sort-mark">
                          {{ sortRank(col.key) }}{{ sortDir(col.key) === "asc" ? "↑" : "↓" }}
                        </span>
                      </button>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(loc, i) in row.sortedLocs" :key="i">
                    <td>{{ show(loc.planetOrHubZh, loc.planetOrHub) }}</td>
                    <td>{{ show(loc.gameModeZh, loc.gameMode || "") }}</td>
                    <td>{{ nodeLabel(loc) }}</td>
                    <td class="drop">{{ dropLabel(loc) }}</td>
                  </tr>
                </tbody>
              </table>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
