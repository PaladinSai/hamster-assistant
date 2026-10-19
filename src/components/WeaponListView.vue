<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import { computed } from "vue";
import type { WeaponFamilyListItem, WeaponMemberDto } from "../lib/api";
import {
  INCARNON_OPTIONS,
  WEAPON_SPECIAL_STATUS_OPTIONS,
  WEAPON_STATUS_OPTIONS,
  isSpecialWeaponVariant,
  modularWeaponKindLabel,
  vaultStatusLabel,
  weaponDisplayName,
  weaponVariantLabel,
  type WeaponIncarnonFilter,
  type WeaponKindFilter,
  type WeaponProgressFilter,
  type WeaponVaultFilter,
} from "../lib/status";
import { huijiWeaponWikiUrl } from "../lib/wiki/huiji";
import { useColumnSort } from "../composables/useColumnSort";
import { useConfirm } from "../composables/useConfirm";
import { useNoteDrafts } from "../composables/useNoteDrafts";
import ConfirmDialog from "./ConfirmDialog.vue";
import StatusPill from "./StatusPill.vue";
import WeaponListToolbar from "./WeaponListToolbar.vue";

type WeaponSortKey =
  | "wish"
  | "name"
  | "variant"
  | "status"
  | "mastered"
  | "vault"
  | "incarnon"
  | "note";

const { sortKey, toggleSort, sortMark, cmpStr, cmpNum } =
  useColumnSort<WeaponSortKey>();

const {
  open: confirmOpen,
  title: confirmTitle,
  message: confirmMessage,
  confirm,
  onConfirm,
  onCancel,
} = useConfirm();

type WeaponRow = {
  family: WeaponFamilyListItem;
  member: WeaponMemberDto;
};

const props = withDefaults(
  defineProps<{
    mode?: "weapon" | "archwing";
    families: WeaponFamilyListItem[];
    slot: number;
    kindFilter: WeaponKindFilter;
    progressFilter: WeaponProgressFilter;
    vaultFilter: WeaponVaultFilter;
    incarnonFilter: WeaponIncarnonFilter;
    query: string;
    selectedFamilyId: string | null;
    selectedWeaponId: string | null;
    updating: boolean;
    markingAll?: boolean;
  }>(),
  { mode: "weapon" },
);

const emit = defineEmits<{
  "update:slot": [value: number];
  "update:kindFilter": [value: WeaponKindFilter];
  "update:progressFilter": [value: WeaponProgressFilter];
  "update:vaultFilter": [value: WeaponVaultFilter];
  "update:incarnonFilter": [value: WeaponIncarnonFilter];
  selectWeapon: [familyId: string, weaponId: string];
  toggleWish: [weaponId: string, wish: number];
  toggleMastered: [weaponId: string, mastered: number];
  updateWeaponStatus: [weaponId: string, status: number];
  updateWeaponIncarnon: [weaponId: string, incarnon: number];
  updateWeaponNote: [weaponId: string, note: string];
  markAllOwned: [items: { weaponId: string; status: number }[]];
  updateData: [];
}>();

const noteDrafts = useNoteDrafts(
  () => props.families.flatMap((f) => f.members),
  (m) => m.weaponId,
  (m) => m.note,
  { deep: true },
);

function matchesQuery(f: WeaponFamilyListItem, m: WeaponMemberDto) {
  const q = props.query.trim().toLowerCase();
  if (!q) return true;
  const hay =
    `${f.nameEn} ${f.nameZh ?? ""} ${f.nickname ?? ""} ${f.familyKey} ${m.nameEn} ${m.nameZh ?? ""}`.toLowerCase();
  return hay.includes(q);
}

function isModularFamily(f: WeaponFamilyListItem) {
  return f.slot === 3 || !!f.modularKind;
}

function matchesKind(f: WeaponFamilyListItem, m: WeaponMemberDto) {
  switch (props.kindFilter) {
    case "kitgun":
      return f.modularKind === "kitgun";
    case "zaw":
      return f.modularKind === "zaw";
    case "amp":
      return f.modularKind === "amp";
    case "kuva":
      return !isModularFamily(f) && m.variant === 2;
    case "tenet":
      return !isModularFamily(f) && m.variant === 3;
    case "coda":
      return !isModularFamily(f) && m.variant === 4;
    case "prime":
      return !isModularFamily(f) && m.variant === 1;
    case "incarnon":
      return !isModularFamily(f) && m.canIncarnon === 1;
    default:
      return true;
  }
}

const isModularView = computed(
  () => props.mode === "weapon" && props.slot === 3,
);

function matchesProgress(m: WeaponMemberDto) {
  switch (props.progressFilter) {
    case "missing":
      return m.status === 0;
    case "progress":
      return m.status === 2;
    case "owned":
      return m.status === 1;
    default:
      return true;
  }
}

