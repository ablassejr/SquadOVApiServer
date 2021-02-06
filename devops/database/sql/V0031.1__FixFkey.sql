ALTER TABLE share_tokens
DROP CONSTRAINT share_tokens_match_uuid_fkey,
ADD CONSTRAINT share_tokens_match_uuid_fkey FOREIGN KEY (match_uuid) REFERENCES matches (uuid) ON DELETE CASCADE;