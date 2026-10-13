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
} from "../lib/api";
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
  followedSyndicates: [],
  followedJobKinds: [],
  lastNotified: [],
});
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
    watchPrefs.value = w;
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

const knownKinds = computed(() => {
  const map = new Map<string, string>();
  for (const b of boards.value) {
    for (const j of b.jobs) {
      const zh = j.kindZh && j.kindZh !== j.kind ? j.kindZh : translateKindFallback(j.kind);
      if (!map.has(j.kind)) map.set(j.kind, zh);
    }
  }
  return [...map.entries()]
    .map(([kind, kindZh]) => ({ kind, kindZh }))
    .sort((a, b) => {
      // 歼灭优先（坚守者 / 六人组 / 科维兽关注主用途）
      if (a.kind === "Exterminate") return -1;
      if (b.kind === "Exterminate") return 1;
      return a.kindZh.localeCompare(b.kindZh, "zh");
    });
});

function translateKindFallback(kind: string) {
  const table: Record<string, string> = {
    Exterminate: "歼灭",
    Survival: "生存",
    Defense: "防御",
    MobileDefense: "移动防御",
    DualDefense: "镜像防御",
    Assassinate: "刺杀",
    Capture: "捕获",
    Attrition: "消耗",
    Cull: "清剿",
    Helping: "援助",
    Legacyte: "传承种收割",
    Alchemy: "元素转换",
    Disruption: "中断",
    VoidFlood: "虚空洪流",
    VoidCascade: "虚空覆涌",
    VoidArmageddon: "虚空决战",
    Hazards: "高危物品",
    HighKill: "天降正义",
    DestroyProps: "摧毁掩护",
    Exploding: "爆炸性科腐者",
    SafeCracker: "破壳专家",
    Operator: "脱离战甲",
    Collect: "收集",
    Vial: "化学课",
    Doppelganger: "致命低语",
    Decoration: "破坏能手",
    Conduit: "坚毅爆破者",
  };
  return table[kind] ?? kind;
}

const watchSummary = computed(() => {
  const nSyn = watchPrefs.value.followedSyndicates.length;
  const nKind = watchPrefs.value.followedJobKinds.length;
  if (nSyn === 0 && nKind === 0) return "未设置关注";
  return `已关注 ${nSyn} 个势力 · ${nKind} 种任务`;
});

function isFollowedSyndicate(tag: string) {
  return watchPrefs.value.followedSyndicates.includes(tag);
}

function isFollowedKind(kind: string) {
  return watchPrefs.value.followedJobKinds.includes(kind);
}

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

async function toggleSyndicate(tag: string) {
  const set = new Set(watchPrefs.value.followedSyndicates);
  if (set.has(tag)) set.delete(tag);
  else set.add(tag);
  await persistWatch({
    ...watchPrefs.value,
    followedSyndicates: [...set],
  });
}

async function toggleKind(kind: string) {
  const set = new Set(watchPrefs.value.followedJobKinds);
  if (set.has(kind)) set.delete(kind);
  else set.add(kind);
  await persistWatch({
    ...watchPrefs.value,
    followedJobKinds: [...set],
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
          关注势力 + 任务种类后，刷新数据时若该势力「声望最高」档命中种类，将弹出系统通知。
          {{ saving ? "保存中…" : "" }}
        </p>
        <div class="standing-watch-block">
          <h3>势力</h3>
          <div class="chip-row">
            <button
              v-for="b in boards"
              :key="b.tag"
              type="button"
              class="watch-chip"
              :class="{ on: isFollowedSyndicate(b.tag) }"
              @click="toggleSyndicate(b.tag)"
            >
              {{ b.name }}
            </button>
          </div>
        </div>
        <div class="standing-watch-block">
          <h3>任务种类</h3>
          <p v-if="knownKinds.length === 0" class="standing-empty soft">
            暂无赏金种类（先更新数据）
          </p>
          <div v-else class="chip-row">
            <button
              v-for="k in knownKinds"
              :key="k.kind"
              type="button"
              class="watch-chip"
              :class="{ on: isFollowedKind(k.kind) }"
              @click="toggleKind(k.kind)"
            >
              {{ k.kindZh }}
            </button>
          </div>
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
            v-if="card.kind === 'bounty'"
            type="button"
            class="ghost standing-follow"
            :class="{ on: isFollowedSyndicate(card.tag) }"
            @click="toggleSyndicate(card.tag)"
          >
            {{ isFollowedSyndicate(card.tag) ? "已关注" : "关注" }}
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
                <span class="standing-rewards-list">{{
                  j.rewards.join(" · ")
                }}</span>
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
  border: 1px solid var(--line);
  background: var(--bg-row);
  color: var(--text-dim);
  font: inherit;
  font-size: 12px;
  padding: 5px 10px;
  border-radius: 6px;
  cursor: pointer;
}
.watch-chip.on {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
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
  word-break: break-word;
}
</style>
