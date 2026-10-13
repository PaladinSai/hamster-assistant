<script setup lang="ts">
/**
 * 战甲 / Archwing / 机甲 / 同伴列表薄包装：统一 WarframeListView 绑定，
 * AppShell 按 view 选 mode + families，不再复制四段模板。
 */
import type { WarframeListItem } from "../lib/api";
import type { FilterKey, WarframeStatusFilter } from "../lib/status";
import type { CompanionTypeFilter } from "../composables/useWarframeProgress";
import WarframeListView from "./WarframeListView.vue";

defineProps<{
  mode: "warframe" | "archwing" | "necramech" | "companion";
  families: WarframeListItem[];
  filter: FilterKey;
  statusFilter: WarframeStatusFilter;
  companionTypeFilter: CompanionTypeFilter;
  query: string;
  selectedId: string | null;
  updating: boolean;
  markingAll: boolean;
}>();

defineEmits<{
  "update:filter": [value: FilterKey];
  "update:statusFilter": [value: WarframeStatusFilter];
  "update:companionTypeFilter": [value: CompanionTypeFilter];
  select: [warframeId: string];
  toggleHelminth: [item: WarframeListItem];
  toggleForma: [item: WarframeListItem];
  toggleMastered: [item: WarframeListItem];
  toggleWish: [item: WarframeListItem];
  updateStatus: [warframeId: string, status: number];
  markAllOwned: [items: { warframeId: string; status: number }[]];
  updateData: [];
}>();
</script>

<template>
  <WarframeListView
    :mode="mode"
    :families="families"
    :filter="filter"
    :status-filter="statusFilter"
    :companion-type-filter="companionTypeFilter"
    :query="query"
    :selected-id="selectedId"
    :updating="updating"
    :marking-all="markingAll"
    @update:filter="$emit('update:filter', $event)"
    @update:status-filter="$emit('update:statusFilter', $event)"
    @update:companion-type-filter="$emit('update:companionTypeFilter', $event)"
    @select="$emit('select', $event)"
    @toggle-helminth="$emit('toggleHelminth', $event)"
    @toggle-forma="$emit('toggleForma', $event)"
    @toggle-mastered="$emit('toggleMastered', $event)"
    @toggle-wish="$emit('toggleWish', $event)"
    @update-status="(id, status) => $emit('updateStatus', id, status)"
    @mark-all-owned="$emit('markAllOwned', $event)"
    @update-data="$emit('updateData')"
  />
</template>
