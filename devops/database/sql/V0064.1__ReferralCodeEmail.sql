ALTER TABLE user_referral_code_usage
DROP COLUMN user_id CASCADE;

ALTER TABLE user_referral_code_usage
ADD COLUMN email VARCHAR UNIQUE NOT NULL;