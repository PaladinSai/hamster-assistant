<script setup lang="ts">
import { computed, onMounted, ref, toRef, watch } from "vue";
import {
  KAHL_MISSIONS,
  STANDING_NAV_ITEMS,
  STANDING_NO_DAILY_SECTIONS,
  STANDING_SECTION_TAG,
  type StandingSection,
} from "../composables/useAppNav";
import { useBountyCache } from "../composables/useLiveCache";
import {
  getBountyWatch,
  getStandingDaily,
  getStandingRanks,
  setBountyWatch,
  setStandingDaily,
  setStandingRanks,
  standingRankMax,
  type BountyBoardDto,
  type BountyWatchPrefs,
  type StandingDailyPrefs,
  type StandingRanksPrefs,
  type SyndicateWatch,
  type SyndicateWatchCatalog,
  type WatchChoice,
} from "../lib/api";
import { statusPillKind } from "../lib/status";
import LiveRemain from "./routine/LiveRemain.vue";

const props = defineProps<{
  section: StandingSection;
  refreshToken?: number;
  /** 当前是否在声望页。 */
  active?: boolean;
}>();

const emit = defineEmits<{
  ranksUpdated: [prefs: StandingRanksPrefs];
  dailyUpdated: [prefs: StandingDailyPrefs];
}>();

const refreshTokenRef = toRef(props, "refreshToken");
const { brief, reload: reloadBounty } = useBountyCache(refreshTokenRef);

const watchPrefs = ref<BountyWatchPrefs>({
  watches: [],
  lastNotified: [],
});
const watchCatalog = ref<SyndicateWatchCatalog[]>([]);
const selectedTag = ref("");
const rewardQuery = ref("");
const challengeQuery = ref("");

const CYCLE_WATCH_TAGS = new Set([
  "ZarimanSyndicate",
  "EntratiLabSyndicate",
  "HexSyndicate",
]);
const ranksPrefs = ref<StandingRanksPrefs>({ ranks: {} });
const dailyPrefs = ref<StandingDailyPrefs>({
  dayKey: "",
  done: {},
  weekKey: "",
  kahlDone: false,
});
const error = ref("");
const saving = ref(false);
const watchOpen = ref(false);
function rankOptionsFor(tag: string) {
  const max = standingRankMax(tag);
  return Array.from({ length: max + 1 }, (_, i) => i);
}

type StandingCard =
  | { kind: "bounty"; tag: string; name: string; board: BountyBoardDto | null }
  | { kind: "no-daily"; tag: string; name: string }
  | { kind: "kahl"; tag: string; name: string };

async function loadPrefs() {
  try {
    const [w, r, d] = await Promise.all([
      getBountyWatch(),
      getStandingRanks(),
      getStandingDaily(),
    ]);
    watchPrefs.value = w.prefs;
    watchCatalog.value = w.catalog;
    if (
      !selectedTag.value ||
      !w.catalog.some((c) => c.tag === selectedTag.value)
    ) {
      selectedTag.value = w.catalog[0]?.tag ?? "";
    }
    ranksPrefs.value = r;
    dailyPrefs.value = d;
    emit("ranksUpdated", r);
    emit("dailyUpdated", d);
    error.value = "";
  } catch (e) {
    error.value = String(e);
  }
}

onMounted(() => {
  void reloadBounty();
  void loadPrefs();
  void ensureNotifyPermission();
});

watch(
  () => props.active,
  (active) => {
    if (active !== false) void reloadBounty();
  },
);

watch(
  () => props.refreshToken,
  (token, prev) => {
    if (token == null || token === prev) return;
    void loadPrefs();
  },
);

async function ensureNotifyPermission() {
  try {
    const { isPermissionGranted, requestPermission } = await import(
      "@tauri-apps/plugin-notification"
    );
    let granted = await isPermissionGranted();
    if (!granted) {
      const p = await requestPermission();
      granted = p === "granted";
    }
  } catch {
    /* 无插件或拒绝时忽略 */
  }
}

const boards = computed(() => brief.value?.boards ?? []);

