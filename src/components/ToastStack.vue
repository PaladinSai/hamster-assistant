<script setup lang="ts">
import { STANDING_SECTION_TAG, type StandingSection } from "../composables/useAppNav";
import { useAppToasts } from "../composables/useAppToasts";
import LiveRemain from "./routine/LiveRemain.vue";

const emit = defineEmits<{
  openStanding: [section: Exclude<StandingSection, "overview">];
}>();

const { toasts, dismissToast } = useAppToasts();

const TAG_TO_SECTION = Object.fromEntries(
  (
    Object.entries(STANDING_SECTION_TAG) as Array<
      [Exclude<StandingSection, "overview">, string]
    >
  ).map(([section, tag]) => [tag, section]),
) as Record<string, Exclude<StandingSection, "overview">>;

function onToastClick(t: { id: string; standingTag: string | null; actions?: readonly unknown[] }) {
  if (t.actions?.length) return;
  const tag = t.standingTag;
  if (!tag) return;
  const section = TAG_TO_SECTION[tag];
  if (!section) return;
  emit("openStanding", section);
  dismissToast(t.id);
}

function onAction(id: string, run: () => void) {
  dismissToast(id);
  run();
}
</script>

<template>
  <div class="toast-stack" aria-live="polite" aria-relevant="additions">
    <div
      v-for="t in toasts"
      :key="t.id"
      class="toast-card"
      :class="{ clickable: !!t.standingTag && !t.actions?.length }"
      role="status"
      @click="onToastClick(t)"
    >
      <div class="toast-top">
        <strong class="toast-title">{{ t.title }}</strong>
        <button
          class="toast-close"
          type="button"
          aria-label="关闭"
          @click.stop="dismissToast(t.id)"
        >
          ×
        </button>
      </div>
      <p class="toast-body">{{ t.body }}</p>
      <div v-if="t.actions?.length" class="toast-actions">
        <button
          v-for="(a, i) in t.actions"
          :key="i"
          type="button"
          class="toast-act"
          :class="a.kind === 'primary' ? 'primary' : 'ghost'"
          @click.stop="onAction(t.id, a.run)"
        >
          {{ a.label }}
        </button>
      </div>
      <div v-if="t.expiry" class="toast-remain">
        <LiveRemain :expiry="t.expiry" @ended="dismissToast(t.id)" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.toast-stack {
  position: fixed;
  right: 16px;
  bottom: 16px;
  z-index: 80;
  display: flex;
  flex-direction: column-reverse;
  gap: 10px;
  max-width: min(360px, calc(100vw - 32px));
  pointer-events: none;
}
.toast-card {
  pointer-events: auto;
  background: var(--bg-elevated);
  border: 1px solid var(--line);
  border-left: 3px solid var(--accent);
  border-radius: var(--radius);
  padding: 12px 12px 12px 14px;
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.35);
  animation: toast-in 0.22s ease-out;
}
.toast-card.clickable {
  cursor: pointer;
}
.toast-card.clickable:hover {
  border-color: var(--accent);
  background: var(--bg-row-hover);
}
.toast-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
}
.toast-title {
  font-size: 13px;
  font-weight: 650;
  color: var(--accent);
}
.toast-close {
  flex-shrink: 0;
  margin: -4px -4px 0 0;
  border: none;
  background: transparent;
  color: var(--text-mute);
  font-size: 18px;
  line-height: 1;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  cursor: pointer;
}
.toast-close:hover {
  color: var(--text);
  background: var(--accent-soft);
}
.toast-body {
  margin: 6px 0 0;
  font-size: 13px;
  line-height: 1.45;
  color: var(--text);
  white-space: pre-wrap;
  word-break: break-word;
}
.toast-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 10px;
}
.toast-act {
  border-radius: 6px;
  padding: 5px 10px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.toast-act.primary {
  border: 1px solid color-mix(in srgb, var(--cyan, #5ec8c8) 45%, transparent);
  background: color-mix(in srgb, var(--cyan, #5ec8c8) 18%, transparent);
  color: var(--cyan, #5ec8c8);
}
.toast-act.primary:hover {
  background: color-mix(in srgb, var(--cyan, #5ec8c8) 28%, transparent);
}
.toast-act.ghost {
  border: 1px solid var(--line-soft);
  background: transparent;
  color: var(--text-mute);
}
.toast-act.ghost:hover {
  color: var(--text);
  border-color: var(--line);
}
.toast-remain {
  margin-top: 8px;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  color: var(--cyan);
}
.toast-remain :deep(.live-remain) {
  color: inherit;
}
@keyframes toast-in {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
