CREATE TABLE hearthstone_match_user_deck (
    deck_id BIGINT NOT NULL REFERENCES hearthstone_decks(deck_id) ON DELETE CASCADE,
    user_id BIGINT REFERENCES users(id) ON DELETE CASCADE,
    match_uuid UUID NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    UNIQUE(match_uuid, user_id)
);

CREATE INDEX ON hearthstone_match_user_deck(deck_id);
CREATE INDEX ON hearthstone_match_user_deck(user_id);