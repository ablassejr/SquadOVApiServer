CREATE TABLE daily_active_sessions (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tm DATE NOT NULL,
    UNIQUE(tm, user_id)
);

CREATE TABLE daily_active_endpoint (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tm DATE NOT NULL,
    UNIQUE(tm, user_id)
);