CREATE TABLE user_downloads (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tm TIMESTAMPTZ NOT NULL
);

CREATE INDEX ON user_downloads(user_id);