/** 周限商店目录（中文以灰机 wiki 为准）。 */

import type { WeeklyShopBrief } from "../api";

export type ShopTier = "must" | "optional";

export type ShopOffer = {
  nameZh: string;
  /** 价格文案，如「75 钢铁精华」 */
  costZh: string;
  /** 周限说明，如「每周限购 1」 */
  limitZh?: string;
  /** 是否为本周轮换特供 */
  featured?: boolean;
};

export type WeeklyShop = {
  id: string;
  tier: ShopTier;
  nameZh: string;
  vendorZh: string;
  locationZh: string;
  currencyZh: string;
  resetZh: string;
  noteZh?: string;
  /** 固定周限货架 */
  offers: ShopOffer[];
  /** 轮换池（随机或轮流上架） */
  rotationPool?: ShopOffer[];
};

const MS_WEEK = 7 * 24 * 60 * 60 * 1000;

/** 取不晚于 now 的最近一个周一 00:00 UTC。 */
export function mondayUtcStart(now = Date.now()): Date {
  const d = new Date(now);
  const day = d.getUTCDay(); // 0=Sun … 1=Mon
  const back = day === 0 ? 6 : day - 1;
  return new Date(
    Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate() - back),
  );
}

/** 下一 UTC 周一 00:00，供 LiveRemain 做周刷新倒计时。 */
export function nextMondayUtcIso(now = Date.now()): string {
  return new Date(mondayUtcStart(now).getTime() + MS_WEEK).toISOString();
}

export function formatRemainingToNextMonday(now = Date.now()): string {
  const next = new Date(mondayUtcStart(now).getTime() + MS_WEEK);
  const ms = next.getTime() - now;
  if (ms <= 0) return "即将刷新";
  const d = Math.floor(ms / 86400000);
  const h = Math.floor((ms % 86400000) / 3600000);
  if (d > 0) return `约 ${d} 天 ${h} 小时后刷新`;
  const m = Math.floor((ms % 3600000) / 60000);
  return `约 ${h} 小时 ${m} 分后刷新`;
}

/** 钢铁之路阁下 · 8 周轮换（与 warframestat rotation 顺序一致）。 */
export const TESHIN_ROTATION: ShopOffer[] = [
  { nameZh: "Umbra Forma 蓝图", costZh: "150 钢铁精华", limitZh: "每周限购 1" },
  { nameZh: "50,000 赤毒", costZh: "55 钢铁精华", limitZh: "每周限购 1" },
  { nameZh: "组合枪裂罅 Mod", costZh: "75 钢铁精华", limitZh: "每周限购 1" },
  { nameZh: "3×Forma", costZh: "75 钢铁精华", limitZh: "每周限购 1" },
  { nameZh: "Zaw 裂罅 Mod", costZh: "75 钢铁精华", limitZh: "每周限购 1" },
  { nameZh: "30,000 内融核心", costZh: "150 钢铁精华", limitZh: "每周限购 1" },
  { nameZh: "步枪裂罅 Mod", costZh: "75 钢铁精华", limitZh: "每周限购 1" },
  { nameZh: "霰弹枪裂罅 Mod", costZh: "75 钢铁精华", limitZh: "每周限购 1" },
];

/** 2026-10-05 周一为「3×Forma」（下标 3），反推 Umbra 周起点。 */
const TESHIN_EPOCH_UMBRA = Date.UTC(2026, 8, 14); // 2026-09-14

export function teshinRotationIndex(now = Date.now()): number {
  const weeks = Math.floor(
    (mondayUtcStart(now).getTime() - TESHIN_EPOCH_UMBRA) / MS_WEEK,
  );
  return ((weeks % 8) + 8) % 8;
}

export function teshinCurrentOffer(now = Date.now()): ShopOffer {
  return { ...TESHIN_ROTATION[teshinRotationIndex(now)], featured: true };
}

