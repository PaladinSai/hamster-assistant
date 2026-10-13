import { onMounted, onUnmounted, ref } from "vue";

export function useConfirm() {
  const open = ref(false);
  const title = ref("确认");
  const message = ref("");
  let resolveFn: ((value: boolean) => void) | null = null;

  function settle(value: boolean) {
    open.value = false;
    const r = resolveFn;
    resolveFn = null;
    r?.(value);
  }

  function confirm(msg: string, dlgTitle = "确认"): Promise<boolean> {
    if (resolveFn) {
      settle(false);
    }
    message.value = msg;
    title.value = dlgTitle;
    open.value = true;
    return new Promise<boolean>((resolve) => {
      resolveFn = resolve;
    });
  }

  function onConfirm() {
    settle(true);
  }

  function onCancel() {
    settle(false);
  }

  function onKeydown(e: KeyboardEvent) {
    if (!open.value) return;
    if (e.key === "Escape") {
      e.preventDefault();
      onCancel();
    }
  }

  onMounted(() => {
    document.addEventListener("keydown", onKeydown);
  });
  onUnmounted(() => {
    document.removeEventListener("keydown", onKeydown);
    if (resolveFn) settle(false);
  });

  return {
    open,
    title,
    message,
    confirm,
    onConfirm,
    onCancel,
  };
}