function matchesVault(m: WeaponMemberDto) {
  switch (props.vaultFilter) {
    case "unvaulted":
      return m.vaultStatus === 1;
    case "vaulted":
      return m.vaultStatus === 2;
    case "resurgence":
      return m.vaultStatus === 3;
    default:
      return true;
  }
}

function matchesIncarnon(m: WeaponMemberDto) {
  switch (props.incarnonFilter) {
    case "missing":
      return m.incarnon === 0;
    case "ready":
      return m.incarnon === 2;
    case "done":
      return m.incarnon === 1;
    default:
      return true;
  }
}

function matchesThird(m: WeaponMemberDto) {
  if (props.kindFilter === "incarnon") {
    return matchesIncarnon(m);
  }
  if (props.kindFilter === "prime") {
    return matchesProgress(m) && matchesVault(m);
  }
  return matchesProgress(m);
}

function weaponRowName(row: WeaponRow) {
  return weaponDisplayName(
    row.member.nameZh ?? row.family.nameZh,
    row.member.nameEn || row.family.nameEn,
  );
}

function compareWeaponRows(a: WeaponRow, b: WeaponRow) {
  const key = sortKey.value;
  if (!key) return 0;
  switch (key) {
    case "wish":
      return cmpNum(a.member.wish, b.member.wish);
    case "name":
      return cmpStr(weaponRowName(a), weaponRowName(b));
    case "variant":
      return cmpNum(a.member.variant, b.member.variant);
    case "status":
      return cmpNum(a.member.status, b.member.status);
    case "mastered":
      return cmpNum(a.member.mastered, b.member.mastered);
    case "vault":
      return cmpNum(a.member.vaultStatus, b.member.vaultStatus);
    case "incarnon":
      return cmpNum(a.member.incarnon, b.member.incarnon);
    case "note":
      return cmpStr(a.member.note ?? "", b.member.note ?? "");
    default:
      return 0;
  }
}

function defaultWeaponOrder(a: WeaponRow, b: WeaponRow) {
  const fk = a.family.familyKey.localeCompare(b.family.familyKey);
  if (fk !== 0) return fk;
  if (a.member.variant !== b.member.variant) {
    return a.member.variant - b.member.variant;
  }
  return a.member.nameEn.localeCompare(b.member.nameEn);
}

const rows = computed(() => {
  const out: WeaponRow[] = [];
  for (const f of props.families) {
    if (props.slot !== -1 && f.slot !== props.slot) continue;
    // 「全部」槽不混入自制精通部件；自制单独看
    if (props.slot === -1 && isModularFamily(f)) continue;
    for (const m of f.members) {
      if (!matchesQuery(f, m)) continue;
      if (!matchesKind(f, m)) continue;
      if (!matchesThird(m)) continue;
      out.push({ family: f, member: m });
    }
  }
  out.sort((a, b) => {
    if (sortKey.value) {
      const c = compareWeaponRows(a, b);
      if (c !== 0) return c;
    }
    return defaultWeaponOrder(a, b);
  });
  return out;
});

const pendingOwnedCount = computed(
  () => rows.value.filter((r) => r.member.status !== 1).length,
);

async function onMarkAllOwned() {
  const n = pendingOwnedCount.value;
  if (n === 0) return;
  if (!(await confirm(`将当前列表 ${n} 项标为已获取？`))) return;
  emit(
    "markAllOwned",
    rows.value.map((r) => ({
      weaponId: r.member.weaponId,
      status: r.member.status,
    })),
  );
}

function canOpenDrawer(m: WeaponMemberDto) {
  return (m.partCount ?? 0) > 1;
}

function displayZh(f: WeaponFamilyListItem, m: WeaponMemberDto) {
  return weaponDisplayName(m.nameZh ?? f.nameZh, m.nameEn || f.nameEn);
}

function displayEn(f: WeaponFamilyListItem, m: WeaponMemberDto) {
  return (m.nameEn || f.nameEn || "").trim();
}

function weaponWikiUrl(f: WeaponFamilyListItem, m: WeaponMemberDto) {
  return huijiWeaponWikiUrl(m.nameZh ?? f.nameZh, m.nameEn || f.nameEn);
}

async function openWiki(
  f: WeaponFamilyListItem,
  m: WeaponMemberDto,
  e: Event,
) {
  e.stopPropagation();
  const url = weaponWikiUrl(f, m);
  if (!url) return;
  try {
    await openUrl(url);
  } catch {
    /* ignore */
  }
}

function nick(f: WeaponFamilyListItem) {
  return f.nickname || "";
}

function statusOptions(f: WeaponFamilyListItem, m: WeaponMemberDto) {
  if (isModularFamily(f) || isSpecialWeaponVariant(m.variant)) {
    return WEAPON_SPECIAL_STATUS_OPTIONS;
  }
  return WEAPON_STATUS_OPTIONS;
}