/** 鸟三 · 执刑官源力石三色轮换。 */
export const BIRD3_ROTATION: ShopOffer[] = [
  {
    nameZh: "蔚蓝执刑官源力石",
    costZh: "30,000 科维兽声望",
    limitZh: "每周限购 1 · 需启迪者（5 级）",
  },
  {
    nameZh: "琥珀执刑官源力石",
    costZh: "30,000 科维兽声望",
    limitZh: "每周限购 1 · 需启迪者（5 级）",
  },
  {
    nameZh: "深红执刑官源力石",
    costZh: "30,000 科维兽声望",
    limitZh: "每周限购 1 · 需启迪者（5 级）",
  },
];

/** 灰机标注 2022-09-05 为深红周；顺序：蔚蓝→琥珀→深红。 */
const BIRD3_EPOCH_CRIMSON = Date.UTC(2022, 8, 5);

export function bird3RotationIndex(now = Date.now()): number {
  const weeks = Math.floor(
    (mondayUtcStart(now).getTime() - BIRD3_EPOCH_CRIMSON) / MS_WEEK,
  );
  // epoch 周为深红（下标 2）
  return (((weeks % 3) + 2) % 3 + 3) % 3;
}

export function bird3CurrentOffer(now = Date.now()): ShopOffer {
  return { ...BIRD3_ROTATION[bird3RotationIndex(now)], featured: true };
}

/** 言录使周货池（本周随机上架 5 件；无公开 live API，不做本周硬编码）。 */
export const ACRITHIS_WEEKLY_POOL: ShopOffer[] = [
  { nameZh: "Orokin 反应堆蓝图", costZh: "20 苦栓", limitZh: "每周限购 1" },
  { nameZh: "Orokin 催化剂蓝图", costZh: "20 苦栓", limitZh: "每周限购 1" },
  { nameZh: "战甲特殊功能槽连接器", costZh: "20 苦栓", limitZh: "每周限购 1" },
  { nameZh: "武器特殊功能槽连接器", costZh: "20 苦栓", limitZh: "每周限购 1" },
  { nameZh: "主要武器赋能槽连接器", costZh: "25 苦栓", limitZh: "每周限购 1" },
  { nameZh: "次要武器赋能槽连接器", costZh: "25 苦栓", limitZh: "每周限购 1" },
  { nameZh: "Forma 蓝图", costZh: "10 苦栓", limitZh: "每周限购 1" },
  { nameZh: "近战裂罅 Mod", costZh: "15 苦栓", limitZh: "每周限购 1" },
  { nameZh: "手枪裂罅 Mod", costZh: "15 苦栓", limitZh: "每周限购 1" },
  { nameZh: "步枪裂罅 Mod", costZh: "15 苦栓", limitZh: "每周限购 1" },
  { nameZh: "组合枪裂罅 Mod", costZh: "15 苦栓", limitZh: "每周限购 1" },
  { nameZh: "Zaw 裂罅 Mod", costZh: "15 苦栓", limitZh: "每周限购 1" },
  { nameZh: "霰弹枪裂罅 Mod", costZh: "15 苦栓", limitZh: "每周限购 1" },
  { nameZh: "同伴裂罅 Mod", costZh: "20 苦栓", limitZh: "每周限购 1" },
  { nameZh: "5,000 赤毒", costZh: "10 苦栓", limitZh: "每周限购 1" },
];

function offerFromLive(
  o: { nameZh: string; costZh: string },
  featured = false,
): ShopOffer {
  return {
    nameZh: o.nameZh,
    costZh: o.costZh,
    limitZh: "每周限购 1",
    featured,
  };
}

