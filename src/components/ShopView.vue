<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { getWeeklyShopBrief, type WeeklyShopBrief } from "../lib/api";
import {
  buildWeeklyShops,
  nextMondayUtcIso,
  type ShopOffer,
  type WeeklyShop,
} from "../lib/shop/weeklyShops";
import LiveRemain from "./routine/LiveRemain.vue";

const props = defineProps<{
  refreshToken?: number;
}>();

type ShopTabKey =
  | "teshin"
  | "acrithis"
  | "bird3"
  | "palladino"
  | "nightcap"
  | "hunhow"
  | "yonta"
  | "flare"
  | "kaya"
  | "nightwave";

const SHOP_TABS: {
  key: ShopTabKey;
  label: string;
  ids: string[];
}[] = [
  { key: "teshin", label: "钢铁之路奖励", ids: ["teshin"] },
  { key: "acrithis", label: "言录使", ids: ["acrithis", "scuttler"] },
  { key: "bird3", label: "鸟三", ids: ["bird3"] },
  { key: "palladino", label: "钢铁守望", ids: ["palladino"] },
  { key: "nightcap", label: "夜帽", ids: ["nightcap"] },
  { key: "hunhow", label: "渡界之塔", ids: ["hunhow"] },
  { key: "yonta", label: "哲士扬塔", ids: ["yonta"] },
  { key: "flare", label: "弗莱尔纪念品", ids: ["flare"] },
  { key: "kaya", label: "卡娅商店", ids: ["kaya"] },
  { key: "nightwave", label: "午夜电波货币商店", ids: ["nightwave"] },
];

const live = ref<WeeklyShopBrief | null>(null);
const loading = ref(false);
const error = ref("");
const activeTab = ref<ShopTabKey>("teshin");
/** 周切换时 bump，驱动轮换货架重算（不必每秒重建）。 */
const shopEpoch = ref(0);

