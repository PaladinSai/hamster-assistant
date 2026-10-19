<script setup lang="ts">
import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  ref,
  toRef,
  watch,
} from "vue";
import {
  createRoutine,
  deleteRoutine,
  reorderRoutines,
  updateRoutine,
  type RoutineItemDto,
  type RoutineKind,
  type WarframeListItem,
  type WeaponFamilyListItem,
} from "../lib/api";
import { useExpandedSet } from "../composables/useExpandedSet";
import { useLiveDailyCache } from "../composables/useLiveCache";
import { useRoutinesList } from "../composables/useRoutinesList";
import { buildLiveDetail } from "./routine/buildLiveDetail";
import type { LiveDetail } from "./routine/liveTypes";
import RoutineLivePanel from "./RoutineLivePanel.vue";

export type RoutinePanelMode = "daily" | "weekly";

const props = withDefaults(
  defineProps<{
    mode: RoutinePanelMode;
    families?: WarframeListItem[];
    weaponFamilies?: WeaponFamilyListItem[];
    refreshToken?: number;
    /** 当前是否在首页（倒计时用）。 */
    active?: boolean;
  }>(),
  { families: () => [], weaponFamilies: () => [] },
);

const { items, ensureLoaded: ensureRoutines } = useRoutinesList();
const { isExpanded, toggleExpand } = useExpandedSet();
const error = ref("");
const showIgnored = ref(false);
const addingKind = ref<RoutineKind | null>(null);
const newTitle = ref("");
const addInputEl = ref<HTMLInputElement | null>(null);
const editingId = ref<string | null>(null);
const editDraft = ref("");
const dragId = ref<string | null>(null);
const dragOverId = ref<string | null>(null);
let reorderBusy = false;
let pointerDragging = false;

const refreshTokenRef = toRef(props, "refreshToken");
const {
  brief: liveBrief,
  ensureLoaded: ensureLive,
  reload: reloadLive,
} = useLiveDailyCache(refreshTokenRef);

let endedReloadTimer: ReturnType<typeof setTimeout> | undefined;

function isNightwaveTitle(title: string) {
  return title.startsWith("午夜电波");
}

async function load() {
  try {
    await ensureRoutines();
    error.value = "";
  } catch (e) {
    error.value = String(e);
  }
}

/** 倒计时到期后防抖重读本地 brief（不拉网）。 */
function onLiveEnded() {
  if (endedReloadTimer != null) clearTimeout(endedReloadTimer);
  endedReloadTimer = setTimeout(() => {
    endedReloadTimer = undefined;
    void reloadLive();
  }, 800);
}

onMounted(() => {
  void load();
  void ensureLive();
});

onUnmounted(() => {
  if (endedReloadTimer != null) clearTimeout(endedReloadTimer);
});

// 切回日常/周常时重读本地 brief；refreshToken 仍由 backend 事件驱动
watch(
  () => props.active,
  (active) => {
    if (active !== false) void reloadLive();
  },
);

// refreshToken 只驱动 live brief（useLiveDailyCache）；例程表不随 worldState 刷新

const panelTitle = computed(() =>
  props.mode === "daily" ? "日常" : "周常",
);

const panelHint = computed(() => {
  if (props.mode === "daily") {
    return "突击加拿大东部正午轮换（UTC 记录）；其余 UTC 零点；忽略会保留";
  }
  return "按游戏时间 UTC 周一重置；忽略会保留";
});

function matchesMode(t: RoutineItemDto) {
  if (isNightwaveTitle(t.title)) return false;
  if (props.mode === "daily") return t.kind === "daily";
  return t.kind === "weekly";
}

const ignoredCount = computed(
  () => items.value.filter((x) => x.ignored && matchesMode(x)).length,
);

function bySort(a: RoutineItemDto, b: RoutineItemDto) {
  return a.sortOrder - b.sortOrder || Number(a.id) - Number(b.id);
}

