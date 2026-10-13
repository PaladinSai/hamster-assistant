import type { LiveDailyBrief } from "../../lib/api";
import type { LiveDetail } from "./liveTypes";
import { formatShortTime } from "./liveUtils";

const LIVE_TITLES = new Set([
  "突击",
  "午夜电波·日常",
  "午夜电波·周常",
  "钢铁之路侵袭",
  "执政官猎杀",
  "深层科研",
  "时光科研",
  "钢铁回廊",
  "无尽回廊",
  "六人组日历",
]);

/** 例程标题 → 时效详情；非 live 标题返回 null。倒计时由叶子组件按 expiry 本地刷新。 */
export function buildLiveDetail(
  title: string,
  brief: LiveDailyBrief | null,
): LiveDetail | null {
  if (!LIVE_TITLES.has(title)) return null;
  if (!brief) return { kind: "empty", summary: "加载中…" };

  if (title === "突击") {
    if (!brief.sortie) return { kind: "empty", summary: "暂无动态" };
    const bits = [
      brief.sortie.boss || null,
      brief.sortie.missions.length ? `${brief.sortie.missions.length} 关` : null,
    ].filter(Boolean);
    return {
      kind: "sortie",
      summary: bits.join(" · ") || "暂无动态",
      boss: brief.sortie.boss || "未知首领",
      faction: brief.sortie.faction || "",
      missions: brief.sortie.missions,
      expiryLabel: brief.sortie.expiry
        ? `至 ${formatShortTime(brief.sortie.expiry)}`
        : "",
      expiry: brief.sortie.expiry,
    };
  }

  if (title === "午夜电波·日常" || title === "午夜电波·周常") {
    const list =
      title === "午夜电波·日常" ? brief.nightwaveDaily : brief.nightwaveWeekly;
    if (!list.length) return { kind: "empty", summary: "暂无动态" };
    const first = list[0]?.title || "挑战";
    const more = list.length > 1 ? ` 等 ${list.length} 项` : "";
    return {
      kind: "nightwave",
      summary: `${first}${more}`,
      challenges: list,
    };
  }

  if (title === "钢铁之路侵袭") {
    if (!brief.steelIncursion) return { kind: "empty", summary: "暂无动态" };
    const { expiry, today, tomorrow } = brief.steelIncursion;
    const first = today[0];
    const summaryBits = [
      first
        ? [first.missionType, first.node.split(" · ")[0]].filter(Boolean).join(" ")
        : null,
      today.length ? `${today.length} 关` : null,
    ].filter(Boolean);
    return {
      kind: "steel",
      summary: summaryBits.join(" · ") || "今日侵袭",
      expiry,
      today: today ?? [],
      tomorrow: tomorrow ?? [],
      expiryLabel: expiry ? `至 ${formatShortTime(expiry)}` : "",
    };
  }

  if (title === "执政官猎杀") {
    if (!brief.archonHunt) return { kind: "empty", summary: "暂无动态" };
    const bits = [
      brief.archonHunt.boss || null,
      brief.archonHunt.shard || null,
      brief.archonHunt.missions.length
        ? `${brief.archonHunt.missions.length} 关`
        : null,
    ].filter(Boolean);
    return {
      kind: "archon",
      summary: bits.join(" · ") || "暂无动态",
      boss: brief.archonHunt.boss || "未知首领",
      shard: brief.archonHunt.shard || "",
      missions: brief.archonHunt.missions,
      expiry: brief.archonHunt.expiry,
    };
  }

  if (title === "深层科研" || title === "时光科研") {
    const data =
      title === "深层科研" ? brief.deepArchimedea : brief.temporalArchimedea;
    if (!data) return { kind: "empty", summary: "暂无动态" };
    const first = data.missions[0]?.missionType;
    const bits = [
      first || null,
      data.missions.length ? `${data.missions.length} 关` : null,
      data.personalModifiers.length
        ? `${data.personalModifiers.length} 修正`
        : null,
    ].filter(Boolean);
    return {
      kind: "archimedea",
      summary: bits.join(" · ") || title,
      kindLabel: data.kindLabel || title,
      missions: data.missions,
      personalModifiers: data.personalModifiers,
      expiry: data.expiry,
    };
  }

  if (title === "钢铁回廊" || title === "无尽回廊") {
    const data = title === "钢铁回廊" ? brief.steelCircuit : brief.normalCircuit;
    if (!data || !data.thisWeek.length)
      return { kind: "empty", summary: "暂无动态" };
    const head = data.thisWeek.slice(0, 2).join("、");
    const more =
      data.thisWeek.length > 2 ? ` 等 ${data.thisWeek.length} 项` : "";
    return {
      kind: "circuit",
      summary: `本周 ${head}${more}`,
      label: title,
      thisWeek: data.thisWeek,
      nextWeek: data.nextWeek,
      rotationWeeks: data.rotationWeeks || [],
      currentWeekIndex: data.currentWeekIndex ?? null,
      expiry: data.expiry,
    };
  }

  if (title === "六人组日历") {
    if (!brief.hexCalendar || !brief.hexCalendar.days.length)
      return { kind: "empty", summary: "暂无动态" };
    const days = brief.hexCalendar.days;
    let firstTitle = "日程";
    for (const d of days) {
      const ch = d.groups.find((g) => g.kind === "challenge");
      if (ch?.choices[0]?.title) {
        firstTitle = ch.choices[0].title;
        break;
      }
      if (d.groups[0]?.choices[0]?.title) {
        firstTitle = d.groups[0].choices[0].title;
        break;
      }
    }
    const more = days.length > 1 ? ` 等 ${days.length} 日` : "";
    return {
      kind: "hex",
      summary: `${firstTitle}${more}`,
      days,
      expiry: brief.hexCalendar.expiry,
    };
  }

  return null;
}
