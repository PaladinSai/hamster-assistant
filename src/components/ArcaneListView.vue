<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import { computed, nextTick, watch } from "vue";
import type { ArcaneListItem } from "../lib/api";
import {
  ARCANE_KIND_FILTER_OPTS,
  ARCANE_PROGRESS_FILTER_OPTS,
  ARCANE_RARITY_FILTER_OPTS,
  ARCANE_SOURCE_FILTER_OPTS,
  ARCANE_STATUS_OPTIONS,
  arcaneKindLabel,
  arcaneRarityLabel,
  arcaneSourceLabel,
  type ArcaneKindFilter,
  type ArcaneProgressFilter,
  type ArcaneRarityFilter,
  type ArcaneSourceFilter,
} from "../lib/status";
import { useColumnSort } from "../composables/useColumnSort";
import { useConfirm } from "../composables/useConfirm";
import { useNoteDrafts } from "../composables/useNoteDrafts";
import { huijiArcaneWikiUrl } from "../lib/wiki/huiji";
import ConfirmDialog from "./ConfirmDialog.vue";

type ArcaneSortKey =
  | "wish"
  | "name"
  | "source"
  | "kind"
  | "rarity"
  | "status"
  | "rank"
  | "copies"
  | "note";

const { sortKey, toggleSort, sortMark, cmpStr, cmpNum } =
  useColumnSort<ArcaneSortKey>();

const {
  open: confirmOpen,
  title: confirmTitle,
  message: confirmMessage,
  confirm,
  onConfirm,
  onCancel,
} = useConfirm();

const props = defineProps<{
  arcanes: ArcaneListItem[];
  sourceFilter: ArcaneSourceFilter;
  kindFilter: ArcaneKindFilter;
  rarityFilter: ArcaneRarityFilter;
  progressFilter: ArcaneProgressFilter;
  query: string;
  selectedId: string | null;
  focusId?: string | null;
  updating: boolean;
  markingAll?: boolean;
}>();

const emit = defineEmits<{
  "update:sourceFilter": [value: ArcaneSourceFilter];
  "update:kindFilter": [value: ArcaneKindFilter];
  "update:rarityFilter": [value: ArcaneRarityFilter];
  "update:progressFilter": [value: ArcaneProgressFilter];
  toggleWish: [arcaneId: string, wish: number];
  updateStatus: [arcaneId: string, status: number];
  updateRank: [arcaneId: string, rank: number];
  updateNote: [arcaneId: string, note: string];
  select: [arcaneId: string];
  clearFocus: [];
  markAllOwned: [items: { arcaneId: string; status: number }[]];
  updateData: [];
}>();

const noteDrafts = useNoteDrafts(
  () => props.arcanes,
  (a) => a.arcaneId,
  (a) => a.note,
);

watch(
  () => props.selectedId,
  async (id) => {
    if (!id) return;
    await nextTick();
    document
      .querySelector(`[data-arcane-id="${id}"]`)
      ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  },
);

function matchesQuery(a: ArcaneListItem) {
  const q = props.query.trim().toLowerCase();
  if (!q) return true;
  const hay = `${a.nameEn} ${a.nameZh ?? ""} ${a.note ?? ""}`.toLowerCase();
  return hay.includes(q);
}

function matchesSource(a: ArcaneListItem) {
  if (props.sourceFilter === "all") return true;
  return a.sourceKind === props.sourceFilter;
}

function matchesKind(a: ArcaneListItem) {
  if (props.kindFilter === "all") return true;
  return a.arcaneKind === props.kindFilter;
}

function matchesRarity(a: ArcaneListItem) {
  if (props.rarityFilter === "all") return true;
  return a.rarity === props.rarityFilter;
}

function matchesProgress(a: ArcaneListItem) {
  switch (props.progressFilter) {
    case "missing":
      return a.status === 0;
    case "partial":
      return a.status === 1;
    case "maxed":
      return a.status === 2;
    default:
      return true;
  }
}

function arcaneName(a: ArcaneListItem) {
  return a.nameZh?.trim() || a.nameEn;
}

function compareArcanes(a: ArcaneListItem, b: ArcaneListItem) {
  const key = sortKey.value;
  if (!key) return 0;
  switch (key) {
    case "wish":
      return cmpNum(a.wish, b.wish);
    case "name":
      return cmpStr(arcaneName(a), arcaneName(b));
    case "source":
      return cmpNum(a.sourceKind, b.sourceKind);
    case "kind":
      return cmpNum(a.arcaneKind, b.arcaneKind);
    case "rarity":
      return cmpNum(a.rarity, b.rarity);
    case "status":
      return cmpNum(a.status, b.status);
    case "rank":
      return cmpNum(a.rank, b.rank);
    case "copies":
      return cmpNum(a.copiesToMax, b.copiesToMax);
    case "note":
      return cmpStr(a.note ?? "", b.note ?? "");
    default:
      return 0;
  }
}

