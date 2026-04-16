import type { AstroCookies } from "astro";

export type Locale = "en" | "zh-CN";

type LocaleContext = {
  url: URL;
  request: Request;
  cookies: AstroCookies;
};

type Dictionary = {
  common: {
    language: string;
    githubRepository: string;
    nav: {
      leaderboard: string;
      tarot: string;
    };
    actions: {
      backToLeaderboard: string;
      fullProfile: string;
      githubProfile: string;
      shareOnX: string;
      copyShareLink: string;
      copied: string;
    };
    missing: {
      userReportTitle: string;
      userReportCopy: string;
      shareTitle: string;
      shareCopy: string;
      backToHome: string;
    };
  };
  leaderboard: {
    pageTitle: string;
    pageDescription: string;
    brandTitle: string;
    brandSubtitle: string;
    submissionSaved: string;
    rankedHere: string;
    sectionTitle: string;
    stats: {
      participants: string;
      averageSbai: string;
      uploadedTokens: string;
    };
    sortLabels: {
      profanityCount: string;
      tokens: string;
      sbai: string;
      updatedAt: string;
    };
    directionLabels: {
      asc: string;
      desc: string;
    };
    columns: {
      rank: string;
      account: string;
      profanityCount: string;
      tokens: string;
      sbai: string;
      updatedAt: string;
      share: string;
    };
    shareOnX: string;
    shareAriaLabel: string;
    rankAnnouncement: string;
  };
  tarot: {
    pageTitle: string;
    pageDescription: string;
    brandTitle: string;
    brandSubtitle: string;
    kicker: string;
    heroTitle: string;
    heroCopy: string;
    cardFaceLabel: string;
    cardFaceHint: string;
    drawButton: string;
    drawAgain: string;
    resultLabel: string;
    openCard: string;
    moreCards: string;
    cardNotFound: string;
    backToTarot: string;
  };
  report: {
    messageCount: string;
    profanityCount: string;
    tokens: string;
    sbaiLabel: string;
    sbaiKicker: string;
    sbaiMantra: string;
    sbaiChant: string;
    sbaiFootnote: string;
    dailyTitle: string;
    dailyChartAria: string;
    dailyChartFallback: string;
    noDailyData: string;
    cloudTitle: string;
    cloudAria: string;
    cloudFallback: string;
    noProfanity: string;
  };
};

export const DEFAULT_LOCALE: Locale = "en";
export const SUPPORTED_LOCALES: Locale[] = ["en", "zh-CN"];

