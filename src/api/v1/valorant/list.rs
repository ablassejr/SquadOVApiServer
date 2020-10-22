use crate::common;
use crate::api;
use actix_web::{web, HttpResponse, HttpRequest};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ValorantUserMatchListInput {
    puuid: String,
}

impl api::ApiApplication {
    pub async fn list_valorant_matches_for_user(&self, puuid: &str, start: i64, end: i64) -> Result<Vec<super::ValorantPlayerMatchSummary>, common::SquadOvError> {
        let matches = sqlx::query_as!(
            super::ValorantPlayerMatchSummary,
            r#"
            SELECT
                vm.match_id,
                vm.server_start_time_utc,
                vm.game_mode,
                vm.map,
                vm.is_ranked,
                vm.provisioning_flow_id,
                vm.game_version,
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
                COALESCE(vvpms.legshots, 0) AS "legshots!",
                vods.end_time IS NOT NULL AS "has_vod!"
            FROM squadov.valorant_matches AS vm
            INNER JOIN squadov.valorant_match_players AS vmp
                ON vmp.match_id = vm.match_id
            INNER JOIN squadov.valorant_match_teams AS vmt
                ON vmt.team_id = vmp.team_id
                    AND vmt.match_id = vm.match_id
            INNER JOIN squadov.view_valorant_player_match_stats AS vvpms
                ON vvpms.puuid = vmp.puuid AND vvpms.match_id = vm.match_id
            LEFT JOIN squadov.vods AS vods
                ON vods.match_uuid = vm.match_uuid
            WHERE vmp.puuid = $1
            ORDER BY server_start_time_utc DESC
            LIMIT $2 OFFSET $3
            "#,
            puuid,
            end - start,
            start
        )
            .fetch_all(&*self.pool)
            .await?;
        return Ok(matches);
    }
}

pub async fn list_valorant_matches_for_user_handler(data : web::Path<ValorantUserMatchListInput>, query: web::Query<api::PaginationParameters>, app : web::Data<Arc<api::ApiApplication>>, req: HttpRequest) -> Result<HttpResponse, common::SquadOvError> {
    let query = query.into_inner();
    let matches = app.list_valorant_matches_for_user(
        &data.puuid,
        query.start,
        query.end,
    ).await?;

    let expected_total = query.end - query.start;
    let got_total = matches.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(matches, &req, &query, expected_total == got_total)?)) 
}