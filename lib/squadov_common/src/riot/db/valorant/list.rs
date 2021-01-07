use crate::{
    SquadOvError,
    riot::games::valorant::{
        ValorantPlayerMatchSummary
    }
};
use sqlx::PgPool;

pub async fn list_valorant_match_summaries_for_puuid(ex: &PgPool, puuid: &str, start: i64, end: i64) -> Result<Vec<ValorantPlayerMatchSummary>, SquadOvError> {
    Ok(
        sqlx::query_as!(
            ValorantPlayerMatchSummary,
            r#"
            SELECT
                vm.match_id,
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
                        AND match_id = vm.match_id
                ) AS "rounds_lost!",
                (
                    SELECT COUNT(puuid) + 1
                    FROM squadov.valorant_match_players
                    WHERE match_id = vm.match_id
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
                COALESCE(vvpms.legshots, 0) AS "legshots!"
            FROM squadov.valorant_matches AS vm
            INNER JOIN squadov.valorant_match_players AS vmp
                ON vmp.match_id = vm.match_id
            INNER JOIN squadov.valorant_match_teams AS vmt
                ON vmt.team_id = vmp.team_id
                    AND vmt.match_id = vm.match_id
            INNER JOIN squadov.view_valorant_player_match_stats AS vvpms
                ON vvpms.puuid = vmp.puuid AND vvpms.match_id = vm.match_id
            INNER JOIN squadov.valorant_match_uuid_link AS vmul
                ON vmul.match_id = vm.match_id
            WHERE vmp.puuid = $1
            ORDER BY server_start_time_utc DESC
            LIMIT $2 OFFSET $3
            "#,
            puuid,
            end - start,
            start
        )
            .fetch_all(ex)
            .await?
    )
}