<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import { remainingText } from "./liveUtils";

const props = defineProps<{
  expiry: string | null | undefined;
}>();

const emit = defineEmits<{
  ended: [];
}>();

const text = ref("");
let timer: ReturnType<typeof setInterval> | undefined;
let endedEmitted = false;

function tick() {
  const next = remainingText(props.expiry);
  text.value = next;
  if (next === "已结束" && !endedEmitted) {
    endedEmitted = true;
    emit("ended");
  }
}

function onVisibility() {
  if (document.visibilityState === "visible") tick();
}

watch(
  () => props.expiry,
  () => {
    endedEmitted = false;
    tick();
  },
);

onMounted(() => {
  tick();
  timer = setInterval(tick, 1000);
  document.addEventListener("visibilitychange", onVisibility);
});

onUnmounted(() => {
  if (timer != null) clearInterval(timer);
  document.removeEventListener("visibilitychange", onVisibility);
});
</script>

<template>
  <span v-if="text" class="live-remain">{{ text }}</span>
</template>
