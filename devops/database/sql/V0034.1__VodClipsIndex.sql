ALTER TABLE vods
DROP CONSTRAINT match_user_uuid_unique,
ADD COLUMN is_clip BOOLEAN NOT NULL DEFAULT FALSE;

CREATE UNIQUE INDEX ON vods (match_uuid, user_uuid) WHERE is_clip = FALSE;