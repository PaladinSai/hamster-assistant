<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import type { ModListItem, ModTagDto } from "../lib/api";
import {
  MOD_KIND_FILTER_OPTS,
  MOD_PROGRESS_FILTER_OPTS,
  MOD_STATUS_OPTIONS,
  modKindLabel,
  type ModKindFilter,
  type ModProgressFilter,
  type ModTagFilter,
} from "../lib/status";
import { useColumnSort } from "../composables/useColumnSort";
import { useConfirm } from "../composables/useConfirm";
import { useNoteDrafts } from "../composables/useNoteDrafts";
import ConfirmDialog from "./ConfirmDialog.vue";

type ModSortKey = "wish" | "name" | "tag" | "kind" | "status" | "note";

const { sortKey, toggleSort, sortMark, cmpStr, cmpNum } =
  useColumnSort<ModSortKey>();

const {
  open: confirmOpen,
  title: confirmTitle,
  message: confirmMessage,
  confirm,
  onConfirm,
  onCancel,
} = useConfirm();

const props = defineProps<{
  mods: ModListItem[];
  tags: ModTagDto[];
  tagFilter: ModTagFilter;
  kindFilter: ModKindFilter;
  progressFilter: ModProgressFilter;
  query: string;
  selectedId: string | null;
  updating: boolean;
  markingAll?: boolean;
}>();

const emit = defineEmits<{
  "update:tagFilter": [value: ModTagFilter];
  "update:kindFilter": [value: ModKindFilter];
  "update:progressFilter": [value: ModProgressFilter];
  toggleWish: [modId: string, wish: number];
  updateStatus: [modId: string, status: number];
  updateNote: [modId: string, note: string];
  updateTag: [modId: string, tagId: string | null];
  createTag: [name: string];
  deleteTag: [tagId: string];
  select: [modId: string];
  markAllOwned: [items: { modId: string; status: number }[]];
  updateData: [];
}>();

const noteDrafts = useNoteDrafts(
  () => props.mods,
  (m) => m.modId,
  (m) => m.note,
);

const addingTag = ref(false);
const newTagName = ref("");

const menuOpen = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const menuTag = ref<ModTagDto | null>(null);

function closeTagMenu() {
  menuOpen.value = false;
  menuTag.value = null;
}

function onTagContextMenu(t: ModTagDto, e: MouseEvent) {
  if (t.isBuiltin) return;
  menuTag.value = t;
  menuX.value = e.clientX;
  menuY.value = e.clientY;
  menuOpen.value = true;
}

async function onMenuDelete() {
  const t = menuTag.value;
  closeTagMenu();
  if (!t) return;
  const ok = await confirm(
    `删除标签「${t.name}」？使用该标签的 Mod 将变为无标签。`,
  );
  if (!ok) return;
  emit("deleteTag", t.id);
}

function onDocKeydown(e: KeyboardEvent) {
  if (!menuOpen.value) return;
  if (e.key === "Escape") {
    e.preventDefault();
    closeTagMenu();
  }
}

onMounted(() => {
  document.addEventListener("keydown", onDocKeydown);
});
onUnmounted(() => {
  document.removeEventListener("keydown", onDocKeydown);
});

watch(
  () => props.selectedId,
  async (id) => {
    if (!id) return;
    await nextTick();
    const el = document.querySelector(`[data-mod-id="${id}"]`);
    el?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  },
);

function matchesQuery(m: ModListItem) {
  const q = props.query.trim().toLowerCase();
  if (!q) return true;
  const hay =
    `${m.nameEn} ${m.nameZh ?? ""} ${m.note ?? ""} ${m.polarity ?? ""} ${m.tagName ?? ""}`.toLowerCase();
  return hay.includes(q);
}

function matchesTag(m: ModListItem) {
  if (props.tagFilter === "all") return true;
  if (props.tagFilter === "none") return !m.tagId;
  return m.tagId === props.tagFilter;
}

function matchesKind(m: ModListItem) {
  if (props.kindFilter === "all") return true;
  return m.modKind === props.kindFilter;
}

function matchesProgress(m: ModListItem) {
  switch (props.progressFilter) {
    case "missing":
      return m.status === 0;
    case "owned":
      return m.status === 1;
    default:
      return true;
  }
}

function modName(m: ModListItem) {
  return m.nameZh?.trim() || m.nameEn;
}

function compareMods(a: ModListItem, b: ModListItem) {
  const key = sortKey.value;
  if (!key) return 0;
  switch (key) {
    case "wish":
      return cmpNum(a.wish, b.wish);
    case "name":
      return cmpStr(modName(a), modName(b));
    case "tag":
      return cmpStr(a.tagName ?? "", b.tagName ?? "");
    case "kind":
      return cmpNum(a.modKind, b.modKind);
    case "status":
      return cmpNum(a.status, b.status);
    case "note":
      return cmpStr(a.note ?? "", b.note ?? "");
    default:
      return 0;
  }
}

