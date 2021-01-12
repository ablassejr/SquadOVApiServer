ALTER TABLE riot_account_links
ADD COLUMN game VARCHAR;

UPDATE riot_account_links
SET game = 'val';

ALTER TABLE riot_account_links
ALTER COLUMN game SET NOT NULL;

ALTER TABLE riot_account_links
ADD CONSTRAINT riot_account_links_puuid_game_fkey FOREIGN KEY (puuid, game) REFERENCES riot_accounts(puuid, game) ON DELETE CASCADE,
DROP CONSTRAINT riot_account_links_user_id_puuid_key,
ADD CONSTRAINT riot_account_links_user_id_puuid_game_key UNIQUE(user_id, puuid, game);

ALTER TABLE riot_account_game_shards
ADD CONSTRAINT riot_account_game_shards_puuid_game_fkey FOREIGN KEY (puuid, game) REFERENCES riot_accounts(puuid, game) ON DELETE CASCADE;