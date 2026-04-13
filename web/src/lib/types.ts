export type Viewer = {
  githubId: number;
  login: string;
  displayName: string;
  avatarUrl: string;
  profileUrl: string;
};

export type LeaderboardEntry = {
  rank: number;
  githubId: number;
  login: string;
  displayName: string;
  avatarUrl: string;
  profileUrl: string;
  profanityCount: number;
  tokens: number;
  sbai: number;
  updatedAt: number;
};

export type ReportDailyCount = {
  label: string;
  count: number;
};

export type ReportWordCount = {
  term: string;
  count: number;
};

export type LeaderboardReportPayload = {
  rangeStart: string;
  rangeEnd: string;
  messageCount: number;
  profanityCount: number;
  tokens: number;
  sbai: number;
  dailyCounts: ReportDailyCount[];
  wordCounts: ReportWordCount[];
};

export type LeaderboardProfile = Omit<LeaderboardEntry, "updatedAt"> & {
  updatedAt: number;
  submittedAt: number;
  report: LeaderboardReportPayload;
};

export type LeaderboardSummary = {
  participants: number;
  totalEvents: number;
  totalTokens: number;
  averageSbai: number;
};
