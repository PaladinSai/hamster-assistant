<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import { computed } from "vue";
import type { WarframeListItem } from "../lib/api";
import type { FilterKey, WarframeStatusFilter } from "../lib/status";
import {
  BASE_STATUS_OPTIONS,
  vaultStatusLabel,
  warframeVariantLabel,
  WARFRAME_STATUS_FILTER_OPTS,
} from "../lib/status";
import { wishTitle } from "../lib/wish";
import { huijiSuitWikiUrl } from "../lib/wiki/huiji";
import { useColumnSort } from "../composables/useColumnSort";
import type { CompanionTypeFilter } from "../composables/useWarframeProgress";
import { useConfirm } from "../composables/useConfirm";
import ConfirmDialog from "./ConfirmDialog.vue";
import StatusPill from "./StatusPill.vue";

type WarframeSortKey =
  | "wish"
  | "name"
  | "variant"
  | "status"
  | "helminth"
  | "vault"
  | "mastered"
  | "forma"
  | "note";

const { sortKey, toggleSort, sortMark, cmpStr, cmpNum } =
  useColumnSort<WarframeSortKey>();

const {
  open: confirmOpen,
  title: confirmTitle,
  message: confirmMessage,
  confirm,
  onConfirm,
  onCancel,
} = useConfirm();

const props = withDefaults(
  defineProps<{
    families: WarframeListItem[];
    filter: FilterKey;
    statusFilter: WarframeStatusFilter;
    companionTypeFilter?: CompanionTypeFilter;
    query: string;
    selectedId: string | null;
    updating: boolean;
    markingAll?: boolean;
    /** 战甲全功能；Archwing / 机甲 / 同伴隐藏 Helminth·重生等筛选项 */
    mode?: "warframe" | "archwing" | "necramech" | "companion";
  }>(),
  { mode: "warframe", companionTypeFilter: "all" },
);

const emit = defineEmits<{
  "update:filter": [value: FilterKey];
  "update:statusFilter": [value: WarframeStatusFilter];
  "update:companionTypeFilter": [value: CompanionTypeFilter];
  select: [warframeId: string];
  toggleHelminth: [item: WarframeListItem];
  toggleForma: [item: WarframeListItem];
  toggleMastered: [item: WarframeListItem];
  toggleWish: [item: WarframeListItem];
  updateStatus: [warframeId: string, status: number];
  markAllOwned: [items: { warframeId: string; status: number }[]];
  updateData: [];
}>();

const companionTypeOpts: { key: CompanionTypeFilter; label: string }[] = [
  { key: "all", label: "全部" },
  { key: "sentinel", label: "哨兵" },
  { key: "kubrow", label: "库娃" },
  { key: "kavat", label: "库珀" },
  { key: "vulpaphyla", label: "狐獴" },
  { key: "predasite", label: "扑猎达赛" },
  { key: "moa", label: "恐鸟" },
  { key: "hound", label: "猎犬" },
];

function isModularCompanion(f: WarframeListItem) {
  return f.companionType === "moa" || f.companionType === "hound";
}

function companionTypePill(f: WarframeListItem) {
  switch (f.companionType) {
    case "moa":
      return "恐鸟";
    case "hound":
      return "猎犬";
    default:
      return warframeVariantLabel(f.variant);
  }
}

function companionTypePillClass(f: WarframeListItem) {
  switch (f.companionType) {
    case "moa":
      return "prime";
    case "hound":
      return "umbra";
    default:
      return variantClass(f.variant);
  }
}

const filters = computed((): { key: FilterKey; label: string }[] => {
  if (props.mode === "warframe") {
    return [
      { key: "all", label: "全部" },
      { key: "helminth", label: "待喂 Helminth" },
      { key: "rebirth", label: "当前重生" },
      { key: "farmable", label: "可刷未拿" },
      { key: "noPrime", label: "缺Prime" },
      { key: "noBase", label: "本体缺口" },
      { key: "unforma", label: "未挖" },
      { key: "unmastered", label: "未掌握" },
    ];
  }
  return [
    { key: "all", label: "全部" },
    { key: "unforma", label: "未挖" },
    { key: "unmastered", label: "未掌握" },
  ];
});

const showHelminthCol = computed(() => props.mode === "warframe");
const showCompanionTypes = computed(() => props.mode === "companion");
const emptyHint = computed(() => {
  if (props.mode === "archwing") return "暂无 Archwing 数据。可点「更新数据」从网络拉取。";
  if (props.mode === "necramech") return "暂无机甲数据。可点「更新数据」从网络拉取。";
  if (props.mode === "companion") return "暂无同伴数据。可点「更新数据」从网络拉取。";
  return "暂无战甲数据。可点「更新数据」从网络拉取。";
});

