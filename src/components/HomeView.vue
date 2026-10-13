<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import type { FeedSection } from "../composables/useAppNav";
import {
  getResurgence,
  type ResurgenceSummary,
  type WarframeListItem,
  type WeaponFamilyListItem,
} from "../lib/api";
import { baseStatusLabel, weaponStatusLabel } from "../lib/status";
import ArbitrationPanel from "./ArbitrationPanel.vue";
import FissurePanel from "./FissurePanel.vue";
import NightwavePanel from "./NightwavePanel.vue";
import PlainsPanel from "./PlainsPanel.vue";
import ShopView from "./ShopView.vue";

const props = defineProps<{
  section: FeedSection;
  families: WarframeListItem[];
  weaponFamilies: WeaponFamilyListItem[];
  refreshToken?: number;
  liveRefreshToken?: number;
}>();

const emit = defineEmits<{
  open: [warframeId: string];
  openWeapon: [familyId: string, weaponId: string];
}>();

const resurgence = ref<ResurgenceSummary | null>(null);
const viewingNext = ref(false);

async function loadResurgence() {
  try {
    resurgence.value = await getResurgence();
  } catch {
    /* keep previous */
  }
}

onMounted(() => {
  void loadResurgence();
});

watch(
  () => props.refreshToken,
  (t, prev) => {
    if (t == null || t === prev || t === 0) return;
    void loadResurgence();
  },
);

const primeRows = computed(() => props.families.filter((f) => f.variant === 1));

const rebirthWarframes = computed(() => {
  if (resurgence.value?.warframes?.length) {
    return resurgence.value.warframes;
  }
  return primeRows.value
    .filter((f) => f.vaultStatus === 3)
    .map((f) => ({
      familyId: f.familyId,
      familyKey: f.familyKey,
      nameEn: f.nameEn,
      nameZh: f.nameZh,
      nickname: f.nickname,
      warframeId: f.warframeId,
    }));
});

const rebirthWeapons = computed(() => resurgence.value?.weapons ?? []);

function nick(f: { nickname: string | null; nameZh: string | null }) {
  return f.nickname || f.nameZh || "";
}

