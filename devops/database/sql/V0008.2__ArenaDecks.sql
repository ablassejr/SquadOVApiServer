CREATE TABLE hearthstone_arena_deck_slots (
    deck_id BIGINT NOT NULL REFERENCES hearthstone_arena_drafts(draft_deck_id) ON DELETE CASCADE,
    card_id VARCHAR NOT NULL,
    selection_time TIMESTAMPTZ NOT NULL
);