/** 标题：中文名（无则英文）；俗称不作主标题 */
function displayZh(f: WarframeListItem) {
  return (f.nameZh || f.nickname || f.nameEn).trim();
}

function displayEn(f: WarframeListItem) {
  return f.nameEn.trim();
}

async function openWiki(f: WarframeListItem, e: Event) {
  e.stopPropagation();
  const url = huijiSuitWikiUrl(f.nameZh || f.nickname, f.nameEn);
  if (!url) return;
  try {
    await openUrl(url);
  } catch {
    /* ignore */
  }
}

function matchesQuery(f: WarframeListItem) {
  const q = props.query.trim().toLowerCase();
  if (!q) return true;
  const hay =
    `${f.nameEn} ${f.nameZh ?? ""} ${f.nickname ?? ""} ${f.note ?? ""} ${f.familyKey}`.toLowerCase();
  return hay.includes(q);
}

function byFamily(familyId: string) {
  return props.families.filter((x) => x.familyId === familyId);
}

function matchesFilter(f: WarframeListItem) {
  const allowed = new Set(filters.value.map((x) => x.key));
  const filter = allowed.has(props.filter) ? props.filter : "all";
  const sibs = byFamily(f.familyId);
  const prime = sibs.find((x) => x.variant === 1);
  switch (filter) {
    case "helminth":
      return (
        f.variant === 0 &&
        f.status === 3 &&
        !!prime &&
        prime.status === 3 &&
        !f.helminth
      );
    case "noBase":
      return f.variant === 0 && f.status !== 3;
    case "noPrime":
      return f.variant === 1 && f.status !== 3;
    case "rebirth":
      return f.variant === 1 && f.vaultStatus === 3;
    case "farmable":
      return f.variant === 1 && f.vaultStatus === 1 && f.status !== 3;
    case "unforma":
      return f.status === 3 && f.formaDone === 0;
    case "unmastered":
      return f.mastered === 0;
    default:
      return true;
  }
}

function matchesStatus(f: WarframeListItem) {
  if (props.statusFilter === "all") return true;
  return f.status === props.statusFilter;
}

function matchesCompanionType(f: WarframeListItem) {
  if (props.mode !== "companion") return true;
  if (props.companionTypeFilter === "all") return true;
  return f.companionType === props.companionTypeFilter;
}

function warframeName(f: WarframeListItem) {
  return displayZh(f);
}

function compareWarframes(a: WarframeListItem, b: WarframeListItem) {
  const key = sortKey.value;
  if (!key) return 0;
  switch (key) {
    case "wish":
      return cmpNum(a.wish, b.wish);
    case "name":
      return cmpStr(warframeName(a), warframeName(b));
    case "variant":
      return cmpNum(a.variant, b.variant);
    case "status":
      return cmpNum(a.status, b.status);
    case "helminth":
      return cmpNum(a.helminth, b.helminth);
    case "vault":
      return cmpNum(a.vaultStatus ?? -1, b.vaultStatus ?? -1);
    case "mastered":
      return cmpNum(a.mastered, b.mastered);
    case "forma":
      return cmpNum(a.formaDone, b.formaDone);
    case "note":
      return cmpStr(a.note ?? "", b.note ?? "");
    default:
      return 0;
  }
}

const rows = computed(() => {
  const list = props.families.filter(
    (f) =>
      matchesQuery(f) &&
      matchesFilter(f) &&
      matchesStatus(f) &&
      matchesCompanionType(f),
  );
  if (!sortKey.value) return list;
  return [...list].sort((a, b) => {
    const c = compareWarframes(a, b);
    if (c !== 0) return c;
    const fk = a.familyKey.localeCompare(b.familyKey);
    if (fk !== 0) return fk;
    return a.variant - b.variant;
  });
});

const pendingOwnedCount = computed(
  () => rows.value.filter((f) => f.status !== 3).length,
);

async function onMarkAllOwned() {
  const n = pendingOwnedCount.value;
  if (n === 0) return;
  if (!(await confirm(`将当前列表 ${n} 项标为已获取？`))) return;
  emit(
    "markAllOwned",
    rows.value.map((f) => ({ warframeId: f.warframeId, status: f.status })),
  );
}

