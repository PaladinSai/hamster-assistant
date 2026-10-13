import { computed, ref, type Ref } from "vue";

export type View =
  | "home"
  | "feed"
  | "standing"
  | "warframes"
  | "archwing"
  | "necramechs"
  | "relics"
  | "weapons"
  | "archwingWeapons"
  | "companions"
  | "mods"
  | "arcanes";

export type HomeSection =
  | "overview"
  | "progress"
  | "todo"
  | "wish"
  | "daily"
  | "weekly";

export type FeedSection =
  | "plains"
  | "rebirth"
  | "shop"
  | "fissures"
  | "arbitration"
  | "nightwave";

export type StandingSection =
  | "overview"
  | "ostron"
  | "quills"
  | "solaris"
  | "ventkids"
  | "vox"
  | "entrati"
  | "necraloid"
  | "holdfasts"
  | "kahl"
  | "cavia"
  | "hex";

/** section → worldState Tag */
export const STANDING_SECTION_TAG: Record<
  Exclude<StandingSection, "overview">,
  string
> = {
  ostron: "CetusSyndicate",
  quills: "QuillsSyndicate",
  solaris: "SolarisSyndicate",
  ventkids: "VentKidsSyndicate",
  vox: "VoxSyndicate",
  entrati: "EntratiSyndicate",
  necraloid: "NecraloidSyndicate",
  holdfasts: "ZarimanSyndicate",
  kahl: "KahlSyndicate",
  cavia: "EntratiLabSyndicate",
  hex: "HexSyndicate",
};

export const STANDING_NAV_ITEMS: {
  section: Exclude<StandingSection, "overview">;
  label: string;
}[] = [
  { section: "ostron", label: "Ostron" },
  { section: "quills", label: "夜羽" },
  { section: "solaris", label: "索拉里斯联盟" },
  { section: "ventkids", label: "通风小子" },
  { section: "vox", label: "索拉里斯之声" },
  { section: "entrati", label: "英择谛" },
  { section: "necraloid", label: "殁世械灵" },
  { section: "holdfasts", label: "坚守者" },
  { section: "kahl", label: "卡尔驻军" },
  { section: "cavia", label: "科维兽" },
  { section: "hex", label: "六人组" },
];

/** 无日常赏金的势力（仍可勾选今日是否已完成）。 */
export const STANDING_NO_DAILY_SECTIONS = new Set<
  Exclude<StandingSection, "overview">
>(["quills", "ventkids", "vox", "necraloid"]);

/** 卡尔驻军：个人进度周常，三组可能任务。 */
export const KAHL_MISSIONS = [
  { key: "sneaky", nameZh: "隐蔽破坏", nameEn: "Sneaky Sabotage" },
  { key: "junk", nameZh: "垃圾回收", nameEn: "Junk Run" },
  { key: "prison", nameZh: "越狱", nameEn: "Prison Break" },
] as const;

const HOME_SECTION_TITLE: Record<HomeSection, string> = {
  overview: "首页",
  progress: "进度",
  todo: "待办",
  wish: "心愿",
  daily: "日常",
  weekly: "周常",
};

const HOME_SECTION_HINT: Record<HomeSection, string> = {
  overview: "进度 · 待办 · 心愿 · 日常 · 周常",
  progress: "战甲与武器获取进度",
  todo: "自定义待办清单",
  wish: "星标关注",
  daily: "突击东部正午 · 其余 UTC 零点",
  weekly: "UTC 周一重置",
};

const FEED_SECTION_TITLE: Record<FeedSection, string> = {
  plains: "平原",
  rebirth: "重生",
  shop: "商店",
  fissures: "裂隙",
  arbitration: "仲裁",
  nightwave: "午夜电波",
};

const FEED_SECTION_HINT: Record<FeedSection, string> = {
  plains: "昼夜 · 冷暖 · 情绪螺旋",
  rebirth: "本期 / 下期重生轮换",
  shop: "周限商店 · 点选项切换",
  fissures: "全部 · 普通 · 钢铁 · 九重天",
  arbitration: "当前 · 后续预测",
  nightwave: "本轮挑战一览",
};

const STANDING_SECTION_TITLE: Record<StandingSection, string> = {
  overview: "声望",
  ostron: "Ostron",
  quills: "夜羽",
  solaris: "索拉里斯联盟",
  ventkids: "通风小子",
  vox: "索拉里斯之声",
  entrati: "英择谛",
  necraloid: "殁世械灵",
  holdfasts: "坚守者",
  kahl: "卡尔驻军",
  cavia: "科维兽",
  hex: "六人组",
};

const STANDING_SECTION_HINT: Record<StandingSection, string> = {
  overview: "开放世界赏金 · 关注高级档提醒",
  ostron: "希图斯赏金板",
  quills: "无日常赏金",
  solaris: "福尔图娜赏金板",
  ventkids: "无日常赏金",
  vox: "无日常赏金",
  entrati: "火卫二赏金板",
  necraloid: "无日常赏金",
  holdfasts: "扎里曼赏金板",
  kahl: "周常 · 个人进度三组任务",
  cavia: "科维兽 / 圣殿",
  hex: "六人组赏金板",
};

