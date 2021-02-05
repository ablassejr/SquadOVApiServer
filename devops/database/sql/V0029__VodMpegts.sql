ALTER TABLE vods
ADD COLUMN raw_container_format VARCHAR;

UPDATE vods
SET raw_container_format = 'mp4';

ALTER TABLE vods
ALTER COLUMN raw_container_format SET NOT NULL;