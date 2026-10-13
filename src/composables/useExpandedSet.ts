import { ref } from "vue";

/** 可展开行/零件集合 */
export function useExpandedSet() {
  const expanded = ref<Set<string>>(new Set());

  function isExpanded(id: string) {
    return expanded.value.has(id);
  }

  function toggleExpand(id: string) {
    const next = new Set(expanded.value);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expanded.value = next;
  }

  function clearExpanded() {
    expanded.value = new Set();
  }

  return { expanded, isExpanded, toggleExpand, clearExpanded };
}
