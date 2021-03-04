CREATE TABLE wow_match_view_subencounter_events (
    event_id BIGINT UNIQUE NOT NULL REFERENCES wow_match_view_events(event_id) ON DELETE CASCADE,
    encounter_id INTEGER NOT NULL,
    encounter_name VARCHAR NOT NULL,
    is_start BOOLEAN NOT NULL
);