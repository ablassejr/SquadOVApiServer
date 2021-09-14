ALTER TABLE lol_match_participant_identities
ADD COLUMN puuid VARCHAR;

UPDATE lol_match_participant_identities AS p1
SET puuid = ra.puuid
FROM lol_match_participant_identities AS p2
INNER JOIN riot_accounts AS ra
    ON ra.summoner_id = p2.summoner_id
WHERE p1.match_uuid = p2.match_uuid AND p1.participant_id = p2.participant_id