const rows = computed(() => {
  let list: ArcaneListItem[];
  if (props.focusId) {
    list = props.arcanes.filter((a) => a.arcaneId === props.focusId);
  } else {
    list = props.arcanes.filter(
      (a) =>
        matchesQuery(a) &&
        matchesSource(a) &&
        matchesKind(a) &&
        matchesRarity(a) &&
        matchesProgress(a),
    );
  }
  if (!sortKey.value) return list;
  return [...list].sort((a, b) => {
    const c = compareArcanes(a, b);
    return c !== 0 ? c : a.arcaneId.localeCompare(b.arcaneId);
  });
});

const pendingOwnedCount = computed(
  () => rows.value.filter((a) => a.status !== 2).length,
);

async function onMarkAllOwned() {
  const n = pendingOwnedCount.value;
  if (n === 0) return;
  if (!(await confirm(`将当前列表 ${n} 项标为已获取？`))) return;
  emit(
    "markAllOwned",
    rows.value.map((a) => ({ arcaneId: a.arcaneId, status: a.status })),
  );
}

function displayName(a: ArcaneListItem) {
  return arcaneName(a);
}

function wikiUrl(a: ArcaneListItem) {
  return huijiArcaneWikiUrl(a.nameZh, a.nameEn);
}

async function openWiki(a: ArcaneListItem, e: Event) {
  e.stopPropagation();
  const url = wikiUrl(a);
  if (!url) return;
  try {
    await openUrl(url);
  } catch {
    /* ignore */
  }
}

function onToggleWish(a: ArcaneListItem, e: Event) {
  e.stopPropagation();
  emit("toggleWish", a.arcaneId, a.wish ? 0 : 1);
}

function onStatusChange(a: ArcaneListItem, e: Event) {
  e.stopPropagation();
  emit("updateStatus", a.arcaneId, Number((e.target as HTMLSelectElement).value));
}

function onRankChange(a: ArcaneListItem, e: Event) {
  e.stopPropagation();
  emit("updateRank", a.arcaneId, Number((e.target as HTMLSelectElement).value));
}

function onNoteBlur(a: ArcaneListItem) {
  const next = noteDrafts.value[a.arcaneId] ?? "";
  if (next === (a.note ?? "")) return;
  emit("updateNote", a.arcaneId, next);
}

function rankOptions(maxRank: number) {
  const n = Math.max(0, maxRank);
  return Array.from({ length: n + 1 }, (_, i) => i);
}

function copiesLabel(a: ArcaneListItem) {
  if (a.status === 2) return "满级";
  return `还差 ${a.copiesToMax}`;
}
</script>

