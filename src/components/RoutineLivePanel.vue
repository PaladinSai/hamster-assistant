<script setup lang="ts">
/**
 * 日常/周常条目的时效详情壳：按 kind 分发到 routine/* 子面板。
 * 裂隙 / 平原 / 动态页电波另见 FissurePanel、PlainsPanel、NightwavePanel。
 */
import type { WarframeListItem, WeaponFamilyListItem } from "../lib/api";
import type { LiveDetail } from "./routine/liveTypes";
import ArchimedeaLiveDetail from "./routine/ArchimedeaLiveDetail.vue";
import ArchonLiveDetail from "./routine/ArchonLiveDetail.vue";
import CircuitLiveDetail from "./routine/CircuitLiveDetail.vue";
import HexLiveDetail from "./routine/HexLiveDetail.vue";
import NightwaveLiveDetail from "./routine/NightwaveLiveDetail.vue";
import SortieLiveDetail from "./routine/SortieLiveDetail.vue";
import SteelLiveDetail from "./routine/SteelLiveDetail.vue";

withDefaults(
  defineProps<{
    detail: LiveDetail;
    families?: WarframeListItem[];
    weaponFamilies?: WeaponFamilyListItem[];
  }>(),
  { families: () => [], weaponFamilies: () => [] },
);

const emit = defineEmits<{ ended: [] }>();
</script>

<template>
  <div class="live-panel">
    <SortieLiveDetail
      v-if="detail.kind === 'sortie'"
      :detail="detail"
      @ended="emit('ended')"
    />
    <NightwaveLiveDetail
      v-else-if="detail.kind === 'nightwave'"
      :detail="detail"
      @ended="emit('ended')"
    />
    <SteelLiveDetail
      v-else-if="detail.kind === 'steel'"
      :detail="detail"
      @ended="emit('ended')"
    />
    <ArchonLiveDetail
      v-else-if="detail.kind === 'archon'"
      :detail="detail"
      @ended="emit('ended')"
    />
    <ArchimedeaLiveDetail
      v-else-if="detail.kind === 'archimedea'"
      :detail="detail"
      @ended="emit('ended')"
    />
    <CircuitLiveDetail
      v-else-if="detail.kind === 'circuit'"
      :detail="detail"
      :families="families"
      :weapon-families="weaponFamilies"
      @ended="emit('ended')"
    />
    <HexLiveDetail
      v-else-if="detail.kind === 'hex'"
      :detail="detail"
      @ended="emit('ended')"
    />
    <p v-else class="live-empty">{{ detail.summary }}</p>
  </div>
</template>

<style src="./routine/routineLive.css"></style>
