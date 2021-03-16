CREATE TABLE wow_match_view_spell_cast_events (
    event_id BIGINT UNIQUE NOT NULL REFERENCES wow_match_view_events(event_id) ON DELETE CASCADE,
    spell_id BIGINT NOT NULL,
    is_start BOOLEAN NOT NULL,
    is_finish BOOLEAN NOT NULL,
    success BOOLEAN NOT NULL
);