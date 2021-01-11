CREATE TABLE riot_accounts (
    puuid VARCHAR PRIMARY KEY,
    game_name VARCHAR NOT NULL,
    tag_line VARCHAR NOT NULL
);

DROP TABLE riot_account_links;
CREATE TABLE riot_account_links (
    puuid VARCHAR NOT NULL REFERENCES riot_accounts(puuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(user_id, puuid)
);