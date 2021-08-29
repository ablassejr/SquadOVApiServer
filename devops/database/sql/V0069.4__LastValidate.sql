ALTER TABLE twitch_accounts
ADD COLUMN last_validate TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE INDEX ON twitch_accounts(last_validate);