CREATE TABLE match_custom_events (
    event_id BIGINT PRIMARY KEY,
    match_uuid UUID NOT NULL REFERENCES matches(uuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tm TIMESTAMPTZ NOT NULL,
    label VARCHAR NOT NULL,
    icon VARCHAR NOT NULL
);

CREATE INDEX ON match_custom_events(match_uuid, user_id);