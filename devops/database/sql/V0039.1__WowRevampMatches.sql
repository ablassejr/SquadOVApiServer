CREATE EXTENSION IF NOT EXISTS btree_gist;
-- Information in this table is primarily used for unique constraints
-- on the match and not for querying from the user!!!
CREATE TABLE new_wow_arenas (
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    tr TSTZRANGE NOT NULL,
    combatants_key VARCHAR NOT NULL,
    instance_id INTEGER NOT NULL,
    arena_type VARCHAR NOT NULL,
    EXCLUDE USING GIST (match_uuid WITH <>, combatants_key WITH =, instance_id WITH =, arena_type WITH =, tr WITH &&)
);

CREATE TABLE new_wow_encounters (
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    tr TSTZRANGE NOT NULL,
    combatants_key VARCHAR NOT NULL,
    encounter_id INTEGER NOT NULL,
    difficulty INTEGER NOT NULL,
    instance_id INTEGER NOT NULL,
    EXCLUDE USING GIST (match_uuid WITH <>,combatants_key WITH =, encounter_id WITH =, difficulty WITH =, instance_id WITH =, tr WITH &&)
);

CREATE TABLE new_wow_challenges (
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    tr TSTZRANGE NOT NULL,
    combatants_key VARCHAR NOT NULL,
    instance_id INTEGER NOT NULL,
    keystone_level INTEGER NOT NULL,
    EXCLUDE USING GIST (match_uuid WITH <>, combatants_key WITH =, instance_id WITH =, keystone_level WITH =, tr WITH &&)
);