CREATE TABLE IF NOT EXISTS repos (
    name TEXT PRIMARY KEY,
    owner TEXT,
    description TEXT,
    stars INTEGER NOT NULL DEFAULT 0,
    forks INTEGER NOT NULL DEFAULT 0,
    open_issues INTEGER NOT NULL DEFAULT 0,
    open_prs INTEGER NOT NULL DEFAULT 0,
    pushed_at TEXT,
    is_fork INTEGER NOT NULL DEFAULT 0,
    monitored INTEGER NOT NULL DEFAULT 0,
    default_branch TEXT,
    synced_at TEXT
);

CREATE TABLE IF NOT EXISTS pull_requests (
    id INTEGER PRIMARY KEY,
    repo TEXT NOT NULL,
    number INTEGER NOT NULL,
    title TEXT,
    body TEXT,
    author TEXT,
    author_association TEXT,
    url TEXT,
    state TEXT,
    is_draft INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    merged_at TEXT,
    closed_at TEXT,
    head_sha TEXT,
    base_ref TEXT,
    additions INTEGER NOT NULL DEFAULT 0,
    deletions INTEGER NOT NULL DEFAULT 0,
    changed_files INTEGER NOT NULL DEFAULT 0,
    mergeable TEXT,
    review_decision TEXT,
    ci_state TEXT NOT NULL DEFAULT 'none',
    labels_json TEXT NOT NULL DEFAULT '[]',
    reviewers_json TEXT NOT NULL DEFAULT '[]',
    unresolved_threads INTEGER NOT NULL DEFAULT 0,
    comments INTEGER NOT NULL DEFAULT 0,
    last_activity_at TEXT,
    last_activity_by TEXT,
    linked_issues_json TEXT NOT NULL DEFAULT '[]',
    files_json TEXT NOT NULL DEFAULT '[]',
    score INTEGER NOT NULL DEFAULT 0,
    score_breakdown_json TEXT NOT NULL DEFAULT '[]',
    synced_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_prs_repo_state ON pull_requests(repo, state);

CREATE TABLE IF NOT EXISTS issues (
    id INTEGER PRIMARY KEY,
    repo TEXT NOT NULL,
    number INTEGER NOT NULL,
    title TEXT,
    body TEXT,
    author TEXT,
    author_association TEXT,
    url TEXT,
    state TEXT,
    created_at TEXT,
    updated_at TEXT,
    closed_at TEXT,
    labels_json TEXT NOT NULL DEFAULT '[]',
    comments INTEGER NOT NULL DEFAULT 0,
    reactions INTEGER NOT NULL DEFAULT 0,
    assignees_json TEXT NOT NULL DEFAULT '[]',
    last_activity_at TEXT,
    last_activity_by TEXT,
    synced_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_issues_repo_state ON issues(repo, state);

CREATE TABLE IF NOT EXISTS crates (
    name TEXT PRIMARY KEY,
    repo TEXT,
    total_downloads INTEGER NOT NULL DEFAULT 0,
    recent_downloads INTEGER NOT NULL DEFAULT 0,
    latest_version TEXT,
    latest_version_at TEXT,
    synced_at TEXT
);

CREATE TABLE IF NOT EXISTS crate_downloads_daily (
    crate_name TEXT NOT NULL,
    date TEXT NOT NULL,
    downloads INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (crate_name, date)
);

CREATE TABLE IF NOT EXISTS crate_versions (
    crate_name TEXT NOT NULL,
    version TEXT NOT NULL,
    downloads INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    PRIMARY KEY (crate_name, version)
);

CREATE TABLE IF NOT EXISTS releases (
    repo TEXT NOT NULL,
    tag TEXT NOT NULL,
    name TEXT,
    published_at TEXT,
    url TEXT,
    is_prerelease INTEGER NOT NULL DEFAULT 0,
    assets_json TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (repo, tag)
);

CREATE TABLE IF NOT EXISTS repo_snapshots (
    repo TEXT NOT NULL,
    date TEXT NOT NULL,
    stars INTEGER NOT NULL DEFAULT 0,
    forks INTEGER NOT NULL DEFAULT 0,
    open_issues INTEGER NOT NULL DEFAULT 0,
    open_prs INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (repo, date)
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT
);

CREATE TABLE IF NOT EXISTS sync_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT,
    repo TEXT,
    started_at TEXT,
    finished_at TEXT,
    ok INTEGER NOT NULL DEFAULT 0,
    message TEXT
);
