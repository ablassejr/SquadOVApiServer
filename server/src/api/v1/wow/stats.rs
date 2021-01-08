use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime,Utc};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct WowStatsQueryParams {
    // How often we sample the start - end time range.
    #[serde(rename="psStepSeconds")]
    ps_step_seconds: i64,
    #[serde(deserialize_with="squadov_common::parse_utc_time_from_milliseconds")]
    start: Option<DateTime<Utc>>,
    #[serde(deserialize_with="squadov_common::parse_utc_time_from_milliseconds")]
    end: Option<DateTime<Utc>>
}

#[derive(Serialize)]
pub struct WowStatDatum {
    tm: f64,
    value: f64
}

impl api::ApiApplication {
    async fn get_wow_match_dps(&self, user_id: i64, match_uuid: &Uuid, params: &WowStatsQueryParams) -> Result<HashMap<String, Vec<WowStatDatum>>, SquadOvError> {
        if params.start.is_none() || params.end.is_none() {
            return Err(SquadOvError::BadRequest);
        }

        let raw_stats = sqlx::query!(
            r#"
            SELECT
                FLOOR((EXTRACT(EPOCH FROM mcl.tm) - EXTRACT(EPOCH FROM $3)) / $5::BIGINT) * $5::BIGINT AS "xtm",
                COALESCE(mcl.owner_guid, mcl.source->>'guid') AS "guid",
                SUM((mcl.evt->>'amount')::INTEGER) / CAST($5::BIGINT AS DOUBLE PRECISION) AS "amount"
            FROM squadov.wow_match_combatants AS wmc
            LEFT JOIN LATERAL (
                SELECT wce.*, wuco.owner_guid
                FROM squadov.wow_combat_log_events AS wce
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = wce.combat_log_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS wma
                    ON wma.combat_log_uuid = wcl.uuid
                LEFT JOIN squadov.wow_combatlog_unit_ownership AS wuco
                    ON wuco.combat_log_uuid = wcl.uuid
                        AND wuco.unit_guid = wce.source->>'guid'
                WHERE wcl.user_id = $1
                    AND wma.match_uuid = $2
                    AND wce.tm BETWEEN $3 AND $4
                    AND wce.evt @> '{"type": "DamageDone"}'
                    AND (wce.source->>'guid' = wmc.combatant_guid OR wuco.owner_guid = wmc.combatant_guid)
            ) AS mcl ON TRUE
            WHERE wmc.match_uuid = $2
            GROUP BY xtm, guid
            ORDER BY xtm, guid
            "#,
            user_id,
            match_uuid,
            params.start.unwrap(),
            params.end.unwrap(),
            params.ps_step_seconds,
        )
            .fetch_all(&*self.pool)
            .await?;

        let mut ret_map: HashMap<String, Vec<WowStatDatum>> = HashMap::new();
        for stat in &raw_stats {
            if stat.guid.is_none() || stat.amount.is_none() || stat.xtm.is_none() {
                continue;
            }

            let guid = stat.guid.as_ref().unwrap();
            let amount = stat.amount.as_ref().unwrap();
            let tm = stat.xtm.as_ref().unwrap();

            if !ret_map.contains_key(guid) {
                ret_map.insert(guid.clone(), vec![]);
            }

            let inner_vec = ret_map.get_mut(guid).unwrap();
            inner_vec.push(WowStatDatum{
                tm: *tm,
                value: *amount,
            });
        }
        Ok(ret_map)
    }

