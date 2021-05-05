CREATE TABLE csgo_event_container (
    id BIGSERIAL PRIMARY KEY,
    view_uuid UUID NOT NULL REFERENCES csgo_match_views(view_uuid) ON DELETE CASCADE
);

CREATE TABLE csgo_event_container_players (
    container_id BIGINT NOT NULL REFERENCES csgo_event_container(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL,
    steam_id BIGINT NOT NULL REFERENCES steam_users_cache(steam_id) ON DELETE CASCADE,
    kills INTEGER NOT NULL,
    deaths INTEGER NOT NULL,
    assists INTEGER NOT NULL,
    mvps INTEGER NOT NULL,
    PRIMARY KEY(container_id, user_id)
);

CREATE TABLE csgo_event_container_rounds (
    container_id BIGINT NOT NULL REFERENCES csgo_event_container(id) ON DELETE CASCADE,
    round_num INTEGER NOT NULL,
    tm_round_start TIMESTAMPTZ,
    tm_round_play TIMESTAMPTZ,
    tm_round_end TIMESTAMPTZ,
    bomb_state INTEGER,
    tm_bomb_plant TIMESTAMPTZ,
    bomb_plant_user INTEGER,
    bomb_plant_site INTEGER,
    tm_bomb_event TIMESTAMPTZ,
    bomb_event_user INTEGER,
    winning_team INTEGER,
    round_win_reason INTEGER,
    round_mvp INTEGER,
    PRIMARY KEY(container_id, round_num),
    FOREIGN KEY(container_id, bomb_plant_user) REFERENCES csgo_event_container_players(container_id, user_id) ON DELETE CASCADE,
    FOREIGN KEY(container_id, bomb_event_user) REFERENCES csgo_event_container_players(container_id, user_id) ON DELETE CASCADE,
    FOREIGN KEY(container_id, round_mvp) REFERENCES csgo_event_container_players(container_id, user_id) ON DELETE CASCADE
);

CREATE TABLE csgo_event_container_round_player_stats (
    container_id BIGINT NOT NULL REFERENCES csgo_event_container(id) ON DELETE CASCADE,
    round_num INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    kills INTEGER NOT NULL,
    deaths INTEGER NOT NULL,
    assists INTEGER NOT NULL,
    mvp BOOLEAN NOT NULL,
    equipment_value INTEGER,
    headshot_kills INTEGER,
    utility_damage INTEGER,
    enemies_flashed INTEGER,
    damage INTEGER,
    armor INTEGER,
    has_defuse BOOLEAN,
    has_helmet BOOLEAN,
    team INTEGER NOT NULL,
    weapons INTEGER[] NOT NULL,
    PRIMARY KEY(container_id, round_num, user_id),
    FOREIGN KEY(container_id, user_id) REFERENCES csgo_event_container_players(container_id, user_id) ON DELETE CASCADE,
    FOREIGN KEY(container_id, round_num) REFERENCES csgo_event_container_rounds(container_id, round_num) ON DELETE CASCADE
);

CREATE TABLE csgo_event_container_round_kills (
    container_id BIGINT NOT NULL REFERENCES csgo_event_container(id) ON DELETE CASCADE,
    round_num INTEGER NOT NULL,
    tm TIMESTAMPTZ NOT NULL,
    victim INTEGER NOT NULL,
    killer INTEGER,
    assister INTEGER,
    flash_assist BOOLEAN,
    headshot BOOLEAN,
    smoke BOOLEAN,
    blind BOOLEAN,
    wallbang BOOLEAN,
    noscope BOOLEAN,
    weapon INTEGER,
    FOREIGN KEY(container_id, victim) REFERENCES csgo_event_container_players(container_id, user_id) ON DELETE CASCADE,
    FOREIGN KEY(container_id, killer) REFERENCES csgo_event_container_players(container_id, user_id) ON DELETE CASCADE,
    FOREIGN KEY(container_id, assister) REFERENCES csgo_event_container_players(container_id, user_id) ON DELETE CASCADE
);

CREATE TABLE csgo_event_container_round_damage (
    container_id BIGINT NOT NULL REFERENCES csgo_event_container(id) ON DELETE CASCADE,
    round_num INTEGER NOT NULL,
    receiver INTEGER NOT NULL,
    attacker INTEGER,
    remaining_health INTEGER NOT NULL,
    remaining_armor INTEGER NOT NULL,
    damage_health INTEGER NOT NULL,
    damage_armor INTEGER NOT NULL,
    weapon INTEGER NOT NULL,
    hitgroup INTEGER NOT NULL,
    FOREIGN KEY(container_id, receiver) REFERENCES csgo_event_container_players(container_id, user_id) ON DELETE CASCADE,
    FOREIGN KEY(container_id, attacker) REFERENCES csgo_event_container_players(container_id, user_id) ON DELETE CASCADE
);