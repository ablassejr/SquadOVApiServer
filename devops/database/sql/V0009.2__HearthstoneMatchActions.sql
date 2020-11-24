CREATE TABLE hearthstone_match_action_blobs (
    match_uuid UUID UNIQUE NOT NULL REFERENCES hearthstone_matches (match_uuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    actions_blob_uuid UUID NOT NULL REFERENCES blob_link_storage(uuid) ON DELETE CASCADE
);

ALTER TABLE hearthstone_matches
DROP COLUMN actions_blob_uuid;