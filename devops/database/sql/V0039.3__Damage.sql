CREATE TABLE wow_match_view_damage_events (
    event_id BIGINT UNIQUE NOT NULL REFERENCES wow_match_view_events(event_id) ON DELETE CASCADE,
    spell_id BIGINT,
    amount INTEGER NOT NULL,
    overkill INTEGER NOT NULL
);