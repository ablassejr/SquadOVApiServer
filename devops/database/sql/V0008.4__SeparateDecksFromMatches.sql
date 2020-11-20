CREATE TABLE hearthstone_decks(
    deck_id BIGINT PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    hero_card VARCHAR NOT NULL,
    hero_premium INTEGER NOT NULL,
    deck_type INTEGER NOT NULL,
    create_date TIMESTAMPTZ NOT NULL,
    is_wild BOOLEAN NOT NULL
);
CREATE INDEX ON hearthstone_decks(user_id, deck_id);

CREATE TABLE hearthstone_deck_slots (
    deck_id BIGINT NOT NULL REFERENCES hearthstone_decks(deck_id) ON DELETE CASCADE,
    index INTEGER NOT NULL,
    card_id VARCHAR NOT NULL,
    owned BOOLEAN NOT NULL,
    normal_count INTEGER NOT NULL,
    golden_count INTEGER NOT NULL,
    UNIQUE(deck_id, card_id)
);

INSERT INTO hearthstone_decks (
    deck_id,
    user_id,
    hero_card,
    hero_premium,
    deck_type,
    create_date,
    is_wild
)
SELECT DISTINCT
    deck_id,
    user_id,
    hero_card,
    hero_premium,
    deck_type,
    create_date,
    is_wild
FROM hearthstone_player_match_decks;

INSERT INTO hearthstone_deck_slots (
    deck_id,
    index,
    card_id,
    owned,
    normal_count,
    golden_count
)
SELECT DISTINCT
    deck_id,
    index,
    card_id,
    owned,
    normal_count,
    golden_count
FROM hearthstone_player_match_deck_slots;