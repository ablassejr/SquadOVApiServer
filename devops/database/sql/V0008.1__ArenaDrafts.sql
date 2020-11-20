CREATE TABLE hearthstone_arena_drafts (
    collection_uuid UUID UNIQUE NOT NULL REFERENCES match_collections (uuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    draft_deck_id BIGINT UNIQUE NOT NULL,
    creation_time TIMESTAMPTZ NOT NULL
);

CREATE INDEX ON hearthstone_arena_drafts(user_id);