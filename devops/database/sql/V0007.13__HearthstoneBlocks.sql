CREATE TABLE hearthstone_blocks (
    match_uuid UUID NOT NULL REFERENCES hearthstone_matches (match_uuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    block_id UUID PRIMARY KEY,
    start_action_index INTEGER NOT NULL,
    end_action_index INTEGER NOT NULL,
    block_type INTEGER NOT NULL,
    parent_block UUID REFERENCES hearthstone_blocks(block_id) ON DELETE CASCADE
);

CREATE INDEX ON hearthstone_blocks(match_uuid, user_id);

ALTER TABLE hearthstone_actions
ADD COLUMN parent_block UUID REFERENCES hearthstone_blocks(block_id) ON DELETE CASCADE;