const rows = computed(() => {
  const list = props.mods.filter(
    (m) =>
      matchesQuery(m) &&
      matchesTag(m) &&
      matchesKind(m) &&
      matchesProgress(m),
  );
  if (!sortKey.value) return list;
  return [...list].sort((a, b) => {
    const c = compareMods(a, b);
    return c !== 0 ? c : a.modId.localeCompare(b.modId);
  });
});

const pendingOwnedCount = computed(
  () => rows.value.filter((m) => m.status !== 1).length,
);

async function onMarkAllOwned() {
  const n = pendingOwnedCount.value;
  if (n === 0) return;
  if (!(await confirm(`将当前列表 ${n} 项标为已获取？`))) return;
  emit(
    "markAllOwned",
    rows.value.map((m) => ({ modId: m.modId, status: m.status })),
  );
}

function displayName(m: ModListItem) {
  return modName(m);
}

function modWikiUrl(m: ModListItem) {
  const page = (m.nameZh?.trim() || m.nameEn.trim()).replace(/ /g, "_");
  if (!page) return "";
  return `https://warframe.huijiwiki.com/wiki/${encodeURIComponent(page)}`;
}

async function openWiki(m: ModListItem, e: Event) {
  e.stopPropagation();
  const url = modWikiUrl(m);
  if (!url) return;
  try {
    await openUrl(url);
  } catch {
    /* ignore */
  }
}

function onToggleWish(m: ModListItem, e: Event) {
  e.stopPropagation();
  emit("toggleWish", m.modId, m.wish ? 0 : 1);
}

function onStatusChange(m: ModListItem, e: Event) {
  e.stopPropagation();
  const status = Number((e.target as HTMLSelectElement).value);
  emit("updateStatus", m.modId, status);
}

function onTagChange(m: ModListItem, e: Event) {
  e.stopPropagation();
  const raw = (e.target as HTMLSelectElement).value;
  emit("updateTag", m.modId, raw === "" ? null : raw);
}

function onNoteBlur(m: ModListItem) {
  const next = noteDrafts.value[m.modId] ?? "";
  const prev = m.note ?? "";
  if (next === prev) return;
  emit("updateNote", m.modId, next);
}

function startAddTag() {
  addingTag.value = true;
  newTagName.value = "";
}

function cancelAddTag() {
  addingTag.value = false;
  newTagName.value = "";
}

function confirmAddTag() {
  const name = newTagName.value.trim();
  if (!name) return;
  emit("createTag", name);
  addingTag.value = false;
  newTagName.value = "";
}
</script>

