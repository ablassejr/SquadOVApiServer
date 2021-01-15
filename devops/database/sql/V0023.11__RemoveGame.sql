DELETE FROM riot_accounts a
USING riot_accounts b
WHERE
    a.game < b.game
    AND a.puuid = b.puuid;

ALTER TABLE riot_accounts
DROP COLUMN game CASCADE;

ALTER TABLE riot_accounts
ADD PRIMARY KEY (puuid);

ALTER TABLE riot_account_links
DROP COLUMN game CASCADE;

ALTER TABLE riot_account_links
ADD CONSTRAINT riot_account_links_puuid_fkey FOREIGN KEY (puuid) REFERENCES riot_accounts(puuid) ON DELETE CASCADE,
ADD CONSTRAINT riot_account_links_user_id_puuid_key UNIQUE(user_id, puuid);

ALTER TABLE riot_account_game_shards
ADD CONSTRAINT riot_account_game_shards_puuid_fkey FOREIGN KEY (puuid) REFERENCES riot_accounts(puuid) ON DELETE CASCADE;