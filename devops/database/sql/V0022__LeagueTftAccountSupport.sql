ALTER TABLE riot_accounts
ADD COLUMN game VARCHAR;

UPDATE riot_accounts
SET game = 'val';

ALTER TABLE riot_accounts
ALTER COLUMN game SET NOT NULL;

ALTER TABLE riot_accounts
ALTER COLUMN game_name SET DEFAULT '',
ALTER COLUMN tag_line SET DEFAULT '',
ADD COLUMN account_id VARCHAR NOT NULL DEFAULT '',
ADD COLUMN summoner_id VARCHAR NOT NULL DEFAULT '',
ADD COLUMN summoner_name VARCHAR NOT NULL DEFAULT '';

ALTER TABLE riot_accounts
DROP CONSTRAINT riot_accounts_pkey CASCADE;

ALTER TABLE riot_accounts
ADD PRIMARY KEY (puuid, game);