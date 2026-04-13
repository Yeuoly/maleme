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

export type LeaderboardSummary = {
  participants: number;
  totalEvents: number;
  totalTokens: number;
  averageSbai: number;
};
