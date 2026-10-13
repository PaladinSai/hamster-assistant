<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import {
  createUserTodo,
  deleteUserTodo,
  listUserTodos,
  reorderUserTodos,
  updateUserTodo,
  type UserTodoDto,
} from "../lib/api";

const todos = ref<UserTodoDto[]>([]);
const newTodo = ref("");
const adding = ref(false);
const addInputEl = ref<HTMLInputElement | null>(null);
const editingId = ref<string | null>(null);
const editDraft = ref("");
const todoError = ref("");
const dragId = ref<string | null>(null);
const dragOverId = ref<string | null>(null);

let pointerDragging = false;
let reorderBusy = false;

function byTodoOrder(a: UserTodoDto, b: UserTodoDto) {
  const doneCmp = Number(a.done) - Number(b.done);
  if (doneCmp !== 0) return doneCmp;
  return a.sortOrder - b.sortOrder || Number(a.id) - Number(b.id);
}

/** 与后端 ORDER BY done, sort_order, id 一致，否则只改 sortOrder 时界面不动。 */
const displayTodos = computed(() => [...todos.value].sort(byTodoOrder));

async function loadTodos() {
  try {
    todos.value = await listUserTodos();
    todoError.value = "";
  } catch (e) {
    todoError.value = String(e);
  }
}

onMounted(() => {
  void loadTodos();
});

async function startAdd() {
  adding.value = true;
  newTodo.value = "";
  await nextTick();
  addInputEl.value?.focus();
}

function cancelAdd() {
  adding.value = false;
  newTodo.value = "";
}

function onAddBlur() {
  if (!newTodo.value.trim()) cancelAdd();
}

async function addTodo() {
  const body = newTodo.value.trim();
  if (!body) {
    cancelAdd();
    return;
  }
  try {
    const t = await createUserTodo(body);
    todos.value = [...todos.value, t];
    newTodo.value = "";
    adding.value = false;
    todoError.value = "";
  } catch (e) {
    todoError.value = String(e);
  }
}

async function toggleDone(t: UserTodoDto) {
  try {
    const next = await updateUserTodo({ id: t.id, done: !t.done });
    todos.value = todos.value.map((x) => (x.id === t.id ? next : x));
  } catch (e) {
    todoError.value = String(e);
  }
}

function startEdit(t: UserTodoDto) {
  editingId.value = t.id;
  editDraft.value = t.body;
}

function cancelEdit() {
  editingId.value = null;
  editDraft.value = "";
}

async function saveEdit(t: UserTodoDto) {
  const body = editDraft.value.trim();
  if (!body) return;
  try {
    const next = await updateUserTodo({ id: t.id, body });
    todos.value = todos.value.map((x) => (x.id === t.id ? next : x));
    cancelEdit();
  } catch (e) {
    todoError.value = String(e);
  }
}

async function removeTodo(t: UserTodoDto) {
  try {
    await deleteUserTodo(t.id);
    todos.value = todos.value.filter((x) => x.id !== t.id);
    if (editingId.value === t.id) cancelEdit();
  } catch (e) {
    todoError.value = String(e);
  }
}

function rowIdFromPoint(clientX: number, clientY: number): string | null {
  const el = document.elementFromPoint(clientX, clientY);
  const row = el?.closest?.(".todo-row[data-todo-id]") as HTMLElement | null;
  return row?.dataset.todoId ?? null;
}

function detachPointerListeners() {
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("pointerup", onPointerUp);
  window.removeEventListener("pointercancel", onPointerUp);
}

function onPointerMove(ev: PointerEvent) {
  if (!pointerDragging || !dragId.value) return;
  const over = rowIdFromPoint(ev.clientX, ev.clientY);
  if (!over) return;
  const from = todos.value.find((x) => x.id === dragId.value);
  const to = todos.value.find((x) => x.id === over);
  if (!from || !to || from.done !== to.done) return;
  dragOverId.value = over;
}

