CREATE TABLE hearthstone_matches (
    id BIGINT NOT NULL,
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    server_ip INET NOT NULL,
    port INTEGER NOT NULL,
    game_id INTEGER NOT NULL,
    match_day DATE NOT NULL,
    UNIQUE(match_day, server_ip, port, game_id)
);

CREATE TABLE hearthstone_player_match_decks(
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    match_uuid UUID NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    deck_id BIGINT NOT NULL,
    hero_card VARCHAR NOT NULL,
    hero_premium INTEGER NOT NULL,
    deck_type INTEGER NOT NULL,
    create_date TIMESTAMPTZ NOT NULL,
    is_wild BOOLEAN NOT NULL,
    FOREIGN KEY (match_uuid) REFERENCES hearthstone_matches(match_uuid) ON DELETE CASCADE,
    UNIQUE(user_id, match_uuid),
    UNIQUE(deck_id, match_uuid)
);
CREATE INDEX ON hearthstone_player_match_decks(user_id, deck_id);

CREATE TABLE hearthstone_player_match_deck_slots (
    match_uuid UUID NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    deck_id BIGINT NOT NULL,
    index INTEGER NOT NULL,
    card_id VARCHAR NOT NULL,
    owned BOOLEAN NOT NULL,
    normal_count INTEGER NOT NULL,
    golden_count INTEGER NOT NULL,
    FOREIGN KEY(deck_id, match_uuid) REFERENCES hearthstone_player_match_decks(deck_id, match_uuid) ON DELETE CASCADE,
    UNIQUE(deck_id, card_id, match_uuid)
);

CREATE TABLE hearthstone_match_players (
    user_id BIGINT REFERENCES users(id) ON DELETE CASCADE,
    match_uuid UUID NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    player_match_id INTEGER NOT NULL,
    player_name VARCHAR NOT NULL,
    is_local BOOLEAN NOT NULL,
    side INTEGER NOT NULL,
    card_back_id INTEGER NOT NULL,
    arena_wins INTEGER NOT NULL,
    arena_loss INTEGER NOT NULL,
    tavern_brawl_wins INTEGER NOT NULL,
    tavern_brawl_loss INTEGER NOT NULL,
    FOREIGN KEY (match_uuid) REFERENCES hearthstone_matches(match_uuid) ON DELETE CASCADE,
    UNIQUE(match_uuid, player_match_id),
    UNIQUE(match_uuid, user_id)
);

CREATE TABLE hearthstone_match_player_medals (
    match_uuid UUID NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    player_match_id INTEGER NOT NULL,
    league_id INTEGER NOT NULL,
    earned_stars INTEGER NOT NULL,
    star_level INTEGER NOT NULL,
    best_star_level INTEGER NOT NULL,
    win_streak INTEGER NOT NULL,
    legend_index INTEGER NOT NULL,
    is_standard BOOLEAN NOT NULL,
    FOREIGN KEY (match_uuid, player_match_id) REFERENCES hearthstone_match_players(match_uuid, player_match_id),
    UNIQUE(match_uuid, player_match_id, is_standard)
);