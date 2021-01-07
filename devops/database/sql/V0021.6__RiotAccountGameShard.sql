CREATE TABLE riot_account_game_shards (
    puuid VARCHAR NOT NULL REFERENCES riot_accounts(puuid) ON DELETE CASCADE,
    game VARCHAR NOT NULL,
    shard VARCHAR NOT NULL,
    UNIQUE(puuid, game)
);