const visibleCards = computed((): StandingCard[] => {
  const items =
    props.section === "overview"
      ? STANDING_NAV_ITEMS
      : STANDING_NAV_ITEMS.filter((x) => x.section === props.section);
  return items.map((item) => {
    const tag = STANDING_SECTION_TAG[item.section];
    if (item.section === "kahl") {
      return { kind: "kahl", tag, name: item.label };
    }
    if (STANDING_NO_DAILY_SECTIONS.has(item.section)) {
      return { kind: "no-daily", tag, name: item.label };
    }
    const board = boards.value.find((b) => b.tag === tag) ?? null;
    return {
      kind: "bounty",
      tag,
      name: board?.name ?? item.label,
      board,
    };
  });
});

const selectedCatalog = computed(
  () => watchCatalog.value.find((c) => c.tag === selectedTag.value) ?? null,
);

const selectedWatch = computed((): SyndicateWatch => {
  return (
    watchPrefs.value.watches.find((w) => w.tag === selectedTag.value) ?? {
      tag: selectedTag.value,
      jobs: [],
      rewards: [],
      challenges: [],
    }
  );
});

function watchCount(tag: string) {
  const row = watchPrefs.value.watches.find((w) => w.tag === tag);
  if (!row) return 0;
  return row.jobs.length + row.rewards.length + row.challenges.length;
}

const watchSummary = computed(() => {
  const n = watchCatalog.value.filter((c) => watchCount(c.tag) > 0).length;
  if (n === 0) return "未设置关注";
  return `已关注 ${n} 个势力`;
});

function isChosen(field: "jobs" | "rewards" | "challenges", id: string) {
  return selectedWatch.value[field].includes(id);
}

function matchesQuery(choices: WatchChoice[], query: string) {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  return choices
    .filter((c) => c.label.toLowerCase().includes(q))
    .slice(0, 40);
}

const rewardHits = computed(() =>
  matchesQuery(selectedCatalog.value?.rewards ?? [], rewardQuery.value),
);
const challengeHits = computed(() =>
  matchesQuery(selectedCatalog.value?.challenges ?? [], challengeQuery.value),
);

function rewardChoice(id: string): WatchChoice {
  return (
    selectedCatalog.value?.rewards.find((choice) => choice.id === id) ?? {
      id,
      label: id,
    }
  );
}

const pickedRewards = computed(() =>
  selectedWatch.value.rewards.map((id) => rewardChoice(id)),
);

function rewardMark(label: string) {
  return brief.value?.rewardMarks?.[label] ?? "";
}
const pickedChallenges = computed(() =>
  selectedWatch.value.challenges.map((id) => ({ id, label: id })),
);

