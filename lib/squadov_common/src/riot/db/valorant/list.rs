use crate::{
    SquadOvError,
    riot::{
        ValorantMatchFilters,
        games::valorant::{
            ValorantPlayerMatchSummary
        },
    },
    matches::MatchPlayerPair,
};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn list_valorant_match_summaries_for_uuids(ex: &PgPool, uuids: &[MatchPlayerPair]) -> Result<Vec<ValorantPlayerMatchSummary>, SquadOvError> {
    let match_uuids = uuids.iter().map(|x| { x.match_uuid.clone() }).collect::<Vec<Uuid>>();
    let player_uuids = uuids.iter().map(|x| { x.player_uuid.clone() }).collect::<Vec<Uuid>>();

    Ok(
        sqlx::query_as!(
            ValorantPlayerMatchSummary,
            r#"
            SELECT
                vmul.match_id,
                vmul.match_uuid,
                vm.server_start_time_utc,
                vm.game_mode,
                vm.map_id,
                vm.is_ranked,
                vm.provisioning_flow_id,
                vmp.character_id,
                vmt.won,
                vmt.rounds_won,
                (
                    SELECT MAX(rounds_won)
                    FROM squadov.valorant_match_teams
                    WHERE team_id != vmp.team_id
                        AND match_uuid = vm.match_uuid
                ) AS "rounds_lost!",
                (
                    SELECT COUNT(puuid) + 1
                    FROM squadov.valorant_match_players
                    WHERE match_uuid = vm.match_uuid
                        AND total_combat_score > vmp.total_combat_score
                ) AS "combat_score_rank!",
                vvpms.competitive_tier AS "competitive_tier!",
                vvpms.kills AS "kills!",
                vvpms.deaths AS "deaths!",
                vvpms.assists AS "assists!",
                vvpms.rounds_played AS "rounds_played!",
                vvpms.total_combat_score AS "total_combat_score!",
                COALESCE(vvpms.total_damage, 0) AS "total_damage!",
                COALESCE(vvpms.headshots, 0) AS "headshots!",
                COALESCE(vvpms.bodyshots, 0) AS "bodyshots!",
                COALESCE(vvpms.legshots, 0) AS "legshots!",
                inp.user_uuid AS "user_uuid!",
                vmp.puuid AS "puuid!"
            FROM UNNEST($1::UUID[], $2::UUID[]) AS inp(match_uuid, user_uuid)
            INNER JOIN squadov.valorant_matches AS vm
                ON vm.match_uuid = inp.match_uuid
            INNER JOIN squadov.users AS u
                ON u.uuid = inp.user_uuid
            INNER JOIN squadov.riot_account_links AS ral
                ON ral.user_id = u.id
            INNER JOIN squadov.valorant_match_players AS vmp
                ON vmp.match_uuid = vm.match_uuid
                    AND vmp.puuid = ral.puuid
            INNER JOIN squadov.valorant_match_teams AS vmt
                ON vmt.team_id = vmp.team_id
                    AND vmt.match_uuid = vm.match_uuid
            CROSS JOIN LATERAL (
                SELECT
                    vmp2.match_uuid,
                    vmp2.puuid,
                    vmp2.competitive_tier,
                    vmp2.kills,
                    vmp2.deaths,
                    vmp2.assists,
                    vmp2.rounds_played,
                    vmp2.total_combat_score,
                    COALESCE(SUM(vmd2.damage), 0) AS "total_damage",
                    COALESCE(SUM(vmd2.headshots), 0) AS "headshots",
                    COALESCE(SUM(vmd2.bodyshots), 0) AS "bodyshots",
                    COALESCE(SUM(vmd2.legshots), 0) AS "legshots",
                    vmt2.won
                FROM squadov.valorant_match_players AS vmp2
                INNER JOIN squadov.valorant_match_teams AS vmt2
                    ON vmt2.match_uuid = vmp2.match_uuid AND vmt2.team_id = vmp2.team_id
                LEFT JOIN squadov.valorant_match_damage AS vmd2
                    ON vmd2.instigator_puuid = vmp.puuid AND vmd2.match_uuid = vmp2.match_uuid
                WHERE vmp2.match_uuid = vmp.match_uuid AND vmp2.puuid = vmp.puuid
                GROUP BY vmp2.match_uuid, vmp2.puuid, vmt2.won
            ) AS vvpms
            INNER JOIN squadov.valorant_match_uuid_link AS vmul
                ON vmul.match_uuid = vm.match_uuid
            ORDER BY vm.server_start_time_utc DESC
            "#,
            &match_uuids,
            &player_uuids,
        )
            .fetch_all(ex)
            .await?
    )
}

pub async fn list_valorant_match_summaries_for_puuid(ex: &PgPool, puuid: &str, user_uuid: &Uuid, req_user_id: i64, start: i64, end: i64, filters: &ValorantMatchFilters) -> Result<Vec<ValorantPlayerMatchSummary>, SquadOvError> {
    let uuids: Vec<Uuid> = sqlx::query!(
        r#"
            SELECT vm.match_uuid
            FROM squadov.valorant_matches AS vm
            INNER JOIN squadov.valorant_match_players AS vmp
                ON vmp.match_uuid = vm.match_uuid
            LEFT JOIN squadov.vods AS v
                ON v.match_uuid = vm.match_uuid
                    AND v.user_uuid = $4
                    AND v.is_clip = FALSE
            LEFT JOIN squadov.view_share_connections_access_users AS sau
                ON sau.match_uuid = vm.match_uuid
                    AND sau.user_id = $9
            CROSS JOIN (
                SELECT *
                FROM squadov.users
                WHERE uuid = $4
            ) AS u
            WHERE vmp.puuid = $1
                AND (CARDINALITY($5::VARCHAR[]) = 0 OR vm.map_id = ANY($5))
                AND (CARDINALITY($6::VARCHAR[]) = 0 OR vm.game_mode = ANY($6))
                AND (NOT $7::BOOLEAN OR v.video_uuid IS NOT NULL)
                AND (NOT $8::BOOLEAN OR vm.is_ranked)
                AND (u.id = $9 OR sau.match_uuid IS NOT NULL)
            ORDER BY vm.server_start_time_utc DESC
            LIMIT $2 OFFSET $3
        "#,
        puuid,
        end - start,
        start,
        user_uuid,
        &filters.maps.as_ref().unwrap_or(&vec![]).iter().map(|x| { x.clone() }).collect::<Vec<String>>(),
        &filters.modes.as_ref().unwrap_or(&vec![]).iter().map(|x| { x.clone() }).collect::<Vec<String>>(),
        filters.has_vod.unwrap_or(false),
        filters.is_ranked.unwrap_or(false),
        req_user_id,
    )
        .fetch_all(&*ex)
        .await?
        .into_iter()
        .map(|x| {
            x.match_uuid
        })
        .collect();

    list_valorant_match_summaries_for_uuids(ex, &uuids.into_iter().map(|x| {
        MatchPlayerPair{
            match_uuid: x,
            player_uuid: user_uuid.clone(),
        }
    }).collect::<Vec<MatchPlayerPair>>()).await
}