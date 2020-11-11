CREATE TABLE hearthstone_match_metadata (
    match_uuid UUID UNIQUE NOT NULL REFERENCES hearthstone_matches (match_uuid) ON DELETE CASCADE,
    game_type INTEGER NOT NULL,
    format_type INTEGER NOT NULL,
    scenario_id INTEGER NOT NULL
);