ALTER TABLE leaderboard_submissions
  ADD COLUMN report_payload_json TEXT;

CREATE TABLE IF NOT EXISTS leaderboard_pending_submissions (
  token TEXT PRIMARY KEY,
  payload_json TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS leaderboard_pending_submissions_expires_idx
  ON leaderboard_pending_submissions (expires_at);
