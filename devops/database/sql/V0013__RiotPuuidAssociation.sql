CREATE TABLE riot_account_links (
    puuid VARCHAR NOT NULL,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(user_id, puuid)
);
CREATE INDEX ON riot_account_links(puuid);