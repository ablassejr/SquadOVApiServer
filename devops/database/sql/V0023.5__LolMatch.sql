CREATE TABLE lol_match_info (
    match_uuid UUID UNIQUE NOT NULL REFERENCES lol_matches (match_uuid) ON DELETE CASCADE,
    game_id BIGINT NOT NULL,
    platform_id VARCHAR NOT NULL,
    queue_id INTEGER NOT NULL,
    game_type VARCHAR NOT NULL,
    game_duration BIGINT NOT NULL,
    game_creation TIMESTAMPTZ NOT NULL,
    season_id INTEGER NOT NULL,
    game_version VARCHAR NOT NULL,
    map_id INTEGER NOT NULL,
    game_mode VARCHAR NOT NULL,
    UNIQUE(platform_id, game_id)
);

CREATE TABLE lol_match_participant_identities (
    match_uuid UUID NOT NULL REFERENCES lol_match_info (match_uuid) ON DELETE CASCADE,
    participant_id INTEGER NOT NULL,
    account_id VARCHAR,
    current_account_id VARCHAR,
    current_platform_id VARCHAR,
    summoner_name VARCHAR,
    summoner_id VARCHAR,
    platform_id VARCHAR,
    PRIMARY KEY(match_uuid, participant_id)
);

CREATE TABLE lol_match_teams (
    match_uuid UUID NOT NULL REFERENCES lol_match_info (match_uuid) ON DELETE CASCADE,
    team_id INTEGER NOT NULL,
    tower_kills INTEGER NOT NULL,
    rift_herald_kills INTEGER NOT NULL,
    first_blood BOOLEAN NOT NULL,
    inhibitor_kills INTEGER NOT NULL,
    first_baron BOOLEAN NOT NULL,
    first_dragon BOOLEAN NOT NULL,
    dragon_kills INTEGER NOT NULL,
    baron_kills INTEGER NOT NULL,
    first_inhibitor BOOLEAN NOT NULL,
    first_tower BOOLEAN NOT NULL,
    first_rift_herald BOOLEAN NOT NULL,
    win VARCHAR NOT NULL,
    PRIMARY KEY(match_uuid, team_id)
);

CREATE TABLE lol_match_bans (
    match_uuid UUID NOT NULL REFERENCES lol_match_info (match_uuid) ON DELETE CASCADE,
    team_id INTEGER NOT NULL,
    champion_id INTEGER NOT NULL,
    pick_turn INTEGER NOT NULL,
    FOREIGN KEY(match_uuid, team_id) REFERENCES lol_match_teams(match_uuid, team_id) ON DELETE CASCADE,
    UNIQUE(match_uuid, team_id, champion_id)
);

