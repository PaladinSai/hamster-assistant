<script setup lang="ts">
import { computed } from "vue";
import type {
  ArcaneListItem,
  ModListItem,
  WarframeListItem,
  WeaponFamilyListItem,
} from "../lib/api";
import { buildWishRows, type WishRow } from "../lib/wish";

const props = defineProps<{
  families: WarframeListItem[];
  weaponFamilies: WeaponFamilyListItem[];
  mods: ModListItem[];
  arcanes: ArcaneListItem[];
}>();

const emit = defineEmits<{
  open: [warframeId: string];
  openWeapon: [familyId: string, weaponId: string];
  openMod: [modId: string];
  openArcane: [arcaneId: string];
}>();

const rows = computed(() =>
  buildWishRows({
    warframes: props.families,
    weaponFamilies: props.weaponFamilies,
    mods: props.mods,
    arcanes: props.arcanes,
  }),
);

const hasAnyWish = computed(() => rows.value.length > 0);

function onOpen(row: WishRow) {
  if (row.kind === "warframe") emit("open", row.warframeId);
  else if (row.kind === "weapon") emit("openWeapon", row.familyId, row.weaponId);
  else if (row.kind === "mod") emit("openMod", row.modId);
  else emit("openArcane", row.arcaneId);
}
</script>

<template>
  <div class="card">
    <h2>星标关注</h2>
    <div class="wish-list">
      <div
        v-for="row in rows"
        :key="row.key"
        class="wish-item"
        @click="onOpen(row)"
      >
        <span class="tag">{{ row.tag }}</span>
        <span class="name">
          {{ row.nameEn }}
          <span v-if="row.nick" style="color: var(--text-dim); font-weight: 500">
            {{ row.nick }}
          </span>
        </span>
        <span class="meta">{{ row.meta }}</span>
      </div>
      <div v-if="!hasAnyWish" style="color: var(--text-mute)">
        暂无星标，可在列表里点 ★
      </div>
    </div>
  </div>
</template>
