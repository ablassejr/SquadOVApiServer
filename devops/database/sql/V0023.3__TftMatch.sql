CREATE TABLE tft_match_info (
    match_uuid UUID UNIQUE NOT NULL REFERENCES tft_matches (match_uuid) ON DELETE CASCADE,
    game_datetime TIMESTAMPTZ NOT NULL,
    game_length REAL NOT NULL,
    game_variation VARCHAR NOT NULL,
    game_version VARCHAR NOT NULL,
    queue_id INTEGER NOT NULL,
    tft_set_number INTEGER NOT NULL
);

CREATE TABLE tft_match_participants (
    match_uuid UUID NOT NULL REFERENCES tft_matches (match_uuid) ON DELETE CASCADE,
    puuid VARCHAR NOT NULL,
    gold_left INTEGER NOT NULL,
    last_round INTEGER NOT NULL,
    level INTEGER NOT NULL,
    placement INTEGER NOT NULL,
    players_eliminated INTEGER NOT NULL,
    time_eliminated REAL NOT NULL,
    total_damage_to_players INTEGER NOT NULL,
    companion_content_id VARCHAR NOT NULL,
    companion_skin_id VARCHAR NOT NULL,
    companion_species VARCHAR NOT NULL,
    UNIQUE(match_uuid, puuid)
);

CREATE TABLE tft_match_participant_traits (
    match_uuid UUID NOT NULL REFERENCES tft_matches (match_uuid) ON DELETE CASCADE,
    puuid VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    num_units INTEGER NOT NULL,
    style INTEGER NOT NULL,
    tier_current INTEGER NOT NULL,
    tier_total INTEGER NOT NULL,
    FOREIGN KEY (match_uuid, puuid) REFERENCES tft_match_participants(match_uuid, puuid) ON DELETE CASCADE
);

CREATE TABLE tft_match_participant_units (
    match_uuid UUID NOT NULL REFERENCES tft_matches (match_uuid) ON DELETE CASCADE,
    puuid VARCHAR NOT NULL,
    character_id VARCHAR,
    chosen VARCHAR,
    name VARCHAR NOT NULL,
    rarity INTEGER NOT NULL,
    tier INTEGER NOT NULL,
    items INTEGER[] NOT NULL,
    FOREIGN KEY (match_uuid, puuid) REFERENCES tft_match_participants(match_uuid, puuid) ON DELETE CASCADE
);