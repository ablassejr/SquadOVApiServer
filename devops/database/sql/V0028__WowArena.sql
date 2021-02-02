CREATE TABLE wow_arenas (
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    tm TIMESTAMPTZ NOT NULL,
    combatants_key VARCHAR NOT NULL,
    instance_id INTEGER NOT NULL,
    arena_type VARCHAR NOT NULL,
    finish_time TIMESTAMPTZ,
    winning_team_id INTEGER,
    match_duration_seconds INTEGER,
    new_ratings INTEGER[]
);

CREATE UNIQUE INDEX wow_arenas_unique_key ON wow_arenas (
    combatants_key,
    instance_id,
    arena_type
)
WHERE finish_time IS NULL;

CREATE UNIQUE INDEX wow_challenges_unique_key ON wow_challenges (
    combatants_key,
    instance_id,
    keystone_level
)
WHERE finish_time IS NULL;

CREATE UNIQUE INDEX wow_encounters_unique_key ON wow_encounters (
    combatants_key,
    encounter_id,
    difficulty,
    instance_id
)
WHERE finish_time IS NULL;