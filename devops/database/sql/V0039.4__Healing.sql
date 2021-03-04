CREATE TABLE wow_match_view_healing_events (
    event_id BIGINT UNIQUE NOT NULL REFERENCES wow_match_view_events(event_id) ON DELETE CASCADE,
    spell_id BIGINT NOT NULL,
    amount INTEGER NOT NULL,
    overheal INTEGER NOT NULL,
    absorbed INTEGER NOT NULL
);