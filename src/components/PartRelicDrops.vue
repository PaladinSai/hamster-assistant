<script setup lang="ts">
import { computed } from "vue";
import type { PartDropDto } from "../lib/api";
import { rarityLabel } from "../lib/status";
import { gradeClass } from "../lib/gradeClass";

const props = defineProps<{
  mode: "toggle" | "list";
  partId: string;
  drops: PartDropDto[];
  /** toggle 模式：是否已展开 */
  expanded?: boolean;
}>();

const emit = defineEmits<{
  toggle: [partId: string];
  openRelic: [relicKey: string];
}>();

/** 当前可刷：未入库，或本期重生。 */
function isAvailable(d: PartDropDto) {
  return d.vaulted === 0 || d.inResurgence;
}

const availableCount = computed(
  () => props.drops.filter(isAvailable).length,
);

const firstDrop = computed(() => props.drops[0] ?? null);

const canExpand = computed(() => props.drops.length > 1);

function onToggleClick() {
  if (!canExpand.value) return;
  emit("toggle", props.partId);
}
</script>

<template>
  <template v-if="mode === 'toggle'">
    <span v-if="!drops.length" style="color: var(--text-mute)">—</span>
    <div
      v-else
      class="relic-summary"
      :class="{
        expandable: canExpand,
        expanded: expanded && canExpand,
      }"
    >
      <button
        type="button"
        class="relic-summary-main"
        :class="{ clickable: canExpand }"
        :title="
          canExpand
            ? expanded
              ? '收起遗物列表'
              : '展开全部遗物'
            : undefined
        "
        :aria-expanded="canExpand ? !!expanded : undefined"
        @click="onToggleClick"
      >
        <span v-if="canExpand" class="relic-chevron" aria-hidden="true">{{
          expanded ? "▾" : "▸"
        }}</span>
        <span class="relic-count" :title="`当前可刷 ${availableCount} / 共 ${drops.length}`">
          <em>{{ availableCount }}</em>/<span>{{ drops.length }}</span>
        </span>
        <template v-if="firstDrop">
          <span class="relic-preview-name">{{ firstDrop.relicName }}</span>
          <span
            v-if="firstDrop.rarity != null"
            class="grade"
            :class="gradeClass(firstDrop.rarity)"
            :title="rarityLabel(firstDrop.rarity)"
          >
            {{ rarityLabel(firstDrop.rarity)?.[0] }}
          </span>
          <span
            v-if="firstDrop.inResurgence"
            class="vault-tag rebirth"
            title="本期重生相关"
          >重生</span>
          <span
            v-else-if="firstDrop.vaulted"
            class="vault-tag"
            title="已入库"
          >入库</span>
        </template>
      </button>
      <button
        v-if="firstDrop"
        type="button"
        class="part-link relic-jump"
        title="打开应用内遗物来源表"
        @click.stop="emit('openRelic', firstDrop.relicKey)"
      >
        来源
      </button>
    </div>
  </template>

  <ul v-else-if="mode === 'list'" class="relic-list">
    <li v-for="(d, i) in drops" :key="`${partId}-${i}`">
      <button
        type="button"
        class="part-link"
        title="打开应用内遗物来源表"
        @click="emit('openRelic', d.relicKey)"
      >
        {{ d.relicName }}
      </button>
      <span
        v-if="d.rarity != null"
        class="grade"
        :class="gradeClass(d.rarity)"
        :title="rarityLabel(d.rarity)"
      >
        {{ rarityLabel(d.rarity)?.[0] }}
      </span>
      <span
        v-if="d.inResurgence"
        class="vault-tag rebirth"
        title="本期重生相关"
      >重生</span>
      <span
        v-if="d.vaulted"
        class="vault-tag"
        title="已入库"
      >入库</span>
    </li>
  </ul>
</template>

<style scoped>
.relic-summary {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  min-width: 0;
}
.relic-summary-main {
  appearance: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  min-width: 0;
  margin: 0;
  padding: 4px 8px;
  border: 1px solid var(--line-soft);
  border-radius: 8px;
  background: rgba(11, 15, 20, 0.35);
  color: var(--text);
  font: inherit;
  font-size: 12px;
  line-height: 1.3;
  text-align: left;
  cursor: default;
}
.relic-summary-main.clickable {
  cursor: pointer;
}
.relic-summary.expandable .relic-summary-main:hover {
  border-color: color-mix(in srgb, var(--cyan, #5ec8c8) 45%, transparent);
  background: color-mix(in srgb, var(--cyan, #5ec8c8) 10%, transparent);
}
.relic-summary.expanded .relic-summary-main {
  border-color: color-mix(in srgb, var(--cyan, #5ec8c8) 40%, transparent);
  background: color-mix(in srgb, var(--cyan, #5ec8c8) 12%, transparent);
}
.relic-chevron {
  flex-shrink: 0;
  color: var(--cyan, #5ec8c8);
  font-size: 11px;
  width: 0.9em;
}
.relic-count {
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
  font-weight: 700;
  letter-spacing: 0.02em;
  color: var(--text-mute);
}
.relic-count em {
  font-style: normal;
  color: var(--cyan, #5ec8c8);
}
.relic-preview-name {
  flex: 0 1 auto;
  min-width: 0;
  max-width: 9.5em;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
}
.relic-jump {
  flex-shrink: 0;
  font-size: 11px;
}
</style>