function formatExpiry(iso: string | null | undefined) {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatRange(start: string | null | undefined, end: string | null | undefined) {
  const a = formatExpiry(start);
  const b = formatExpiry(end);
  if (a && b) return `${a} – ${b}`;
  if (b) return `至 ${b}`;
  if (a) return `自 ${a}`;
  return "";
}

const nextWarframes = computed(() => resurgence.value?.next?.warframes ?? []);

const hasNext = computed(() => nextWarframes.value.length > 0);

watch(hasNext, (ok) => {
  if (!ok) viewingNext.value = false;
});

function toggleResurgencePeriod() {
  if (!hasNext.value) return;
  viewingNext.value = !viewingNext.value;
}

function openRebirth(item: {
  warframeId?: string | null;
  familyId?: string | null;
} | string | null) {
  if (item == null) return;
  if (typeof item === "string") {
    const prime = props.families.find(
      (f) => f.familyId === item && f.variant === 1,
    );
    if (prime) emit("open", prime.warframeId);
    return;
  }
  if (item.warframeId) {
    emit("open", item.warframeId);
    return;
  }
  if (!item.familyId) return;
  const prime = props.families.find(
    (f) => f.familyId === item.familyId && f.variant === 1,
  );
  if (prime) emit("open", prime.warframeId);
}

function openRebirthWeapon(w: {
  familyId: string | null;
  weaponId: string | null;
}) {
  if (w.familyId && w.weaponId) emit("openWeapon", w.familyId, w.weaponId);
}

function familyProgress(familyKey: string) {
  const rows = props.families.filter((x) => x.familyKey === familyKey);
  if (!rows.length) return "";
  const base = rows.find((x) => x.variant === 0);
  const prime = rows.find((x) => x.variant === 1);
  const baseLabel = base ? baseStatusLabel(base.status) : "—";
  const primeLabel = prime ? baseStatusLabel(prime.status) : "尚无";
  return `本体 ${baseLabel} · Prime ${primeLabel}`;
}

function weaponProgressMeta(w: {
  familyId: string | null;
  weaponId: string | null;
}) {
  if (!w.familyId || !w.weaponId) return "未入库";
  for (const f of props.weaponFamilies) {
    if (f.familyId !== w.familyId) continue;
    const m = f.members.find((x) => x.weaponId === w.weaponId);
    if (m) return weaponStatusLabel(m.status);
  }
  return "重生中";
}
</script>

<template>
  <div class="feed-stack">
  <PlainsPanel
    v-if="section === 'plains'"
    :refresh-token="liveRefreshToken"
  />

  <div
    v-if="section === 'rebirth'"
    class="card"
  >
    <h2 class="resurgence-title">
      <span>{{ viewingNext ? "下期重生" : "本期重生" }}</span>
      <button
        v-if="hasNext"
        type="button"
        class="resurgence-arrow"
        :aria-label="viewingNext ? '返回本期' : '查看下期'"
        @click="toggleResurgencePeriod"
      >
        {{ viewingNext ? "‹" : "›" }}
      </button>
      <span
        v-if="!viewingNext && resurgence?.expiry"
        class="resurgence-when"
      >
        至 {{ formatExpiry(resurgence.expiry) }}
      </span>
      <span
        v-else-if="viewingNext && resurgence?.next"
        class="resurgence-when"
      >
        {{ formatRange(resurgence.next.activation, resurgence.next.expiry) }}
      </span>
    </h2>

    <template v-if="viewingNext">
      <div class="wish-list">
        <div
          v-for="f in nextWarframes"
          :key="`next-${f.familyKey}`"
          class="wish-item"
          :class="{ muted: !f.familyId }"
          @click="openRebirth(f.familyId)"
        >
          <span class="tag">战甲</span>
          <span class="name">
            {{ f.nameEn }}
            <span style="color: var(--text-dim); font-weight: 500">{{ nick(f) }}</span>
          </span>
          <span class="meta">预告</span>
        </div>
      </div>
    </template>
    <template v-else>
      <p
        v-if="rebirthWarframes.length === 0 && rebirthWeapons.length === 0"
        style="margin: 0; color: var(--text-dim); line-height: 1.6"
      >
        当前没有重生轮数据。启动会静默同步，也可点「更新数据」强制拉取。
      </p>
      <template v-else>
        <div v-if="rebirthWarframes.length" class="wish-list" style="margin-bottom: 12px">
          <div
            v-for="f in rebirthWarframes"
            :key="f.familyKey"
            class="wish-item"
            :class="{ muted: !f.familyId }"
            @click="openRebirth(f.familyId)"
          >
            <span class="tag">战甲</span>
            <span class="name">
              {{ f.nameEn }}
              <span style="color: var(--text-dim); font-weight: 500">{{ nick(f) }}</span>
            </span>
            <span class="meta">{{ familyProgress(f.familyKey) || "重生中" }}</span>
          </div>
        </div>
        <div v-if="rebirthWeapons.length" class="wish-list">
          <div
            v-for="(w, i) in rebirthWeapons"
            :key="`${w.nameEn}-${i}`"
            class="wish-item"
            :class="{ muted: !(w.familyId && w.weaponId) }"
            @click="openRebirthWeapon(w)"
          >
            <span class="tag">武器</span>
            <span class="name">{{ w.nameZh || w.nameEn }}</span>
            <span class="meta">{{ weaponProgressMeta(w) }}</span>
          </div>
        </div>
        <p
          v-if="resurgence && resurgence.relicCount > 0"
          style="margin: 10px 0 0; color: var(--text-mute); font-size: 12px"
        >
          本期相关核桃约 {{ resurgence.relicCount }} 种（抽屉遗物会优先标「重生」）
        </p>
      </template>
    </template>
  </div>

  <ShopView
    v-if="section === 'shop'"
    :refresh-token="liveRefreshToken"
  />

  <FissurePanel
    v-if="section === 'fissures'"
    :refresh-token="liveRefreshToken"
  />

  <ArbitrationPanel
    v-if="section === 'arbitration'"
    :refresh-token="liveRefreshToken"
  />

  <NightwavePanel
    v-if="section === 'nightwave'"
    :refresh-token="liveRefreshToken"
  />
  </div>
</template>

<style scoped>
.feed-stack {
  display: flex;
  flex-direction: column;
  gap: 22px;
}
.resurgence-title {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px 0;
}

.resurgence-arrow {
  appearance: none;
  box-sizing: border-box;
  margin-left: 8px;
  width: 28px;
  height: 28px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--line);
  background: var(--bg-elevated);
  color: var(--accent);
  font-size: 18px;
  font-weight: 600;
  line-height: 1;
  cursor: pointer;
  border-radius: 6px;
}

.resurgence-arrow:hover {
  color: var(--accent);
  background: var(--accent-soft);
  border-color: var(--accent);
}

.resurgence-when {
  margin-left: 8px;
  color: var(--text-mute);
  font-weight: 500;
  font-size: 12px;
}
</style>
