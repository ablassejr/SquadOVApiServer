CREATE TABLE hearthstone_deck_versions (
    version_id BIGSERIAL PRIMARY KEY,
    deck_id BIGINT NOT NULL REFERENCES hearthstone_decks(deck_id) ON DELETE CASCADE
);

CREATE INDEX ON hearthstone_deck_versions(deck_id);

ALTER TABLE hearthstone_deck_slots
DROP COLUMN deck_id CASCADE;

ALTER TABLE hearthstone_deck_slots
ADD COLUMN deck_version_id BIGINT NOT NULL REFERENCES hearthstone_deck_versions(version_id) ON DELETE CASCADE;

ALTER TABLE hearthstone_deck_slots
ADD CONSTRAINT hearthstone_deck_slots_deck_version_id_card_id_unique UNIQUE(deck_version_id, card_id);

ALTER TABLE hearthstone_match_user_deck
DROP COLUMN deck_id CASCADE;

ALTER TABLE hearthstone_match_user_deck
ADD COLUMN deck_version_id BIGINT NOT NULL REFERENCES hearthstone_deck_versions(version_id) ON DELETE CASCADE;

CREATE INDEX ON hearthstone_match_user_deck(deck_version_id);