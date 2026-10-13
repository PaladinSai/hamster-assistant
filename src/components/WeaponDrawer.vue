<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import { computed, ref, watch } from "vue";
import type { WeaponDetailDto, WeaponFamilyDetail, WeaponPartDto } from "../lib/api";
import {
  INCARNON_OPTIONS,
  WEAPON_PART_STATUS_OPTIONS,
  WEAPON_SPECIAL_STATUS_OPTIONS,
  WEAPON_STATUS_OPTIONS,
  isSpecialWeaponVariant,
  weaponDisplayName,
  weaponVariantLabel,
  weaponVaultStatusLabel,
} from "../lib/status";
import { huijiWeaponWikiUrl } from "../lib/wiki/huiji";
import { useExpandedSet } from "../composables/useExpandedSet";
import PartRelicDrops from "./PartRelicDrops.vue";

const props = defineProps<{
  open: boolean;
  detail: WeaponFamilyDetail | null;
  loading: boolean;
  initialWeaponId?: string | null;
}>();

const emit = defineEmits<{
  close: [];
  toggleWish: [weaponId: string, wish: number];
  toggleMastered: [weaponId: string, mastered: number];
  updateWeaponStatus: [weaponId: string, status: number];
  updateWeaponIncarnon: [weaponId: string, incarnon: number];
  updateWeaponNote: [weaponId: string, note: string];
  updatePartStatus: [partId: string, status: number];
  updatePartHaveQty: [partId: string, haveQty: number];
  updatePartNote: [partId: string, note: string];
  openRelic: [relicKey: string];
}>();

const noteDraft = ref("");
const { isExpanded, toggleExpand, clearExpanded } = useExpandedSet();

const currentWeapon = computed<WeaponDetailDto | null>(() => {
  if (!props.detail || !props.initialWeaponId) return null;
  return (
    props.detail.weapons.find((w) => w.weaponId === props.initialWeaponId) ??
    null
  );
});

const titleName = computed(() => {
  const w = currentWeapon.value;
  if (!w) return "";
  return weaponDisplayName(w.nameZh, w.nameEn);
});

const isPrime = computed(() => currentWeapon.value?.variant === 1);
const isSpecial = computed(() =>
  currentWeapon.value ? isSpecialWeaponVariant(currentWeapon.value.variant) : false,
);
const showIncarnon = computed(() => (currentWeapon.value?.canIncarnon ?? 0) === 1);
const parts = computed<WeaponPartDto[]>(() => currentWeapon.value?.parts ?? []);

const statusOptions = computed(() =>
  isSpecial.value ? WEAPON_SPECIAL_STATUS_OPTIONS : WEAPON_STATUS_OPTIONS,
);

watch(
  () => [props.initialWeaponId, props.open] as const,
  () => {
    clearExpanded();
    noteDraft.value = currentWeapon.value?.note ?? "";
  },
);

watch(
  () => currentWeapon.value?.note,
  (n) => {
    if (currentWeapon.value) noteDraft.value = n ?? "";
  },
);

function onNoteBlur() {
  if (!currentWeapon.value) return;
  const next = noteDraft.value;
  const prev = currentWeapon.value.note ?? "";
  if (next !== prev) emit("updateWeaponNote", currentWeapon.value.weaponId, next);
}

function wikiUrl() {
  const w = currentWeapon.value;
  if (!w) return "";
  return huijiWeaponWikiUrl(w.nameZh, w.nameEn);
}

async function openWiki() {
  const url = wikiUrl();
  if (!url) return;
  try {
    await openUrl(url);
  } catch {
    // ignore
  }
}

function openRelicInApp(relicKey: string) {
  emit("openRelic", relicKey);
}

function variantClass(variant: number) {
  switch (variant) {
    case 1:
      return "var-prime";
    case 2:
      return "var-kuva";
    case 3:
      return "var-tenet";
    case 4:
      return "var-coda";
    default:
      return "var-base";
  }
}
</script>

