<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import { computed, ref, watch } from "vue";
import type { PartDto, WarframeDetail } from "../lib/api";
import {
  BASE_STATUS_OPTIONS,
  PART_STATUS_OPTIONS,
  vaultStatusLabel,
  warframeVariantLabel,
} from "../lib/status";
import { useExpandedSet } from "../composables/useExpandedSet";
import StatusPill from "./StatusPill.vue";
import PartRelicDrops from "./PartRelicDrops.vue";

const props = defineProps<{
  open: boolean;
  detail: WarframeDetail | null;
  loading: boolean;
}>();

const emit = defineEmits<{
  close: [];
  updateStatus: [status: number];
  updatePartStatus: [partId: string, status: number];
  updatePartNote: [partId: string, note: string];
  updateNote: [note: string];
  toggleWish: [];
  toggleHelminth: [];
  toggleForma: [];
  toggleMastered: [];
  openRelic: [relicKey: string];
}>();

const noteDraft = ref("");
const { isExpanded, toggleExpand, clearExpanded } = useExpandedSet();

watch(
  // 用 warframeId：避免 detail 对象替换（改状态/部件）时误清展开态
  () => props.detail?.warframe.warframeId,
  () => {
    if (!props.detail) return;
    noteDraft.value = props.detail.warframe.note ?? "";
    clearExpanded();
  },
);

watch(
  () => props.detail?.warframe.note,
  (n) => {
    if (props.detail) noteDraft.value = n ?? "";
  },
);

const warframe = computed(() => props.detail?.warframe ?? null);
const parts = computed<PartDto[]>(() => props.detail?.parts ?? []);
const isPrime = computed(() => warframe.value?.variant === 1);
const isBase = computed(() => warframe.value?.variant === 0);

function onNoteBlur() {
  if (!warframe.value) return;
  const next = noteDraft.value;
  const prev = warframe.value.note ?? "";
  if (next !== prev) emit("updateNote", next);
}

function nick() {
  if (!warframe.value) return "";
  return warframe.value.nickname || warframe.value.nameZh || "";
}

function wikiUrl() {
  if (!warframe.value) return "";
  const name = warframe.value.nameEn.trim().replace(/ /g, "_");
  return `https://warframe.huijiwiki.com/wiki/${encodeURIComponent(name)}`;
}

async function openPartWiki() {
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
</script>

<template>
  <div class="drawer-root" :class="{ open }" :aria-hidden="!open">
    <div class="drawer-mask" @click="emit('close')" />
    <aside
      class="drawer"
      role="dialog"
      :aria-label="warframe ? `${warframe.nameEn} 详情` : '详情'"
    >
      <template v-if="warframe">
        <div class="drawer-head">
          <div style="flex: 1; min-width: 0">
            <h3>
              {{ warframe.nameEn }}
              <span v-if="nick()" style="color: var(--text-dim); font-weight: 500">
                {{ nick() }}
              </span>
              <span class="pill" style="margin-left: 8px; font-size: 12px">
                {{ warframeVariantLabel(warframe.variant) }}
              </span>
            </h3>
          </div>
          <button
            class="ghost"
            type="button"
            :title="warframe.wish ? '取消心愿' : '加入心愿'"
            @click="emit('toggleWish')"
          >
            {{ warframe.wish ? "★ 心愿" : "☆ 心愿" }}
          </button>
          <button class="ghost" type="button" @click="emit('close')">关闭</button>
        </div>

        <div class="drawer-summary">
          <label class="summary-field">
            <span class="summary-label">状态</span>
            <select
              :value="warframe.status"
              @change="
                emit(
                  'updateStatus',
                  Number(($event.target as HTMLSelectElement).value),
                )
              "
            >
              <option
                v-for="o in BASE_STATUS_OPTIONS"
                :key="o.value"
                :value="o.value"
              >
                {{ o.label }}
              </option>
            </select>
          </label>
          <StatusPill
            v-if="isPrime"
            :label="vaultStatusLabel(warframe.vaultStatus, true)"
          />
          <button
            v-if="isBase"
            type="button"
            class="pill toggle"
            :class="warframe.helminth ? 'ok' : 'mute'"
            @click="emit('toggleHelminth')"
          >
            {{ warframe.helminth ? "已喂" : "未喂" }} Helminth
          </button>
          <button
            type="button"
            class="pill toggle"
            :class="warframe.mastered ? 'ok' : 'mute'"
            @click="emit('toggleMastered')"
          >
            {{ warframe.mastered ? "已掌握" : "未掌握" }}
          </button>
          <button
            type="button"
            class="pill toggle"
            :class="warframe.formaDone ? 'ok' : 'mute'"
            @click="emit('toggleForma')"
          >
            {{ warframe.formaDone ? "已挖" : "未挖" }}
          </button>
        </div>

        <div class="drawer-body">
          <div v-if="loading" style="color: var(--text-mute); margin-bottom: 12px">
            加载部件…
          </div>

          <table class="part-table">
            <colgroup>
              <col class="col-part" />
              <col class="col-status" />
              <col v-if="isPrime" class="col-relic" />
              <col class="col-note" />
            </colgroup>
            <thead>
              <tr>
                <th>部件</th>
                <th>状态</th>
                <th v-if="isPrime">遗物</th>
                <th>备注</th>
              </tr>
            </thead>
            <tbody>
              <template v-for="p in parts" :key="p.partId">
                <tr>
                  <td>
                    <button
                      type="button"
                      class="part-link"
                      title="在浏览器打开 Wiki"
                      @click="openPartWiki"
                    >
                      {{ p.nameZh }}
                    </button>
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
                        v-for="o in PART_STATUS_OPTIONS"
                        :key="o.value"
                        :value="o.value"
                      >
                        {{ o.label }}
                      </option>
                    </select>
                  </td>
                  <td v-if="isPrime">
                    <PartRelicDrops
                      mode="toggle"
                      :part-id="p.partId"
                      :drops="p.drops"
                      :expanded="isExpanded(p.partId)"
                      @toggle="toggleExpand"
                      @open-relic="openRelicInApp"
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
                  v-if="isPrime && isExpanded(p.partId) && p.drops.length"
                  class="relic-expand-row"
                >
                  <td :colspan="isPrime ? 4 : 3">
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
                <td :colspan="isPrime ? 4 : 3" style="color: var(--text-mute)">
                  暂无部件数据
                </td>
              </tr>
            </tbody>
          </table>

          <div class="detail-note">
            <div style="color: var(--text-mute); font-size: 11px; margin-bottom: 6px">
              备注
            </div>
            <textarea v-model="noteDraft" @blur="onNoteBlur" />
          </div>
        </div>
      </template>
    </aside>
  </div>
</template>
