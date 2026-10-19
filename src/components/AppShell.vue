<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  STANDING_NAV_ITEMS,
  STANDING_SECTION_TAG,
  useAppNav,
  type StandingSection,
} from "../composables/useAppNav";
import { useArcaneProgress } from "../composables/useArcaneProgress";
import { useCatalogData } from "../composables/useCatalogData";
import { useCatalogUpdates } from "../composables/useCatalogUpdates";
import { useModProgress } from "../composables/useModProgress";
import { useWarframeProgress } from "../composables/useWarframeProgress";
import { useWeaponProgress } from "../composables/useWeaponProgress";
import {
  getStandingDaily,
  getStandingRanks,
  type StandingDailyPrefs,
  type StandingRanksPrefs,
} from "../lib/api";
import { useBackendReload } from "../composables/useBackendReload";
import { useAppUpdater } from "../composables/useAppUpdater";
import ArcaneListView from "./ArcaneListView.vue";
import HomeView from "./HomeView.vue";
import ModListView from "./ModListView.vue";
import PagePane from "./PagePane.vue";
import RelicSourceView from "./RelicSourceView.vue";
import StandingView from "./StandingView.vue";
import SuitListPage from "./SuitListPage.vue";
import ToastStack from "./ToastStack.vue";
import WarframeDrawer from "./WarframeDrawer.vue";
import WeaponDrawer from "./WeaponDrawer.vue";
import WeaponListView from "./WeaponListView.vue";
import WishView from "./WishView.vue";

const relicFocusKey = ref<string | null>(null);
const standingRanks = ref<StandingRanksPrefs>({ ranks: {} });
const standingDaily = ref<StandingDailyPrefs>({
  dayKey: "",
  done: {},
  weekKey: "",
  kahlDone: false,
});

function standingRankOf(section: Exclude<StandingSection, "overview">) {
  const tag = STANDING_SECTION_TAG[section];
  const v = standingRanks.value.ranks[tag];
  return typeof v === "number" && Number.isFinite(v) ? v : 0;
}

/** 侧栏完成提示：已完成 / 未完成（含无日常势力的今日上交） */
function standingDoneMark(section: Exclude<StandingSection, "overview">) {
  if (section === "kahl") {
    const done = standingDaily.value.kahlDone;
    return done
      ? { text: "✓", kind: "done" as const, title: "本周已完成" }
      : { text: "○", kind: "todo" as const, title: "本周未完成" };
  }
  const tag = STANDING_SECTION_TAG[section];
  const done = standingDaily.value.done[tag] === true;
  return done
    ? { text: "✓", kind: "done" as const, title: "今日已完成" }
    : { text: "○", kind: "todo" as const, title: "今日未完成" };
}

async function loadStandingSidebar() {
  try {
    const [r, d] = await Promise.all([getStandingRanks(), getStandingDaily()]);
    standingRanks.value = r;
    standingDaily.value = d;
  } catch {
    /* 侧栏状态失败不挡导航 */
  }
}

function onStandingRanksUpdated(prefs: StandingRanksPrefs) {
  standingRanks.value = prefs;
}

function onStandingDailyUpdated(prefs: StandingDailyPrefs) {
  standingDaily.value = prefs;
}

const catalog = useCatalogData();
const {
  families,
  archwingFamilies,
  necramechFamilies,
  companionFamilies,
  weaponFamilies,
  archwingWeaponFamilies,
  mods,
  modTags,
  arcanes,
  error,
  loadFamilies,
  loadArchwingFamilies,
  loadNecramechFamilies,
  loadCompanionFamilies,
  loadWeaponFamilies,
  loadArchwingWeaponFamilies,
  loadMods,
  loadArcanes,
  ensureLoaded,
  preloadAll,
} = catalog;

