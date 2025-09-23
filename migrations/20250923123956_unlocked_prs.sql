CREATE TABLE unlocked_prs (
    id TEXT PRIMARY KEY,
    repository TEXT NOT NULL,
    installation_id INTEGER NOT NULL,
    pr_number INTEGER NOT NULL,
    unlocked_by TEXT NOT NULL,
    unlocked_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(installation_id, repository, pr_number)
);

CREATE INDEX idx_unlocked_prs_repo ON unlocked_prs(repository, installation_id);
CREATE INDEX idx_unlocked_prs_pr ON unlocked_prs(installation_id, repository, pr_number);