export function buildWeeklyShops(
  now = Date.now(),
  live?: WeeklyShopBrief | null,
): WeeklyShop[] {
  const teshinFeatured = live
    ? offerFromLive(live.teshinCurrent, true)
    : teshinCurrentOffer(now);
  const teshinPool = live?.teshinRotation?.length
    ? live.teshinRotation.map((o) => offerFromLive(o))
    : TESHIN_ROTATION;
  const bird3Featured = live
    ? offerFromLive(live.bird3Current, true)
    : bird3CurrentOffer(now);
  const bird3Pool = live?.bird3Rotation?.length
    ? live.bird3Rotation.map((o) => offerFromLive(o))
    : BIRD3_ROTATION;

  const teshinNote = live?.source === "live"
    ? "本周特供已自动同步；常驻货另有周限（裂罅破解器 / 赤毒 / 遗物包）。"
    : "本周特供按周序推算（网络未同步时）；常驻货另有周限（裂罅破解器 / 赤毒 / 遗物包）。";

  return [
    {
      id: "teshin",
      tier: "must",
      nameZh: "钢铁之路阁下",
      vendorZh: "Teshin",
      locationZh: "任意中继站 · Teshin 房间",
      currencyZh: "钢铁精华",
      resetZh: "每周一 00:00 UTC",
      noteZh: teshinNote,
      offers: [
        teshinFeatured,
        {
          nameZh: "裂罅破解器",
          costZh: "20 钢铁精华",
          limitZh: "每周限购 1",
        },
        {
          nameZh: "10,000 赤毒",
          costZh: "15 钢铁精华",
          limitZh: "每周限购 25",
        },
        {
          nameZh: "遗物组合包（3 枚）",
          costZh: "15 钢铁精华",
          limitZh: "每周限购 25",
        },
      ],
      rotationPool: teshinPool,
    },
    {
      id: "acrithis",
      tier: "must",
      nameZh: "言录使 · 周货",
      vendorZh: "言录使",
      locationZh: "扎里曼居住舱 / 双衍王境",
      currencyZh: "苦栓",
      resetZh: "每周一 00:00 UTC",
      noteZh:
        "下列为货池（每周随机上架 5 件）。官方与公开第三方均无本周货架接口，暂无法自动同步；赋能 / Captura 为日刷。",
      offers: [],
      rotationPool: ACRITHIS_WEEKLY_POOL,
    },
    {
      id: "bird3",
      tier: "must",
      nameZh: "鸟三 · 闪亮珍宝",
      vendorZh: "鸟三",
      locationZh: "解剖圣所 · 科维兽",
      currencyZh: "科维兽声望",
      resetZh: "每周一 00:00 UTC",
      noteZh: "需科维兽声望 5 级（启迪者）。三色源力石按周自动轮换。",
      offers: [bird3Featured],
      rotationPool: bird3Pool,
    },
    {
      id: "palladino",
      tier: "optional",
      nameZh: "Palladino 商店",
      vendorZh: "Palladino（圣言者）",
      locationZh: "地球 · 钢铁守望",
      currencyZh: "裂罅碎块",
      resetZh: "每周一 00:00 UTC",
      noteZh: "完成「Harrow 的枷锁」后可交易。",
      offers: [
        { nameZh: "150,000 现金", costZh: "5 裂罅碎块", limitZh: "每周限购 1" },
        { nameZh: "6,000 内融核心", costZh: "10 裂罅碎块", limitZh: "每周限购 1" },
        { nameZh: "35,000 赤毒", costZh: "10 裂罅碎块", limitZh: "每周限购 1" },
        { nameZh: "安魂通牒", costZh: "10 裂罅碎块", limitZh: "每周限购 1" },
        { nameZh: "裂罅破解器", costZh: "10 裂罅碎块", limitZh: "每周限购 1" },
        { nameZh: "随机裂罅 Mod（×2）", costZh: "10 裂罅碎块 / 件", limitZh: "每周限购" },
        { nameZh: "裂罅转换器", costZh: "10 裂罅碎块", limitZh: "每周限购 3" },
        {
          nameZh: "安魂遗物 I–IV",
          costZh: "10 裂罅碎块 / 枚",
          limitZh: "各每周限购 10",
        },
      ],
    },
    {
      id: "nightcap",
      tier: "optional",
      nameZh: "夜帽商店",
      vendorZh: "夜帽",
      locationZh: "福尔图娜 · 深矿入口",
      currencyZh: "夜帽商店通货（随等级）",
      resetZh: "每周一 00:00 UTC",
      noteZh: "需完成新纪之战；部分货架需野植图鉴等级。",
      offers: [
        {
          nameZh: "阿耶檀识 Chattraka 塑像",
          costZh: "见店内标价",
          limitZh: "每周限购 1",
        },
        { nameZh: "6,000 内融核心", costZh: "见店内标价", limitZh: "每周限购 6" },
        { nameZh: "蒸汽幽灵", costZh: "见店内标价", limitZh: "每周限购 7" },
      ],
    },
    {
      id: "hunhow",
      tier: "optional",
      nameZh: "Hunhow 纪念品",
      vendorZh: "Hunhow",
      locationZh: "天王星比邻星域 · 渡界之塔",
      currencyZh: "翠绿天赋 / 腥红天赋",
      resetZh: "每周一 00:00 UTC",
      noteZh: "完成「Jade 之影：众星」后开放。",
      offers: [
        {
          nameZh: "7,000 赤毒",
          costZh: "35 翠绿天赋 + 35 腥红天赋",
          limitZh: "每周限购 6",
        },
      ],
    },
    {
      id: "yonta",
      tier: "optional",
      nameZh: "哲士扬塔",
      vendorZh: "哲士扬塔",
      locationZh: "羽化之穹 · 扎里曼",
      currencyZh: "虚空之翎",
      resetZh: "每周一 00:00 UTC",
      offers: [
        {
          nameZh: "35,000 赤毒",
          costZh: "5 枚虚空之翎（小齿轮档）",
          limitZh: "每周限购 1",
        },
      ],
    },
    {
      id: "scuttler",
      tier: "optional",
      nameZh: "言录使 · 急行蛛外壳兑换",
      vendorZh: "言录使",
      locationZh: "扎里曼居住舱 / 双衍王境",
      currencyZh: "急行蛛外壳",
      resetZh: "每周一 00:00 UTC",
      noteZh: "Hex /「食莲人」相关进度解锁。",
      offers: [
        {
          nameZh: "6,000 赤毒",
          costZh: "100 急行蛛外壳",
          limitZh: "每周限购 7",
        },
      ],
    },
    {
      id: "flare",
      tier: "optional",
      nameZh: "弗莱尔纪念品",
      vendorZh: "弗莱尔",
      locationZh: "霍瓦尼亚中央商场",
      currencyZh: "律动心弦",
      resetZh: "每周一 00:00 UTC",
      noteZh: "六人组声望需达到「新鲜出炉」（4 级）。",
      offers: [
        {
          nameZh: "6,000 赤毒",
          costZh: "110 律动心弦",
          limitZh: "每周限购 7",
        },
      ],
    },
    {
      id: "kaya",
      tier: "optional",
      nameZh: "卡娅商店",
      vendorZh: "卡娅",
      locationZh: "霍瓦尼亚中央商场",
      currencyZh: "贴纸芯片",
      resetZh: "每周一 00:00 UTC",
      noteZh: "六人组声望需达到「披萨派对」（5 级）。",
      offers: [
        { nameZh: "脱逃赋能", costZh: "5 贴纸芯片", limitZh: "每周限购" },
        { nameZh: "热火赋能", costZh: "5 贴纸芯片", limitZh: "每周限购" },
        { nameZh: "辐照赋能", costZh: "5 贴纸芯片", limitZh: "每周限购" },
      ],
    },
    {
      id: "nightwave",
      tier: "optional",
      nameZh: "午夜电波货币商店",
      vendorZh: "诺拉 · 夜",
      locationZh: "轨道飞行器电台 / 电波界面",
      currencyZh: "电波货币（Cred）",
      resetZh: "随电波赛季与周常",
      noteZh: "土豆等常驻货架长期可换；另有少量随机周货（如狼信标）。",
      offers: [
        { nameZh: "10,000 赤毒", costZh: "50 电波货币", limitZh: "常驻" },
        { nameZh: "氮气提炼物", costZh: "15 电波货币", limitZh: "常驻" },
        { nameZh: "Orokin 催化剂", costZh: "75 电波货币", limitZh: "常驻" },
        { nameZh: "Orokin 反应堆", costZh: "75 电波货币", limitZh: "常驻" },
        {
          nameZh: "随机周货（如狼信标等）",
          costZh: "见店内",
          limitZh: "随周出现",
        },
      ],
    },
  ];
}