<template>
  <div class="list-page">
    <div v-if="focusId" class="focus-banner">
      <span>来自心愿 · 仅显示这一项</span>
      <button type="button" class="focus-clear" @click="emit('clearFocus')">
        显示全部
      </button>
    </div>
    <div class="list-filters">
      <div class="toolbar">
        <div class="seg">
          <button
            v-for="o in ARCANE_SOURCE_FILTER_OPTS"
            :key="String(o.key)"
            type="button"
            :class="{ on: sourceFilter === o.key }"
            @click="emit('update:sourceFilter', o.key)"
          >
            {{ o.label }}
          </button>
        </div>
      </div>

      <div class="toolbar" style="padding-top: 0">
        <div class="seg">
          <button
            v-for="o in ARCANE_KIND_FILTER_OPTS"
            :key="String(o.key)"
            type="button"
            :class="{ on: kindFilter === o.key }"
            @click="emit('update:kindFilter', o.key)"
          >
            {{ o.label }}
          </button>
        </div>
      </div>

      <div class="toolbar" style="padding-top: 0">
        <div class="seg">
          <button
            v-for="o in ARCANE_RARITY_FILTER_OPTS"
            :key="String(o.key)"
            type="button"
            :class="{ on: rarityFilter === o.key }"
            @click="emit('update:rarityFilter', o.key)"
          >
            {{ o.label }}
          </button>
        </div>
      </div>

      <div class="toolbar" style="padding-top: 0">
        <div class="seg">
          <button
            v-for="o in ARCANE_PROGRESS_FILTER_OPTS"
            :key="o.key"
            type="button"
            :class="{ on: progressFilter === o.key }"
            @click="emit('update:progressFilter', o.key)"
          >
            {{ o.label }}
          </button>
        </div>
        <button
          class="ghost"
          type="button"
          :disabled="updating || markingAll || pendingOwnedCount === 0"
          @click="onMarkAllOwned"
        >
          {{ markingAll ? "处理中…" : "一键全有" }}
        </button>
      </div>
    </div>

    <div v-if="arcanes.length === 0" class="empty-state table-wrap">
      <p v-if="updating">正在更新数据…</p>
      <template v-else>
        <p>暂无赋能数据。可点「更新数据」从网络拉取。</p>
        <button class="ghost" type="button" @click="emit('updateData')">
          更新数据
        </button>
      </template>
    </div>

    <div v-else class="table-wrap">
      <table class="list">
        <thead>
          <tr>
            <th style="width: 52px">
              <button type="button" class="list-sort" @click="toggleSort('wish')">
                心愿<span v-if="sortMark('wish')" class="sort-mark">{{ sortMark("wish") }}</span>
              </button>
            </th>
            <th>
              <button type="button" class="list-sort" @click="toggleSort('name')">
                名称<span v-if="sortMark('name')" class="sort-mark">{{ sortMark("name") }}</span>
              </button>
            </th>
            <th style="width: 100px">
              <button type="button" class="list-sort" @click="toggleSort('source')">
                来源<span v-if="sortMark('source')" class="sort-mark">{{ sortMark("source") }}</span>
              </button>
            </th>
            <th style="width: 110px">
              <button type="button" class="list-sort" @click="toggleSort('kind')">
                品类<span v-if="sortMark('kind')" class="sort-mark">{{ sortMark("kind") }}</span>
              </button>
            </th>
            <th style="width: 64px">
              <button type="button" class="list-sort" @click="toggleSort('rarity')">
                品级<span v-if="sortMark('rarity')" class="sort-mark">{{ sortMark("rarity") }}</span>
              </button>
            </th>
            <th style="width: 100px">
              <button type="button" class="list-sort" @click="toggleSort('status')">
                状态<span v-if="sortMark('status')" class="sort-mark">{{ sortMark("status") }}</span>
              </button>
            </th>
            <th style="width: 72px">
              <button type="button" class="list-sort" @click="toggleSort('rank')">
                等级<span v-if="sortMark('rank')" class="sort-mark">{{ sortMark("rank") }}</span>
              </button>
            </th>
            <th style="width: 88px">
              <button type="button" class="list-sort" @click="toggleSort('copies')">
                还差<span v-if="sortMark('copies')" class="sort-mark">{{ sortMark("copies") }}</span>
              </button>
            </th>
            <th style="width: 160px">
              <button type="button" class="list-sort" @click="toggleSort('note')">
                备注<span v-if="sortMark('note')" class="sort-mark">{{ sortMark("note") }}</span>
              </button>
            </th>
          </tr>
      </thead>
      <tbody>
        <tr
          v-for="a in rows"
          :key="a.arcaneId"
          :data-arcane-id="a.arcaneId"
          :class="{ selected: a.arcaneId === selectedId }"
          @click="emit('select', a.arcaneId)"
        >
          <td @click.stop>
            <span
              class="check wish"
              :class="{ on: !!a.wish }"
              :title="a.wish ? '取消心愿' : '加入心愿'"
              @click="onToggleWish(a, $event)"
              >★</span
            >
          </td>
          <td
            class="name-cell arcane-wiki-name"
            title="打开灰机 Wiki"
            @click="openWiki(a, $event)"
          >
            <span class="en">{{ displayName(a) }}</span>
            <span
              v-if="a.nameZh && a.nameZh !== a.nameEn"
              class="zh"
              >{{ a.nameEn }}</span
            >
          </td>
          <td>
            <span class="note" style="max-width: none">{{
              arcaneSourceLabel(a.sourceKind)
            }}</span>
          </td>
          <td>
            <span class="note" style="max-width: none">{{
              arcaneKindLabel(a.arcaneKind)
            }}</span>
          </td>
          <td>
            <span class="note" style="max-width: none">{{
              arcaneRarityLabel(a.rarity)
            }}</span>
          </td>
          <td @click.stop>
            <select
              class="list-status"
              :value="a.status"
              @change="onStatusChange(a, $event)"
            >
              <option
                v-for="o in ARCANE_STATUS_OPTIONS"
                :key="o.value"
                :value="o.value"
              >
                {{ o.label }}
              </option>
            </select>
          </td>
          <td @click.stop>
            <select
              class="list-status"
              :value="a.rank"
              :disabled="a.status === 0"
              @change="onRankChange(a, $event)"
            >
              <option
                v-for="r in rankOptions(a.maxRank)"
                :key="r"
                :value="r"
              >
                {{ r }}
              </option>
            </select>
          </td>
          <td>
            <span class="note" style="max-width: none">{{ copiesLabel(a) }}</span>
          </td>
          <td @click.stop>
            <input
              class="list-note"
              type="text"
              :value="noteDrafts[a.arcaneId] ?? ''"
              placeholder="备注"
              @input="
                noteDrafts[a.arcaneId] = ($event.target as HTMLInputElement)
                  .value
              "
              @blur="onNoteBlur(a)"
            />
          </td>
        </tr>
        <tr v-if="rows.length === 0">
          <td colspan="9" style="padding: 24px; color: var(--text-mute)">
            无匹配
          </td>
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
.focus-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
  padding: 8px 12px;
  border-radius: 8px;
  background: var(--accent-soft);
  color: var(--text);
  font-size: 13px;
}
.focus-clear {
  appearance: none;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--bg-elevated);
  color: var(--accent);
  font: inherit;
  font-size: 12px;
  padding: 4px 10px;
  cursor: pointer;
}
.focus-clear:hover {
  border-color: var(--accent);
}
.arcane-wiki-name {
  cursor: pointer;
}
.arcane-wiki-name:hover .en {
  color: var(--cyan);
  text-decoration: underline;
}
</style>
