use squadov_common;
use crate::api;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ValorantUserStatSummaryInput {
    puuid: String,
}

impl api::ApiApplication {
    async fn get_player_stats_summary(&self, puuid: &str) -> Result<Option<super::ValorantPlayerStatsSummary>, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
            super::ValorantPlayerStatsSummary,
            r#"
            SELECT
                COALESCE((
                    SELECT vmp.competitive_tier
                    FROM squadov.valorant_matches AS vm
                    INNER JOIN squadov.valorant_match_players AS vmp
                        ON vmp.match_uuid = vm.match_uuid
                    WHERE vmp.puuid = $1
                        AND vm.is_ranked IS TRUE
                    ORDER BY vm.server_start_time_utc DESC
                    LIMIT 1
                ), 0) AS "rank!",
                SUM(vvpms.kills)::BIGINT AS "kills!",
                SUM(vvpms.deaths)::BIGINT AS "deaths!",
                SUM(vvpms.assists)::BIGINT AS "assists!",
                SUM(vvpms.rounds_played)::BIGINT AS "rounds!",
                SUM(vvpms.total_combat_score)::BIGINT AS "total_combat_score!",
                SUM(vvpms.total_damage)::BIGINT AS "total_damage!",
                SUM(vvpms.headshots)::BIGINT AS "headshots!",
                SUM(vvpms.bodyshots)::BIGINT AS "bodyshots!",
                SUM(vvpms.legshots)::BIGINT AS "legshots!",
                SUM(CASE WHEN vvpms.won THEN 1 END)::BIGINT AS "wins!",
                COUNT(vvpms.match_uuid) AS "games!"
            FROM squadov.view_valorant_player_match_stats AS vvpms
            WHERE vvpms.puuid = $1
            GROUP BY vvpms.puuid
            "#,
            puuid,
        )
            .fetch_optional(&*self.pool)
            .await?)
    }
}

pub async fn get_player_stats_summary_handler(data : web::Path<ValorantUserStatSummaryInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    match app.get_player_stats_summary(&data.puuid).await? {
        Some(x) => Ok(HttpResponse::Ok().json(&x)),
        None => Ok(HttpResponse::Ok().json(super::ValorantPlayerStatsSummary{ ..Default::default() }))
    }
}