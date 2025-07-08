CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE TABLE freeze_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    repository VARCHAR NOT NULL,
    installation_id BIGINT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    reason TEXT,
    initiated_by VARCHAR NOT NULL,
    ended_by VARCHAR,
    status VARCHAR NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE permission_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    installation_id BIGINT NOT NULL,
    repository VARCHAR NOT NULL,
    user_login VARCHAR NOT NULL,
    role VARCHAR NOT NULL,
    can_freeze BOOLEAN NOT NULL DEFAULT FALSE,
    can_unfreeze BOOLEAN NOT NULL DEFAULT FALSE,
    can_emergency_override BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(installation_id, repository, user_login)
);

CREATE TABLE command_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    installation_id BIGINT NOT NULL,
    repository VARCHAR NOT NULL,
    user_login VARCHAR NOT NULL,
    command VARCHAR NOT NULL,
    comment_id BIGINT NOT NULL,
    result VARCHAR NOT NULL,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_freeze_records_repo ON freeze_records(repository, status);
CREATE INDEX idx_freeze_records_installation ON freeze_records(installation_id);
CREATE INDEX idx_permission_records_user ON permission_records(installation_id, user_login);
CREATE INDEX idx_command_logs_repo ON command_logs(repository, created_at);
