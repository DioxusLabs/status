CREATE TABLE devin_sessions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT UNIQUE,
  url TEXT,
  kind TEXT NOT NULL,
  repo TEXT NOT NULL, number INTEGER NOT NULL, head_sha TEXT,
  title TEXT, prompt TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'created',
  result_pr_url TEXT, structured_output_json TEXT, error TEXT,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL, last_polled_at TEXT
);
CREATE INDEX idx_devin_sessions_pr ON devin_sessions(repo, number);

CREATE TABLE pr_assessments (
  repo TEXT NOT NULL, number INTEGER NOT NULL, head_sha TEXT NOT NULL,
  session_id TEXT, verdict TEXT, summary TEXT, risks_json TEXT, suggestions_json TEXT,
  quality_score INTEGER, created_at TEXT NOT NULL,
  PRIMARY KEY (repo, number, head_sha)
);

CREATE TABLE llm_budget (day TEXT PRIMARY KEY, used INTEGER NOT NULL DEFAULT 0);