function variantClass(variant: number) {
  switch (variant) {
    case 1:
      return "var-prime";
    case 2:
      return "var-kuva";
    case 3:
      return "var-tenet";
    case 4:
      return "var-coda";
    default:
      return "var-base";
  }
}

function typePill(f: WeaponFamilyListItem, m: WeaponMemberDto) {
  if (isModularFamily(f)) {
    return modularWeaponKindLabel(f.modularKind);
  }
  return weaponVariantLabel(m.variant);
}

function typePillClass(f: WeaponFamilyListItem, m: WeaponMemberDto) {
  if (isModularFamily(f)) {
    switch (f.modularKind) {
      case "kitgun":
        return "var-kuva";
      case "zaw":
        return "var-tenet";
      case "amp":
        return "var-coda";
      default:
        return "var-base";
    }
  }
  return variantClass(m.variant);
}

/** 与战甲一致：点行开抽屉；控件用 data-toggle 排除。多部件（>1）才开。 */
function onRowClick(row: WeaponRow, e: MouseEvent) {
  if (!canOpenDrawer(row.member)) return;
  const t = e.target as HTMLElement;
  if (t.closest("[data-toggle]")) return;
  emit("selectWeapon", row.family.familyId, row.member.weaponId);
}

function onToggleWish(m: WeaponMemberDto, e: MouseEvent) {
  e.stopPropagation();
  emit("toggleWish", m.weaponId, m.wish ? 0 : 1);
}

function onToggleMastered(m: WeaponMemberDto, e: MouseEvent) {
  e.stopPropagation();
  emit("toggleMastered", m.weaponId, m.mastered ? 0 : 1);
}

function onStatusChange(m: WeaponMemberDto, e: Event) {
  e.stopPropagation();
  const status = Number((e.target as HTMLSelectElement).value);
  emit("updateWeaponStatus", m.weaponId, status);
}

function onIncarnonChange(m: WeaponMemberDto, e: Event) {
  e.stopPropagation();
  const incarnon = Number((e.target as HTMLSelectElement).value);
  emit("updateWeaponIncarnon", m.weaponId, incarnon);
}

function onNoteBlur(m: WeaponMemberDto) {
  const next = noteDrafts.value[m.weaponId] ?? "";
  const prev = m.note ?? "";
  if (next !== prev) emit("updateWeaponNote", m.weaponId, next);
}
</script>