<template>
  <div class="list-page">
    <div class="list-filters">
      <div class="toolbar">
        <div class="seg">
          <button
            type="button"
            :class="{ on: tagFilter === 'all' }"
            @click="emit('update:tagFilter', 'all')"
          >
            全部
          </button>
          <button
            type="button"
            :class="{ on: tagFilter === 'none' }"
            @click="emit('update:tagFilter', 'none')"
          >
            无标签
          </button>
          <button
            v-for="t in tags"
            :key="t.id"
            type="button"
            :class="{ on: tagFilter === t.id }"
            :title="t.isBuiltin ? undefined : '右键菜单'"
            @click="emit('update:tagFilter', t.id)"
            @contextmenu.prevent="onTagContextMenu(t, $event)"
          >
            {{ t.name }}
          </button>
          <button
            v-if="!addingTag"
            type="button"
            class="add-tag"
            title="新增标签"
            @click="startAddTag"
          >
            +
          </button>
          <span v-else class="add-tag-form" @click.stop>
            <input
              v-model="newTagName"
              class="add-tag-input"
              type="text"
              maxlength="32"
              placeholder="标签名"
              @keydown.enter.prevent="confirmAddTag"
              @keydown.escape.prevent="cancelAddTag"
            />
            <button type="button" class="add-tag" @click="confirmAddTag">确定</button>
            <button type="button" class="add-tag" @click="cancelAddTag">取消</button>
          </span>
        </div>
      </div>

      <div class="toolbar" style="padding-top: 0">
        <div class="seg">
          <button
            v-for="o in MOD_KIND_FILTER_OPTS"
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
            v-for="o in MOD_PROGRESS_FILTER_OPTS"
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

    <div v-if="mods.length === 0" class="empty-state table-wrap">
      <p v-if="updating">正在更新数据…</p>
      <template v-else>
        <p>暂无 Mod 数据。可点「更新数据」从网络拉取。</p>
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
          <th style="width: 120px">
            <button type="button" class="list-sort" @click="toggleSort('tag')">
              标签<span v-if="sortMark('tag')" class="sort-mark">{{ sortMark("tag") }}</span>
            </button>
          </th>
          <th style="width: 90px">
            <button type="button" class="list-sort" @click="toggleSort('kind')">
              品类<span v-if="sortMark('kind')" class="sort-mark">{{ sortMark("kind") }}</span>
            </button>
          </th>
          <th style="width: 100px">
            <button type="button" class="list-sort" @click="toggleSort('status')">
              状态<span v-if="sortMark('status')" class="sort-mark">{{ sortMark("status") }}</span>
            </button>
          </th>
          <th style="width: 180px">
            <button type="button" class="list-sort" @click="toggleSort('note')">
              备注<span v-if="sortMark('note')" class="sort-mark">{{ sortMark("note") }}</span>
            </button>
          </th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="m in rows"
          :key="m.modId"
          :data-mod-id="m.modId"
          :class="{ selected: m.modId === selectedId }"
          @click="emit('select', m.modId)"
        >
          <td @click.stop>
            <span
              class="check wish"
              :class="{ on: !!m.wish }"
              :title="m.wish ? '取消心愿' : '加入心愿'"
              @click="onToggleWish(m, $event)"
              >★</span
            >
          </td>
          <td
            class="name-cell mod-wiki-name"
            title="打开灰机 Wiki"
            @click="openWiki(m, $event)"
          >
            <span class="en">{{ displayName(m) }}</span>
            <span
              v-if="m.nameZh && m.nameZh !== m.nameEn"
              class="zh"
              >{{ m.nameEn }}</span
            >
          </td>
          <td @click.stop>
            <select
              class="list-status"
              :value="m.tagId ?? ''"
              @change="onTagChange(m, $event)"
            >
              <option value="">无</option>
              <option v-for="t in tags" :key="t.id" :value="t.id">
                {{ t.name }}
              </option>
            </select>
          </td>
          <td>
            <span class="note" style="max-width: none">{{
              modKindLabel(m.modKind)
            }}</span>
          </td>
          <td @click.stop>
            <select
              class="list-status"
              :value="m.status"
              @change="onStatusChange(m, $event)"
            >
              <option
                v-for="o in MOD_STATUS_OPTIONS"
                :key="o.value"
                :value="o.value"
              >
                {{ o.label }}
              </option>
            </select>
          </td>
          <td @click.stop>
            <input
              class="list-note"
              type="text"
              :value="noteDrafts[m.modId] ?? ''"
              placeholder="备注"
              @input="
                noteDrafts[m.modId] = ($event.target as HTMLInputElement).value
              "
              @blur="onNoteBlur(m)"
            />
          </td>
        </tr>
        <tr v-if="rows.length === 0">
          <td colspan="6" style="padding: 24px; color: var(--text-mute)">
            无匹配
          </td>
        </tr>
      </tbody>
    </table>
    </div>
  </div>

  <Teleport to="body">
    <div
      v-if="menuOpen"
      class="tag-menu-root"
      @mousedown.self="closeTagMenu"
      @contextmenu.prevent="closeTagMenu"
    >
      <div
        class="tag-menu"
        role="menu"
        :style="{ left: `${menuX}px`, top: `${menuY}px` }"
        @mousedown.stop
      >
        <button
          type="button"
          class="tag-menu-item danger"
          role="menuitem"
          @click="onMenuDelete"
        >
          删除
        </button>
      </div>
    </div>
  </Teleport>

  <ConfirmDialog
    :open="confirmOpen"
    :title="confirmTitle"
    :message="confirmMessage"
    @confirm="onConfirm"
    @cancel="onCancel"
  />
</template>

<style scoped>
.mod-wiki-name {
  cursor: pointer;
}
.mod-wiki-name:hover .en {
  color: var(--cyan);
  text-decoration: underline;
}
.add-tag {
  min-width: 28px;
  font-weight: 600;
}
.add-tag-form {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.add-tag-input {
  width: 88px;
  font: inherit;
  font-size: 12px;
  padding: 4px 6px;
  border: 1px solid var(--line);
  border-radius: 4px;
  background: var(--bg-elevated);
  color: var(--text);
}
.list-note {
  width: 100%;
  max-width: 180px;
  font: inherit;
  font-size: 12px;
  color: var(--text-mute);
  background: transparent;
  border: 1px solid transparent;
  border-radius: 4px;
  padding: 4px 6px;
}
.list-note:hover,
.list-note:focus {
  border-color: var(--line);
  background: var(--bg-elevated);
  color: var(--text);
  outline: none;
}
</style>

<style>
.tag-menu-root {
  position: fixed;
  inset: 0;
  z-index: 70;
}
.tag-menu {
  position: fixed;
  min-width: 112px;
  padding: 4px;
  border-radius: 8px;
  background: var(--bg-elevated, #18212c);
  border: 1px solid var(--line, #2a3645);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}
.tag-menu-item {
  appearance: none;
  display: block;
  width: 100%;
  text-align: left;
  border: none;
  background: transparent;
  color: var(--text, #e8eef5);
  border-radius: 6px;
  padding: 7px 10px;
  font-size: 12px;
  cursor: pointer;
}
.tag-menu-item:hover {
  background: var(--bg-row-hover, #1c2734);
}
.tag-menu-item.danger {
  color: var(--bad, #d47878);
}
</style>
