/** 节点文案「地点 · 星球」拆分。 */
export function splitNode(node: string) {
  const sep = " · ";
  const i = node.indexOf(sep);
  if (i < 0) return { place: node.trim(), planet: "" };
  return {
    place: node.slice(0, i).trim(),
    planet: node.slice(i + sep.length).trim(),
  };
}

/** 本地倒计时文案；由叶子组件每秒刷新，勿塞进会重建的 detail。 */
export function remainingText(
  expiry: string | null | undefined,
  nowMs: number = Date.now(),
) {
  if (!expiry) return "";
  const end = new Date(expiry).getTime();
  if (Number.isNaN(end)) return "";
  const ms = end - nowMs;
  if (ms <= 0) return "已结束";
  const totalSec = Math.floor(ms / 1000);
  const d = Math.floor(totalSec / 86400);
  const h = Math.floor((totalSec % 86400) / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  if (d >= 1) return `剩余 ${d} 天 ${h} 小时`;
  if (h >= 1) return `剩余 ${h} 小时 ${m} 分`;
  if (m >= 1) return `剩余 ${m} 分 ${s} 秒`;
  return `剩余 ${s} 秒`;
}

export function formatShortTime(iso: string | null | undefined) {
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

export function hexKindLabel(kind: string) {
  if (kind === "challenge") return "挑战";
  if (kind === "reward") return "奖励";
  if (kind === "upgrade") return "增益";
  return kind;
}