const dictionaries: Record<Locale, Dictionary> = {
  en: {
    common: {
      language: "Language",
      githubRepository: "GitHub repository",
      nav: {
        leaderboard: "Leaderboard",
        tarot: "AI Tarot",
      },
      actions: {
        backToLeaderboard: "Back to Leaderboard",
        fullProfile: "Full Profile",
        githubProfile: "GitHub Profile",
        shareOnX: "Share on X",
        copyShareLink: "Copy Share Link",
        copied: "Copied",
      },
      missing: {
        userReportTitle: "This user's report was not found.",
        userReportCopy: "They may not have submitted to the leaderboard yet, or the login has changed.",
        shareTitle: "This share card was not found.",
        shareCopy: "The user has not submitted a record yet, or this share link is no longer valid.",
        backToHome: "Back to Leaderboard",
      },
    },
    leaderboard: {
      pageTitle: "maleme leaderboard",
      pageDescription: "Rank the finest AI rage.",
      brandTitle: "maleme leaderboard",
      brandSubtitle: "rank the finest AI rage",
      submissionSaved: "Submission saved",
      rankedHere: "You are now ranked #{rank} here.",
      sectionTitle: "Leaderboard",
      stats: {
        participants: "Participants",
        averageSbai: "Average SBAI",
        uploadedTokens: "Uploaded Tokens",
      },
      sortLabels: {
        profanityCount: "Times Cursed",
        tokens: "Tokens",
        sbai: "SBAI",
        updatedAt: "Updated",
      },
      directionLabels: {
        asc: "Ascending",
        desc: "Descending",
      },
      columns: {
        rank: "Rank",
        account: "Account",
        profanityCount: "Times Cursed",
        tokens: "Tokens",
        sbai: "SBAI",
        updatedAt: "Updated",
        share: "Share",
      },
      shareOnX: "Share on X",
      shareAriaLabel: "Share this leaderboard card on X",
      rankAnnouncement: "You are ranked #{rank} here.",
    },
    tarot: {
      pageTitle: "AI Tarot",
      pageDescription: "The thirteen sins of cursed agent code.",
      brandTitle: "AI Tarot",
      brandSubtitle: "the thirteen sins of cursed agent code",
      kicker: "AI Tarot",
      heroTitle: "Draw your AI tarot.",
      heroCopy: "One pull, one sin. Tap the card and see which cursed habit is following you today.",
      cardFaceLabel: "Tap to draw",
      cardFaceHint: "The card will reveal today's sin.",
      drawButton: "Draw a Card",
      drawAgain: "Draw Again",
      resultLabel: "Today's pull",
      openCard: "Open the card",
      moreCards: "More Cards",
      cardNotFound: "Card not found.",
      backToTarot: "Back to tarot",
    },
    report: {
      messageCount: "Messages",
      profanityCount: "Profanities",
      tokens: "Tokens",
      sbaiLabel: "SBAI Index",
      sbaiKicker: "The more confident the AI sounds",
      sbaiMantra: "the closer a human gets to snapping",
      sbaiChant: "hallucinate / provoke / derail",
      sbaiFootnote: "Profanity events per ten million tokens",
      dailyTitle: "How many times did you curse at AI that day?",
      dailyChartAria: "daily profanity chart",
      dailyChartFallback: "The chart failed to load.",
      noDailyData: "No chat input data was found.",
      cloudTitle: "This is how you like to curse!",
      cloudAria: "High-frequency profanity word cloud with zoom and drag support",
      cloudFallback: "The word cloud failed to load.",
      noProfanity: "No profanity was detected.",
    },
  },
  "zh-CN": {
    common: {
      language: "语言",
      githubRepository: "GitHub 仓库",
      nav: {
        leaderboard: "排行榜",
        tarot: "AI 塔罗",
      },
      actions: {
        backToLeaderboard: "回到排行榜",
        fullProfile: "完整主页",
        githubProfile: "GitHub 主页",
        shareOnX: "分享到 X",
        copyShareLink: "复制分享链接",
        copied: "已复制",
      },
      missing: {
        userReportTitle: "没有找到这个用户的报告",
        userReportCopy: "可能还没提交到 leaderboard，或者登录名已经发生变化。",
        shareTitle: "没有找到这个用户的分享面板",
        shareCopy: "这个用户还没有提交记录，或者分享链接对应的用户 ID 已失效。",
        backToHome: "回到排行榜",
      },
    },
    leaderboard: {
      pageTitle: "maleme 排行榜",
      pageDescription: "看看谁最能骂 AI。",
      brandTitle: "maleme 排行榜",
      brandSubtitle: "看看谁最能骂 AI",
      submissionSaved: "提交成功",
      rankedHere: "你现在排在这里的第 #{rank} 名。",
      sectionTitle: "排行榜",
      stats: {
        participants: "参与人数",
        averageSbai: "平均 SBAI",
        uploadedTokens: "上传 Tokens",
      },
      sortLabels: {
        profanityCount: "骂了多少次",
        tokens: "Tokens",
        sbai: "SBAI",
        updatedAt: "最近更新",
      },
      directionLabels: {
        asc: "正序",
        desc: "倒序",
      },
      columns: {
        rank: "排名",
        account: "账号",
        profanityCount: "骂了多少次",
        tokens: "Tokens",
        sbai: "SBAI",
        updatedAt: "更新时间",
        share: "分享",
      },
      shareOnX: "分享到 X",
      shareAriaLabel: "把这张排行榜卡片分享到 X",
      rankAnnouncement: "你在这里排行第 #{rank} 名。",
    },
    tarot: {
      pageTitle: "AI 塔罗",
      pageDescription: "AI 坏习惯十三宗罪。",
      brandTitle: "AI 塔罗",
      brandSubtitle: "AI 坏习惯十三宗罪",
      kicker: "AI 塔罗",
      heroTitle: "抽一张你的塔罗吧！",
      heroCopy: "轻轻一点，看看今天缠上你的，到底是哪一种最让人血压上来的 AI 坏习惯。",
      cardFaceLabel: "点一下抽牌",
      cardFaceHint: "抽出来的，就是今天的霉运。",
      drawButton: "抽一张",
      drawAgain: "再抽一张",
      resultLabel: "你抽到了",
      openCard: "打开这张牌",
      moreCards: "更多牌面",
      cardNotFound: "没有找到这张牌。",
      backToTarot: "回到塔罗",
    },
    report: {
      messageCount: "聊天输入",
      profanityCount: "脏话次数",
      tokens: "总 Tokens",
      sbaiLabel: "SBAI 指数",
      sbaiKicker: "AI 写得越自信",
      sbaiMantra: "人越接近发疯",
      sbaiChant: "乱写 / 破防 / 暴走",
      sbaiFootnote: "每千万 tokens 的骂人次数",
      dailyTitle: "你这一天骂了 AI 多少次！",
      dailyChartAria: "按天统计的骂人次数折线图",
      dailyChartFallback: "折线图加载失败。",
      noDailyData: "没有聊天输入数据。",
      cloudTitle: "你最喜欢这么骂！",
      cloudAria: "高频脏话词云，支持缩放和拖拽",
      cloudFallback: "词云加载失败。",
      noProfanity: "没有检测到脏话。",
    },
  },
};

export function normalizeLocale(value: string | null | undefined): Locale {
  if (value === "zh" || value === "zh-CN" || value === "zh_CN" || value === "cn") {
    return "zh-CN";
  }

  return "en";
}

export function getNumberLocale(locale: Locale) {
  return locale === "zh-CN" ? "zh-CN" : "en-US";
}

export function getI18n(locale: Locale) {
  return dictionaries[locale];
}

export function resolveLocale(context: LocaleContext): Locale {
  const queryLocale = context.url.searchParams.get("lang");
  if (queryLocale) {
    const locale = normalizeLocale(queryLocale);
    context.cookies.set("maleme_lang", locale, {
      path: "/",
      maxAge: 60 * 60 * 24 * 365,
      sameSite: "lax",
      secure: context.url.protocol === "https:",
      domain: context.url.hostname.endsWith(".sbai.uk") ? ".sbai.uk" : undefined,
    });
    return locale;
  }

  const cookieLocale = context.cookies.get("maleme_lang")?.value;
  if (cookieLocale) {
    return normalizeLocale(cookieLocale);
  }

  const acceptLanguage = context.request.headers.get("accept-language") || "";
  if (acceptLanguage.toLowerCase().includes("zh")) {
    return "zh-CN";
  }

  return DEFAULT_LOCALE;
}

export function formatTemplate(template: string, values: Record<string, string | number>) {
  return template.replace(/\{(\w+)\}/g, (_, key) => String(values[key] ?? ""));
}