function onRowClick(f: WarframeListItem, e: MouseEvent) {
  const t = e.target as HTMLElement;
  if (t.closest("[data-toggle]")) return;
  emit("select", f.warframeId);
}

function onToggle(
  kind: "helminth" | "forma" | "mastered" | "wish",
  f: WarframeListItem,
  e: MouseEvent,
) {
  e.stopPropagation();
  if (kind === "helminth") emit("toggleHelminth", f);
  else if (kind === "forma") emit("toggleForma", f);
  else if (kind === "mastered") emit("toggleMastered", f);
  else emit("toggleWish", f);
}

function onStatusChange(f: WarframeListItem, e: Event) {
  e.stopPropagation();
  const status = Number((e.target as HTMLSelectElement).value);
  emit("updateStatus", f.warframeId, status);
}

function isClusterStart(f: WarframeListItem, i: number) {
  if (i === 0) return true;
  return rows.value[i - 1].familyId !== f.familyId;
}

function variantClass(variant: number) {
  if (variant === 1) return "prime";
  if (variant === 2) return "umbra";
  return "base";
}
</script>

<template>
  <div class="list-page">
    <div class="list-filters">
      <div class="toolbar">
        <div class="seg">
          <button
            v-for="f in filters"
            :key="f.key"
            type="button"
            :class="{ on: filter === f.key }"
            @click="emit('update:filter', f.key)"
          >
            {{ f.label }}
          </button>
        </div>
        <span style="color: var(--text-mute); font-size: 12px">
          {{ rows.length }} 条 · 点行打开右侧抽屉
        </span>
        <button
          class="ghost"
          type="button"
          :disabled="updating || markingAll || pendingOwnedCount === 0"
          @click="onMarkAllOwned"
        >
          {{ markingAll ? "处理中…" : "一键全有" }}
        </button>
      </div>
      <div v-if="showCompanionTypes" class="toolbar" style="padding-top: 0">
        <div class="seg">
          <button
            v-for="o in companionTypeOpts"
            :key="o.key"
            type="button"
            :class="{ on: companionTypeFilter === o.key }"
            @click="emit('update:companionTypeFilter', o.key)"
          >
            {{ o.label }}
          </button>
        </div>
      </div>
      <div class="toolbar" style="padding-top: 0">
        <div class="seg">
          <button
            v-for="o in WARFRAME_STATUS_FILTER_OPTS"
            :key="String(o.key)"
            type="button"
            :class="{ on: statusFilter === o.key }"
            @click="emit('update:statusFilter', o.key)"
          >
            {{ o.label }}
          </button>
        </div>
      </div>
    </div>

    <div v-if="families.length === 0" class="empty-state table-wrap">
      <p v-if="updating">正在更新数据…</p>
      <template v-else>
        <p>{{ emptyHint }}</p>
        <button
          class="ghost"
          type="button"
          @click="emit('updateData')"
        >
          更新数据
        </button>
      </template>
    </div>

    <div v-else class="table-wrap">
      <table class="list">
        <thead>
          <tr>
            <th>
              <button type="button" class="list-sort" @click="toggleSort('wish')">
                心愿<span v-if="sortMark('wish')" class="sort-mark">{{ sortMark("wish") }}</span>
              </button>
            </th>
            <th>
              <button type="button" class="list-sort" @click="toggleSort('name')">
                {{
                  mode === "archwing"
                    ? "Archwing"
                    : mode === "necramech"
                      ? "机甲"
                      : mode === "companion"
                        ? "同伴"
                        : "战甲"
                }}<span v-if="sortMark('name')" class="sort-mark">{{ sortMark("name") }}</span>
              </button>
            </th>
            <th>
              <button type="button" class="list-sort" @click="toggleSort('variant')">
                变体<span v-if="sortMark('variant')" class="sort-mark">{{ sortMark("variant") }}</span>
              </button>
            </th>
            <th>
              <button type="button" class="list-sort" @click="toggleSort('status')">
                状态<span v-if="sortMark('status')" class="sort-mark">{{ sortMark("status") }}</span>
              </button>
            </th>
            <th v-if="showHelminthCol">
              <button type="button" class="list-sort" @click="toggleSort('helminth')">
                Helminth<span v-if="sortMark('helminth')" class="sort-mark">{{ sortMark("helminth") }}</span>
              </button>
            </th>
            <th>
              <button type="button" class="list-sort" @click="toggleSort('vault')">
                入库<span v-if="sortMark('vault')" class="sort-mark">{{ sortMark("vault") }}</span>
              </button>
            </th>
            <th>
              <button type="button" class="list-sort" @click="toggleSort('mastered')">
                已掌握<span v-if="sortMark('mastered')" class="sort-mark">{{ sortMark("mastered") }}</span>
              </button>
            </th>
            <th>
              <button type="button" class="list-sort" @click="toggleSort('forma')">
                已挖<span v-if="sortMark('forma')" class="sort-mark">{{ sortMark("forma") }}</span>
              </button>
            </th>
            <th>
              <button type="button" class="list-sort" @click="toggleSort('note')">
                备注<span v-if="sortMark('note')" class="sort-mark">{{ sortMark("note") }}</span>
              </button>
            </th>
          </tr>
        </thead>
      <tbody>
        <tr
          v-for="(f, i) in rows"
          :key="f.warframeId"
          :class="{
            selected: f.warframeId === selectedId,
            'cluster-start': isClusterStart(f, i),
          }"
          @click="onRowClick(f, $event)"
        >
          <td>
            <span
              class="check wish"
              :class="{ on: !!f.wish }"
              data-toggle="wish"
              :title="wishTitle(f)"
              @click="onToggle('wish', f, $event)"
              >★</span
            >
          </td>
          <td
            class="name-cell suit-wiki-name"
            title="打开灰机 Wiki"
            @click="openWiki(f, $event)"
          >
            <span class="en">{{ displayZh(f) }}</span>
            <span
              v-if="displayEn(f) && displayEn(f) !== displayZh(f)"
              class="zh"
              >{{ displayEn(f) }}</span
            >
          </td>
          <td>
            <span
              class="pill"
              :class="
                isModularCompanion(f)
                  ? companionTypePillClass(f)
                  : variantClass(f.variant)
              "
              >{{
                isModularCompanion(f)
                  ? companionTypePill(f)
                  : warframeVariantLabel(f.variant)
              }}</span
            >
          </td>
          <td>
            <select
              class="list-status"
              data-toggle="status"
              :value="f.status"
              @click.stop
              @change="onStatusChange(f, $event)"
            >
              <option
                v-for="o in BASE_STATUS_OPTIONS"
                :key="o.value"
                :value="o.value"
              >
                {{ o.label }}
              </option>
            </select>
          </td>
          <td v-if="showHelminthCol">
            <span
              v-if="f.variant === 0"
              class="check"
              :class="{ on: !!f.helminth }"
              data-toggle="helminth"
              @click="onToggle('helminth', f, $event)"
              >✓</span
            >
            <span v-else style="color: var(--text-mute)">—</span>
          </td>
          <td>
            <StatusPill
              v-if="f.variant === 1"
              :label="vaultStatusLabel(f.vaultStatus, true)"
            />
            <span v-else style="color: var(--text-mute)">—</span>
          </td>
          <td>
            <span
              class="check"
              :class="{ on: !!f.mastered }"
              data-toggle="mastered"
              :title="f.mastered ? '取消已掌握' : '标为已掌握'"
              @click="onToggle('mastered', f, $event)"
              >✓</span
            >
          </td>
          <td>
            <span
              class="check"
              :class="{ on: !!f.formaDone }"
              data-toggle="forma"
              @click="onToggle('forma', f, $event)"
              >✓</span
            >
          </td>
          <td>
            <span class="note" :title="f.note || ''">{{ f.note || "—" }}</span>
          </td>
        </tr>
        <tr v-if="rows.length === 0">
          <td
            :colspan="showHelminthCol ? 9 : 8"
            style="padding: 24px; color: var(--text-mute)"
          >无匹配</td>
        </tr>
      </tbody>
    </table>
    </div>
  </div>

  <ConfirmDialog
    :open="confirmOpen"
    :title="confirmTitle"
    :message="confirmMessage"
    @confirm="onConfirm"
    @cancel="onCancel"
  />
</template>

<style scoped>
.cluster-start td {
  border-top: 1px solid color-mix(in srgb, var(--border) 80%, transparent);
}
.cluster-start:first-child td {
  border-top: none;
}
.suit-wiki-name {
  cursor: pointer;
}
.suit-wiki-name:hover .en {
  color: var(--cyan);
  text-decoration: underline;
}
.pill.base {
  opacity: 0.85;
}
.pill.prime {
  color: var(--accent, #c9a227);
}
.pill.umbra {
  color: var(--text-dim);
}
</style>