export function useAppNav(opts: {
  ensureLoaded: (next: View) => void;
  relicFocusKey: Ref<string | null>;
}) {
  const collapsed = ref(false);
  const view = ref<View>("home");
  const query = ref("");
  const homeSection = ref<HomeSection>("overview");
  const homeNavOpen = ref(false);
  const feedSection = ref<FeedSection>("plains");
  const feedNavOpen = ref(false);
  const standingSection = ref<StandingSection>("overview");
  const standingNavOpen = ref(false);

  const pageTitle = computed(() => {
    if (view.value === "home") return HOME_SECTION_TITLE[homeSection.value];
    if (view.value === "feed") return FEED_SECTION_TITLE[feedSection.value];
    if (view.value === "standing")
      return STANDING_SECTION_TITLE[standingSection.value];
    if (view.value === "relics") return "遗物来源表";
    if (view.value === "weapons") return "武器";
    if (view.value === "archwingWeapons") return "空战武器";
    if (view.value === "companions") return "同伴";
    if (view.value === "mods") return "Mod";
    if (view.value === "arcanes") return "赋能";
    if (view.value === "archwing") return "Archwing";
    if (view.value === "necramechs") return "机甲";
    return "战甲";
  });

  const pageHint = computed(() => {
    if (view.value === "home") return HOME_SECTION_HINT[homeSection.value];
    if (view.value === "feed") return FEED_SECTION_HINT[feedSection.value];
    if (view.value === "standing")
      return STANDING_SECTION_HINT[standingSection.value];
    if (view.value === "relics") return "搜战甲/武器 · 看核桃掉落点";
    if (view.value === "weapons")
      return "主/副/近战 · 家族展开 · 赤毒/信条/终幕 · 灵化";
    if (view.value === "archwingWeapons") return "枪械 / 近战 · 掌握 / 心愿";
    if (view.value === "companions")
      return "哨兵 / 库娃 / 库珀 / 狐獴 / 扑猎达赛 · 掌握 / 心愿";
    if (view.value === "mods") return "按标签 · 未获取/已获取 · 心愿";
    if (view.value === "arcanes")
      return "来源 · 品类 · 品级 · 未获取/未满级/已获取 · 还差几个";
    if (view.value === "archwing") return "双翼 / 系统 / 掌握 / 心愿";
    if (view.value === "necramechs") return "引擎 / 武器舱 / 掌握 / 心愿";
    return "本体 / Prime / Helminth / 入库 / 已挖";
  });

  const showTopbarCatalogTools = computed(
    () =>
      view.value === "warframes" ||
      view.value === "archwing" ||
      view.value === "necramechs" ||
      view.value === "companions" ||
      view.value === "weapons" ||
      view.value === "archwingWeapons" ||
      view.value === "mods" ||
      view.value === "arcanes" ||
      view.value === "relics",
  );

  const searchPlaceholder = computed(() => {
    if (view.value === "weapons") return "搜索武器…";
    if (view.value === "archwingWeapons") return "搜索空战武器…";
    if (view.value === "companions") return "搜索同伴…";
    if (view.value === "mods") return "搜索 Mod…";
    if (view.value === "arcanes") return "搜索赋能…";
    if (view.value === "relics") return "战甲 / 武器（简中或英文）或核桃键";
    if (view.value === "archwing") return "搜索 Archwing…";
    if (view.value === "necramechs") return "搜索机甲…";
    return "搜索战甲…";
  });

  function setView(next: View) {
    view.value = next;
    if (next === "home") {
      homeNavOpen.value = true;
    }
    if (next === "feed") {
      feedNavOpen.value = true;
    }
    if (next === "standing") {
      standingNavOpen.value = true;
    }
    if (next !== "relics") opts.relicFocusKey.value = null;
    opts.ensureLoaded(next);
  }

  function openHomeOverview() {
    if (view.value === "home" && homeNavOpen.value) {
      homeNavOpen.value = false;
      return;
    }
    homeNavOpen.value = true;
    homeSection.value = "overview";
    setView("home");
  }

  function setHomeSection(section: HomeSection) {
    homeNavOpen.value = true;
    homeSection.value = section;
    setView("home");
  }

  function toggleHomeNav() {
    homeNavOpen.value = !homeNavOpen.value;
  }

  function openFeedOverview() {
    if (view.value === "feed" && feedNavOpen.value) {
      feedNavOpen.value = false;
      return;
    }
    feedNavOpen.value = true;
    feedSection.value = "plains";
    setView("feed");
  }

  function setFeedSection(section: FeedSection) {
    feedNavOpen.value = true;
    feedSection.value = section;
    setView("feed");
  }

  function toggleFeedNav() {
    feedNavOpen.value = !feedNavOpen.value;
  }

  function openStandingOverview() {
    if (view.value === "standing" && standingNavOpen.value) {
      standingNavOpen.value = false;
      return;
    }
    standingNavOpen.value = true;
    standingSection.value = "overview";
    setView("standing");
  }

  function setStandingSection(section: StandingSection) {
    standingNavOpen.value = true;
    standingSection.value = section;
    setView("standing");
  }

  function toggleStandingNav() {
    standingNavOpen.value = !standingNavOpen.value;
  }

  function openRelicSource(relicKey: string) {
    opts.relicFocusKey.value = relicKey;
    view.value = "relics";
  }

  function onSearchInput() {
    if (view.value === "relics") return;
    if (view.value === "weapons" || view.value === "archwingWeapons") return;
    if (
      view.value === "warframes" ||
      view.value === "archwing" ||
      view.value === "necramechs" ||
      view.value === "companions"
    )
      return;
    if (view.value === "mods") return;
    if (view.value === "arcanes") return;
    setView("warframes");
  }

  return {
    collapsed,
    view,
    query,
    homeSection,
    homeNavOpen,
    feedSection,
    feedNavOpen,
    standingSection,
    standingNavOpen,
    pageTitle,
    pageHint,
    showTopbarCatalogTools,
    searchPlaceholder,
    setView,
    openHomeOverview,
    setHomeSection,
    toggleHomeNav,
    openFeedOverview,
    setFeedSection,
    toggleFeedNav,
    openStandingOverview,
    setStandingSection,
    toggleStandingNav,
    openRelicSource,
    onSearchInput,
  };
}
