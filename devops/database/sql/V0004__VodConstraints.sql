ALTER TABLE vods
ADD CONSTRAINT video_uuid UNIQUE (video_uuid);

ALTER TABLE vods
ADD CONSTRAINT video_uuid UNIQUE (match_uuid, user_uuid);

CREATE INDEX on vods (user_uuid);