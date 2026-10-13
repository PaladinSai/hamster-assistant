<script setup lang="ts">
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
</script>

<template>
  <button
    v-if="mode === 'toggle' && drops.length"
    type="button"
    class="relic-toggle"
    @click="emit('toggle', partId)"
  >
    {{ expanded ? "▾" : "▸" }}
    {{ drops.length }} 个遗物
  </button>
  <span
    v-else-if="mode === 'toggle'"
    style="color: var(--text-mute)"
  >—</span>

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
