CREATE TABLE IF NOT EXISTS squadov.valorant_match_damage_tmp (
    match_id VARCHAR NOT NULL REFERENCES squadov.valorant_matches(match_id) ON DELETE CASCADE,
    round_num INTEGER NOT NULL,
    instigator_puuid VARCHAR NOT NULL,
    receiver_puuid VARCHAR NOT NULL,
    damage INTEGER NOT NULL,
    legshots INTEGER NOT NULL,
    bodyshots INTEGER NOT NULL,
    headshots INTEGER NOT NULL
);

INSERT INTO squadov.valorant_match_damage_tmp (
    match_id,
    round_num,
    instigator_puuid,
    receiver_puuid,
    damage,
    legshots,
    bodyshots,
    headshots
)
SELECT DISTINCT
    match_id,
    round_num,
    instigator_puuid,
    receiver_puuid,
    damage,
    legshots,
    bodyshots,
    headshots
FROM squadov.valorant_match_damage;

DELETE FROM squadov.valorant_match_damage;

CREATE OR REPLACE FUNCTION migrate_valorant_match_damage() RETURNS VOID AS
$$
DECLARE
    local_match_id VARCHAR;
BEGIN
    FOR local_match_id IN
        SELECT DISTINCT vmd.match_id FROM squadov.valorant_match_damage_tmp AS vmd ORDER BY match_id ASC
    LOOP
		CREATE TEMPORARY SEQUENCE valorant_match_damage_seq;
		
		INSERT INTO squadov.valorant_match_damage (
			match_id,
			round_num,
			instigator_puuid,
			receiver_puuid,
			damage,
			legshots,
			bodyshots,
			headshots,
			sequence_id
		)
		SELECT
			vmdt.match_id,
			vmdt.round_num,
			vmdt.instigator_puuid,
			vmdt.receiver_puuid,
			vmdt.damage,
			vmdt.legshots,
			vmdt.bodyshots,
			vmdt.headshots,
			NEXTVAL('valorant_match_damage_seq')
		FROM squadov.valorant_match_damage_tmp AS vmdt
		WHERE vmdt.match_id = local_match_id
		ORDER BY round_num ASC, instigator_puuid ASC, receiver_puuid ASC, damage ASC, legshots ASC, bodyshots ASC, headshots ASC;
		
		DROP SEQUENCE valorant_match_damage_seq;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

SELECT migrate_valorant_match_damage();
DROP TABLE squadov.valorant_match_damage_tmp;