const visibleItems = computed(() =>
  items.value
    .filter(
      (x) => matchesMode(x) && (showIgnored.value ? x.ignored : !x.ignored),
    )
    .sort(bySort),
);

/** 可见项按 title 预计算；仅随 brief / 列表变，倒计时由 LiveRemain 本地刷新 */
const liveDetailsByTitle = computed(() => {
  const map = new Map<string, LiveDetail | null>();
  const brief = liveBrief.value;
  for (const t of visibleItems.value) {
    if (!map.has(t.title)) {
      map.set(t.title, buildLiveDetail(t.title, brief));
    }
  }
  return map;
});

function liveDetail(title: string): LiveDetail | null {
  return liveDetailsByTitle.value.get(title) ?? null;
}

function canExpand(detail: LiveDetail | null) {
  return !!detail && detail.kind !== "empty";
}

function onMainClick(t: RoutineItemDto) {
  const detail = liveDetail(t.title);
  if (!canExpand(detail)) return;
  toggleExpand(t.id);
}

function patchLocal(next: RoutineItemDto) {
  items.value = items.value.map((x) => (x.id === next.id ? next : x));
}

async function toggleDone(t: RoutineItemDto) {
  try {
    const next = await updateRoutine({ id: t.id, done: !t.done });
    patchLocal(next);
  } catch (e) {
    error.value = String(e);
  }
}

async function setIgnored(t: RoutineItemDto, ignored: boolean) {
  try {
    const next = await updateRoutine({ id: t.id, ignored });
    patchLocal(next);
  } catch (e) {
    error.value = String(e);
  }
}

async function startAdd(kind: RoutineKind) {
  addingKind.value = kind;
  newTitle.value = "";
  await nextTick();
  addInputEl.value?.focus();
}

function cancelAdd() {
  addingKind.value = null;
  newTitle.value = "";
}

async function addItem() {
  const kind = addingKind.value;
  const title = newTitle.value.trim();
  if (!kind) return;
  if (!title) {
    cancelAdd();
    return;
  }
  try {
    const created = await createRoutine({ kind, title });
    items.value = [...items.value, created];
    cancelAdd();
    error.value = "";
  } catch (e) {
    error.value = String(e);
  }
}

function startEdit(t: RoutineItemDto) {
  if (t.isBuiltin) return;
  editingId.value = t.id;
  editDraft.value = t.title;
}

function cancelEdit() {
  editingId.value = null;
  editDraft.value = "";
}

async function saveEdit(t: RoutineItemDto) {
  const title = editDraft.value.trim();
  if (!title) return;
  try {
    const next = await updateRoutine({ id: t.id, title });
    patchLocal(next);
    cancelEdit();
  } catch (e) {
    error.value = String(e);
  }
}

async function removeItem(t: RoutineItemDto) {
  if (t.isBuiltin) return;
  try {
    await deleteRoutine(t.id);
    items.value = items.value.filter((x) => x.id !== t.id);
    if (editingId.value === t.id) cancelEdit();
  } catch (e) {
    error.value = String(e);
  }
}

function rowIdFromPoint(clientX: number, clientY: number): string | null {
  const el = document.elementFromPoint(clientX, clientY);
  const row = el?.closest?.(".routine-row[data-routine-id]") as HTMLElement | null;
  return row?.dataset.routineId ?? null;
}

function detachPointerListeners() {
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("pointerup", onPointerUp);
  window.removeEventListener("pointercancel", onPointerUp);
}

function onPointerMove(ev: PointerEvent) {
  if (!pointerDragging || !dragId.value) return;
  const over = rowIdFromPoint(ev.clientX, ev.clientY);
  if (over) dragOverId.value = over;
}

