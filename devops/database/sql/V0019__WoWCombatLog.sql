CREATE TABLE wow_combat_logs (
    uuid UUID PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    combat_log_version VARCHAR NOT NULL,
    advanced_log BOOLEAN NOT NULL,
    build_version VARCHAR NOT NULL
);