const nav = useAppNav({ ensureLoaded, relicFocusKey });
const {
  collapsed,
  view,
  query,
  homeSection,
  homeNavOpen,
  feedSection,
  feedNavOpen,
  standingSection,
  standingNavOpen,
  pageTitle,
  pageHint,
  showTopbarCatalogTools,
  searchPlaceholder,
  setView,
  openHomeOverview,
  setHomeSection,
  toggleHomeNav,
  openFeedOverview,
  setFeedSection,
  toggleFeedNav,
  openStandingOverview,
  setStandingSection,
  toggleStandingNav,
  openRelicSource,
  onSearchInput,
} = nav;

const updates = useCatalogUpdates({ error });
const {
  updating,
  updateMsg,
  relicRefreshToken,
  homeRefreshToken,
  liveRefreshToken,
  runCatalogUpdate,
  topbarUpdateData: topbarUpdateForView,
  onBackendUpdateFinished,
} = updates;

function topbarUpdateData() {
  topbarUpdateForView(view.value);
}

/** 当前机体页 mode；非机体页为 null（模板用 v-else-if） */
const suitListMode = computed(() => {
  switch (view.value) {
    case "warframes":
      return "warframe" as const;
    case "archwing":
      return "archwing" as const;
    case "necramechs":
      return "necramech" as const;
    case "companions":
      return "companion" as const;
    default:
      return null;
  }
});

/** 当前机体页列表：战甲 / Archwing / 机甲 / 同伴共用进度 composable */
const activeSuitFamilies = computed(() => {
  if (view.value === "archwing") return archwingFamilies.value;
  if (view.value === "necramechs") return necramechFamilies.value;
  if (view.value === "companions") return companionFamilies.value;
  return families.value;
});

const warframe = useWarframeProgress({
  families: activeSuitFamilies as typeof families,
  error,
  view,
});
const {
  filter,
  statusFilter,
  companionTypeFilter,
  selectedId,
  detail,
  detailLoading,
  markingAll: warframeMarkingAll,
  closeDrawer,
  openWarframe,
  toggleHelminth,
  toggleForma,
  toggleMastered,
  updateStatus,
  updateStatusFromDrawer,
  updatePartStatus,
  updatePartNote,
  updateNote,
  toggleWish,
  toggleWishFromDrawer,
  toggleHelminthFromDrawer,
  toggleFormaFromDrawer,
  toggleMasteredFromDrawer,
  markAllOwned: markAllWarframesOwned,
} = warframe;

const weapon = useWeaponProgress({
  weaponFamilies,
  archwingWeaponFamilies,
  error,
  loadWeaponFamilies,
  loadArchwingWeaponFamilies,
  view,
});
const {
  weaponSlot,
  archwingWeaponSlot,
  weaponKindFilter,
  weaponProgressFilter,
  weaponVaultFilter,
  weaponIncarnonFilter,
  selectedWeaponFamilyId,
  selectedWeaponId,
  weaponDetail,
  weaponDetailLoading,
  closeWeaponDrawer,
  openWeapon,
  openWeaponFromHome,
  updateWeaponStatus,
  updateWeaponIncarnon,
  updateWeaponNote,
  updateWeaponPartStatus,
  updateWeaponPartHaveQty,
  toggleWeaponWish,
  toggleWeaponMastered,
  updateWeaponPartNote,
  markingAll: weaponMarkingAll,
  markAllOwned: markAllWeaponsOwned,
} = weapon;

const mod = useModProgress({
  mods,
  modTags,
  error,
  loadMods,
  setView,
});
const {
  modTagFilter,
  modKindFilter,
  modProgressFilter,
  selectedModId,
  markingAll: modMarkingAll,
  toggleModWish,
  updateModStatus,
  updateModNote,
  updateModTag,
  onCreateModTag,
  onDeleteModTag,
  openModFromHome,
  markAllOwned: markAllModsOwned,
} = mod;

const arcane = useArcaneProgress({
  arcanes,
  error,
  loadArcanes,
  setView,
  view,
  query,
});
const {
  arcaneSourceFilter,
  arcaneKindFilter,
  arcaneRarityFilter,
  arcaneProgressFilter,
  selectedArcaneId,
  focusArcaneId,
  clearArcaneFocus,
  markingAll: arcaneMarkingAll,
  toggleArcaneWish,
  updateArcaneStatus,
  updateArcaneRank,
  updateArcaneNote,
  openArcaneFromHome,
  markAllOwned: markAllArcanesOwned,
} = arcane;