CREATE TABLE lol_match_participants (
    match_uuid UUID NOT NULL REFERENCES lol_match_info (match_uuid) ON DELETE CASCADE,
    participant_id INTEGER NOT NULL,
    champion_id INTEGER NOT NULL,
    team_id INTEGER NOT NULL,
    spell1_id INTEGER NOT NULL,
    spell2_id INTEGER NOT NULL,
    champ_level INTEGER NOT NULL,
    win BOOLEAN NOT NULL,
    kills INTEGER NOT NULL,
    deaths INTEGER NOT NULL,
    assists INTEGER NOT NULL,
    item0 INTEGER NOT NULL,
    item1 INTEGER NOT NULL,
    item2 INTEGER NOT NULL,
    item3 INTEGER NOT NULL,
    item4 INTEGER NOT NULL,
    item5 INTEGER NOT NULL,
    item6 INTEGER NOT NULL,
    double_kills INTEGER NOT NULL,
    triple_kills INTEGER NOT NULL,
    quadra_kills INTEGER NOT NULL,
    penta_kills INTEGER NOT NULL,
    first_blood_kill INTEGER NOT NULL,
    gold_earned INTEGER NOT NULL,
    gold_spent INTEGER NOT NULL,
    neutral_minions_killed_team_jungle INTEGER NOT NULL,
    neutral_minions_killed_enemy_jungle INTEGER NOT NULL,
    wards_killed INTEGER NOT NULL,
    wards_placed INTEGER NOT NULL,
    vision_wards_bought_in_game INTEGER NOT NULL,
    sight_wards_bought_in_game INTEGER NOT NULL,
    neutral_minions_kills INTEGER NOT NULL,
    total_minions_killed INTEGER NOT NULL,
    damage_dealt_to_objectives BIGINT NOT NULL,
    inhibitor_kills INTEGER NOT NULL,
    turret_kills INTEGER NOT NULL,
    damage_dealt_to_turrets BIGINT NOT NULL,
    total_player_score INTEGER NOT NULL,
    total_score_rank INTEGER NOT NULL,
    objective_player_score INTEGER NOT NULL,
    combat_player_score INTEGER NOT NULL,
    vision_score BIGINT NOT NULL,
    total_damage_dealt_to_champions BIGINT NOT NULL,
    physical_damage_dealt_to_champions BIGINT NOT NULL,
    magic_damage_dealt_to_champions BIGINT NOT NULL,
    true_damage_dealt_to_champions BIGINT NOT NULL,
    total_damage_dealt BIGINT NOT NULL,
    physical_damage_dealt BIGINT NOT NULL,
    magic_damage_dealt BIGINT NOT NULL, 
    true_damage_dealt BIGINT NOT NULL,
    total_damage_taken BIGINT NOT NULL, 
    physical_damage_token BIGINT NOT NULL,
    magical_damage_taken BIGINT NOT NULL,
    true_damage_taken BIGINT NOT NULL,
    total_heal BIGINT NOT NULL,
    damage_self_mitigated BIGINT NOT NULL,
    FOREIGN KEY(match_uuid, participant_id) REFERENCES lol_match_participant_identities(match_uuid, participant_id) ON DELETE CASCADE,
    FOREIGN KEY(match_uuid, team_id) REFERENCES lol_match_teams(match_uuid, team_id) ON DELETE CASCADE,
    UNIQUE(match_uuid, participant_id)
);

CREATE TABLE lol_match_timeline (
    match_uuid UUID UNIQUE NOT NULL REFERENCES lol_match_info (match_uuid) ON DELETE CASCADE,
    frame_interval BIGINT NOT NULL
);

CREATE TABLE lol_match_timeline_participant_frames (
    match_uuid UUID UNIQUE NOT NULL REFERENCES lol_match_timeline (match_uuid) ON DELETE CASCADE,
    timestamp BIGINT NOT NULL,
    participant_id INTEGER NOT NULL,
    minions_killed INTEGER NOT NULL,
    total_gold INTEGER NOT NULL,
    level INTEGER NOT NULL,
    xp INTEGER NOT NULL,
    current_gold INTEGER NOT NULL,
    jungle_minions_killed INTEGER NOT NULL,
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    PRIMARY KEY(match_uuid, participant_id, timestamp)
);

CREATE TABLE lol_match_timeline_events (
    match_uuid UUID UNIQUE NOT NULL REFERENCES lol_match_timeline (match_uuid) ON DELETE CASCADE,
    timestamp BIGINT NOT NULL,
    real_type VARCHAR NOT NULL,
    lane_type VARCHAR,
    skill_slot INTEGER,
    ascended_type VARCHAR,
    creator_id INTEGER,
    after_id INTEGER,
    event_type VARCHAR,
    level_up_type VARCHAR,
    ward_type VARCHAR,
    participant_id INTEGER,
    tower_type VARCHAR,
    item_id INTEGER,
    before_id INTEGER,
    monster_type VARCHAR,
    monster_sub_type VARCHAR,
    team_id INTEGER,
    x INTEGER,
    y INTEGER,
    killer_id INTEGER,
    assisting_participant_ids INTEGER[],
    building_type VARCHAR,
    victim_id VARCHAR
);

CREATE INDEX ON lol_match_timeline_events(match_uuid, real_type);