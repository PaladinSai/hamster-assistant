<script setup lang="ts">
withDefaults(
  defineProps<{
    open: boolean;
    title?: string;
    message: string;
  }>(),
  { title: "确认" },
);

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();
</script>

<template>
  <Teleport to="body">
    <div class="confirm-root" :class="{ open }" :aria-hidden="!open">
      <div class="confirm-mask" @click="emit('cancel')" />
      <div
        class="confirm-panel"
        role="dialog"
        aria-modal="true"
        :aria-label="title"
      >
        <h3 class="confirm-title">{{ title }}</h3>
        <p class="confirm-msg">{{ message }}</p>
        <div class="confirm-actions">
          <button class="ghost" type="button" @click="emit('cancel')">
            取消
          </button>
          <button class="confirm-ok" type="button" @click="emit('confirm')">
            确定
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.confirm-root {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.15s ease;
}
.confirm-root.open {
  pointer-events: auto;
  opacity: 1;
}
.confirm-mask {
  position: absolute;
  inset: 0;
  background: rgba(4, 8, 12, 0.55);
}
.confirm-panel {
  position: relative;
  width: min(360px, calc(100vw - 32px));
  padding: 18px 18px 14px;
  border-radius: var(--radius, 10px);
  background: var(--bg-elevated, #18212c);
  border: 1px solid var(--line, #2a3645);
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.45);
}
.confirm-title {
  margin: 0 0 8px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text, #e8eef5);
}
.confirm-msg {
  margin: 0 0 16px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-dim, #8b9aab);
}
.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.confirm-ok {
  appearance: none;
  border: 1px solid var(--cyan, #5ec8c8);
  background: var(--cyan-soft, rgba(94, 200, 200, 0.12));
  color: var(--cyan, #5ec8c8);
  border-radius: 8px;
  padding: 7px 14px;
  cursor: pointer;
  font-size: 12px;
}
.confirm-ok:hover {
  background: rgba(94, 200, 200, 0.22);
}
</style>