async function commitReorder(fromId: string, toId: string) {
  if (!fromId || !toId || fromId === toId || reorderBusy) return;
  if (props.mode !== "daily" && props.mode !== "weekly") return;

  const kind = props.mode;
  const visible = visibleItems.value;
  const fromIdx = visible.findIndex((x) => x.id === fromId);
  const toIdx = visible.findIndex((x) => x.id === toId);
  if (fromIdx < 0 || toIdx < 0 || fromIdx === toIdx) return;

  const nextVisible = [...visible];
  const [moved] = nextVisible.splice(fromIdx, 1);
  nextVisible.splice(toIdx, 0, moved);

  const ofKind = items.value
    .filter((x) => x.kind === kind)
    .sort((a, b) => a.sortOrder - b.sortOrder || Number(a.id) - Number(b.id));
  const visibleSet = new Set(nextVisible.map((x) => x.id));
  const queue = [...nextVisible];
  const merged: RoutineItemDto[] = [];
  for (const item of ofKind) {
    if (visibleSet.has(item.id)) {
      const next = queue.shift();
      if (next) merged.push(next);
    } else {
      merged.push(item);
    }
  }

  const prev = items.value;
  const byId = new Map(merged.map((x, i) => [x.id, { ...x, sortOrder: i }]));
  items.value = prev.map((x) => byId.get(x.id) ?? x);

  reorderBusy = true;
  try {
    await reorderRoutines({ kind, ids: merged.map((x) => x.id) });
    error.value = "";
  } catch (e) {
    items.value = prev;
    error.value = String(e);
  } finally {
    reorderBusy = false;
  }
}

async function onPointerUp(ev: PointerEvent) {
  if (!pointerDragging) return;
  const fromId = dragId.value;
  const toId =
    rowIdFromPoint(ev.clientX, ev.clientY) || dragOverId.value || fromId;
  pointerDragging = false;
  dragId.value = null;
  dragOverId.value = null;
  document.body.classList.remove("routine-sorting");
  detachPointerListeners();
  if (fromId && toId) await commitReorder(fromId, toId);
}

function onHandlePointerDown(t: RoutineItemDto, ev: PointerEvent) {
  if (showIgnored.value) return;
  if (ev.button !== 0) return;
  ev.preventDefault();
  ev.stopPropagation();
  pointerDragging = true;
  dragId.value = t.id;
  dragOverId.value = t.id;
  document.body.classList.add("routine-sorting");
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
  window.addEventListener("pointercancel", onPointerUp);
}

onUnmounted(() => {
  document.body.classList.remove("routine-sorting");
  detachPointerListeners();
});

const addKind = computed<RoutineKind | null>(() => {
  if (props.mode === "daily") return "daily";
  if (props.mode === "weekly") return "weekly";
  return null;
});
</script>

