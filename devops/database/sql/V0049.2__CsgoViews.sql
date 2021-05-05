CREATE TABLE csgo_match_views (
    view_uuid UUID PRIMARY KEY,
    match_uuid UUID REFERENCES matches (uuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    has_gsi BOOLEAN NOT NULL DEFAULT FALSE,
    has_demo BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(match_uuid, user_id)
);