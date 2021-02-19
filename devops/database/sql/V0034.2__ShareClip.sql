ALTER TABLE share_tokens
ALTER COLUMN match_uuid DROP NOT NULL,
ADD COLUMN clip_uuid UUID,
ADD CONSTRAINT share_tokens_clip_uuid_user_id_key UNIQUE(clip_uuid, user_id);