import { ref } from "vue";
import { listRoutines, type RoutineItemDto } from "../lib/api";

/**
 * 日常/周常面板共享同一份例程列表，避免 overview 双挂载时重复 listRoutines IPC。
 */

const items = ref<RoutineItemDto[]>([]);
let inflight: Promise<RoutineItemDto[]> | null = null;
let hasLoaded = false;

export async function reloadRoutines(): Promise<RoutineItemDto[]> {
  if (inflight) return inflight;
  inflight = (async () => {
    try {
      items.value = await listRoutines();
      hasLoaded = true;
      return items.value;
    } finally {
      inflight = null;
    }
  })();
  return inflight;
}

export async function ensureRoutinesLoaded(): Promise<RoutineItemDto[]> {
  if (hasLoaded) return items.value;
  return reloadRoutines();
}

/** 供 RoutinePanel：共享 items + 加载入口。 */
export function useRoutinesList() {
  return {
    items,
    ensureLoaded: ensureRoutinesLoaded,
    reload: reloadRoutines,
  };
}
