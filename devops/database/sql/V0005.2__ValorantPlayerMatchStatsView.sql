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
    legshots
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
    SUM(vmd.damage),
    SUM(vmd.headshots),
    SUM(vmd.bodyshots),
    SUM(vmd.legshots)
FROM valorant_match_players AS vmp
LEFT JOIN valorant_match_damage AS vmd
    ON vmd.instigator_puuid = vmp.puuid AND vmd.match_id = vmp.match_id
GROUP BY vmp.match_id, vmp.puuid