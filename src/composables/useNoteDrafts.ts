import { ref, watch, type WatchSource } from "vue";

/** 列表备注草稿：与 props 同步，编辑时不立刻写回 */
export function useNoteDrafts<T>(
  source: WatchSource<T[]>,
  getId: (item: T) => string,
  getNote: (item: T) => string | null | undefined,
  options?: { deep?: boolean },
) {
  const noteDrafts = ref<Record<string, string>>({});
  watch(
    source,
    (list) => {
      const next: Record<string, string> = {};
      for (const item of list) {
        next[getId(item)] = getNote(item) ?? "";
      }
      noteDrafts.value = next;
    },
    { immediate: true, deep: options?.deep },
  );
  return noteDrafts;
}
