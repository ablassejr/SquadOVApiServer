-- Allow items to be null so that we can create a match immediately even if pulling the data fails.
CREATE TABLE valorant_matches (
    match_id VARCHAR PRIMARY KEY,
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    game_mode VARCHAR,
    map VARCHAR,
    is_ranked BOOLEAN,
    provisioning_flow_id VARCHAR,
    game_version VARCHAR,
    server_start_time_utc TIMESTAMPTZ,
    raw_data JSONB
);