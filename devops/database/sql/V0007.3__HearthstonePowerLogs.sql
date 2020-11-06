CREATE TABLE hearthstone_raw_power_logs (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    match_uuid UUID NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    raw_logs JSONB NOT NULL,
    parsed BOOLEAN NOT NULL DEFAULT false,
    FOREIGN KEY (match_uuid, user_id) REFERENCES hearthstone_match_players (match_uuid, user_id) ON DELETE CASCADE
);