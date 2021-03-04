CREATE TABLE wow_match_view_resurrect_events (
    event_id BIGINT UNIQUE NOT NULL REFERENCES wow_match_view_events(event_id) ON DELETE CASCADE,
    spell_id BIGINT NOT NULL
);