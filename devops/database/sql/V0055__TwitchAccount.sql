CREATE TABLE linked_twitch_accounts (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    twitch_user_id BIGINT NOT NULL,
    twitch_name VARCHAR NOT NULL,
    access_token VARCHAR NOT NULL,
    refresh_token VARCHAR NOT NULL,
    UNIQUE(user_id, twitch_user_id)
);