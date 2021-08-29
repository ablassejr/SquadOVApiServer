DROP TABLE linked_twitch_accounts CASCADE;

CREATE TABLE twitch_accounts (
    twitch_user_id VARCHAR PRIMARY KEY,
    twitch_name VARCHAR NOT NULL,
    access_token VARCHAR NOT NULL,
    refresh_token VARCHAR NOT NULL
);

CREATE TABLE linked_twitch_accounts (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    twitch_user_id VARCHAR NOT NULL REFERENCES twitch_accounts(twitch_user_id) ON DELETE CASCADE,
    linked_tm TIMESTAMPTZ NOT NULL,
    UNIQUE(user_id, twitch_user_id)
);

/* DO NOT REFERENCE THE TWITCH ACCOUNTS TABLE AS THIS TABLE WILL CONTAIN ACCOUNTS THAT HAVE NOT YET BEEN LINKED */
CREATE TABLE cached_twitch_subs (
    broadcast_user_id VARCHAR NOT NULL,
    viewer_user_id VARCHAR NOT NULL,
    tier VARCHAR NOT NULL,
    UNIQUE(broadcast_user_id, viewer_user_id)
);