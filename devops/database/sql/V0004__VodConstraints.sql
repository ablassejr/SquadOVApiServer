ALTER TABLE vods
ADD CONSTRAINT video_uuid_unique UNIQUE (video_uuid);

ALTER TABLE vods
ADD CONSTRAINT match_user_uuid_unique UNIQUE (match_uuid, user_uuid);

CREATE INDEX on vods (user_uuid);