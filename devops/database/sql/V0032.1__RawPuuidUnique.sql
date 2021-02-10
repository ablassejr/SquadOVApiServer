ALTER TABLE riot_accounts
ADD CONSTRAINT riot_accounts_raw_puuid_key UNIQUE(raw_puuid);