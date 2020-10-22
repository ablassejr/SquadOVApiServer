CREATE TABLE valorant_player_match_metadata (
    match_id VARCHAR NOT NULL REFERENCES valorant_matches(match_id) ON DELETE CASCADE,
    puuid VARCHAR NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    UNIQUE(match_id, puuid)
);

CREATE TABLE valorant_player_round_metadata (
    match_id VARCHAR NOT NULL REFERENCES valorant_matches(match_id) ON DELETE CASCADE,
    puuid VARCHAR NOT NULL,
    round INTEGER NOT NULL,
    buy_time TIMESTAMPTZ,
    round_time TIMESTAMPTZ,
    FOREIGN KEY (match_id, puuid) REFERENCES valorant_player_match_metadata(match_id, puuid) ON DELETE CASCADE,
    UNIQUE(match_id, puuid, round)
);