<template>
  <div class="list-page">
    <div class="list-filters">
      <WeaponListToolbar
        :mode="mode"
        :weapon-slot="slot"
        :kind-filter="kindFilter"
        :progress-filter="progressFilter"
        :vault-filter="vaultFilter"
        :incarnon-filter="incarnonFilter"
        :row-count="rows.length"
        :updating="updating"
        :marking-all="markingAll"
        :pending-owned-count="pendingOwnedCount"
        @update:weapon-slot="emit('update:slot', $event)"
        @update:kind-filter="emit('update:kindFilter', $event)"
        @update:progress-filter="emit('update:progressFilter', $event)"
        @update:vault-filter="emit('update:vaultFilter', $event)"
        @update:incarnon-filter="emit('update:incarnonFilter', $event)"
        @mark-all-owned="onMarkAllOwned"
      />
    </div>

    <div v-if="families.length === 0" class="empty-state table-wrap">
      <p v-if="updating">正在更新数据…</p>
      <template v-else>
        <p>
          {{
            mode === "archwing"
              ? "暂无空战武器数据。可点「更新数据」从网络拉取。"
              : "暂无武器数据。可点「更新数据」从网络拉取。"
          }}
        </p>
        <button class="ghost" type="button" @click="emit('updateData')">
          更新数据
        </button>
      </template>
    </div>

    <div v-else class="table-wrap">
      <table class="list wep-list">
        <thead>
          <tr>
            <th style="width: 52px">
              <button type="button" class="list-sort" @click="toggleSort('wish')">
                心愿<span v-if="sortMark('wish')" class="sort-mark">{{ sortMark("wish") }}</span>
              </button>
            </th>
            <th style="width: 160px">
              <button type="button" class="list-sort" @click="toggleSort('name')">
                武器<span v-if="sortMark('name')" class="sort-mark">{{ sortMark("name") }}</span>
              </button>
            </th>
            <th style="width: 88px">
              <button type="button" class="list-sort" @click="toggleSort('variant')">
                {{ isModularView ? "类型" : "变体"
                }}<span v-if="sortMark('variant')" class="sort-mark">{{
                  sortMark("variant")
                }}</span>
              </button>
            </th>
            <th style="width: 120px">
              <button type="button" class="list-sort" @click="toggleSort('status')">
                状态<span v-if="sortMark('status')" class="sort-mark">{{ sortMark("status") }}</span>
              </button>
            </th>
            <th style="width: 72px">
              <button type="button" class="list-sort" @click="toggleSort('mastered')">
                已掌握<span v-if="sortMark('mastered')" class="sort-mark">{{ sortMark("mastered") }}</span>
              </button>
            </th>
            <th v-if="!isModularView" style="width: 88px">
              <button type="button" class="list-sort" @click="toggleSort('vault')">
                入库<span v-if="sortMark('vault')" class="sort-mark">{{ sortMark("vault") }}</span>
              </button>
            </th>
            <th v-if="!isModularView" style="width: 140px">
              <button type="button" class="list-sort" @click="toggleSort('incarnon')">
                灵化<span v-if="sortMark('incarnon')" class="sort-mark">{{ sortMark("incarnon") }}</span>
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
          v-for="row in rows"
          :key="row.member.weaponId"
          class="wep-row"
          :class="{
            selected:
              canOpenDrawer(row.member) &&
              row.member.weaponId === selectedWeaponId,
            clickable: canOpenDrawer(row.member),
          }"
          @click="onRowClick(row, $event)"
        >
          <td>
            <span
              class="check wish"
              :class="{ on: !!row.member.wish }"
              data-toggle="wish"
              :title="row.member.wish ? '取消心愿' : '加入心愿'"
              @click="onToggleWish(row.member, $event)"
              >★</span
            >
          </td>
          <td
            class="name-cell wep-name wep-wiki-name"
            title="打开灰机 Wiki"
            @click="openWiki(row.family, row.member, $event)"
          >
            <span class="en">{{ displayZh(row.family, row.member) }}</span>
            <span
              v-if="
                displayEn(row.family, row.member) &&
                displayEn(row.family, row.member) !==
                  displayZh(row.family, row.member)
              "
              class="zh"
              >{{ displayEn(row.family, row.member) }}</span
            >
            <span v-else-if="nick(row.family)" class="zh">{{
              nick(row.family)
            }}</span>
          </td>
          <td>
            <span
              class="pill"
              :class="typePillClass(row.family, row.member)"
              >{{ typePill(row.family, row.member) }}</span
            >
          </td>
          <td>
            <select
              class="list-status"
              data-toggle="status"
              :value="row.member.status"
              @click.stop
              @change="onStatusChange(row.member, $event)"
            >
              <option
                v-for="o in statusOptions(row.family, row.member)"
                :key="o.value"
                :value="o.value"
              >
                {{ o.label }}
              </option>
            </select>
          </td>
          <td>
            <span
              class="check"
              :class="{ on: !!row.member.mastered }"
              data-toggle="mastered"
              :title="row.member.mastered ? '取消已掌握' : '标为已掌握'"
              @click="onToggleMastered(row.member, $event)"
              >✓</span
            >
          </td>
          <td v-if="!isModularView">
            <StatusPill
              :label="
                vaultStatusLabel(row.member.vaultStatus, row.member.variant === 1)
              "
            />
          </td>
          <td v-if="!isModularView">
            <select
              v-if="row.member.canIncarnon === 1"
              class="list-status"
              data-toggle="incarnon"
              :value="row.member.incarnon"
              @click.stop
              @change="onIncarnonChange(row.member, $event)"
            >
              <option
                v-for="o in INCARNON_OPTIONS"
                :key="o.value"
                :value="o.value"
              >
                {{ o.label }}
              </option>
            </select>
            <span v-else style="color: var(--text-mute)">—</span>
          </td>
          <td>
            <input
              class="list-note"
              type="text"
              data-toggle="note"
              :value="noteDrafts[row.member.weaponId] ?? ''"
              placeholder="备注"
              @click.stop
              @input="
                noteDrafts[row.member.weaponId] = (
                  $event.target as HTMLInputElement
                ).value
              "
              @blur="onNoteBlur(row.member)"
            />
          </td>
        </tr>
        <tr v-if="rows.length === 0">
          <td colspan="8" style="padding: 24px; color: var(--text-mute)">
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
.wep-row.clickable {
  cursor: pointer;
}
.wep-row:not(.clickable) {
  cursor: default;
}
.wep-name {
  max-width: 160px;
}
.wep-wiki-name {
  cursor: pointer;
}
.wep-wiki-name:hover .en {
  color: var(--cyan);
  text-decoration: underline;
}
.wep-name .en,
.wep-name .zh {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.list-note {
  width: 100%;
  min-width: 96px;
  max-width: 220px;
  background: var(--bg-elevated);
  border: 1px solid var(--line);
  color: var(--text);
  border-radius: 6px;
  padding: 4px 8px;
  font-size: 12px;
}
.list-note:focus {
  outline: none;
  border-color: var(--cyan);
}
</style>
