<script setup lang="ts">
import { computed } from "vue";
import type { WarframeListItem, WeaponFamilyListItem } from "../lib/api";

const props = defineProps<{
  families: WarframeListItem[];
  weaponFamilies: WeaponFamilyListItem[];
}>();

const baseRows = computed(() => props.families.filter((f) => f.variant === 0));
const primeRows = computed(() => props.families.filter((f) => f.variant === 1));
const ownedBase = computed(
  () => baseRows.value.filter((f) => f.status === 3).length,
);
const withPrime = computed(() => primeRows.value.length);
const ownedPrime = computed(
  () => primeRows.value.filter((f) => f.status === 3).length,
);

function weaponSlotStats(slot: number) {
  let total = 0;
  let owned = 0;
  for (const f of props.weaponFamilies) {
    if (f.slot !== slot) continue;
    for (const m of f.members) {
      total += 1;
      if (m.status === 1) owned += 1;
    }
  }
  return { owned, total };
}

const primaryStats = computed(() => weaponSlotStats(0));
const secondaryStats = computed(() => weaponSlotStats(1));
const meleeStats = computed(() => weaponSlotStats(2));
</script>

<template>
  <div class="hero-row">
    <div class="card">
      <h2>战甲进度</h2>
      <div class="stat-grid stat-grid--2">
        <div class="stat">
          <div class="n">
            {{ ownedBase }}<em>/{{ baseRows.length || "—" }}</em>
          </div>
          <div class="l">本体已获取</div>
          <div class="progress-bar">
            <i
              :style="{
                width:
                  baseRows.length > 0
                    ? `${(ownedBase / baseRows.length) * 100}%`
                    : '0%',
              }"
            />
          </div>
        </div>
        <div class="stat">
          <div class="n">
            {{ ownedPrime }}<em>/{{ withPrime || "—" }}</em>
          </div>
          <div class="l">Prime 已获取</div>
          <div class="progress-bar">
            <i
              :style="{
                width: `${(ownedPrime / Math.max(withPrime, 1)) * 100}%`,
              }"
            />
          </div>
        </div>
      </div>
    </div>

    <div class="card">
      <h2>武器进度</h2>
      <div class="stat-grid">
        <div class="stat">
          <div class="n">
            {{ primaryStats.owned }}<em>/{{ primaryStats.total || "—" }}</em>
          </div>
          <div class="l">主武器</div>
          <div class="progress-bar">
            <i
              :style="{
                width:
                  primaryStats.total > 0
                    ? `${(primaryStats.owned / primaryStats.total) * 100}%`
                    : '0%',
              }"
            />
          </div>
        </div>
        <div class="stat">
          <div class="n">
            {{ secondaryStats.owned }}<em>/{{ secondaryStats.total || "—" }}</em>
          </div>
          <div class="l">副武器</div>
          <div class="progress-bar">
            <i
              :style="{
                width:
                  secondaryStats.total > 0
                    ? `${(secondaryStats.owned / secondaryStats.total) * 100}%`
                    : '0%',
              }"
            />
          </div>
        </div>
        <div class="stat">
          <div class="n">
            {{ meleeStats.owned }}<em>/{{ meleeStats.total || "—" }}</em>
          </div>
          <div class="l">近战</div>
          <div class="progress-bar">
            <i
              :style="{
                width:
                  meleeStats.total > 0
                    ? `${(meleeStats.owned / meleeStats.total) * 100}%`
                    : '0%',
              }"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
