ALTER TABLE valorant_match_kill
DROP CONSTRAINT valorant_match_kill_pkey CASCADE,
ALTER COLUMN killer_puuid DROP NOT NULL,
ADD CONSTRAINT valorant_match_kill_unique_key UNIQUE (match_uuid, round_num, killer_puuid, victim_puuid, time_since_round_start_millis);