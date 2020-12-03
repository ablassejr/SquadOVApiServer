CREATE TABLE valorant_match_damage_new (
    match_id VARCHAR NOT NULL REFERENCES valorant_matches(match_id) ON DELETE CASCADE,
    round_num INTEGER NOT NULL,
    instigator_puuid VARCHAR NOT NULL,
    receiver_puuid VARCHAR NOT NULL,
    damage INTEGER NOT NULL,
    legshots INTEGER NOT NULL,
    bodyshots INTEGER NOT NULL,
    headshots INTEGER NOT NULL,
    -- This sequence ID is LOW KEY INSANE. Effectively we're assuming that we're going to be inserting
    -- player damage into the table in the same order EVERY TIME so that the 5th damage insertion is going
    -- to be the same assuming we parse the same match history JSON multiple times. Why do we need to do that?
    -- Because Valorant's damage information is NOT UNIQUE. It's possible for the game to give us multiple
    -- damage dealt objects from one player to another in a single round. Thus we need to find some way of being
    -- able to detect if we're trying to insert the same damage element. Hence this sequence_id. It'll be up
    -- to the application to create a temporary sequence AND USE IT in the insertion. Y I K E S.
    sequence_id BIGINT NOT NULL,
    FOREIGN KEY(match_id, round_num) REFERENCES valorant_match_rounds(match_id, round_num) ON DELETE CASCADE,
    FOREIGN KEY(match_id, instigator_puuid) REFERENCES valorant_match_players(match_id, puuid) ON DELETE CASCADE,
    FOREIGN KEY(match_id, receiver_puuid) REFERENCES valorant_match_players(match_id, puuid) ON DELETE CASCADE,
    UNIQUE(match_id, round_num, instigator_puuid, receiver_puuid, damage, legshots, bodyshots, headshots, sequence_id)
);

CREATE TEMPORARY SEQUENCE valorant_damage_seq;

INSERT INTO valorant_match_damage_new (
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
SELECT DISTINCT ON (match_id, round_num, instigator_puuid, receiver_puuid, damage, legshots, bodyshots, headshots)
    match_id,
    round_num,
    instigator_puuid,
    receiver_puuid,
    damage,
    legshots,
    bodyshots,
    headshots,
    NEXTVAL('valorant_damage_seq')
FROM valorant_match_damage AS vmd;

DROP TABLE valorant_match_damage CASCADE;
ALTER TABLE valorant_match_damage_new
RENAME TO valorant_match_damage;

CREATE VIEW view_valorant_player_match_stats (
    match_id,
    puuid,
    competitive_tier,
    kills,
    deaths,
    assists,
    rounds_played,
    total_combat_score,
    total_damage,
    headshots,
    bodyshots,
    legshots,
    won
)
AS
SELECT
    vmp.match_id,
    vmp.puuid,
    vmp.competitive_tier,
    vmp.kills,
    vmp.deaths,
    vmp.assists,
    vmp.rounds_played,
    vmp.total_combat_score,
    COALESCE(SUM(vmd.damage), 0),
    COALESCE(SUM(vmd.headshots), 0),
    COALESCE(SUM(vmd.bodyshots), 0),
    COALESCE(SUM(vmd.legshots), 0),
    vmt.won
FROM valorant_match_players AS vmp
INNER JOIN valorant_match_teams AS vmt
    ON vmt.match_id = vmp.match_id AND vmt.team_id = vmp.team_id
LEFT JOIN valorant_match_damage AS vmd
    ON vmd.instigator_puuid = vmp.puuid AND vmd.match_id = vmp.match_id
GROUP BY vmp.match_id, vmp.puuid, vmt.won