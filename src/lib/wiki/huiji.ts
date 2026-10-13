/** 灰机 Wiki URL 辅助。 */

const HUIJI_WIKI = "https://warframe.huijiwiki.com/wiki/";

/**
 * 将目录里的赋能中文名（WFCD：`前缀·后缀`）转成灰机页面标题。
 *
 * 规则（对照 huiji `api.php?action=query`）：
 * - `赋能·充沛` → `充沛赋能`（「赋能」挪到词尾，去掉间隔号）
 * - 其它如 `双枪·滑射` / `蜕化·升腾` / `魔导·充盈` → 仅去掉 `·`
 */
export function huijiArcanePageTitle(nameZh: string): string {
  const s = nameZh.trim();
  if (!s) return "";
  if (s.startsWith("赋能·")) {
    const rest = s.slice("赋能·".length).trim();
    return rest ? `${rest}赋能` : "赋能";
  }
  return s.replace(/·/g, "");
}

export function huijiWikiUrl(pageTitle: string): string {
  const page = pageTitle.trim().replace(/ /g, "_");
  if (!page) return "";
  return `${HUIJI_WIKI}${encodeURIComponent(page)}`;
}

/** 赋能灰机链接：优先中文（按规则转换），否则英文。 */
export function huijiArcaneWikiUrl(nameZh: string | null | undefined, nameEn: string): string {
  const zh = nameZh?.trim();
  if (zh) return huijiWikiUrl(huijiArcanePageTitle(zh));
  return huijiWikiUrl(nameEn.trim());
}

/**
 * 武器灰机页面标题。
 *
 * 目录简中常带空格/间隔号（`布莱顿 Prime`、`赤毒·卡拉克`），
 * 灰机页为紧贴写法（`布莱顿Prime`、`赤毒卡拉克`）。
 */
export function huijiWeaponPageTitle(
  nameZh: string | null | undefined,
  nameEn: string,
): string {
  const zh = nameZh?.trim();
  if (zh) {
    return zh
      .replace(/·/g, "")
      .replace(/\s+Prime\b/gi, "Prime")
      .replace(/\s+/g, "");
  }
  return nameEn.trim();
}

/** 武器灰机链接：优先规范化中文名，否则英文。 */
export function huijiWeaponWikiUrl(
  nameZh: string | null | undefined,
  nameEn: string,
): string {
  return huijiWikiUrl(huijiWeaponPageTitle(nameZh, nameEn));
}

/**
 * 战甲 / 同伴等机体灰机标题：优先简中（去空格·间隔号），否则英文。
 */
export function huijiSuitPageTitle(
  nameZh: string | null | undefined,
  nameEn: string,
): string {
  return huijiWeaponPageTitle(nameZh, nameEn);
}

export function huijiSuitWikiUrl(
  nameZh: string | null | undefined,
  nameEn: string,
): string {
  return huijiWikiUrl(huijiSuitPageTitle(nameZh, nameEn));
}
