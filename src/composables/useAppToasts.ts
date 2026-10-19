import { readonly, ref } from "vue";

export interface AppToastAction {
  label: string;
  kind?: "primary" | "ghost";
  run: () => void;
}

export interface AppToast {
  id: string;
  title: string;
  body: string;
  expiry: string | null;
  /** 势力 tag；有则点击可跳转对应声望页 */
  standingTag: string | null;
  actions?: AppToastAction[];
}

export interface BountyAlertPayload {
  body: string;
  expiry: string | null;
  tag?: string | null;
}

const toasts = ref<AppToast[]>([]);
const expiryTimers = new Map<string, ReturnType<typeof setTimeout>>();
let seq = 0;

function clearExpiryTimer(id: string) {
  const t = expiryTimers.get(id);
  if (t != null) {
    clearTimeout(t);
    expiryTimers.delete(id);
  }
}

function scheduleExpiry(toast: AppToast) {
  clearExpiryTimer(toast.id);
  if (!toast.expiry) return;
  const end = Date.parse(toast.expiry);
  if (Number.isNaN(end)) return;
  const ms = end - Date.now();
  if (ms <= 0) {
    dismissToast(toast.id);
    return;
  }
  expiryTimers.set(
    toast.id,
    setTimeout(() => {
      expiryTimers.delete(toast.id);
      dismissToast(toast.id);
    }, ms),
  );
}

export function pushToast(input: {
  title: string;
  body: string;
  expiry?: string | null;
  standingTag?: string | null;
  actions?: AppToastAction[];
}) {
  const id = `toast-${Date.now()}-${++seq}`;
  const toast: AppToast = {
    id,
    title: input.title,
    body: input.body,
    expiry: input.expiry ?? null,
    standingTag: input.standingTag?.trim() || null,
    actions: input.actions,
  };
  toasts.value = [...toasts.value, toast];
  scheduleExpiry(toast);
  return id;
}

export function pushBountyAlerts(alerts: BountyAlertPayload[]) {
  for (const a of alerts) {
    pushToast({
      title: "声望赏金",
      body: a.body,
      expiry: a.expiry,
      standingTag: a.tag ?? null,
    });
  }
}

export function dismissToast(id: string) {
  clearExpiryTimer(id);
  toasts.value = toasts.value.filter((t) => t.id !== id);
}

/** AppShell / ToastStack 共用单例列表。 */
export function useAppToasts() {
  return {
    toasts: readonly(toasts),
    pushToast,
    pushBountyAlerts,
    dismissToast,
  };
}
