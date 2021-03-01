ALTER TABLE referral_codes
ADD COLUMN tm TIMESTAMPTZ NOT NULL;

CREATE INDEX ON referral_downloads(code);
CREATE INDEX ON referral_visits(code);