CREATE TABLE share_match_vod_connections (
    id BIGSERIAL PRIMARY KEY,
    match_uuid UUID REFERENCES matches(uuid) ON DELETE SET NULL,
    video_uuid UUID REFERENCES vods(video_uuid) ON DELETE SET NULL,
    source_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    dest_user_id BIGINT REFERENCES users(id) ON DELETE CASCADE,
    dest_squad_id BIGINT REFERENCES squads(id) ON DELETE CASCADE,
    can_share BOOLEAN NOT NULL,
    can_download BOOLEAN NOT NULL,
    can_clip BOOLEAN NOT NULL,
    parent_connection_id BIGINT REFERENCES share_match_vod_connections(id) ON DELETE CASCADE
);

CREATE INDEX ON share_match_vod_connections(dest_user_id);
CREATE INDEX ON share_match_vod_connections(dest_squad_id);
CREATE INDEX ON share_match_vod_connections(match_uuid);
CREATE INDEX ON share_match_vod_connections(video_uuid);