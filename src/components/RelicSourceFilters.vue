<script setup lang="ts">
import { computed, ref } from "vue";
import type { FilterChip, RelicSourceFilterOptions } from "../lib/api";

const eras = defineModel<number[]>("eras", { required: true });
const relicKeys = defineModel<string[]>("relicKeys", { required: true });
const rotations = defineModel<string[]>("rotations", { required: true });
const gameModes = defineModel<string[]>("gameModes", { required: true });
const planets = defineModel<string[]>("planets", { required: true });

const props = defineProps<{
  filterOpts: RelicSourceFilterOptions | null;
  loading: boolean;
}>();

const emit = defineEmits<{
  search: [];
  scheduleSearch: [];
  clearFilters: [];
}>();

const ERA_ALL = [
  { v: 0, label: "所有古纪" },
  { v: 1, label: "所有前纪" },
  { v: 2, label: "所有中纪" },
  { v: 3, label: "所有后纪" },
];

const relicsByEra = computed(() => {
  const groups = new Map<number, FilterChip[]>();
  for (const r of props.filterOpts?.relics ?? []) {
    const era = r.era ?? -1;
    const list = groups.get(era) ?? [];
    list.push(r);
    groups.set(era, list);
  }
  return ERA_ALL.filter((e) => groups.has(e.v)).map((e) => ({
    ...e,
    relics: groups.get(e.v) ?? [],
  }));
});

const relicAll = computed(
  () => eras.value.length === 0 && relicKeys.value.length === 0,
);
const rotAll = computed(() => rotations.value.length === 0);
const modeAll = computed(() => gameModes.value.length === 0);
const planetAll = computed(() => planets.value.length === 0);

const filtersOpen = ref(false);
const filterActiveCount = computed(
  () =>
    relicKeys.value.length +
    eras.value.length +
    rotations.value.length +
    gameModes.value.length +
    planets.value.length,
);
const filterToggleLabel = computed(() => {
  if (filtersOpen.value) return "收起筛选";
  const n = filterActiveCount.value;
  return n > 0 ? `筛选 (${n})` : "筛选";
});

function toggleNum(list: number[], v: number) {
  const i = list.indexOf(v);
  if (i >= 0) list.splice(i, 1);
  else list.push(v);
}

function toggleStr(list: string[], v: string) {
  const i = list.indexOf(v);
  if (i >= 0) list.splice(i, 1);
  else list.push(v);
}

function clearRelics() {
  eras.value = [];
  relicKeys.value = [];
  emit("scheduleSearch");
}
</script>

<template>
  <div class="relic-src-toolbar">
    <button
      class="qbtn"
      type="button"
      :class="{ on: filtersOpen || filterActiveCount > 0 }"
      @click="filtersOpen = !filtersOpen"
    >
      {{ filterToggleLabel }}
    </button>
    <button class="qbtn" type="button" @click="emit('clearFilters')">
      清空筛选
    </button>
    <button
      class="qbtn"
      type="button"
      :disabled="loading"
      @click="emit('search')"
    >
      {{ loading ? "搜索中…" : "搜索" }}
    </button>
  </div>

  <div v-show="filtersOpen" class="relic-src-filters">
    <div class="filter-block">
      <div class="filter-row">
        <span class="filter-label">虚空遗物</span>
        <button
          type="button"
          class="chip-toggle"
          :class="{ on: relicAll }"
          @click="clearRelics"
        >
          全部
        </button>
      </div>
      <div v-for="g in relicsByEra" :key="g.v" class="filter-row wrap">
        <span class="filter-label" />
        <button
          type="button"
          class="chip-toggle"
          :class="{ on: eras.includes(g.v) }"
          @click="toggleNum(eras, g.v); emit('scheduleSearch')"
        >
          {{ g.label }}
        </button>
        <button
          v-for="r in g.relics"
          :key="r.key"
          type="button"
          class="chip-toggle"
          :class="{ on: relicKeys.includes(r.key) }"
          @click="toggleStr(relicKeys, r.key); emit('scheduleSearch')"
        >
          {{ r.labelZh }}
        </button>
      </div>
    </div>

    <div class="filter-row wrap">
      <span class="filter-label">轮次</span>
      <button
        type="button"
        class="chip-toggle"
        :class="{ on: rotAll }"
        @click="rotations = []; emit('scheduleSearch')"
      >
        全部
      </button>
      <button
        v-for="r in filterOpts?.rotations ?? []"
        :key="r"
        type="button"
        class="chip-toggle"
        :class="{ on: rotations.includes(r) }"
        @click="toggleStr(rotations, r); emit('scheduleSearch')"
      >
        {{ r }}
      </button>
      <button
        v-if="filterOpts?.hasBlankRotation"
        type="button"
        class="chip-toggle"
        :class="{ on: rotations.includes('') }"
        @click="toggleStr(rotations, ''); emit('scheduleSearch')"
      >
        无轮次
      </button>
    </div>

    <div class="filter-row wrap">
      <span class="filter-label">任务类型</span>
      <button
        type="button"
        class="chip-toggle"
        :class="{ on: modeAll }"
        @click="gameModes = []; emit('scheduleSearch')"
      >
        全部
      </button>
      <button
        v-for="m in filterOpts?.modes ?? []"
        :key="m.key"
        type="button"
        class="chip-toggle"
        :class="{ on: gameModes.includes(m.key) }"
        @click="toggleStr(gameModes, m.key); emit('scheduleSearch')"
      >
        {{ m.labelZh }}
      </button>
    </div>

    <div class="filter-row wrap">
      <span class="filter-label">任务区域</span>
      <button
        type="button"
        class="chip-toggle"
        :class="{ on: planetAll }"
        @click="planets = []; emit('scheduleSearch')"
      >
        全部
      </button>
      <button
        v-for="p in filterOpts?.planets ?? []"
        :key="p.key"
        type="button"
        class="chip-toggle"
        :class="{ on: planets.includes(p.key) }"
        @click="toggleStr(planets, p.key); emit('scheduleSearch')"
      >
        {{ p.labelZh }}
      </button>
    </div>
  </div>
</template>
