CREATE TABLE user_event_record (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_name VARCHAR NOT NULL,
    tm TIMESTAMPTZ NOT NULL,
    UNIQUE(user_id, event_name)
);