<template>
  <div class="drawer-root" :class="{ open }" :aria-hidden="!open">
    <div class="drawer-mask" @click="emit('close')" />
    <aside
      class="drawer"
      role="dialog"
      :aria-label="titleName ? `${titleName} 详情` : '武器详情'"
    >
      <template v-if="currentWeapon">
        <div class="drawer-head">
          <div style="flex: 1; min-width: 0">
            <h3>{{ titleName }}</h3>
            <div class="sub">{{ currentWeapon.nameEn }}</div>
          </div>
          <button class="ghost" type="button" title="Wiki" @click="openWiki">
            Wiki
          </button>
          <button
            class="ghost"
            type="button"
            :title="currentWeapon.wish ? '取消心愿' : '加入心愿'"
            @click="
              emit(
                'toggleWish',
                currentWeapon!.weaponId,
                currentWeapon!.wish ? 0 : 1,
              )
            "
          >
            {{ currentWeapon.wish ? "★ 心愿" : "☆ 心愿" }}
          </button>
          <button class="ghost" type="button" @click="emit('close')">关闭</button>
        </div>

        <div class="drawer-summary">
          <span
            class="pill"
            :class="variantClass(currentWeapon.variant)"
          >{{ weaponVariantLabel(currentWeapon.variant) }}</span>
          <label class="summary-field">
            <span class="summary-label">整枪状态</span>
            <select
              :value="currentWeapon.status"
              @change="
                emit(
                  'updateWeaponStatus',
                  currentWeapon!.weaponId,
                  Number(($event.target as HTMLSelectElement).value),
                )
              "
            >
              <option
                v-for="o in statusOptions"
                :key="o.value"
                :value="o.value"
              >
                {{ o.label }}
              </option>
            </select>
          </label>
          <label v-if="showIncarnon" class="summary-field">
            <span class="summary-label">灵化</span>
            <select
              :value="currentWeapon.incarnon"
              aria-label="灵化状态"
              @change="
                emit(
                  'updateWeaponIncarnon',
                  currentWeapon!.weaponId,
                  Number(($event.target as HTMLSelectElement).value),
                )
              "
            >
              <option
                v-for="o in INCARNON_OPTIONS"
                :key="o.value"
                :value="o.value"
              >
                {{ o.label }}
              </option>
            </select>
          </label>
          <button
            type="button"
            class="pill toggle"
            :class="currentWeapon.mastered ? 'ok' : 'mute'"
            @click="
              emit(
                'toggleMastered',
                currentWeapon!.weaponId,
                currentWeapon!.mastered ? 0 : 1,
              )
            "
          >
            {{ currentWeapon.mastered ? "已掌握" : "未掌握" }}
          </button>
          <span
            v-if="isPrime"
            class="pill"
            :title="weaponVaultStatusLabel(currentWeapon.vaultStatus)"
          >{{ weaponVaultStatusLabel(currentWeapon.vaultStatus) }}</span>
        </div>

        <div class="drawer-body">
          <div v-if="loading" style="color: var(--text-mute); margin-bottom: 12px">
            加载中…
          </div>

          <template v-if="isSpecial">
            <p class="wep-own-hint">
              无铸造过程：击杀/兑换完成后整枪到手（可记价态等）。
            </p>
            <label class="family-note">
              <span class="summary-label">备注</span>
              <input
                v-model="noteDraft"
                type="text"
                placeholder="价态 / 来源…"
                @blur="onNoteBlur"
              />
            </label>
          </template>

          <template v-else-if="isPrime">
            <table class="part-table">
              <thead>
                <tr>
                  <th>部件</th>
                  <th style="width: 64px">数量</th>
                  <th style="width: 80px">已获取</th>
                  <th style="width: 100px">状态</th>
                  <th style="min-width: 120px">遗物</th>
                  <th>备注</th>
                </tr>
              </thead>
              <tbody>
                <template v-for="p in parts" :key="p.partId">
                  <tr>
                    <td>{{ p.nameZh }}</td>
                    <td>{{ p.needQty }}</td>
                    <td>
                      <input
                        class="qty-input"
                        type="number"
                        min="0"
                        :value="p.haveQty"
                        @change="
                          emit(
                            'updatePartHaveQty',
                            p.partId,
                            Number(($event.target as HTMLInputElement).value) ||
                              0,
                          )
                        "
                      />
                    </td>
                    <td>
                      <select
                        :value="p.status"
                        @change="
                          emit(
                            'updatePartStatus',
                            p.partId,
                            Number(($event.target as HTMLSelectElement).value),
                          )
                        "
                      >
                        <option
                          v-for="o in WEAPON_PART_STATUS_OPTIONS"
                          :key="o.value"
                          :value="o.value"
                        >
                          {{ o.label }}
                        </option>
                      </select>
                    </td>
                    <td>
                      <PartRelicDrops
                        mode="toggle"
                        :part-id="p.partId"
                        :drops="p.drops"
                        :expanded="isExpanded(p.partId)"
                        @toggle="toggleExpand"
                      />
                    </td>
                    <td>
                      <input
                        type="text"
                        :value="p.note ?? ''"
                        placeholder="备注…"
                        @change="
                          emit(
                            'updatePartNote',
                            p.partId,
                            ($event.target as HTMLInputElement).value,
                          )
                        "
                      />
                    </td>
                  </tr>
                  <tr
                    v-if="isExpanded(p.partId) && p.drops.length"
                    class="relic-expand-row"
                  >
                    <td colspan="6">
                      <PartRelicDrops
                        mode="list"
                        :part-id="p.partId"
                        :drops="p.drops"
                        @open-relic="openRelicInApp"
                      />
                    </td>
                  </tr>
                </template>
                <tr v-if="parts.length === 0">
                  <td colspan="6" style="color: var(--text-mute)">暂无部件数据</td>
                </tr>
              </tbody>
            </table>
            <label class="family-note">
              <span class="summary-label">备注</span>
              <textarea
                v-model="noteDraft"
                rows="2"
                placeholder="用户备注…"
                @blur="onNoteBlur"
              />
            </label>
          </template>

          <template v-else>
            <table class="part-table">
              <thead>
                <tr>
                  <th>部件</th>
                  <th style="width: 110px">状态</th>
                  <th>备注</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="p in parts" :key="p.partId">
                  <td>{{ p.nameZh }}</td>
                  <td>
                    <select
                      :value="p.status"
                      @change="
                        emit(
                          'updatePartStatus',
                          p.partId,
                          Number(($event.target as HTMLSelectElement).value),
                        )
                      "
                    >
                      <option
                        v-for="o in WEAPON_PART_STATUS_OPTIONS"
                        :key="o.value"
                        :value="o.value"
                      >
                        {{ o.label }}
                      </option>
                    </select>
                  </td>
                  <td>
                    <input
                      type="text"
                      :value="p.note ?? ''"
                      placeholder="备注…"
                      @change="
                        emit(
                          'updatePartNote',
                          p.partId,
                          ($event.target as HTMLInputElement).value,
                        )
                      "
                    />
                  </td>
                </tr>
                <tr v-if="parts.length === 0">
                  <td colspan="3" style="color: var(--text-mute)">暂无部件数据</td>
                </tr>
              </tbody>
            </table>
            <label class="family-note">
              <span class="summary-label">备注</span>
              <textarea
                v-model="noteDraft"
                rows="2"
                placeholder="用户备注…"
                @blur="onNoteBlur"
              />
            </label>
          </template>
        </div>
      </template>
      <div v-else-if="loading" class="drawer-body" style="color: var(--text-mute)">
        加载中…
      </div>
      <div
        v-else-if="open && detail"
        class="drawer-body"
        style="color: var(--text-mute)"
      >
        未找到该武器
      </div>
    </aside>
  </div>
</template>
