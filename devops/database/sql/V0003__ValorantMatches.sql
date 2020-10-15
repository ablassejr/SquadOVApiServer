CREATE TABLE valorant_matches (
    match_id VARCHAR PRIMARY KEY,
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    game_mode VARCHAR NOT NULL,
    map VARCHAR NOT NULL,
    is_ranked BOOLEAN NOT NULL,
    provisioning_flow_id VARCHAR NOT NULL,
    game_version VARCHAR NOT NULL,
    server_start_time_utc TIMESTAMPTZ NOT NULL,
    raw_data JSONB NOT NULL
);