ALTER TABLE valorant_match_uuid_link
ADD COLUMN shard VARCHAR NOT NULL,
DROP CONSTRAINT valorant_match_uuid_link_match_id_key CASCADE,
ADD CONSTRAINT valorant_match_uuid_link_match_id_shard_key UNIQUE (match_id, shard);

ALTER TABLE valorant_matches
DROP COLUMN match_id CASCADE,
ADD COLUMN match_uuid UUID NOT NULL UNIQUE REFERENCES valorant_match_uuid_link(match_uuid) ON DELETE CASCADE,
ADD PRIMARY KEY (match_uuid);

ALTER TABLE valorant_player_match_metadata
DROP COLUMN match_id CASCADE,
ADD COLUMN match_uuid UUID NOT NULL REFERENCES valorant_match_uuid_link(match_uuid) ON DELETE CASCADE,
ADD CONSTRAINT valorant_player_match_metadata_match_uuid_puuid_key UNIQUE (match_uuid, puuid);

ALTER TABLE valorant_player_round_metadata
DROP COLUMN match_id CASCADE,
ADD COLUMN match_uuid UUID NOT NULL REFERENCES valorant_match_uuid_link(match_uuid) ON DELETE CASCADE,
ADD CONSTRAINT valorant_player_round_metadata_match_uuid_puuid_round_key UNIQUE(match_uuid, puuid, round),
ADD CONSTRAINT valorant_player_round_metadata_match_uuid_puuid_fkey FOREIGN KEY (match_uuid, puuid) REFERENCES valorant_player_match_metadata(match_uuid, puuid) ON DELETE CASCADE;

ALTER TABLE valorant_match_teams
DROP COLUMN match_id CASCADE,
ADD COLUMN match_uuid UUID NOT NULL REFERENCES valorant_match_uuid_link(match_uuid) ON DELETE CASCADE,
ADD PRIMARY KEY (match_uuid, team_id);

ALTER TABLE valorant_match_players
DROP COLUMN match_id CASCADE,
ADD COLUMN match_uuid UUID NOT NULL REFERENCES valorant_match_uuid_link(match_uuid) ON DELETE CASCADE,
ADD PRIMARY KEY (match_uuid, puuid),
ADD CONSTRAINT valorant_match_players_match_uuid_team_id_fkey FOREIGN KEY (match_uuid, team_id) REFERENCES valorant_match_teams(match_uuid, team_id) ON DELETE CASCADE;

ALTER TABLE valorant_match_rounds
DROP COLUMN match_id CASCADE,
ADD COLUMN match_uuid UUID NOT NULL REFERENCES valorant_match_uuid_link(match_uuid) ON DELETE CASCADE,
ADD PRIMARY KEY (match_uuid, round_num),
ADD CONSTRAINT valorant_match_rounds_match_uuid_planter_puuid_fkey FOREIGN KEY (match_uuid, planter_puuid) REFERENCES valorant_match_players(match_uuid, puuid) ON DELETE CASCADE,
ADD CONSTRAINT valorant_match_rounds_match_uuid_defuser_puuid_fkey FOREIGN KEY (match_uuid, defuser_puuid) REFERENCES valorant_match_players(match_uuid, puuid) ON DELETE CASCADE,
ADD CONSTRAINT valorant_match_rounds_match_uuid_team_round_winner_fkey FOREIGN KEY (match_uuid, team_round_winner) REFERENCES valorant_match_teams(match_uuid, team_id) ON DELETE CASCADE;

ALTER TABLE valorant_match_round_player_stats
DROP COLUMN match_id CASCADE,
ADD COLUMN match_uuid UUID NOT NULL REFERENCES valorant_match_uuid_link(match_uuid) ON DELETE CASCADE,
ADD PRIMARY KEY (match_uuid, round_num, puuid),
ADD CONSTRAINT valorant_match_round_player_stats_match_uuid_puuid_fkey FOREIGN KEY (match_uuid, puuid) REFERENCES valorant_match_players(match_uuid, puuid) ON DELETE CASCADE,
ADD CONSTRAINT valorant_match_round_player_stats_match_uuid_round_num_fkey FOREIGN KEY (match_uuid, round_num) REFERENCES valorant_match_rounds(match_uuid, round_num) ON DELETE CASCADE;

ALTER TABLE valorant_match_round_player_loadout
DROP COLUMN match_id CASCADE,
ADD COLUMN match_uuid UUID NOT NULL REFERENCES valorant_match_uuid_link(match_uuid) ON DELETE CASCADE,
ADD PRIMARY KEY (match_uuid, round_num, puuid),
ADD CONSTRAINT valorant_match_round_player_loadout_match_uuid_puuid_fkey FOREIGN KEY (match_uuid, puuid) REFERENCES valorant_match_players(match_uuid, puuid) ON DELETE CASCADE,
ADD CONSTRAINT valorant_match_round_player_loadout_match_uuid_round_num_fkey FOREIGN KEY (match_uuid, round_num) REFERENCES valorant_match_rounds(match_uuid, round_num) ON DELETE CASCADE;

ALTER TABLE valorant_match_kill
DROP COLUMN match_id CASCADE,
ADD COLUMN match_uuid UUID NOT NULL REFERENCES valorant_match_uuid_link(match_uuid) ON DELETE CASCADE,
ADD CONSTRAINT valorant_match_kill_match_uuid_killer_puuid_fkey FOREIGN KEY (match_uuid, killer_puuid) REFERENCES valorant_match_players(match_uuid, puuid) ON DELETE CASCADE,
ADD CONSTRAINT valorant_match_kill_match_uuid_victim_puuid_fkey FOREIGN KEY (match_uuid, victim_puuid) REFERENCES valorant_match_players(match_uuid, puuid) ON DELETE CASCADE,
ADD CONSTRAINT valorant_match_kill_match_uuid_round_num_fkey FOREIGN KEY (match_uuid, round_num) REFERENCES valorant_match_rounds(match_uuid, round_num) ON DELETE CASCADE,
ADD PRIMARY KEY (match_uuid, round_num, killer_puuid, victim_puuid, time_since_round_start_millis);

ALTER TABLE valorant_match_damage
DROP COLUMN match_id CASCADE,
ADD COLUMN match_uuid UUID NOT NULL REFERENCES valorant_match_uuid_link(match_uuid) ON DELETE CASCADE,
ADD CONSTRAINT valorant_match_damage_match_uuid_instigator_puuid_fkey FOREIGN KEY (match_uuid, instigator_puuid) REFERENCES valorant_match_players(match_uuid, puuid) ON DELETE CASCADE,
ADD CONSTRAINT valorant_match_damage_match_uuid_receiver_puuid_fkey FOREIGN KEY (match_uuid, receiver_puuid) REFERENCES valorant_match_players(match_uuid, puuid) ON DELETE CASCADE,
ADD CONSTRAINT valorant_match_damage_match_uuid_round_num_fkey FOREIGN KEY (match_uuid, round_num) REFERENCES valorant_match_rounds(match_uuid, round_num) ON DELETE CASCADE,
ADD PRIMARY KEY (match_uuid, round_num, instigator_puuid, receiver_puuid, sequence_id);