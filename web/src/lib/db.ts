import { env } from "cloudflare:workers";
import { createFallbackReportPayload, parseReportPayloadJson } from "./report";
import type {
  LeaderboardEntry,
  LeaderboardProfile,
  LeaderboardReportPayload,
  LeaderboardSummary,
  Viewer,
} from "./types";

type LeaderboardRow = Omit<LeaderboardEntry, "rank">;
type SummaryRow = {
  participants: number;
  totalEvents: number;
  totalTokens: number;
  averageSbai: number;
};
type PendingSubmissionRow = {
  payloadJson: string;
};
type LeaderboardProfileRow = LeaderboardRow & {
  rank: number;
  reportPayloadJson: string | null;
  submittedAt: number | null;
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

export async function getLeaderboardProfile(login: string) {
  const database = getDatabase();
  const row = await database
    .prepare(
      `
        WITH ranked_entries AS (
          SELECT
            github_id,
            login,
            display_name,
            avatar_url,
            profile_url,
            profanity_count,
            tokens,
            sbai,
            updated_at,
            ROW_NUMBER() OVER (
              ORDER BY profanity_count DESC, sbai DESC, tokens DESC, updated_at ASC
            ) AS rank
          FROM leaderboard_entries
        )
        SELECT
          ranked_entries.rank AS rank,
          ranked_entries.github_id AS githubId,
          ranked_entries.login AS login,
          ranked_entries.display_name AS displayName,
          ranked_entries.avatar_url AS avatarUrl,
          ranked_entries.profile_url AS profileUrl,
          ranked_entries.profanity_count AS profanityCount,
          ranked_entries.tokens AS tokens,
          ranked_entries.sbai AS sbai,
          ranked_entries.updated_at AS updatedAt,
          latest.report_payload_json AS reportPayloadJson,
          latest.created_at AS submittedAt
        FROM ranked_entries
        LEFT JOIN leaderboard_submissions AS latest
          ON latest.id = (
            SELECT id
            FROM leaderboard_submissions
            WHERE github_id = ranked_entries.github_id
            ORDER BY created_at DESC
            LIMIT 1
          )
        WHERE ranked_entries.login = ? COLLATE NOCASE
      `,
    )
    .bind(login)
    .first<LeaderboardProfileRow>();

  if (!row) {
    return null;
  }

  const fallback = createFallbackReportPayload({
    profanityCount: row.profanityCount,
    tokens: row.tokens,
    sbai: row.sbai,
  });

  return {
    rank: row.rank,
    githubId: row.githubId,
    login: row.login,
    displayName: row.displayName,
    avatarUrl: row.avatarUrl,
    profileUrl: row.profileUrl,
    profanityCount: row.profanityCount,
    tokens: row.tokens,
    sbai: row.sbai,
    updatedAt: row.updatedAt,
    submittedAt: row.submittedAt ?? row.updatedAt,
    report: parseReportPayloadJson(row.reportPayloadJson, fallback),
  } satisfies LeaderboardProfile;
}

export async function createPendingSubmission(payload: LeaderboardReportPayload, ttlMs: number) {
  const database = getDatabase();
  const createdAt = Date.now();
  const expiresAt = createdAt + ttlMs;
  const token = crypto.randomUUID();

  await database
    .prepare(
      `
        INSERT INTO leaderboard_pending_submissions (
          token,
          payload_json,
          created_at,
          expires_at
        ) VALUES (?, ?, ?, ?)
      `,
    )
    .bind(token, JSON.stringify(payload), createdAt, expiresAt)
    .run();

  return token;
}

export async function consumePendingSubmission(token: string) {
  const database = getDatabase();
  const now = Date.now();
  const row = await database
    .prepare(
      `
        SELECT payload_json AS payloadJson
        FROM leaderboard_pending_submissions
        WHERE token = ?
          AND expires_at > ?
      `,
    )
    .bind(token, now)
    .first<PendingSubmissionRow>();

  await database
    .prepare(
      `
        DELETE FROM leaderboard_pending_submissions
        WHERE token = ?
           OR expires_at <= ?
      `,
    )
    .bind(token, now)
    .run();

  if (!row) {
    return null;
  }

  return parseReportPayloadJson(row.payloadJson);
}

export async function upsertLeaderboardEntry(
  viewer: Viewer,
  submission: LeaderboardReportPayload,
) {
  const database = getDatabase();
  const updatedAt = Date.now();
  const reportPayloadJson = JSON.stringify(submission);

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
      submission.profanityCount,
      submission.tokens,
      submission.sbai,
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
          report_payload_json,
          created_at
        ) VALUES (?, ?, ?, ?, ?, ?)
      `,
    )
    .bind(
      viewer.githubId,
      submission.profanityCount,
      submission.tokens,
      submission.sbai,
      reportPayloadJson,
      updatedAt,
    )
    .run();
}
