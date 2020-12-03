CREATE TABLE valorant_match_kill_new (
    match_id VARCHAR NOT NULL REFERENCES valorant_matches(match_id) ON DELETE CASCADE,
    round_num INTEGER NOT NULL,
    killer_puuid VARCHAR,
    victim_puuid VARCHAR NOT NULL,
    round_time INTEGER NOT NULL,
    damage_type VARCHAR NOT NULL,
    damage_item VARCHAR NOT NULL,
    is_secondary_fire BOOLEAN NOT NULL,
    FOREIGN KEY(match_id, round_num) REFERENCES valorant_match_rounds(match_id, round_num) ON DELETE CASCADE,
    FOREIGN KEY(match_id, killer_puuid) REFERENCES valorant_match_players(match_id, puuid) ON DELETE CASCADE,
    FOREIGN KEY(match_id, victim_puuid) REFERENCES valorant_match_players(match_id, puuid) ON DELETE CASCADE,
    UNIQUE(match_id, round_num, killer_puuid, victim_puuid, round_time)
);

INSERT INTO valorant_match_kill_new (
    match_id,
    round_num,
    killer_puuid,
    victim_puuid,
    round_time,
    damage_type,
    damage_item,
    is_secondary_fire
)
SELECT DISTINCT vmk.*
FROM valorant_match_kill AS vmk;

DROP TABLE valorant_match_kill;
ALTER TABLE valorant_match_kill_new
RENAME TO valorant_match_kill;