<template>
  <div class="card routine-card">
    <div class="routine-head">
      <h2>{{ panelTitle }}</h2>
      <div class="routine-head-actions">
        <button
          type="button"
          class="ghost routine-ignored-btn"
          :class="{ active: showIgnored }"
          @click="showIgnored = !showIgnored"
        >
          {{ showIgnored ? "返回清单" : `已忽略${ignoredCount ? ` (${ignoredCount})` : ""}` }}
        </button>
      </div>
    </div>
    <p class="routine-hint">{{ panelHint }}</p>
    <p v-if="error" class="routine-error">{{ error }}</p>

    <template v-if="showIgnored">
      <p v-if="ignoredCount === 0" class="routine-empty">暂无已忽略项</p>
      <div v-else class="routine-list">
        <div
          v-for="t in visibleItems"
          :key="t.id"
          class="routine-row"
        >
          <span class="routine-title">{{ t.title }}</span>
          <button
            type="button"
            class="routine-act"
            title="取消忽略"
            @click="setIgnored(t, false)"
          >
            恢复
          </button>
        </div>
      </div>
    </template>

    <template v-else>
      <div class="routine-list">
        <div
          v-for="t in visibleItems"
          :key="t.id"
          class="routine-row"
          :data-routine-id="t.id"
          :class="{
            done: t.done,
            open: isExpanded(t.id),
            dragging: dragId === t.id,
            'drag-over': dragOverId === t.id && dragId !== t.id,
          }"
        >
          <span
            class="routine-drag"
            title="拖动排序"
            @pointerdown="onHandlePointerDown(t, $event)"
          >⋮⋮</span>
          <span
            class="check"
            :class="{ on: t.done }"
            role="checkbox"
            :aria-checked="t.done"
            title="完成"
            @click="toggleDone(t)"
            >✓</span
          >
          <template v-if="editingId === t.id">
            <input
              v-model="editDraft"
              class="routine-edit"
              type="text"
              @keydown.enter.prevent="saveEdit(t)"
              @keydown.escape.prevent="cancelEdit"
            />
            <div class="routine-actions always">
              <button type="button" class="routine-act" title="保存" @click="saveEdit(t)">✓</button>
              <button type="button" class="routine-act" title="取消" @click="cancelEdit">×</button>
            </div>
          </template>
          <template v-else>
            <div
              class="routine-main"
              :class="{ clickable: canExpand(liveDetail(t.title)) }"
              @click="onMainClick(t)"
            >
              <div class="routine-title-row">
                <span
                  class="routine-title"
                  :class="{ editable: !t.isBuiltin }"
                  @dblclick.stop="startEdit(t)"
                >{{ t.title }}</span>
                <button
                  v-if="canExpand(liveDetail(t.title))"
                  type="button"
                  class="routine-chevron"
                  :aria-expanded="isExpanded(t.id)"
                  :title="isExpanded(t.id) ? '收起详情' : '展开详情'"
                  @click.stop="toggleExpand(t.id)"
                >{{ isExpanded(t.id) ? "▾" : "▸" }}</button>
              </div>
              <p
                v-if="liveDetail(t.title) && !isExpanded(t.id)"
                class="routine-summary"
              >{{ liveDetail(t.title)?.summary }}</p>
              <RoutineLivePanel
                v-if="liveDetail(t.title) && isExpanded(t.id)"
                :detail="liveDetail(t.title)!"
                :families="families"
                :weapon-families="weaponFamilies"
                @ended="onLiveEnded"
                @click.stop
              />
            </div>
            <div class="routine-actions">
              <button
                v-if="!t.isBuiltin"
                type="button"
                class="routine-act"
                title="编辑"
                @click="startEdit(t)"
              >✎</button>
              <button
                type="button"
                class="routine-act"
                title="忽略"
                @click="setIgnored(t, true)"
              >隐</button>
              <button
                v-if="!t.isBuiltin"
                type="button"
                class="routine-act danger"
                title="删除"
                @click="removeItem(t)"
              >×</button>
            </div>
          </template>
        </div>
        <div v-if="addingKind === addKind" class="routine-row compose">
          <span class="routine-drag compose-spacer" aria-hidden="true">⋮⋮</span>
          <span class="check compose-spacer" aria-hidden="true">✓</span>
          <input
            ref="addInputEl"
            v-model="newTitle"
            class="routine-edit"
            type="text"
            :placeholder="mode === 'daily' ? '自定义日常…' : '自定义周常…'"
            @keydown.enter.prevent="addItem"
            @keydown.escape.prevent="cancelAdd"
          />
          <div class="routine-actions always">
            <button type="button" class="routine-act" title="添加" @click="addItem">✓</button>
            <button type="button" class="routine-act" title="取消" @click="cancelAdd">×</button>
          </div>
        </div>
        <button
          v-else-if="addKind"
          type="button"
          class="routine-plus"
          @click="startAdd(addKind)"
        >
          + {{ panelTitle }}
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.routine-card {
  margin-bottom: 0;
}
.routine-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.routine-head h2 {
  margin: 0;
}
.routine-ignored-btn.active {
  color: var(--accent);
}
.routine-hint {
  margin: 6px 0 10px;
  font-size: 12px;
  color: var(--text-mute);
}
.routine-error {
  color: var(--danger);
  font-size: 12px;
  margin: 0 0 8px;
}
.routine-sub {
  margin: 14px 0 6px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-mute);
}
.routine-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.routine-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px 10px;
  border-radius: 8px;
}
.routine-row:hover {
  background: var(--bg-elevated);
}
.routine-row.open {
  background: var(--bg-elevated);
}
.routine-row.dragging {
  opacity: 0.45;
}
.routine-row.drag-over {
  box-shadow: inset 0 2px 0 0 var(--accent);
}
:global(body.routine-sorting) {
  cursor: grabbing !important;
  user-select: none !important;
}
:global(body.routine-sorting *) {
  cursor: grabbing !important;
}
.routine-drag {
  flex-shrink: 0;
  margin-top: 4px;
  width: 18px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  text-align: center;
  color: var(--text-mute);
  font-size: 12px;
  letter-spacing: -2px;
  cursor: grab;
  user-select: none;
  touch-action: none;
  opacity: 0.55;
}
.routine-drag:active {
  cursor: grabbing;
}
.routine-row:hover .routine-drag,
.routine-row.dragging .routine-drag {
  opacity: 1;
  color: var(--text-dim);
}
.routine-row.done .routine-title {
  text-decoration: line-through;
  color: var(--text-mute);
}
.check {
  flex-shrink: 0;
  margin-top: 3px;
}
.compose-spacer {
  visibility: hidden;
  pointer-events: none;
}
.routine-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.routine-main.clickable {
  cursor: pointer;
}
.routine-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.routine-title {
  font-size: 15px;
  line-height: 1.35;
  font-weight: 560;
}
.routine-title.editable {
  cursor: text;
}
.routine-chevron {
  appearance: none;
  border: none;
  background: transparent;
  color: var(--text-mute);
  font: inherit;
  font-size: 14px;
  line-height: 1;
  padding: 2px 4px;
  border-radius: 4px;
  cursor: pointer;
}
.routine-chevron:hover {
  color: var(--accent);
  background: var(--accent-soft);
}
.routine-summary {
  margin: 0;
  font-size: 13px;
  line-height: 1.4;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.routine-actions {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: flex-end;
  gap: 2px;
  min-width: 72px;
  opacity: 0;
  transition: opacity 0.12s ease;
}
.routine-actions.always,
.routine-row:hover .routine-actions,
.routine-row:focus-within .routine-actions {
  opacity: 1;
}
.routine-act {
  appearance: none;
  border: none;
  background: transparent;
  color: var(--text-mute);
  font: inherit;
  font-size: 12px;
  line-height: 1;
  min-width: 26px;
  height: 26px;
  padding: 0 4px;
  border-radius: 4px;
  cursor: pointer;
}
.routine-act:hover {
  color: var(--accent);
  background: var(--accent-soft);
}
.routine-act.danger:hover {
  color: var(--danger, #c45c5c);
  background: rgba(196, 92, 92, 0.12);
}
.routine-empty {
  margin: 0;
  padding: 12px 8px 4px;
  color: var(--text-mute);
  font-size: 13px;
}
.routine-empty.soft {
  padding: 4px 8px;
  font-size: 12px;
}
.routine-edit {
  flex: 1 1 auto;
  min-width: 0;
  font: inherit;
  font-size: 14px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--bg-elevated);
  color: var(--text);
  outline: none;
  box-sizing: border-box;
}
.routine-edit:focus {
  border-color: var(--accent);
}
.routine-plus {
  appearance: none;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  margin-top: 8px;
  padding: 8px;
  border: 1px dashed var(--line);
  border-radius: 8px;
  background: transparent;
  color: var(--text-mute);
  font-size: 13px;
  cursor: pointer;
}
.routine-plus:hover {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-soft);
}
</style>
