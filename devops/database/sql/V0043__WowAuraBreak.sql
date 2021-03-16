CREATE TABLE wow_match_view_aura_break_events (
    event_id BIGINT UNIQUE NOT NULL REFERENCES wow_match_view_events(event_id) ON DELETE CASCADE,
    aura_spell_id BIGINT NOT NULL,
    aura_type VARCHAR NOT NULL,
    removed_by_spell_id BIGINT
);