import type { LeaderboardReportPayload, ReportDailyCount, ReportWordCount } from "./types";

type ReportTheme = {
  bg: string;
  bgAccent: string;
  panel: string;
  panelStrong: string;
  muted: string;
  text: string;
  line: string;
  lineSoft: string;
  accent: string;
  accentSoft: string;
  accentWarm: string;
  accentPink: string;
  border: string;
  shadow: string;
  heroFrom: string;
  heroTo: string;
  heroGlowA: string;
  heroGlowB: string;
  sbaiGlow: string;
  sbaiSurfaceTop: string;
  sbaiSurfaceBottom: string;
  sbaiBorder: string;
  sbaiText: string;
  sbaiMuted: string;
};

function asNonNegativeInteger(value: unknown, fallback = 0) {
  const number = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(number) || number < 0) {
    return fallback;
  }

  return Math.trunc(number);
}

function asNonNegativeFloat(value: unknown, fallback = 0) {
  const number = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(number) || number < 0) {
    return fallback;
  }

  return number;
}

function normalizeDailyCounts(value: unknown) {
  if (!Array.isArray(value)) {
    return [] satisfies ReportDailyCount[];
  }

  return value.flatMap((item) => {
    if (!item || typeof item !== "object") {
      return [];
    }

    const label = "label" in item ? String(item.label ?? "").trim() : "";
    const count = "count" in item ? asNonNegativeInteger(item.count) : 0;

    if (!label) {
      return [];
    }

    return [{ label, count }];
  });
}

function normalizeWordCounts(value: unknown) {
  if (!Array.isArray(value)) {
    return [] satisfies ReportWordCount[];
  }

  return value.flatMap((item) => {
    if (!item || typeof item !== "object") {
      return [];
    }

    const term = "term" in item ? String(item.term ?? "").trim() : "";
    const count = "count" in item ? asNonNegativeInteger(item.count) : 0;

    if (!term) {
      return [];
    }

    return [{ term, count }];
  });
}

export function createFallbackReportPayload(
  input: Partial<Pick<LeaderboardReportPayload, "profanityCount" | "tokens" | "sbai">> = {},
) {
  return {
    rangeStart: "还没有记录",
    rangeEnd: "还没有记录",
    messageCount: 0,
    profanityCount: asNonNegativeInteger(input.profanityCount),
    tokens: asNonNegativeInteger(input.tokens),
    sbai: asNonNegativeFloat(input.sbai),
    dailyCounts: [],
    wordCounts: [],
  } satisfies LeaderboardReportPayload;
}

export function normalizeReportPayload(
  value: unknown,
  fallback: Partial<LeaderboardReportPayload> = {},
): LeaderboardReportPayload {
  const source = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  const defaults = createFallbackReportPayload(fallback);

  return {
    rangeStart: typeof source.rangeStart === "string" && source.rangeStart.trim() ? source.rangeStart : defaults.rangeStart,
    rangeEnd: typeof source.rangeEnd === "string" && source.rangeEnd.trim() ? source.rangeEnd : defaults.rangeEnd,
    messageCount: asNonNegativeInteger(source.messageCount, defaults.messageCount),
    profanityCount: asNonNegativeInteger(source.profanityCount, defaults.profanityCount),
    tokens: asNonNegativeInteger(source.tokens, defaults.tokens),
    sbai: asNonNegativeFloat(source.sbai, defaults.sbai),
    dailyCounts: normalizeDailyCounts(source.dailyCounts),
    wordCounts: normalizeWordCounts(source.wordCounts),
  };
}

export function parseReportPayloadJson(
  value: string | null | undefined,
  fallback: Partial<LeaderboardReportPayload> = {},
) {
  if (!value?.trim()) {
    return createFallbackReportPayload(fallback);
  }

  try {
    return normalizeReportPayload(JSON.parse(value), fallback);
  } catch {
    return createFallbackReportPayload(fallback);
  }
}

export function getSbaiStatus(sbai: number) {
  if (sbai < 0.5) {
    return {
      state: "还能忍",
      copy: "AI 还在试探，你已经有点绷不住了。",
    };
  }

  if (sbai < 2) {
    return {
      state: "开始红温",
      copy: "AI 每多自信一分，人就更想当场开骂。",
    };
  }

  if (sbai < 5) {
    return {
      state: "马上开骂",
      copy: "AI 写得越笃定，人越接近发疯。",
    };
  }

  return {
    state: "彻底爆炸",
    copy: "已经不是调试，是一场精神消耗战。",
  };
}

