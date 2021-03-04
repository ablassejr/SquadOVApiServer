CREATE TABLE wow_match_view_death_events (
    event_id BIGINT UNIQUE NOT NULL REFERENCES wow_match_view_events(event_id) ON DELETE CASCADE
);