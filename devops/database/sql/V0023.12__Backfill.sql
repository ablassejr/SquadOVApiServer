ALTER TABLE riot_accounts
RENAME COLUMN last_backfill_time TO last_backfill_tft_time;

ALTER TABLE riot_accounts
ADD COLUMN last_backfill_lol_time TIMESTAMPTZ;