async function commitReorder(fromId: string, toId: string) {
  if (!fromId || !toId || fromId === toId || reorderBusy) return;

  const from = todos.value.find((x) => x.id === fromId);
  const to = todos.value.find((x) => x.id === toId);
  if (!from || !to || from.done !== to.done) return;

  const group = todos.value
    .filter((x) => x.done === from.done)
    .sort(byTodoOrder);
  const fromIdx = group.findIndex((x) => x.id === fromId);
  const toIdx = group.findIndex((x) => x.id === toId);
  if (fromIdx < 0 || toIdx < 0 || fromIdx === toIdx) return;

  const nextGroup = [...group];
  const [moved] = nextGroup.splice(fromIdx, 1);
  nextGroup.splice(toIdx, 0, moved);

  const prev = todos.value;
  const byId = new Map(nextGroup.map((x, i) => [x.id, { ...x, sortOrder: i }]));
  todos.value = prev.map((x) => byId.get(x.id) ?? x).sort(byTodoOrder);

  reorderBusy = true;
  try {
    await reorderUserTodos({ ids: nextGroup.map((x) => x.id) });
    todoError.value = "";
  } catch (e) {
    todos.value = prev;
    todoError.value = String(e);
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
  document.body.classList.remove("todo-sorting");
  detachPointerListeners();
  if (fromId && toId) await commitReorder(fromId, toId);
}

function onHandlePointerDown(t: UserTodoDto, ev: PointerEvent) {
  if (ev.button !== 0) return;
  ev.preventDefault();
  ev.stopPropagation();
  pointerDragging = true;
  dragId.value = t.id;
  dragOverId.value = t.id;
  document.body.classList.add("todo-sorting");
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
  window.addEventListener("pointercancel", onPointerUp);
}

onUnmounted(() => {
  document.body.classList.remove("todo-sorting");
  detachPointerListeners();
});
</script>

<template>
  <div class="card todo-card">
    <h2>待办</h2>
    <p v-if="todoError" class="todo-error">{{ todoError }}</p>
    <div class="todo-list">
      <div
        v-for="t in displayTodos"
        :key="t.id"
        class="todo-row"
        :data-todo-id="t.id"
        :class="{
          done: t.done,
          dragging: dragId === t.id,
          'drag-over': dragOverId === t.id && dragId !== t.id,
        }"
      >
        <span
          class="todo-drag"
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
            class="todo-edit"
            type="text"
            @keydown.enter.prevent="saveEdit(t)"
            @keydown.escape.prevent="cancelEdit"
          />
          <div class="todo-actions always">
            <button type="button" class="todo-act" title="保存" @click="saveEdit(t)">✓</button>
            <button type="button" class="todo-act" title="取消" @click="cancelEdit">×</button>
          </div>
        </template>
        <template v-else>
          <span class="todo-body" @dblclick="startEdit(t)">{{ t.body }}</span>
          <div class="todo-actions">
            <button type="button" class="todo-act" title="编辑" @click="startEdit(t)">✎</button>
            <button type="button" class="todo-act danger" title="删除" @click="removeTodo(t)">×</button>
          </div>
        </template>
      </div>
      <p v-if="todos.length === 0 && !adding" class="todo-empty">暂无待办</p>
    </div>

    <div v-if="adding" class="todo-row todo-compose">
      <span class="todo-drag compose-spacer" aria-hidden="true">⋮⋮</span>
      <span class="check compose-spacer" aria-hidden="true">✓</span>
      <input
        ref="addInputEl"
        v-model="newTodo"
        class="todo-input"
        type="text"
        placeholder="写下一件待办…"
        @keydown.enter.prevent="addTodo"
        @keydown.escape.prevent="cancelAdd"
        @blur="onAddBlur"
      />
      <span class="todo-actions spacer" aria-hidden="true" />
    </div>
    <button
      v-else
      type="button"
      class="todo-plus"
      title="新增待办"
      @click="startAdd"
    >
      +
    </button>
  </div>
</template>

<style scoped>
.todo-error {
  color: var(--danger);
  font-size: 12px;
  margin: 0 0 8px;
}
.todo-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.todo-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 8px;
  border-radius: 6px;
  border-bottom: 1px solid transparent;
}
.todo-row:hover {
  background: var(--bg-elevated);
}
.todo-row.dragging {
  opacity: 0.45;
}
.todo-row.drag-over {
  box-shadow: inset 0 2px 0 0 var(--accent);
}
:global(body.todo-sorting) {
  cursor: grabbing !important;
  user-select: none !important;
}
:global(body.todo-sorting *) {
  cursor: grabbing !important;
}
.todo-drag {
  flex-shrink: 0;
  width: 18px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--text-mute);
  font-size: 12px;
  letter-spacing: -2px;
  cursor: grab;
  user-select: none;
  touch-action: none;
  opacity: 0.55;
}
.todo-row:hover .todo-drag {
  opacity: 0.9;
}
.todo-row.done .todo-body {
  text-decoration: line-through;
  color: var(--text-mute);
}
.check {
  flex-shrink: 0;
}
.compose-spacer {
  visibility: hidden;
  pointer-events: none;
}
.todo-body {
  flex: 1;
  min-width: 0;
  width: 100%;
  font-size: 13px;
  line-height: 1.4;
  cursor: text;
}
.todo-actions {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: flex-end;
  gap: 2px;
  width: 56px;
  opacity: 0;
  transition: opacity 0.12s ease;
}
.todo-actions.spacer {
  visibility: hidden;
  pointer-events: none;
  opacity: 1;
}
.todo-actions.always,
.todo-row:hover .todo-actions,
.todo-row:focus-within .todo-actions {
  opacity: 1;
}
.todo-act {
  appearance: none;
  border: none;
  background: transparent;
  color: var(--text-mute);
  font: inherit;
  font-size: 13px;
  line-height: 1;
  width: 26px;
  height: 26px;
  padding: 0;
  border-radius: 4px;
  cursor: pointer;
}
.todo-act:hover {
  color: var(--accent);
  background: var(--accent-soft);
}
.todo-act.danger:hover {
  color: var(--danger, #c45c5c);
  background: rgba(196, 92, 92, 0.12);
}
.todo-empty {
  margin: 0;
  padding: 12px 8px 4px;
  color: var(--text-mute);
  font-size: 13px;
}
.todo-edit,
.todo-input {
  flex: 1 1 auto;
  min-width: 0;
  width: 100%;
  font: inherit;
  font-size: 13px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--bg-elevated);
  color: var(--text);
  outline: none;
  box-sizing: border-box;
}
.todo-edit:focus,
.todo-input:focus {
  border-color: var(--accent);
}
.todo-compose {
  margin-top: 4px;
}
.todo-plus {
  appearance: none;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  margin-top: 10px;
  padding: 10px;
  border: 1px dashed var(--line);
  border-radius: 8px;
  background: transparent;
  color: var(--text-mute);
  font-size: 20px;
  font-weight: 500;
  line-height: 1;
  cursor: pointer;
  transition:
    color 0.12s ease,
    border-color 0.12s ease,
    background 0.12s ease;
}
.todo-plus:hover {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-soft);
}
</style>