function onKeydown(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  if (
    selectedId.value != null &&
    (view.value === "warframes" ||
      view.value === "archwing" ||
      view.value === "necramechs" ||
      view.value === "companions")
  ) {
    closeDrawer();
  } else if (
    selectedWeaponFamilyId.value != null &&
    (view.value === "weapons" || view.value === "archwingWeapons")
  ) {
    closeWeaponDrawer();
  }
}

useBackendReload({
  liveRefreshToken,
  homeRefreshToken,
  relicRefreshToken,
  loadFamilies,
  loadArchwingFamilies,
  loadNecramechFamilies,
  loadCompanionFamilies,
  loadWeaponFamilies,
  loadArchwingWeaponFamilies,
  loadMods,
  loadArcanes,
  onBackendUpdateFinished,
});

watch(liveRefreshToken, () => {
  void loadStandingSidebar();
});

const {
  checking: updateChecking,
  installing: updateInstalling,
  checkAppUpdate,
  scheduleIdleUpdateCheck,
  ensureCurrentVersion,
} = useAppUpdater();

onMounted(() => {
  document.addEventListener("keydown", onKeydown);
  // 后台预热本地列表；时效网络由 Rust 线程拉，事件订阅在 useBackendReload
  preloadAll();
  void ensureCurrentVersion();
  scheduleIdleUpdateCheck();
  ensureLoaded("home");
  // 侧栏声望勾选状态延后，避免和首屏读库挤在一起
  window.setTimeout(() => {
    void loadStandingSidebar();
  }, 400);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div class="app" :class="{ collapsed }">
    <aside class="rail">
      <div class="brand">
        <div class="brand-mark">仓</div>
        <div class="brand-text">
          <strong>仓鼠小助手</strong>
          <span>Warframe 收藏管理</span>
        </div>
      </div>
      <div class="rail-nav">
      <div class="nav-group" :class="{ open: homeNavOpen && !collapsed }">
        <button
          class="nav-btn"
          type="button"
          :class="{ active: view === 'home' }"
          @click="openHomeOverview()"
        >
          <span class="nav-ico">⌂</span><span class="nav-label">首页</span>
          <span
            class="nav-chevron"
            :title="homeNavOpen ? '收起' : '展开'"
            @click.stop="toggleHomeNav()"
          >{{ homeNavOpen ? "▾" : "▸" }}</span>
        </button>
        <div v-show="homeNavOpen && !collapsed" class="nav-sub">
          <button
            class="nav-btn nav-sub-btn"
            type="button"
            :class="{ active: view === 'home' && homeSection === 'progress' }"
            @click="setHomeSection('progress')"
          >
            <span class="nav-ico">进</span><span class="nav-label">进度</span>
          </button>
          <button
            class="nav-btn nav-sub-btn"
            type="button"
            :class="{ active: view === 'home' && homeSection === 'todo' }"
            @click="setHomeSection('todo')"
          >
            <span class="nav-ico">待</span><span class="nav-label">待办</span>
          </button>
          <button
            class="nav-btn nav-sub-btn"
            type="button"
            :class="{ active: view === 'home' && homeSection === 'wish' }"
            @click="setHomeSection('wish')"
          >
            <span class="nav-ico">愿</span><span class="nav-label">心愿</span>
          </button>
          <button
            class="nav-btn nav-sub-btn"
            type="button"
            :class="{ active: view === 'home' && homeSection === 'daily' }"
            @click="setHomeSection('daily')"
          >
            <span class="nav-ico">日</span><span class="nav-label">日常</span>
          </button>
          <button
            class="nav-btn nav-sub-btn"
            type="button"
            :class="{ active: view === 'home' && homeSection === 'weekly' }"
            @click="setHomeSection('weekly')"
          >
            <span class="nav-ico">周</span><span class="nav-label">周常</span>
          </button>
        </div>
      </div>
      <div class="nav-group" :class="{ open: feedNavOpen && !collapsed }">
        <button
          class="nav-btn"
          type="button"
          :class="{ active: view === 'feed' }"
          @click="openFeedOverview()"
        >
          <span class="nav-ico">动</span><span class="nav-label">动态</span>
          <span
            class="nav-chevron"
            :title="feedNavOpen ? '收起' : '展开'"
            @click.stop="toggleFeedNav()"
          >{{ feedNavOpen ? "▾" : "▸" }}</span>
        </button>
        <div v-show="feedNavOpen && !collapsed" class="nav-sub">
          <button
            class="nav-btn nav-sub-btn"
            type="button"
            :class="{ active: view === 'feed' && feedSection === 'plains' }"
            @click="setFeedSection('plains')"
          >
            <span class="nav-ico">原</span><span class="nav-label">平原</span>
          </button>
          <button
            class="nav-btn nav-sub-btn"
            type="button"
            :class="{ active: view === 'feed' && feedSection === 'rebirth' }"
            @click="setFeedSection('rebirth')"
          >
            <span class="nav-ico">生</span><span class="nav-label">重生</span>
          </button>
          <button
            class="nav-btn nav-sub-btn"
            type="button"
            :class="{ active: view === 'feed' && feedSection === 'shop' }"
            @click="setFeedSection('shop')"
          >
            <span class="nav-ico">店</span><span class="nav-label">商店</span>
          </button>
          <button
            class="nav-btn nav-sub-btn"
            type="button"
            :class="{ active: view === 'feed' && feedSection === 'fissures' }"
            @click="setFeedSection('fissures')"
          >
            <span class="nav-ico">隙</span><span class="nav-label">裂隙</span>
          </button>
          <button
            class="nav-btn nav-sub-btn"
            type="button"
            :class="{ active: view === 'feed' && feedSection === 'arbitration' }"
            @click="setFeedSection('arbitration')"
          >
            <span class="nav-ico">裁</span><span class="nav-label">仲裁</span>
          </button>
          <button
            class="nav-btn nav-sub-btn"
            type="button"
            :class="{ active: view === 'feed' && feedSection === 'nightwave' }"
            @click="setFeedSection('nightwave')"
          >
            <span class="nav-ico">电</span><span class="nav-label">午夜电波</span>
          </button>
        </div>
      </div>
      <div class="nav-group" :class="{ open: standingNavOpen && !collapsed }">
        <button
          class="nav-btn"
          type="button"
          :class="{ active: view === 'standing' }"
          @click="openStandingOverview()"
        >
          <span class="nav-ico">望</span><span class="nav-label">声望</span>
          <span
            class="nav-chevron"
            :title="standingNavOpen ? '收起' : '展开'"
            @click.stop="toggleStandingNav()"
          >{{ standingNavOpen ? "▾" : "▸" }}</span>
        </button>
        <div v-show="standingNavOpen && !collapsed" class="nav-sub">
          <button
            v-for="item in STANDING_NAV_ITEMS"
            :key="item.section"
            class="nav-btn nav-sub-btn"
            type="button"
            :class="{ active: view === 'standing' && standingSection === item.section }"
            @click="setStandingSection(item.section)"
          >
            <span class="nav-ico">·</span>
            <span class="nav-label">
              {{ item.label }}
              <span class="nav-standing-rank">{{ standingRankOf(item.section) }}</span>
              <span
                v-for="mark in [standingDoneMark(item.section)]"
                :key="item.section + '-done'"
                class="nav-standing-done"
                :class="mark.kind"
                :title="mark.title"
              >{{ mark.text }}</span>
            </span>
          </button>
        </div>
      </div>
      <button
        class="nav-btn"
        type="button"
        :class="{ active: view === 'warframes' }"
        @click="setView('warframes')"
      >
        <span class="nav-ico">甲</span><span class="nav-label">战甲</span>
      </button>
      <button
        class="nav-btn"
        type="button"
        :class="{ active: view === 'archwing' }"
        @click="setView('archwing')"
      >
        <span class="nav-ico">翼</span><span class="nav-label">Archwing</span>
      </button>
      <button
        class="nav-btn"
        type="button"
        :class="{ active: view === 'necramechs' }"
        @click="setView('necramechs')"
      >
        <span class="nav-ico">机</span><span class="nav-label">机甲</span>
      </button>
      <button
        class="nav-btn"
        type="button"
        :class="{ active: view === 'companions' }"
        @click="setView('companions')"
      >
        <span class="nav-ico">伴</span><span class="nav-label">同伴</span>
      </button>
      <button
        class="nav-btn"
        type="button"
        :class="{ active: view === 'weapons' }"
        @click="setView('weapons')"
      >
        <span class="nav-ico">武</span><span class="nav-label">武器</span>
      </button>
      <button
        class="nav-btn"
        type="button"
        :class="{ active: view === 'archwingWeapons' }"
        @click="setView('archwingWeapons')"
      >
        <span class="nav-ico">空</span><span class="nav-label">空战武器</span>
      </button>
      <button
        class="nav-btn"
        type="button"
        :class="{ active: view === 'mods' }"
        @click="setView('mods')"
      >
        <span class="nav-ico">M</span><span class="nav-label">Mod</span>
      </button>
      <button
        class="nav-btn"
        type="button"
        :class="{ active: view === 'arcanes' }"
        @click="setView('arcanes')"
      >
        <span class="nav-ico">赋</span><span class="nav-label">赋能</span>
      </button>
      <button
        class="nav-btn"
        type="button"
        :class="{ active: view === 'relics' }"
        @click="setView('relics')"
      >
        <span class="nav-ico">遗</span><span class="nav-label">遗物来源表</span>
      </button>
      </div>
      <div class="rail-foot">
        <button
          class="nav-btn"
          type="button"
          title="折叠侧栏"
          @click="collapsed = !collapsed"
        >
          <span class="nav-ico">⇄</span><span class="nav-label">折叠侧栏</span>
        </button>
        <span v-show="!collapsed" class="rail-clan">Dawn Empire氏族</span>
      </div>
    </aside>

    <section class="main">
      <header class="topbar">
        <h1>{{ pageTitle }}</h1>
        <span class="hint">{{ pageHint }}</span>
        <div class="spacer" />
        <span v-if="updating" class="chip">更新中…</span>
        <span v-else-if="updateMsg" class="chip" :title="updateMsg">已更新</span>
        <button
          class="ghost"
          type="button"
          title="从 GitHub Releases 检查应用版本"
          :disabled="updateChecking || updateInstalling"
          @click="checkAppUpdate(true)"
        >
          {{
            updateInstalling
              ? "安装更新…"
              : updateChecking
                ? "检查中…"
                : "检查更新"
          }}
        </button>
        <button
          v-if="view === 'home' || view === 'feed' || view === 'standing'"
          class="ghost"
          type="button"
          :disabled="updating"
          @click="topbarUpdateData"
        >
          {{ updating ? "更新中…" : "更新数据" }}
        </button>
        <template v-if="showTopbarCatalogTools">
          <button
            class="ghost"
            type="button"
            :disabled="updating"
            @click="topbarUpdateData"
          >
            {{ updating ? "更新中…" : "更新数据" }}
          </button>
          <input
            v-model="query"
            class="search"
            type="search"
            :placeholder="searchPlaceholder"
            @input="onSearchInput"
          />
        </template>
      </header>
      <div class="content">
        <div v-if="error" class="banner">
          <strong>错误</strong> {{ error }}
        </div>
        <!-- 按需挂载：勿用 v-show 同时挂首页/动态/声望，冷启动 IPC 会把前端卡住 -->
        <PagePane v-if="view === 'home'">
          <WishView
            :section="homeSection"
            :families="families"
            :weapon-families="weaponFamilies"
            :mods="mods"
            :arcanes="arcanes"
            :refresh-token="liveRefreshToken"
            :active="view === 'home'"
            @open="openWarframe"
            @open-weapon="openWeaponFromHome"
            @open-mod="openModFromHome"
            @open-arcane="openArcaneFromHome"
          />
        </PagePane>
        <PagePane v-else-if="view === 'feed'">
          <HomeView
            :section="feedSection"
            :families="families"
            :weapon-families="weaponFamilies"
            :refresh-token="homeRefreshToken"
            :live-refresh-token="liveRefreshToken"
            @open="openWarframe"
            @open-weapon="openWeaponFromHome"
          />
        </PagePane>
        <PagePane v-else-if="view === 'standing'">
          <StandingView
            :section="standingSection"
            :refresh-token="liveRefreshToken"
            :active="view === 'standing'"
            @ranks-updated="onStandingRanksUpdated"
            @daily-updated="onStandingDailyUpdated"
          />
        </PagePane>
        <PagePane v-else-if="view === 'relics'">
          <RelicSourceView
            :initial-relic-key="relicFocusKey"
            :external-query="query"
            :updating="updating"
            :refresh-token="relicRefreshToken"
            :active="view === 'relics'"
          />
        </PagePane>
        <PagePane v-else-if="suitListMode">
          <SuitListPage
            :mode="suitListMode"
            :families="activeSuitFamilies"
            :filter="filter"
            :status-filter="statusFilter"
            :companion-type-filter="companionTypeFilter"
            :query="query"
            :selected-id="selectedId"
            :updating="updating"
            :marking-all="warframeMarkingAll"
            @update:filter="filter = $event"
            @update:status-filter="statusFilter = $event"
            @update:companion-type-filter="companionTypeFilter = $event"
            @select="openWarframe"
            @toggle-helminth="toggleHelminth"
            @toggle-forma="toggleForma"
            @toggle-mastered="toggleMastered"
            @toggle-wish="toggleWish"
            @update-status="updateStatus"
            @mark-all-owned="markAllWarframesOwned"
            @update-data="runCatalogUpdate('warframes')"
          />
        </PagePane>
        <PagePane v-else-if="view === 'weapons'">
          <WeaponListView
            mode="weapon"
            :families="weaponFamilies"
            :slot="weaponSlot"
            :kind-filter="weaponKindFilter"
            :progress-filter="weaponProgressFilter"
            :vault-filter="weaponVaultFilter"
            :incarnon-filter="weaponIncarnonFilter"
            :query="query"
            :selected-family-id="selectedWeaponFamilyId"
            :selected-weapon-id="selectedWeaponId"
            :updating="updating"
            :marking-all="weaponMarkingAll"
            @update:slot="weaponSlot = $event"
            @update:kind-filter="weaponKindFilter = $event"
            @update:progress-filter="weaponProgressFilter = $event"
            @update:vault-filter="weaponVaultFilter = $event"
            @update:incarnon-filter="weaponIncarnonFilter = $event"
            @select-weapon="openWeapon"
            @toggle-wish="toggleWeaponWish"
            @toggle-mastered="toggleWeaponMastered"
            @update-weapon-status="updateWeaponStatus"
            @update-weapon-incarnon="updateWeaponIncarnon"
            @update-weapon-note="updateWeaponNote"
            @mark-all-owned="markAllWeaponsOwned"
            @update-data="runCatalogUpdate('weapons')"
          />
        </PagePane>
        <PagePane v-else-if="view === 'archwingWeapons'">
          <WeaponListView
            mode="archwing"
            :families="archwingWeaponFamilies"
            :slot="archwingWeaponSlot"
            :kind-filter="weaponKindFilter"
            :progress-filter="weaponProgressFilter"
            :vault-filter="weaponVaultFilter"
            :incarnon-filter="weaponIncarnonFilter"
            :query="query"
            :selected-family-id="selectedWeaponFamilyId"
            :selected-weapon-id="selectedWeaponId"
            :updating="updating"
            :marking-all="weaponMarkingAll"
            @update:slot="archwingWeaponSlot = $event"
            @update:kind-filter="weaponKindFilter = $event"
            @update:progress-filter="weaponProgressFilter = $event"
            @update:vault-filter="weaponVaultFilter = $event"
            @update:incarnon-filter="weaponIncarnonFilter = $event"
            @select-weapon="openWeapon"
            @toggle-wish="toggleWeaponWish"
            @toggle-mastered="toggleWeaponMastered"
            @update-weapon-status="updateWeaponStatus"
            @update-weapon-incarnon="updateWeaponIncarnon"
            @update-weapon-note="updateWeaponNote"
            @mark-all-owned="markAllWeaponsOwned"
            @update-data="runCatalogUpdate('weapons')"
          />
        </PagePane>
        <PagePane v-else-if="view === 'mods'">
          <ModListView
            :mods="mods"
            :tags="modTags"
            :tag-filter="modTagFilter"
            :kind-filter="modKindFilter"
            :progress-filter="modProgressFilter"
            :query="query"
            :selected-id="selectedModId"
            :updating="updating"
            :marking-all="modMarkingAll"
            @update:tag-filter="modTagFilter = $event"
            @update:kind-filter="modKindFilter = $event"
            @update:progress-filter="modProgressFilter = $event"
            @toggle-wish="toggleModWish"
            @update-status="updateModStatus"
            @update-note="updateModNote"
            @update-tag="updateModTag"
            @create-tag="onCreateModTag"
            @delete-tag="onDeleteModTag"
            @select="selectedModId = $event"
            @mark-all-owned="markAllModsOwned"
            @update-data="runCatalogUpdate('mods')"
          />
        </PagePane>
        <PagePane v-else-if="view === 'arcanes'">
          <ArcaneListView
            :arcanes="arcanes"
            :source-filter="arcaneSourceFilter"
            :kind-filter="arcaneKindFilter"
            :rarity-filter="arcaneRarityFilter"
            :progress-filter="arcaneProgressFilter"
            :query="query"
            :selected-id="selectedArcaneId"
            :focus-id="focusArcaneId"
            :updating="updating"
            :marking-all="arcaneMarkingAll"
            @update:source-filter="arcaneSourceFilter = $event"
            @update:kind-filter="arcaneKindFilter = $event"
            @update:rarity-filter="arcaneRarityFilter = $event"
            @update:progress-filter="arcaneProgressFilter = $event"
            @toggle-wish="toggleArcaneWish"
            @update-status="updateArcaneStatus"
            @update-rank="updateArcaneRank"
            @update-note="updateArcaneNote"
            @select="selectedArcaneId = $event"
            @clear-focus="clearArcaneFocus"
            @mark-all-owned="markAllArcanesOwned"
            @update-data="runCatalogUpdate('arcanes')"
          />
        </PagePane>
      </div>
    </section>
  </div>

  <WarframeDrawer
    :open="
      selectedId != null &&
      (view === 'warframes' ||
        view === 'archwing' ||
        view === 'necramechs' ||
        view === 'companions')
    "
    :detail="detail"
    :loading="detailLoading"
    @close="closeDrawer"
    @update-status="updateStatusFromDrawer"
    @update-part-status="updatePartStatus"
    @update-part-note="updatePartNote"
    @update-note="updateNote"
    @toggle-wish="toggleWishFromDrawer"
    @toggle-helminth="toggleHelminthFromDrawer"
    @toggle-forma="toggleFormaFromDrawer"
    @toggle-mastered="toggleMasteredFromDrawer"
    @open-relic="openRelicSource"
  />

  <WeaponDrawer
    :open="
      selectedWeaponFamilyId != null &&
      (view === 'weapons' || view === 'archwingWeapons')
    "
    :detail="weaponDetail"
    :loading="weaponDetailLoading"
    :initial-weapon-id="selectedWeaponId"
    @close="closeWeaponDrawer"
    @toggle-wish="toggleWeaponWish"
    @toggle-mastered="toggleWeaponMastered"
    @update-weapon-status="updateWeaponStatus"
    @update-weapon-incarnon="updateWeaponIncarnon"
    @update-weapon-note="updateWeaponNote"
    @update-part-status="updateWeaponPartStatus"
    @update-part-have-qty="updateWeaponPartHaveQty"
    @update-part-note="updateWeaponPartNote"
    @open-relic="openRelicSource"
  />

  <ToastStack @open-standing="setStandingSection" />
</template>
