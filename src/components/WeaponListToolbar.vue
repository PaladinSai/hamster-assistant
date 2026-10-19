<script setup lang="ts">
import { computed } from "vue";
import type {
  WeaponIncarnonFilter,
  WeaponKindFilter,
  WeaponProgressFilter,
  WeaponVaultFilter,
} from "../lib/status";

const props = withDefaults(
  defineProps<{
    mode?: "weapon" | "archwing";
    weaponSlot: number;
    kindFilter: WeaponKindFilter;
    progressFilter: WeaponProgressFilter;
    vaultFilter: WeaponVaultFilter;
    incarnonFilter: WeaponIncarnonFilter;
    rowCount: number;
    updating?: boolean;
    markingAll?: boolean;
    pendingOwnedCount?: number;
  }>(),
  { mode: "weapon" },
);

const emit = defineEmits<{
  "update:weaponSlot": [value: number];
  "update:kindFilter": [value: WeaponKindFilter];
  "update:progressFilter": [value: WeaponProgressFilter];
  "update:vaultFilter": [value: WeaponVaultFilter];
  "update:incarnonFilter": [value: WeaponIncarnonFilter];
  markAllOwned: [];
}>();

const isModularSlot = computed(
  () => props.mode === "weapon" && props.weaponSlot === 3,
);

const slots = computed(() =>
  props.mode === "archwing"
    ? [
        { value: -1, label: "全部" },
        { value: 0, label: "枪械" },
        { value: 2, label: "近战" },
      ]
    : [
        { value: -1, label: "全部" },
        { value: 0, label: "主武器" },
        { value: 1, label: "副武器" },
        { value: 2, label: "近战武器" },
        { value: 3, label: "自制" },
      ],
);

const kinds = computed((): { key: WeaponKindFilter; label: string }[] =>
  isModularSlot.value
    ? [
        { key: "all", label: "全部" },
        { key: "kitgun", label: "组合枪" },
        { key: "zaw", label: "Zaw" },
        { key: "amp", label: "增幅器" },
      ]
    : [
        { key: "all", label: "全部" },
        { key: "kuva", label: "赤毒" },
        { key: "tenet", label: "信条" },
        { key: "coda", label: "终幕" },
        { key: "prime", label: "Prime" },
        { key: "incarnon", label: "灵化" },
      ],
);

const showKindRow = computed(() => props.mode !== "archwing");

const progressOpts = computed((): { key: WeaponProgressFilter; label: string }[] =>
  isModularSlot.value
    ? [
        { key: "all", label: "全部" },
        { key: "missing", label: "未获取" },
        { key: "owned", label: "已获取" },
      ]
    : [
        { key: "all", label: "全部" },
        { key: "missing", label: "未获取" },
        { key: "progress", label: "蓝图已齐" },
        { key: "owned", label: "已获取" },
      ],
);

const vaultOpts: { key: WeaponVaultFilter; label: string }[] = [
  { key: "all", label: "全部" },
  { key: "unvaulted", label: "未入库" },
  { key: "vaulted", label: "已入库" },
  { key: "resurgence", label: "重生中" },
];

const incarnonOpts: { key: WeaponIncarnonFilter; label: string }[] = [
  { key: "all", label: "全部" },
  { key: "missing", label: "未获取" },
  { key: "ready", label: "已获取灵化之源" },
  { key: "done", label: "已灵化" },
];

const slotLabel = computed(
  () => slots.value.find((s) => s.value === props.weaponSlot)?.label ?? "",
);

const countUnit = computed(() => (isModularSlot.value ? "件" : "把"));

const showProgressThird = computed(
  () =>
    props.mode === "archwing" ||
    isModularSlot.value ||
    props.kindFilter !== "incarnon",
);
const showVaultThird = computed(
  () =>
    props.mode !== "archwing" &&
    !isModularSlot.value &&
    props.kindFilter === "prime",
);
const showIncarnonThird = computed(
  () =>
    props.mode !== "archwing" &&
    !isModularSlot.value &&
    props.kindFilter === "incarnon",
);

function onSlotClick(value: number) {
  emit("update:weaponSlot", value);
  emit("update:kindFilter", "all");
  emit("update:progressFilter", "all");
  emit("update:vaultFilter", "all");
  emit("update:incarnonFilter", "all");
}

function onKindClick(key: WeaponKindFilter) {
  emit("update:kindFilter", key);
  emit("update:progressFilter", "all");
  emit("update:vaultFilter", "all");
  emit("update:incarnonFilter", "all");
}
</script>

<template>
  <div class="toolbar">
    <div class="seg">
      <button
        v-for="s in slots"
        :key="s.value"
        type="button"
        :class="{ on: weaponSlot === s.value }"
        @click="onSlotClick(s.value)"
      >
        {{ s.label }}
      </button>
    </div>
    <span style="color: var(--text-mute); font-size: 12px">
      {{ slotLabel }} · {{ rowCount }} {{ countUnit
      }}{{ isModularSlot ? " · 枪膛 / 击打部 / 棱镜" : " · 多部件点行打开抽屉" }}
    </span>
  </div>

  <div v-if="showKindRow" class="toolbar" style="padding-top: 0">
    <div class="seg">
      <button
        v-for="k in kinds"
        :key="k.key"
        type="button"
        :class="{ on: kindFilter === k.key }"
        @click="onKindClick(k.key)"
      >
        {{ k.label }}
      </button>
    </div>
  </div>

  <div v-if="showProgressThird" class="toolbar" style="padding-top: 0">
    <div class="seg">
      <button
        v-for="o in progressOpts"
        :key="o.key"
        type="button"
        :class="{ on: progressFilter === o.key }"
        @click="emit('update:progressFilter', o.key)"
      >
        {{ o.label }}
      </button>
    </div>
    <button
      class="ghost"
      type="button"
      :disabled="updating || markingAll || (pendingOwnedCount ?? 0) === 0"
      @click="emit('markAllOwned')"
    >
      {{ markingAll ? "处理中…" : "一键全有" }}
    </button>
  </div>

  <div v-if="showVaultThird" class="toolbar" style="padding-top: 0">
    <div class="seg">
      <button
        v-for="o in vaultOpts"
        :key="o.key"
        type="button"
        :class="{ on: vaultFilter === o.key }"
        @click="emit('update:vaultFilter', o.key)"
      >
        {{ o.label }}
      </button>
    </div>
  </div>

  <div v-if="showIncarnonThird" class="toolbar" style="padding-top: 0">
    <div class="seg">
      <button
        v-for="o in incarnonOpts"
        :key="o.key"
        type="button"
        :class="{ on: incarnonFilter === o.key }"
        @click="emit('update:incarnonFilter', o.key)"
      >
        {{ o.label }}
      </button>
    </div>
    <button
      class="ghost"
      type="button"
      :disabled="updating || markingAll || (pendingOwnedCount ?? 0) === 0"
      @click="emit('markAllOwned')"
    >
      {{ markingAll ? "处理中…" : "一键全有" }}
    </button>
  </div>
</template>
