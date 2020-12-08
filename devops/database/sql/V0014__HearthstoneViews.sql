DELETE FROM matches AS m
USING hearthstone_matches AS hm
WHERE hm.match_uuid = m.uuid;

CREATE TABLE hearthstone_match_view (
    view_uuid UUID PRIMARY KEY,
    match_uuid UUID NOT NULL REFERENCES hearthstone_matches (match_uuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(match_uuid, user_id)
);

ALTER TABLE hearthstone_match_action_blobs
ADD CONSTRAINT hearthstone_match_action_blobs_match_uuid_user_id_fkey FOREIGN KEY (match_uuid, user_id) REFERENCES hearthstone_match_view(match_uuid, user_id) ON DELETE CASCADE;

ALTER TABLE hearthstone_match_metadata
DROP COLUMN match_uuid,
ADD COLUMN view_uuid UUID NOT NULL REFERENCES hearthstone_match_view(view_uuid) ON DELETE CASCADE;

ALTER TABLE hearthstone_match_player_medals
DROP COLUMN match_uuid,
ADD COLUMN view_uuid UUID NOT NULL REFERENCES hearthstone_match_view(view_uuid) ON DELETE CASCADE;

ALTER TABLE hearthstone_match_players
DROP COLUMN match_uuid CASCADE,
ADD COLUMN view_uuid UUID NOT NULL REFERENCES hearthstone_match_view(view_uuid) ON DELETE CASCADE;

ALTER TABLE hearthstone_match_user_deck
ADD CONSTRAINT hearthstone_match_user_deck_match_uuid_user_id_fkey FOREIGN KEY (match_uuid, user_id) REFERENCES hearthstone_match_view(match_uuid, user_id) ON DELETE CASCADE;

ALTER TABLE hearthstone_raw_power_logs
ADD CONSTRAINT hearthstone_raw_power_logs_match_uuid_user_id_fkey FOREIGN KEY (match_uuid, user_id) REFERENCES hearthstone_match_view(match_uuid, user_id) ON DELETE CASCADE;

ALTER TABLE hearthstone_snapshots
ADD CONSTRAINT hearthstone_snapshots_match_uuid_user_id_fkey FOREIGN KEY (match_uuid, user_id) REFERENCES hearthstone_match_view(match_uuid, user_id) ON DELETE CASCADE;