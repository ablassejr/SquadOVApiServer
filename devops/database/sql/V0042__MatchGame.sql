ALTER TABLE matches
ADD COLUMN game INTEGER;

UPDATE matches
SET game = 0
WHERE uuid IN (
    SELECT match_uuid
    FROM aimlab_tasks
);

UPDATE matches
SET game = 1
WHERE uuid IN (
    SELECT match_uuid
    FROM hearthstone_matches
);

UPDATE matches
SET game = 2
WHERE uuid IN (
    SELECT match_uuid
    FROM lol_matches
);

UPDATE matches
SET game = 3
WHERE uuid IN (
    SELECT match_uuid
    FROM tft_matches
);

UPDATE matches
SET game = 4
WHERE uuid IN (
    SELECT match_uuid
    FROM valorant_matches
);

UPDATE matches
SET game = 5
WHERE uuid IN (
    SELECT match_uuid
    FROM new_wow_arenas
    UNION
    SELECT match_uuid
    FROM new_wow_challenges
    UNION
    SELECT match_uuid
    FROM new_wow_encounters
);