CREATE TABLE IF NOT EXISTS leaderboard_entries (
  github_id INTEGER PRIMARY KEY,
  login TEXT NOT NULL,
  display_name TEXT NOT NULL,
  avatar_url TEXT NOT NULL,
  profile_url TEXT NOT NULL,
  profanity_count INTEGER NOT NULL,
  tokens INTEGER NOT NULL,
  sbai REAL NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS leaderboard_entries_rank_idx
  ON leaderboard_entries (profanity_count DESC, sbai DESC, tokens DESC, updated_at ASC);

CREATE TABLE IF NOT EXISTS leaderboard_submissions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  github_id INTEGER NOT NULL,
  profanity_count INTEGER NOT NULL,
  tokens INTEGER NOT NULL,
  sbai REAL NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS leaderboard_submissions_user_idx
  ON leaderboard_submissions (github_id, created_at DESC);
