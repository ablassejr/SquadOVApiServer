CREATE TABLE aimlab_tasks (
    id BIGINT NOT NULL,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    task_name VARCHAR NOT NULL,
    mode INTEGER NOT NULL,
    score BIGINT NOT NULL,
    version VARCHAR NOT NULL,
    raw_data JSONB NOT NULL
);