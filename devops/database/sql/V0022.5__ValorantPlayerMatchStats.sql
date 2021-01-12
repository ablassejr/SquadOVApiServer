CREATE VIEW view_valorant_player_match_stats (
    match_uuid,
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
    vmp.match_uuid,
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
    ON vmt.match_uuid = vmp.match_uuid AND vmt.team_id = vmp.team_id
LEFT JOIN valorant_match_damage AS vmd
    ON vmd.instigator_puuid = vmp.puuid AND vmd.match_uuid = vmp.match_uuid
GROUP BY vmp.match_uuid, vmp.puuid, vmt.won