CREATE TABLE hearthstone_actions (
    match_uuid UUID UNIQUE NOT NULL REFERENCES hearthstone_matches (match_uuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action_id BIGINT NOT NULL,
    tm TIMESTAMPTZ NOT NULL,
    entity_id INTEGER NOT NULL,
    tags JSONB NOT NULL,
    attributes JSONB NOT NULL,
    PRIMARY KEY(match_uuid, user_id, action_id)
);