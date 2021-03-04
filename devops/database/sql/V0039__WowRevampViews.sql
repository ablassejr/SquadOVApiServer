CREATE TABLE wow_match_view (
    id UUID PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    start_tm TIMESTAMPTZ NOT NULL,
    end_tm TIMESTAMPTZ,
    match_uuid UUID REFERENCES matches(uuid) ON DELETE CASCADE
);

CREATE UNIQUE INDEX ON wow_match_view (match_uuid, user_id);

CREATE TABLE wow_arena_view (
    view_id UUID UNIQUE NOT NULL REFERENCES wow_match_view(id) ON DELETE CASCADE,
    instance_id INTEGER NOT NULL,
    arena_type VARCHAR NOT NULL,
    winning_team_id INTEGER,
    match_duration_seconds INTEGER,
    new_ratings INTEGER[]
);

CREATE TABLE wow_encounter_view (
    view_id UUID UNIQUE NOT NULL REFERENCES wow_match_view(id) ON DELETE CASCADE,
    encounter_id INTEGER NOT NULL,
    encounter_name VARCHAR NOT NULL,
    difficulty INTEGER NOT NULL,
    num_players INTEGER NOT NULL,
    instance_id INTEGER NOT NULL,
    success BOOLEAN
);

CREATE TABLE wow_challenge_view (
    view_id UUID UNIQUE NOT NULL REFERENCES wow_match_view(id) ON DELETE CASCADE,
    challenge_name VARCHAR NOT NULL,
    instance_id INTEGER NOT NULL,
    keystone_level INTEGER NOT NULL,
    time_ms BIGINT,
    success BOOLEAN
);