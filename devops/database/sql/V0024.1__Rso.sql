ALTER TABLE squadov.riot_account_links
ADD COLUMN rso_access_token VARCHAR,
ADD COLUMN rso_refresh_token VARCHAR,
ADD COLUMN rso_expiration TIMESTAMPTZ;