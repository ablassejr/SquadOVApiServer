CREATE INDEX CONCURRENTLY ON new_wow_arenas USING GIST(match_uuid, tr, combatants_key, instance_id, arena_type);
CREATE INDEX CONCURRENTLY ON new_wow_challenges USING GIST(match_uuid, tr, combatants_key, instance_id, keystone_level);
CREATE INDEX CONCURRENTLY ON new_wow_encounters USING GIST(match_uuid, tr, combatants_key, instance_id, encounter_id, difficulty);
CREATE INDEX CONCURRENTLY ON new_wow_instances USING GIST(match_uuid, tr, players, instance_id, instance_type);