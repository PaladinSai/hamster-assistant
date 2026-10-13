import { ref, watch, type Ref } from "vue";
import {
  getBountyBrief,
  getLiveDailyBrief,
  type BountyBriefDto,
  type LiveDailyBrief,
} from "../lib/api";

/**
 * 前端只读本地 SQLite（经 IPC DTO）。
 * 网络/JSON 解析只在 Rust；有更新时靠 backend://* 事件 bump token 再 reload。
 */

const liveBrief = ref<LiveDailyBrief | null>(null);
let liveInflight: Promise<LiveDailyBrief | null> | null = null;
let liveTokenSeen = -1;

const bountyBrief = ref<BountyBriefDto | null>(null);
let bountyInflight: Promise<BountyBriefDto | null> | null = null;
let bountyTokenSeen = -1;

export async function reloadLiveDailyBrief(): Promise<LiveDailyBrief | null> {
  if (liveInflight) return liveInflight;
  liveInflight = (async () => {
    try {
      liveBrief.value = await getLiveDailyBrief();
      return liveBrief.value;
    } catch {
      return liveBrief.value;
    } finally {
      liveInflight = null;
    }
  })();
  return liveInflight;
}

export async function reloadBountyBrief(): Promise<BountyBriefDto | null> {
  if (bountyInflight) return bountyInflight;
  bountyInflight = (async () => {
    try {
      bountyBrief.value = await getBountyBrief();
      return bountyBrief.value;
    } catch {
      return bountyBrief.value;
    } finally {
      bountyInflight = null;
    }
  })();
  return bountyInflight;
}

/** 日常/周常/电波共用同一份本地 brief，避免多面板重复读库。 */
export function useLiveDailyCache(refreshToken?: Ref<number | undefined>) {
  if (refreshToken) {
    watch(
      refreshToken,
      (t, prev) => {
        if (t == null || t === prev || t === liveTokenSeen) return;
        liveTokenSeen = t;
        void reloadLiveDailyBrief();
      },
      { immediate: false },
    );
  }

  async function ensureLoaded() {
    const t = refreshToken?.value;
    if (liveBrief.value != null && (t == null || t === liveTokenSeen)) {
      return liveBrief.value;
    }
    if (t != null) liveTokenSeen = t;
    return reloadLiveDailyBrief();
  }

  return {
    brief: liveBrief,
    reload: reloadLiveDailyBrief,
    ensureLoaded,
  };
}

/** 声望赏金本地 brief。 */
export function useBountyCache(refreshToken?: Ref<number | undefined>) {
  if (refreshToken) {
    watch(
      refreshToken,
      (t, prev) => {
        if (t == null || t === prev || t === bountyTokenSeen) return;
        bountyTokenSeen = t;
        void reloadBountyBrief();
      },
      { immediate: false },
    );
  }

  async function ensureLoaded() {
    const t = refreshToken?.value;
    if (bountyBrief.value != null && (t == null || t === bountyTokenSeen)) {
      return bountyBrief.value;
    }
    if (t != null) bountyTokenSeen = t;
    return reloadBountyBrief();
  }

  return {
    brief: bountyBrief,
    reload: reloadBountyBrief,
    ensureLoaded,
  };
}
