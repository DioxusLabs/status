CREATE TABLE milestones (
  repo TEXT NOT NULL,
  number INTEGER NOT NULL,
  title TEXT,
  description TEXT,
  due_on TEXT,
  state TEXT,
  open_issues INTEGER,
  closed_issues INTEGER,
  url TEXT,
  synced_at TEXT,
  PRIMARY KEY (repo, number)
);

CREATE TABLE release_targets (
  id INTEGER PRIMARY KEY,
  repo TEXT NOT NULL,
  version TEXT NOT NULL,
  kind TEXT NOT NULL CHECK(kind IN ('pr','issue','note')),
  number INTEGER,
  title TEXT NOT NULL,
  done INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);
CREATE INDEX idx_release_targets_repo ON release_targets(repo);