async function loadLive() {
  loading.value = true;
  try {
    live.value = await getWeeklyShopBrief();
    error.value = "";
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function onShopRemainEnded() {
  shopEpoch.value += 1;
  void loadLive();
}

onMounted(() => {
  void loadLive();
});

watch(
  () => props.refreshToken,
  (t, prev) => {
    if (t == null || t === prev || t === 0) return;
    void loadLive();
  },
);

const shops = computed(() => {
  void shopEpoch.value;
  return buildWeeklyShops(Date.now(), live.value);
});

const activeShops = computed<WeeklyShop[]>(() => {
  const tab = SHOP_TABS.find((t) => t.key === activeTab.value);
  if (!tab) return [];
  const byId = new Map(shops.value.map((s) => [s.id, s]));
  return tab.ids.map((id) => byId.get(id)).filter((s): s is WeeklyShop => !!s);
});

const shopResetExpiry = computed(() => {
  const exp = live.value?.teshinExpiry;
  if (exp && !Number.isNaN(new Date(exp).getTime())) return exp;
  return nextMondayUtcIso();
});

const sourceHint = computed(() => {
  if (!live.value) return loading.value ? "同步中…" : "";
  return live.value.source === "live"
    ? "钢铁之路阁下已实时同步"
    : "钢铁之路阁下为周序回退（网络未通）";
});

const expandedPool = ref<Record<string, boolean>>({});

function togglePool(id: string) {
  expandedPool.value = {
    ...expandedPool.value,
    [id]: !expandedPool.value[id],
  };
}

function offerClass(o: ShopOffer) {
  return { featured: !!o.featured };
}

function tabTitle(s: WeeklyShop) {
  const tab = SHOP_TABS.find((t) => t.ids.includes(s.id));
  if (s.id === "scuttler") return "言录使 · 急行蛛外壳兑换";
  if (tab && tab.ids.length === 1) return tab.label;
  return s.nameZh;
}
</script>

<template>
  <div class="shop-stack list-page">
    <div class="list-filters">
      <div class="toolbar">
        <div class="seg wrap">
          <button
            v-for="t in SHOP_TABS"
            :key="t.key"
            type="button"
            :class="{ on: activeTab === t.key }"
            @click="activeTab = t.key"
          >
            {{ t.label }}
          </button>
        </div>
      </div>
    </div>

    <p class="shop-lead">
      周限可购清单 ·
      <LiveRemain
        class="shop-remain"
        :expiry="shopResetExpiry"
        @ended="onShopRemainEnded"
      />
      <span class="shop-lead-sub">
        译名按灰机 wiki · {{ sourceHint || "打开本页自动拉取本周特供" }}
      </span>
    </p>
    <p v-if="error" class="shop-error">{{ error }}</p>

    <article
      v-for="s in activeShops"
      :key="s.id"
      class="card shop-card"
    >
      <header class="shop-head">
        <div>
          <h3 class="shop-name">{{ tabTitle(s) }}</h3>
          <p class="shop-meta">
            {{ s.vendorZh }} · {{ s.locationZh }}
          </p>
        </div>
        <div class="shop-tags">
          <span class="shop-tag">{{ s.currencyZh }}</span>
          <span class="shop-tag mute">{{ s.resetZh }}</span>
        </div>
      </header>
      <p v-if="s.noteZh" class="shop-note">{{ s.noteZh }}</p>

      <ul v-if="s.offers.length" class="offer-list">
        <li
          v-for="(o, i) in s.offers"
          :key="`${s.id}-o-${i}`"
          class="offer-row"
          :class="offerClass(o)"
        >
          <div class="offer-main">
            <span v-if="o.featured" class="offer-badge">本周</span>
            <span class="offer-name">{{ o.nameZh }}</span>
          </div>
          <div class="offer-side">
            <span class="offer-cost">{{ o.costZh }}</span>
            <span v-if="o.limitZh" class="offer-limit">{{ o.limitZh }}</span>
          </div>
        </li>
      </ul>

      <div v-if="s.rotationPool?.length" class="pool-block">
        <button
          type="button"
          class="pool-toggle"
          @click="togglePool(s.id)"
        >
          {{ expandedPool[s.id] ? "收起货池" : "查看完整轮换 / 货池" }}
          <span>{{ expandedPool[s.id] ? "▴" : "▾" }}</span>
        </button>
        <ul v-if="expandedPool[s.id]" class="offer-list pool">
          <li
            v-for="(o, i) in s.rotationPool"
            :key="`${s.id}-p-${i}`"
            class="offer-row"
            :class="{
              featured:
                s.offers.some((x) => x.featured && x.nameZh === o.nameZh),
            }"
          >
            <div class="offer-main">
              <span
                v-if="s.offers.some((x) => x.featured && x.nameZh === o.nameZh)"
                class="offer-badge"
              >本周</span>
              <span class="offer-name">{{ o.nameZh }}</span>
            </div>
            <div class="offer-side">
              <span class="offer-cost">{{ o.costZh }}</span>
              <span v-if="o.limitZh" class="offer-limit">{{ o.limitZh }}</span>
            </div>
          </li>
        </ul>
      </div>
    </article>
  </div>
</template>

<style scoped>
.shop-stack {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.seg.wrap {
  flex-wrap: wrap;
}
.shop-lead {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
  line-height: 1.5;
}
.shop-remain {
  color: var(--cyan);
  font-variant-numeric: tabular-nums;
  font-weight: 550;
}
.shop-lead-sub {
  display: block;
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-mute);
}
.shop-error {
  margin: 0;
  font-size: 12px;
  color: var(--danger);
}
.shop-card {
  padding: 14px 16px 12px;
}
.shop-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.shop-name {
  margin: 0;
  font-size: 16px;
  font-weight: 650;
}
.shop-meta {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-mute);
  line-height: 1.4;
}
.shop-tags {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
}
.shop-tag {
  font-size: 11px;
  padding: 3px 8px;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
  white-space: nowrap;
}
.shop-tag.mute {
  background: var(--bg-elevated);
  color: var(--text-mute);
}
.shop-note {
  margin: 10px 0 0;
  font-size: 12px;
  color: var(--text-dim);
  line-height: 1.5;
}
.offer-list {
  list-style: none;
  margin: 12px 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.offer-list.pool {
  margin-top: 8px;
}
.offer-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 10px;
  border-radius: 8px;
  background: var(--bg-elevated);
}
.offer-row.featured {
  background: var(--accent-soft);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 35%, transparent);
}
.offer-main {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.offer-badge {
  flex-shrink: 0;
  font-size: 11px;
  font-weight: 650;
  color: var(--accent);
  letter-spacing: 0.04em;
}
.offer-name {
  font-size: 13px;
  font-weight: 550;
  line-height: 1.35;
}
.offer-side {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  flex-shrink: 0;
  text-align: right;
}
.offer-cost {
  font-size: 12px;
  color: var(--text);
  font-variant-numeric: tabular-nums;
}
.offer-limit {
  font-size: 11px;
  color: var(--text-mute);
}
.pool-block {
  margin-top: 10px;
}
.pool-toggle {
  appearance: none;
  border: none;
  background: transparent;
  color: var(--text-mute);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
  padding: 0;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.pool-toggle:hover {
  color: var(--accent);
}
</style>
