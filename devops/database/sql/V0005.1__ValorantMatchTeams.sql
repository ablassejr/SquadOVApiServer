CREATE TABLE valorant_match_teams (
    match_id VARCHAR NOT NULL REFERENCES valorant_matches(match_id) ON DELETE CASCADE,
    team_id VARCHAR NOT NULL,
    won BOOLEAN NOT NULL,
    rounds_won INTEGER NOT NULL,
    rounds_played INTEGER NOT NULL,
    PRIMARY KEY(match_id, team_id)
);

CREATE TABLE valorant_match_players (
    match_id VARCHAR NOT NULL REFERENCES valorant_matches(match_id) ON DELETE CASCADE,
    team_id VARCHAR NOT NULL,
    puuid VARCHAR NOT NULL,
    character_id VARCHAR NOT NULL,
    competitive_tier INTEGER NOT NULL,
    total_combat_score INTEGER NOT NULL,
    rounds_played INTEGER NOT NULL,
    kills INTEGER NOT NULL,
    deaths INTEGER NOT NULL,
    assists INTEGER NOT NULL,
    PRIMARY KEY(match_id, puuid),
    FOREIGN KEY(match_id, team_id) REFERENCES valorant_match_teams(match_id, team_id) ON DELETE CASCADE
);

CREATE TABLE valorant_match_rounds (
    match_id VARCHAR NOT NULL REFERENCES valorant_matches(match_id) ON DELETE CASCADE,
    round_num INTEGER NOT NULL,
    plant_round_time INTEGER,
    planter_puuid VARCHAR,
    defuse_round_time INTEGER,
    defuser_puuid VARCHAR,
    team_round_winner VARCHAR NOT NULL,
    PRIMARY KEY(match_id, round_num),
    FOREIGN KEY(match_id, planter_puuid) REFERENCES valorant_match_players(match_id, puuid) ON DELETE CASCADE,
    FOREIGN KEY(match_id, defuser_puuid) REFERENCES valorant_match_players(match_id, puuid) ON DELETE CASCADE,
    FOREIGN KEY(match_id, team_round_winner) REFERENCES valorant_match_teams(match_id, team_id) ON DELETE CASCADE
);

CREATE TABLE valorant_match_round_player_loadout (
    match_id VARCHAR NOT NULL REFERENCES valorant_matches(match_id) ON DELETE CASCADE,
    round_num INTEGER NOT NULL,
    puuid VARCHAR NOT NULL,
    loadout_value INTEGER NOT NULL,
    remaining_money INTEGER NOT NULL,
    spent_money INTEGER NOT NULL,
    weapon VARCHAR NOT NULL,
    armor VARCHAR NOT NULL,
    PRIMARY KEY(match_id, round_num, puuid),
    FOREIGN KEY(match_id, round_num) REFERENCES valorant_match_rounds(match_id, round_num) ON DELETE CASCADE,
    FOREIGN KEY(match_id, puuid) REFERENCES valorant_match_players(match_id, puuid) ON DELETE CASCADE
);

CREATE TABLE valorant_match_kill (
    match_id VARCHAR NOT NULL REFERENCES valorant_matches(match_id) ON DELETE CASCADE,
    round_num INTEGER NOT NULL,
    killer_puuid VARCHAR,
    victim_puuid VARCHAR NOT NULL,
    round_time INTEGER NOT NULL,
    damage_type VARCHAR NOT NULL,
    damage_item VARCHAR NOT NULL,
    is_secondary_fire BOOLEAN NOT NULL,
    FOREIGN KEY(match_id, round_num) REFERENCES valorant_match_rounds(match_id, round_num) ON DELETE CASCADE,
    FOREIGN KEY(match_id, killer_puuid) REFERENCES valorant_match_players(match_id, puuid) ON DELETE CASCADE,
    FOREIGN KEY(match_id, victim_puuid) REFERENCES valorant_match_players(match_id, puuid) ON DELETE CASCADE
);

CREATE TABLE valorant_match_damage (
    match_id VARCHAR NOT NULL REFERENCES valorant_matches(match_id) ON DELETE CASCADE,
    round_num INTEGER NOT NULL,
    instigator_puuid VARCHAR NOT NULL,
    receiver_puuid VARCHAR NOT NULL,
    damage INTEGER NOT NULL,
    legshots INTEGER NOT NULL,
    bodyshots INTEGER NOT NULL,
    headshots INTEGER NOT NULL,
    FOREIGN KEY(match_id, round_num) REFERENCES valorant_match_rounds(match_id, round_num) ON DELETE CASCADE,
    FOREIGN KEY(match_id, instigator_puuid) REFERENCES valorant_match_players(match_id, puuid) ON DELETE CASCADE,
    FOREIGN KEY(match_id, receiver_puuid) REFERENCES valorant_match_players(match_id, puuid) ON DELETE CASCADE
);

CREATE TABLE valorant_match_round_player_stats (
    match_id VARCHAR NOT NULL REFERENCES valorant_matches(match_id) ON DELETE CASCADE,
    round_num INTEGER NOT NULL,
    puuid VARCHAR NOT NULL,
    combat_score INTEGER NOT NULL,
    PRIMARY KEY(match_id, round_num, puuid),
    FOREIGN KEY(match_id, round_num) REFERENCES valorant_match_rounds(match_id, round_num) ON DELETE CASCADE,
    FOREIGN KEY(match_id, puuid) REFERENCES valorant_match_players(match_id, puuid) ON DELETE CASCADE
);