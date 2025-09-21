CREATE TABLE freeze_records (
    id TEXT PRIMARY KEY,
    repository TEXT NOT NULL,
    installation_id INTEGER NOT NULL,
    started_at TEXT NOT NULL,
    expires_at TEXT,
    ended_at TEXT,
    reason TEXT,
    initiated_by TEXT NOT NULL,
    ended_by TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE permission_records (
    id TEXT PRIMARY KEY,
    installation_id INTEGER NOT NULL,
    repository TEXT NOT NULL,
    user_login TEXT NOT NULL,
    role TEXT NOT NULL,
    can_freeze INTEGER NOT NULL DEFAULT 0,
    can_unfreeze INTEGER NOT NULL DEFAULT 0,
    can_emergency_override INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(installation_id, repository, user_login)
);

CREATE TABLE command_logs (
    id TEXT PRIMARY KEY,
    installation_id INTEGER NOT NULL,
    repository TEXT NOT NULL,
    user_login TEXT NOT NULL,
    command TEXT NOT NULL,
    comment_id INTEGER NOT NULL,
    result TEXT NOT NULL,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_freeze_records_repo ON freeze_records(repository, status);
CREATE INDEX idx_freeze_records_installation ON freeze_records(installation_id);
CREATE INDEX idx_permission_records_user ON permission_records(installation_id, user_login);
CREATE INDEX idx_command_logs_repo ON command_logs(repository, created_at);