    async fn get_wow_match_heals_per_second(&self, user_id: i64, match_uuid: &Uuid, params: &WowStatsQueryParams) -> Result<HashMap<String, Vec<WowStatDatum>>, SquadOvError> {
        if params.start.is_none() || params.end.is_none() {
            return Err(SquadOvError::BadRequest);
        }

        let raw_stats = sqlx::query!(
            r#"
            SELECT
                FLOOR((EXTRACT(EPOCH FROM mcl.tm) - EXTRACT(EPOCH FROM $3)) / $5::BIGINT) * $5::BIGINT AS "xtm",
                mcl.source->>'guid' AS "guid",
                SUM(GREATEST((mcl.evt->>'amount')::INTEGER - (mcl.evt->>'overheal')::INTEGER, 0)) / CAST($5::BIGINT AS DOUBLE PRECISION) AS "amount"
            FROM squadov.wow_match_combatants AS wmc
            LEFT JOIN LATERAL (
                SELECT wce.*
                FROM squadov.wow_combat_log_events AS wce
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = wce.combat_log_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS wma
                    ON wma.combat_log_uuid = wcl.uuid
                WHERE wcl.user_id = $1
                    AND wma.match_uuid = $2
                    AND wce.tm BETWEEN $3 AND $4
                    AND wce.evt @> '{"type": "Healing"}'
                    AND wce.source->>'guid' = wmc.combatant_guid
            ) AS mcl ON TRUE
            WHERE wmc.match_uuid = $2
            GROUP BY xtm, guid
            ORDER BY xtm, guid
            "#,
            user_id,
            match_uuid,
            params.start.unwrap(),
            params.end.unwrap(),
            params.ps_step_seconds,
        )
            .fetch_all(&*self.pool)
            .await?;

        let mut ret_map: HashMap<String, Vec<WowStatDatum>> = HashMap::new();
        for stat in &raw_stats {
            if stat.guid.is_none() || stat.amount.is_none() || stat.xtm.is_none() {
                continue;
            }

            let guid = stat.guid.as_ref().unwrap();
            let amount = stat.amount.as_ref().unwrap();
            let tm = stat.xtm.as_ref().unwrap();

            if !ret_map.contains_key(guid) {
                ret_map.insert(guid.clone(), vec![]);
            }

            let inner_vec = ret_map.get_mut(guid).unwrap();
            inner_vec.push(WowStatDatum{
                tm: *tm,
                value: *amount,
            });
        }
        Ok(ret_map)
    }

    async fn get_wow_match_damage_received_per_second(&self, user_id: i64, match_uuid: &Uuid, params: &WowStatsQueryParams) -> Result<HashMap<String, Vec<WowStatDatum>>, SquadOvError> {
        if params.start.is_none() || params.end.is_none() {
            return Err(SquadOvError::BadRequest);
        }

        let raw_stats = sqlx::query!(
            r#"
            SELECT
                FLOOR((EXTRACT(EPOCH FROM mcl.tm) - EXTRACT(EPOCH FROM $3)) / $5::BIGINT) * $5::BIGINT AS "xtm",
                mcl.dest->>'guid' AS "guid",
                SUM((mcl.evt->>'amount')::INTEGER) / CAST($5::BIGINT AS DOUBLE PRECISION) AS "amount"
            FROM squadov.wow_match_combatants AS wmc
            LEFT JOIN LATERAL (
                SELECT wce.*
                FROM squadov.wow_combat_log_events AS wce
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = wce.combat_log_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS wma
                    ON wma.combat_log_uuid = wcl.uuid
                WHERE wcl.user_id = $1
                    AND wma.match_uuid = $2
                    AND wce.tm BETWEEN $3 AND $4
                    AND wce.evt @> '{"type": "DamageDone"}'
                    AND wce.dest->>'guid' = wmc.combatant_guid
            ) AS mcl ON TRUE
            WHERE wmc.match_uuid = $2
            GROUP BY xtm, guid
            ORDER BY xtm, guid
            "#,
            user_id,
            match_uuid,
            params.start.unwrap(),
            params.end.unwrap(),
            params.ps_step_seconds,
        )
            .fetch_all(&*self.pool)
            .await?;

        let mut ret_map: HashMap<String, Vec<WowStatDatum>> = HashMap::new();
        for stat in &raw_stats {
            if stat.guid.is_none() || stat.amount.is_none() || stat.xtm.is_none() {
                continue;
            }

            let guid = stat.guid.as_ref().unwrap();
            let amount = stat.amount.as_ref().unwrap();
            let tm = stat.xtm.as_ref().unwrap();

            if !ret_map.contains_key(guid) {
                ret_map.insert(guid.clone(), vec![]);
            }

            let inner_vec = ret_map.get_mut(guid).unwrap();
            inner_vec.push(WowStatDatum{
                tm: *tm,
                value: *amount,
            });
        }
        Ok(ret_map)
    }
}

pub async fn get_wow_match_dps_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>, query: web::Query<WowStatsQueryParams>) -> Result<HttpResponse, SquadOvError> {
    let stats = app.get_wow_match_dps(path.user_id, &path.match_uuid, &query).await?;
    Ok(HttpResponse::Ok().json(stats))
}

pub async fn get_wow_match_heals_per_second_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>, query: web::Query<WowStatsQueryParams>) -> Result<HttpResponse, SquadOvError> {
    let stats = app.get_wow_match_heals_per_second(path.user_id, &path.match_uuid, &query).await?;
    Ok(HttpResponse::Ok().json(stats))
}

pub async fn get_wow_match_damage_received_per_second_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>, query: web::Query<WowStatsQueryParams>) -> Result<HttpResponse, SquadOvError> {
    let stats = app.get_wow_match_damage_received_per_second(path.user_id, &path.match_uuid, &query).await?;
    Ok(HttpResponse::Ok().json(stats))
}
