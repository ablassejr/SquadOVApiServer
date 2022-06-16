ALTER TABLE squads
ADD COLUMN max_members BIGINT DEFAULT 20;

UPDATE squads
SET max_members = NULL
WHERE is_public AND is_discoverable;