<script setup lang="ts">
import type { HomeSection } from "../composables/useAppNav";
import type {
  ArcaneListItem,
  ModListItem,
  WarframeListItem,
  WeaponFamilyListItem,
} from "../lib/api";
import ProgressPanel from "./ProgressPanel.vue";
import RoutinePanel from "./RoutinePanel.vue";
import TodoPanel from "./TodoPanel.vue";
import WishList from "./WishList.vue";

defineProps<{
  section: HomeSection;
  families: WarframeListItem[];
  weaponFamilies: WeaponFamilyListItem[];
  mods: ModListItem[];
  arcanes: ArcaneListItem[];
  refreshToken?: number;
  /** 当前是否在首页。 */
  active?: boolean;
}>();

defineEmits<{
  open: [warframeId: string];
  openWeapon: [familyId: string, weaponId: string];
  openMod: [modId: string];
  openArcane: [arcaneId: string];
}>();
</script>

<template>
  <div class="home-stack">
    <ProgressPanel
      v-if="section === 'overview' || section === 'progress'"
      :families="families"
      :weapon-families="weaponFamilies"
    />
    <TodoPanel v-if="section === 'overview' || section === 'todo'" />
    <WishList
      v-if="section === 'overview' || section === 'wish'"
      :families="families"
      :weapon-families="weaponFamilies"
      :mods="mods"
      :arcanes="arcanes"
      @open="(id) => $emit('open', id)"
      @open-weapon="(fid, wid) => $emit('openWeapon', fid, wid)"
      @open-mod="(id) => $emit('openMod', id)"
      @open-arcane="(id) => $emit('openArcane', id)"
    />
    <RoutinePanel
      v-if="section === 'overview' || section === 'daily'"
      mode="daily"
      :families="families"
      :weapon-families="weaponFamilies"
      :refresh-token="refreshToken"
      :active="active !== false"
    />
    <RoutinePanel
      v-if="section === 'overview' || section === 'weekly'"
      mode="weekly"
      :families="families"
      :weapon-families="weaponFamilies"
      :refresh-token="refreshToken"
      :active="active !== false"
    />
  </div>
</template>

<style scoped>
.home-stack {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
</style>