async function persistWatch(next: BountyWatchPrefs) {
  saving.value = true;
  try {
    watchPrefs.value = await setBountyWatch(next);
    error.value = "";
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

function selectSyndicate(tag: string) {
  selectedTag.value = tag;
  rewardQuery.value = "";
  challengeQuery.value = "";
  watchOpen.value = true;
}

async function toggleChoice(
  field: "jobs" | "rewards" | "challenges",
  id: string,
) {
  const tag = selectedTag.value;
  if (!tag) return;
  const watches = watchPrefs.value.watches.map((w) => ({
    tag: w.tag,
    jobs: [...w.jobs],
    rewards: [...w.rewards],
    challenges: [...w.challenges],
  }));
  let row = watches.find((w) => w.tag === tag);
  if (!row) {
    row = { tag, jobs: [], rewards: [], challenges: [] };
    watches.push(row);
  }
  const list = row[field];
  const at = list.indexOf(id);
  if (at >= 0) list.splice(at, 1);
  else list.push(id);
  await persistWatch({
    watches: watches.filter(
      (w) => w.jobs.length || w.rewards.length || w.challenges.length,
    ),
    lastNotified: watchPrefs.value.lastNotified,
  });
}

function rankOf(tag: string) {
  const v = ranksPrefs.value.ranks[tag];
  return typeof v === "number" && Number.isFinite(v) ? v : 0;
}

function isDailyDone(tag: string) {
  return dailyPrefs.value.done[tag] === true;
}

async function persistDaily(next: StandingDailyPrefs) {
  saving.value = true;
  try {
    dailyPrefs.value = await setStandingDaily(next);
    emit("dailyUpdated", dailyPrefs.value);
    error.value = "";
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

async function toggleDailyDone(tag: string) {
  const done = { ...dailyPrefs.value.done };
  if (done[tag]) delete done[tag];
  else done[tag] = true;
  await persistDaily({ ...dailyPrefs.value, done });
}

async function toggleKahlDone() {
  await persistDaily({
    ...dailyPrefs.value,
    kahlDone: !dailyPrefs.value.kahlDone,
  });
}

async function onRankChange(tag: string, raw: string | number) {
  const n = typeof raw === "number" ? raw : Number(raw);
  const max = standingRankMax(tag);
  const rank = Number.isFinite(n)
    ? Math.max(0, Math.min(max, Math.trunc(n)))
    : 0;
  saving.value = true;
  try {
    ranksPrefs.value = await setStandingRanks({
      ranks: { ...ranksPrefs.value.ranks, [tag]: rank },
    });
    emit("ranksUpdated", ranksPrefs.value);
    error.value = "";
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
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

function advancedJob(board: BountyBoardDto) {
  return board.jobs.find((j) => j.isAdvanced) ?? null;
}

/** 科维兽 / 六人组：钢路档为同任务 +50% 声望（向上取整）。平原钢路是独立第六档，不套此规则。 */
const STEEL_STANDING_BONUS_TAGS = new Set([
  "EntratiLabSyndicate",
  "HexSyndicate",
]);

function steelStandingOf(
  tag: string,
  job: { standingTotal: number; isSteelPath: boolean },
): number | null {
  if (job.isSteelPath || job.standingTotal <= 0) return null;
  if (!STEEL_STANDING_BONUS_TAGS.has(tag)) return null;
  return Math.ceil(job.standingTotal * 1.5);
}

function standingLabel(
  tag: string,
  job: { standingTotal: number; isSteelPath: boolean },
): string {
  if (job.standingTotal <= 0) return "";
  if (job.isSteelPath) return `钢铁声望 ${job.standingTotal}`;
  const steel = steelStandingOf(tag, job);
  if (steel != null && steel !== job.standingTotal) {
    return `声望 ${job.standingTotal} · 钢铁 ${steel}`;
  }
  return `声望 ${job.standingTotal}`;
}

</script>

<template>
  <div class="standing-stack">
    <p v-if="error" class="standing-error">{{ error }}</p>

    <div class="card standing-watch">
      <button
        type="button"
        class="standing-watch-toggle"
        :aria-expanded="watchOpen"
        @click="watchOpen = !watchOpen"
      >
        <h2>关注设置</h2>
        <span class="standing-watch-summary">{{ watchSummary }}</span>
        <span class="standing-watch-caret">{{ watchOpen ? "▾" : "▸" }}</span>
      </button>
      <div v-show="watchOpen" class="standing-watch-body">
        <p class="standing-hint">
          先选势力，再勾它的任务或掉落。坚守者、科维兽、六人组还可以勾挑战。任务只看最高档，掉落和挑战任一档出现就提醒。只选势力不会提醒。
          {{ saving ? "保存中…" : "" }}
        </p>
        <div class="standing-watch-block">
          <h3>势力</h3>
          <div class="chip-row">
            <button
              v-for="c in watchCatalog"
              :key="c.tag"
              type="button"
              class="watch-chip"
              :class="{
                on: c.tag === selectedTag,
                armed: watchCount(c.tag) > 0,
              }"
              @click="selectSyndicate(c.tag)"
            >
              {{ c.name }}
              <template v-if="watchCount(c.tag) > 0">
                {{ watchCount(c.tag) }}
              </template>
            </button>
          </div>
        </div>
        <div v-if="selectedCatalog" class="standing-watch-block">
          <h3>任务</h3>
          <div class="chip-row">
            <button
              v-for="job in selectedCatalog.jobs"
              :key="job.id"
              type="button"
              class="watch-chip"
              :class="{ on: isChosen('jobs', job.id) }"
              :disabled="saving"
              @click="toggleChoice('jobs', job.id)"
            >
              {{ job.label }}
            </button>
          </div>
        </div>
        <div v-if="selectedCatalog" class="standing-watch-block">
          <h3>掉落</h3>
          <div v-if="pickedRewards.length" class="chip-row watch-picked">
            <button
              v-for="item in pickedRewards"
              :key="item.id"
              type="button"
              class="watch-chip on"
              :disabled="saving"
              @click="toggleChoice('rewards', item.id)"
            >
              {{ item.label }}
              <span
                v-if="item.mark"
                class="pill reward-mark"
                :class="statusPillKind(item.mark)"
              >
                {{ item.mark }}
              </span>
            </button>
          </div>
          <input
            v-model="rewardQuery"
            class="watch-search"
            type="search"
            placeholder="搜索掉落名称"
          />
          <p v-if="!rewardQuery.trim()" class="standing-empty soft">
            输入目标后点选
          </p>
          <p
            v-else-if="rewardHits.length === 0"
            class="standing-empty soft"
          >
            没有匹配的掉落
          </p>
          <div v-else class="chip-row">
            <button
              v-for="item in rewardHits"
              :key="item.id"
              type="button"
              class="watch-chip"
              :class="{ on: isChosen('rewards', item.id) }"
              :disabled="saving"
              @click="toggleChoice('rewards', item.id)"
            >
              {{ item.label }}
              <span
                v-if="item.mark"
                class="pill reward-mark"
                :class="statusPillKind(item.mark)"
              >
                {{ item.mark }}
              </span>
            </button>
          </div>
        </div>
        <div
          v-if="selectedCatalog && CYCLE_WATCH_TAGS.has(selectedTag)"
          class="standing-watch-block"
        >
          <h3>挑战</h3>
          <div v-if="pickedChallenges.length" class="chip-row watch-picked">
            <button
              v-for="item in pickedChallenges"
              :key="item.id"
              type="button"
              class="watch-chip on"
              :disabled="saving"
              @click="toggleChoice('challenges', item.id)"
            >
              {{ item.label }}
            </button>
          </div>
          <p
            v-if="selectedCatalog.challenges.length === 0"
            class="standing-empty soft"
          >
            挑战清单还没准备好，更新数据后再选
          </p>
          <template v-else>
            <input
              v-model="challengeQuery"
              class="watch-search"
              type="search"
              placeholder="搜索挑战，例如交给梅利卡"
            />
            <p v-if="!challengeQuery.trim()" class="standing-empty soft">
              输入目标后点选
            </p>
            <p
              v-else-if="challengeHits.length === 0"
              class="standing-empty soft"
            >
              没有匹配的挑战
            </p>
            <div v-else class="chip-row">
              <button
                v-for="item in challengeHits"
                :key="item.id"
                type="button"
                class="watch-chip"
                :class="{ on: isChosen('challenges', item.id) }"
                :disabled="saving"
                @click="toggleChoice('challenges', item.id)"
              >
                {{ item.label }}
              </button>
            </div>
          </template>
        </div>
      </div>
    </div>

    <div
      v-for="card in visibleCards"
      :key="card.tag"
      class="card standing-board"
    >
      <div class="standing-board-head">
        <h2>{{ card.name }}</h2>
        <div class="standing-board-actions">
          <label
            v-if="card.kind === 'bounty' || card.kind === 'no-daily'"
            class="standing-done"
          >
            <input
              type="checkbox"
              :checked="isDailyDone(card.tag)"
              :disabled="saving"
              @change="toggleDailyDone(card.tag)"
            />
            <span>今日已完成</span>
          </label>
          <label
            v-else-if="card.kind === 'kahl'"
            class="standing-done"
          >
            <input
              type="checkbox"
              :checked="dailyPrefs.kahlDone"
              :disabled="saving"
              @change="toggleKahlDone()"
            />
            <span>本周已完成</span>
          </label>
          <label class="standing-rank">
            <span>等级</span>
            <select
              :value="rankOf(card.tag)"
              :disabled="saving"
              @change="
                onRankChange(
                  card.tag,
                  ($event.target as HTMLSelectElement).value,
                )
              "
            >
              <option v-for="r in rankOptionsFor(card.tag)" :key="r" :value="r">
                {{ r }}
              </option>
            </select>
          </label>
          <button
            v-if="card.kind === 'bounty' && watchCatalog.some((c) => c.tag === card.tag)"
            type="button"
            class="ghost standing-follow"
            :class="{ on: watchCount(card.tag) > 0 }"
            @click="selectSyndicate(card.tag)"
          >
            {{ watchCount(card.tag) > 0 ? `已关注 ${watchCount(card.tag)}` : "关注" }}
          </button>
        </div>
      </div>

      <template v-if="card.kind === 'no-daily'">
        <p class="standing-empty">该势力没有日常赏金</p>
      </template>

      <template v-else-if="card.kind === 'kahl'">
        <p class="standing-meta">
          周常 · 任务随个人进度，非全服轮换 · UTC 周一重置
        </p>
        <p class="standing-empty-note">
          粉碎纳拉摩共三组可能任务，按驻军进度解锁；挑战可重复刷取库存。
        </p>
        <div class="standing-jobs">
          <div
            v-for="m in KAHL_MISSIONS"
            :key="m.key"
            class="standing-job"
          >
            <div class="standing-job-title">
              <span>{{ m.nameZh }}</span>
            </div>
            <div class="standing-job-meta">{{ m.nameEn }}</div>
          </div>
        </div>
      </template>

      <template v-else>
        <p v-if="card.board?.expiry" class="standing-meta">
          赏金刷新
          <LiveRemain
            :expiry="card.board.expiry"
            @ended="reloadBounty"
          />
          <span v-if="formatExpiry(card.board.expiry)">
            · 至 {{ formatExpiry(card.board.expiry) }}
          </span>
        </p>
        <template v-if="!card.board || card.board.jobs.length === 0">
          <p class="standing-empty">暂无赏金任务列表</p>
          <p class="standing-empty-note">
            本轮暂无任务，或实时赏金源拉取失败。可稍后再点「更新数据」重试。
          </p>
        </template>
        <template v-else>
          <p v-if="advancedJob(card.board)" class="standing-advanced-line">
            高级档：{{ advancedJob(card.board)!.title }}
            <template v-if="advancedJob(card.board)!.challengeZh">
              · {{ advancedJob(card.board)!.challengeZh }}
            </template>
            · Lv {{ advancedJob(card.board)!.minEnemyLevel }}–{{
              advancedJob(card.board)!.maxEnemyLevel
            }}
            <template v-if="standingLabel(card.tag, advancedJob(card.board)!)">
              · {{ standingLabel(card.tag, advancedJob(card.board)!) }}
            </template>
          </p>
          <div class="standing-jobs">
            <div
              v-for="(j, idx) in card.board.jobs"
              :key="j.jobType + idx"
              class="standing-job"
              :class="{ advanced: j.isAdvanced, steel: j.isSteelPath }"
            >
              <div class="standing-job-title">
                <span>{{ j.title }}</span>
                <span v-if="j.isAdvanced" class="badge">高级</span>
                <span v-if="j.isSteelPath" class="badge steel">钢路</span>
              </div>
              <div v-if="j.challengeZh" class="standing-job-challenge">
                {{ j.challengeZh }}
              </div>
              <div class="standing-job-meta">
                {{ j.kindZh }}
                · Lv {{ j.minEnemyLevel }}–{{ j.maxEnemyLevel }}
                <template v-if="standingLabel(card.tag, j)">
                  · {{ standingLabel(card.tag, j) }}
                </template>
              </div>
              <div
                v-if="j.rewards && j.rewards.length"
                class="standing-job-rewards"
              >
                <span class="standing-rewards-label">奖励</span>
                <span class="standing-rewards-list">
                  <span
                    v-for="(reward, ri) in j.rewards"
                    :key="reward + ri"
                    class="standing-reward"
                  >
                    <span>{{ reward }}</span>
                    <span
                      v-if="rewardMark(reward)"
                      class="pill reward-mark"
                      :class="statusPillKind(rewardMark(reward))"
                    >
                      {{ rewardMark(reward) }}
                    </span>
                  </span>
                </span>
              </div>
            </div>
          </div>
        </template>
      </template>
    </div>
  </div>
</template>

<style scoped>
.standing-stack {
  display: flex;
  flex-direction: column;
  gap: 18px;
}
.standing-error {
  color: var(--bad);
  margin: 0;
  font-size: 13px;
}
.standing-watch-toggle {
  appearance: none;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0;
  cursor: pointer;
  text-align: left;
}
.standing-watch-toggle h2 {
  margin: 0;
  font-size: 16px;
}
.standing-watch-summary {
  flex: 1;
  font-size: 12px;
  color: var(--text-mute);
}
.standing-watch-caret {
  color: var(--text-dim);
  font-size: 12px;
}
.standing-watch-body {
  margin-top: 10px;
}
.standing-hint {
  margin: 0 0 12px;
  font-size: 12px;
  color: var(--text-mute);
}
.standing-board h2 {
  margin: 0;
  font-size: 16px;
}
.standing-watch-block {
  margin-top: 10px;
}
.standing-watch-block h3 {
  margin: 0 0 8px;
  font-size: 13px;
  color: var(--text-dim);
  font-weight: 600;
}
.chip-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.watch-chip {
  appearance: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--line);
  background: var(--bg-row);
  color: var(--text-dim);
  font: inherit;
  font-size: 12px;
  padding: 5px 10px;
  border-radius: 6px;
  cursor: pointer;
}
.watch-chip.on,
.watch-chip.armed {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}
.watch-chip:disabled {
  cursor: default;
}
.watch-search {
  width: 100%;
  margin: 8px 0;
  box-sizing: border-box;
  font: inherit;
  font-size: 13px;
  color: var(--text);
  background: var(--bg-row);
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 6px 8px;
}
.watch-picked {
  margin-bottom: 4px;
}
.standing-board-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
.standing-board-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}
.standing-done {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-dim);
  cursor: pointer;
  user-select: none;
}
.standing-done input {
  accent-color: var(--cyan, #5ec8c8);
}
.standing-rank {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-dim);
}
.standing-rank select {
  font: inherit;
  font-size: 12px;
  color: var(--text);
  background: var(--bg-row);
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 3px 6px;
}
.standing-follow.on {
  color: var(--accent);
}
.standing-meta {
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--text-mute);
}
.standing-advanced-line {
  margin: 10px 0 0;
  font-size: 13px;
  color: var(--accent);
}
.standing-empty {
  margin: 10px 0 0;
  font-size: 13px;
  color: var(--text-mute);
}
.standing-empty.soft {
  margin-top: 0;
}
.standing-empty-note {
  margin: 6px 0 0;
  font-size: 12px;
  line-height: 1.45;
  color: var(--text-mute);
}
.standing-jobs {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 12px;
}
.standing-job {
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--bg-row);
}
.standing-job.advanced {
  background: var(--accent-soft);
  box-shadow: inset 2px 0 0 var(--accent);
}
.standing-job.steel {
  box-shadow: inset 2px 0 0 #c4a35a;
}
.standing-job-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
}
.badge {
  font-size: 11px;
  color: var(--accent);
  border: 1px solid var(--accent);
  border-radius: 4px;
  padding: 1px 5px;
}
.badge.steel {
  color: #c4a35a;
  border-color: #c4a35a;
}
.standing-job-challenge {
  margin-top: 6px;
  font-size: 14px;
  font-weight: 550;
  line-height: 1.4;
  color: var(--text);
  letter-spacing: 0.01em;
}
.standing-job-meta {
  margin-top: 5px;
  font-size: 12px;
  color: var(--text-dim);
}
.standing-job-rewards {
  margin-top: 6px;
  font-size: 12px;
  line-height: 1.45;
  color: var(--text-dim);
}
.standing-rewards-label {
  display: inline-block;
  margin-right: 6px;
  color: var(--text-mute);
  font-weight: 600;
}
.standing-rewards-list {
  display: inline;
  word-break: break-word;
}
.standing-reward {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-right: 8px;
}
.reward-mark {
  font-size: 10px;
  padding: 0 5px;
  line-height: 1.6;
}
</style>
