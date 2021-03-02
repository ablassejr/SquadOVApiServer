ALTER TABLE wow_realms
DROP CONSTRAINT wow_realms_name_key,
DROP CONSTRAINT wow_realms_slug_key;

ALTER TABLE wow_realms
ADD CONSTRAINT wow_realms_region_name_key UNIQUE (region, name),
ADD CONSTRAINT wow_realms_region_slug_key UNIQUE (region, slug);

CREATE INDEX ON wow_realms(name);
CREATE INDEX ON wow_realms(slug);