export function getReportTheme(sbai: number): ReportTheme {
  if (sbai < 0.5) {
    return {
      bg: "#090d12",
      bgAccent: "#121821",
      panel: "rgba(10, 15, 20, 0.84)",
      panelStrong: "rgba(13, 18, 24, 0.94)",
      muted: "#8ca5af",
      text: "#edf8ff",
      line: "#ffb800",
      lineSoft: "rgba(255, 184, 0, 0.18)",
      accent: "#59e8ff",
      accentSoft: "rgba(89, 232, 255, 0.16)",
      accentWarm: "#ffe85b",
      accentPink: "#ff5f7d",
      border: "rgba(89, 232, 255, 0.22)",
      shadow: "0 20px 48px rgba(4, 7, 12, 0.34)",
      heroFrom: "rgba(11, 16, 22, 0.96)",
      heroTo: "rgba(13, 20, 27, 0.96)",
      heroGlowA: "rgba(255, 232, 91, 0.16)",
      heroGlowB: "rgba(89, 232, 255, 0.16)",
      sbaiGlow: "rgba(255, 184, 0, 0.24)",
      sbaiSurfaceTop: "#0b0e13",
      sbaiSurfaceBottom: "#1b1910",
      sbaiBorder: "rgba(255, 232, 171, 0.24)",
      sbaiText: "rgba(255, 246, 210, 0.96)",
      sbaiMuted: "rgba(255, 226, 158, 0.82)",
    };
  }

  if (sbai < 2) {
    return {
      bg: "#0a0d12",
      bgAccent: "#16151f",
      panel: "rgba(12, 15, 21, 0.86)",
      panelStrong: "rgba(14, 18, 25, 0.95)",
      muted: "#9ca0b1",
      text: "#f7f7ff",
      line: "#ff8f1f",
      lineSoft: "rgba(255, 143, 31, 0.2)",
      accent: "#57d8ff",
      accentSoft: "rgba(87, 216, 255, 0.18)",
      accentWarm: "#ffd447",
      accentPink: "#ff5f7d",
      border: "rgba(104, 143, 166, 0.34)",
      shadow: "0 20px 48px rgba(4, 7, 12, 0.36)",
      heroFrom: "rgba(14, 18, 24, 0.96)",
      heroTo: "rgba(19, 18, 28, 0.96)",
      heroGlowA: "rgba(255, 212, 71, 0.16)",
      heroGlowB: "rgba(87, 216, 255, 0.18)",
      sbaiGlow: "rgba(255, 143, 31, 0.28)",
      sbaiSurfaceTop: "#0b0d12",
      sbaiSurfaceBottom: "#251717",
      sbaiBorder: "rgba(255, 192, 108, 0.26)",
      sbaiText: "rgba(255, 236, 199, 0.96)",
      sbaiMuted: "rgba(255, 198, 123, 0.84)",
    };
  }

  if (sbai < 5) {
    return {
      bg: "#0b0a0f",
      bgAccent: "#1a1017",
      panel: "rgba(15, 12, 18, 0.88)",
      panelStrong: "rgba(18, 14, 20, 0.96)",
      muted: "#b39aa3",
      text: "#fff3ef",
      line: "#ff5d3d",
      lineSoft: "rgba(255, 93, 61, 0.24)",
      accent: "#4fdbff",
      accentSoft: "rgba(79, 219, 255, 0.16)",
      accentWarm: "#ffd54a",
      accentPink: "#ff4f7a",
      border: "rgba(118, 69, 84, 0.8)",
      shadow: "0 24px 54px rgba(5, 4, 9, 0.42)",
      heroFrom: "rgba(18, 13, 19, 0.96)",
      heroTo: "rgba(24, 13, 20, 0.96)",
      heroGlowA: "rgba(255, 213, 74, 0.14)",
      heroGlowB: "rgba(79, 219, 255, 0.12)",
      sbaiGlow: "rgba(255, 93, 61, 0.34)",
      sbaiSurfaceTop: "#0a0a0e",
      sbaiSurfaceBottom: "#4a1017",
      sbaiBorder: "rgba(255, 173, 118, 0.26)",
      sbaiText: "rgba(255, 238, 202, 0.97)",
      sbaiMuted: "rgba(255, 189, 138, 0.86)",
    };
  }

  return {
    bg: "#07080b",
    bgAccent: "#170b10",
    panel: "rgba(14, 10, 14, 0.9)",
    panelStrong: "rgba(17, 12, 16, 0.97)",
    muted: "#c7a4ac",
    text: "#fff4ef",
    line: "#ff3b30",
    lineSoft: "rgba(255, 59, 48, 0.28)",
    accent: "#46dcff",
    accentSoft: "rgba(70, 220, 255, 0.18)",
    accentWarm: "#ffd93d",
    accentPink: "#ff3f76",
    border: "rgba(123, 45, 63, 0.86)",
    shadow: "0 26px 58px rgba(3, 3, 6, 0.48)",
    heroFrom: "rgba(18, 11, 15, 0.97)",
    heroTo: "rgba(22, 11, 16, 0.97)",
    heroGlowA: "rgba(255, 217, 61, 0.14)",
    heroGlowB: "rgba(70, 220, 255, 0.1)",
    sbaiGlow: "rgba(255, 59, 48, 0.4)",
    sbaiSurfaceTop: "#09090c",
    sbaiSurfaceBottom: "#5d0914",
    sbaiBorder: "rgba(255, 158, 105, 0.28)",
    sbaiText: "rgba(255, 239, 201, 0.98)",
    sbaiMuted: "rgba(255, 184, 125, 0.88)",
  };
}
