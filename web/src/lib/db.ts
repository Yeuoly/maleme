import { env } from "cloudflare:workers";
import type { LeaderboardEntry, LeaderboardSummary, Viewer } from "./types";

type LeaderboardRow = Omit<LeaderboardEntry, "rank">;
type SummaryRow = {
  participants: number;
  totalEvents: number;
  totalTokens: number;
  averageSbai: number;
};

function getDatabase() {
  if (!env.DB) {
    throw new Error("D1 binding DB is missing.");
  }

  return env.DB;
}

export function hasDatabaseBinding() {
  return Boolean(env.DB);
}

export async function listLeaderboard(limit = 50) {
  const database = getDatabase();
  const result = await database
    .prepare(
      `
        SELECT
          github_id AS githubId,
          login,
          display_name AS displayName,
          avatar_url AS avatarUrl,
          profile_url AS profileUrl,
          profanity_count AS profanityCount,
          tokens,
          sbai,
          updated_at AS updatedAt
        FROM leaderboard_entries
        ORDER BY profanity_count DESC, sbai DESC, tokens DESC, updated_at ASC
        LIMIT ?
      `,
    )
    .bind(limit)
    .all<LeaderboardRow>();

  return result.results.map((row, index) => ({
    rank: index + 1,
    ...row,
  })) satisfies LeaderboardEntry[];
}

export async function getLeaderboardSummary() {
  const database = getDatabase();
  const row = await database
    .prepare(
      `
        SELECT
          COUNT(*) AS participants,
          COALESCE(SUM(profanity_count), 0) AS totalEvents,
          COALESCE(SUM(tokens), 0) AS totalTokens,
          COALESCE(AVG(sbai), 0) AS averageSbai
        FROM leaderboard_entries
      `,
    )
    .first<SummaryRow>();

  return {
    participants: Number(row?.participants || 0),
    totalEvents: Number(row?.totalEvents || 0),
    totalTokens: Number(row?.totalTokens || 0),
    averageSbai: Number(row?.averageSbai || 0),
  } satisfies LeaderboardSummary;
}

export async function getViewerEntry(githubId: number) {
  const database = getDatabase();
  const row = await database
    .prepare(
      `
        SELECT
          github_id AS githubId,
          login,
          display_name AS displayName,
          avatar_url AS avatarUrl,
          profile_url AS profileUrl,
          profanity_count AS profanityCount,
          tokens,
          sbai,
          updated_at AS updatedAt
        FROM leaderboard_entries
        WHERE github_id = ?
      `,
    )
    .bind(githubId)
    .first<LeaderboardRow>();

  if (!row) {
    return null;
  }

  return row;
}

export async function upsertLeaderboardEntry(
  viewer: Viewer,
  profanityCount: number,
  tokens: number,
  sbai: number,
) {
  const database = getDatabase();
  const updatedAt = Date.now();

  await database
    .prepare(
      `
        INSERT INTO leaderboard_entries (
          github_id,
          login,
          display_name,
          avatar_url,
          profile_url,
          profanity_count,
          tokens,
          sbai,
          updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(github_id) DO UPDATE SET
          login = excluded.login,
          display_name = excluded.display_name,
          avatar_url = excluded.avatar_url,
          profile_url = excluded.profile_url,
          profanity_count = excluded.profanity_count,
          tokens = excluded.tokens,
          sbai = excluded.sbai,
          updated_at = excluded.updated_at
      `,
    )
    .bind(
      viewer.githubId,
      viewer.login,
      viewer.displayName,
      viewer.avatarUrl,
      viewer.profileUrl,
      profanityCount,
      tokens,
      sbai,
      updatedAt,
    )
    .run();

  await database
    .prepare(
      `
        INSERT INTO leaderboard_submissions (
          github_id,
          profanity_count,
          tokens,
          sbai,
          created_at
        ) VALUES (?, ?, ?, ?, ?)
      `,
    )
    .bind(viewer.githubId, profanityCount, tokens, sbai, updatedAt)
    .run();
}
