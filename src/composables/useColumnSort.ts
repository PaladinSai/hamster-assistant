import { ref } from "vue";

export type SortDir = "asc" | "desc";

/** 表头单击排序：升序 → 降序 → 取消（回默认序）。 */
export function useColumnSort<K extends string>() {
  const sortKey = ref<K | null>(null);
  const sortDir = ref<SortDir>("asc");

  function toggleSort(key: K) {
    if (sortKey.value !== key) {
      sortKey.value = key;
      sortDir.value = "asc";
      return;
    }
    if (sortDir.value === "asc") {
      sortDir.value = "desc";
      return;
    }
    sortKey.value = null;
    sortDir.value = "asc";
  }

  function sortMark(key: K) {
    if (sortKey.value !== key) return "";
    return sortDir.value === "asc" ? "↑" : "↓";
  }

  function applyDir(cmp: number) {
    if (cmp === 0) return 0;
    return sortDir.value === "asc" ? cmp : -cmp;
  }

  function cmpStr(a: string, b: string) {
    return applyDir(a.localeCompare(b, "zh"));
  }

  function cmpNum(a: number, b: number) {
    return applyDir(a - b);
  }

  return { sortKey, sortDir, toggleSort, sortMark, applyDir, cmpStr, cmpNum };
}
