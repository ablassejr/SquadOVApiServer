CREATE TABLE wow_encounters (
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    tm TIMESTAMPTZ NOT NULL,
    match_day DATE NOT NULL,
    combatants_key VARCHAR NOT NULL,
    encounter_id INTEGER NOT NULL,
    encounter_name VARCHAR NOT NULL,
    difficulty INTEGER NOT NULL,
    num_players INTEGER NOT NULL,
    instance_id INTEGER NOT NULL,
    finish_time TIMESTAMPTZ
);

CREATE UNIQUE INDEX wow_encounters_unique_key ON wow_encounters (
    match_day,
    combatants_key,
    encounter_id,
    difficulty,
    instance_id
)
WHERE finish_time IS NULL;

CREATE TABLE wow_challenges (
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    tm TIMESTAMPTZ NOT NULL,
    match_day DATE NOT NULL,
    combatants_key VARCHAR NOT NULL,
    challenge_name VARCHAR NOT NULL,
    instance_id INTEGER NOT NULL,
    keystone_level INTEGER NOT NULL,
    finish_time TIMESTAMPTZ
);

CREATE UNIQUE INDEX wow_challenges_unique_key ON wow_challenges (
    match_day,
    combatants_key,
    instance_id,
    keystone_level
)
WHERE finish_time IS NULL;