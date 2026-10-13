<script setup lang="ts">
import type { ArchimedeaLiveDetail } from "./liveTypes";
import LiveRemain from "./LiveRemain.vue";

defineProps<{ detail: ArchimedeaLiveDetail }>();
const emit = defineEmits<{ ended: [] }>();
</script>

<template>
  <div class="live-archimedea">
    <div class="live-hero">
      <div class="live-hero-text">
        <span class="live-kicker">{{ detail.kindLabel }}</span>
        <span class="live-hero-title">三关科研</span>
      </div>
      <div class="live-hero-meta">
        <LiveRemain :expiry="detail.expiry" @ended="emit('ended')" />
      </div>
    </div>
    <ol class="live-missions">
      <li
        v-for="(m, i) in detail.missions"
        :key="i"
        class="live-mission"
      >
        <span class="live-mission-idx">{{ i + 1 }}</span>
        <div class="live-mission-body">
          <div class="live-mission-type">{{ m.missionType || "任务" }}</div>
          <div v-if="m.faction" class="live-mission-planet">{{ m.faction }}</div>
          <div v-if="m.deviation?.title" class="live-term-row">
            <span class="live-term-label">偏差</span>
            <span
              class="live-term-chip"
              :class="{ tip: !!m.deviation.description }"
              :data-tip="m.deviation.description || undefined"
            >{{ m.deviation.title }}</span>
          </div>
          <div v-if="m.risksNormal.length" class="live-term-row">
            <span class="live-term-label">普通</span>
            <span
              v-for="(r, ri) in m.risksNormal"
              :key="'n' + ri"
              class="live-term-chip"
              :class="{ tip: !!r.description }"
              :data-tip="r.description || undefined"
            >{{ r.title }}</span>
          </div>
          <div v-if="m.risksElite.length" class="live-term-row">
            <span class="live-term-label elite">精英</span>
            <span
              v-for="(r, ri) in m.risksElite"
              :key="'e' + ri"
              class="live-term-chip elite"
              :class="{ tip: !!r.description }"
              :data-tip="r.description || undefined"
            >{{ r.title }}</span>
          </div>
        </div>
      </li>
    </ol>
    <div v-if="detail.personalModifiers.length" class="live-mods">
      <span class="live-kicker">本周个人修正</span>
      <ul class="live-mod-list">
        <li
          v-for="(p, i) in detail.personalModifiers"
          :key="i"
          class="live-term-chip"
          :class="{ tip: !!p.description }"
          :data-tip="p.description || undefined"
        >{{ p.title }}</li>
      </ul>
    </div>
  </div>
</template>
