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
use sqlx::postgres::types::PgInterval;

#[derive(Deserialize)]
pub struct WowStatsQueryParams {
    // How often we sample the start - end time range.
    #[serde(rename="psStepSeconds")]
    ps_step_seconds: i64,
    // For each sample, the time (half) range to perform an average over.
    #[serde(rename="psIntervalSeconds")]
    ps_interval_seconds: i64,
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

        let step_interval = PgInterval{
            months: 0,
            days: 0,
            microseconds: params.ps_step_seconds * (1e+6 as i64),
        };

        let sample_interval = PgInterval{
            months: 0,
            days: 0,
            microseconds: params.ps_interval_seconds * (1e+6 as i64),
        };

        let raw_stats = sqlx::query!(
            r#"
            WITH match_combat_logs AS (
                SELECT wce.*
                FROM squadov.wow_combat_log_events AS wce
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = wce.combat_log_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS wma
                    ON wma.combat_log_uuid = wcl.uuid
                WHERE wcl.user_id = $1
                    AND wma.match_uuid = $2
            )
            SELECT
                generate_series AS "xtm",
                mcl.source->>'guid' AS "guid",
                SUM((mcl.evt->>'amount')::INTEGER) / CAST($7::BIGINT AS DOUBLE PRECISION) AS "amount"
            FROM generate_series(
                $3::TIMESTAMPTZ,
                $4::TIMESTAMPTZ,
                $5::INTERVAL
            )
            CROSS JOIN squadov.wow_match_combatants AS wmc
            LEFT JOIN match_combat_logs AS mcl
                ON mcl.tm BETWEEN (generate_series - $6::INTERVAL) AND (generate_series + $6::INTERVAL)
                    AND mcl.source->>'guid' = wmc.combatant_guid
            WHERE mcl.evt @> '{"type": "DamageDone"}'
            GROUP BY xtm, guid
            ORDER BY xtm, guid
            "#,
            user_id,
            match_uuid,
            params.start.unwrap(),
            params.end.unwrap(),
            step_interval,
            sample_interval,
            params.ps_interval_seconds * 2
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
                tm: (*tm - params.start.unwrap()).num_milliseconds() as f64 / 1000.0,
                value: *amount,
            });
        }
        Ok(ret_map)
    }

    async fn get_wow_match_heals_per_second(&self, user_id: i64, match_uuid: &Uuid, params: &WowStatsQueryParams) -> Result<HashMap<String, Vec<WowStatDatum>>, SquadOvError> {
        if params.start.is_none() || params.end.is_none() {
            return Err(SquadOvError::BadRequest);
        }

        let step_interval = PgInterval{
            months: 0,
            days: 0,
            microseconds: params.ps_step_seconds * (1e+6 as i64),
        };

        let sample_interval = PgInterval{
            months: 0,
            days: 0,
            microseconds: params.ps_interval_seconds * (1e+6 as i64),
        };

        let raw_stats = sqlx::query!(
            r#"
            WITH match_combat_logs AS (
                SELECT wce.*
                FROM squadov.wow_combat_log_events AS wce
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = wce.combat_log_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS wma
                    ON wma.combat_log_uuid = wcl.uuid
                WHERE wcl.user_id = $1
                    AND wma.match_uuid = $2
            )
            SELECT
                generate_series AS "xtm",
                mcl.source->>'guid' AS "guid",
                SUM(GREATEST((mcl.evt->>'amount')::INTEGER - (mcl.evt->>'overheal')::INTEGER, 0)) / CAST($7::BIGINT AS DOUBLE PRECISION) AS "amount"
            FROM generate_series(
                $3::TIMESTAMPTZ,
                $4::TIMESTAMPTZ,
                $5::INTERVAL
            )
            CROSS JOIN squadov.wow_match_combatants AS wmc
            LEFT JOIN match_combat_logs AS mcl
                ON mcl.tm BETWEEN (generate_series - $6::INTERVAL) AND (generate_series + $6::INTERVAL)
                    AND mcl.source->>'guid' = wmc.combatant_guid
            WHERE mcl.evt @> '{"type": "Healing"}'
            GROUP BY xtm, guid
            ORDER BY xtm, guid
            "#,
            user_id,
            match_uuid,
            params.start.unwrap(),
            params.end.unwrap(),
            step_interval,
            sample_interval,
            params.ps_interval_seconds * 2
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
                tm: (*tm - params.start.unwrap()).num_milliseconds() as f64 / 1000.0,
                value: *amount,
            });
        }
        Ok(ret_map)
    }

    async fn get_wow_match_damage_received_per_second(&self, user_id: i64, match_uuid: &Uuid, params: &WowStatsQueryParams) -> Result<HashMap<String, Vec<WowStatDatum>>, SquadOvError> {

        if params.start.is_none() || params.end.is_none() {
            return Err(SquadOvError::BadRequest);
        }

        let step_interval = PgInterval{
            months: 0,
            days: 0,
            microseconds: params.ps_step_seconds * (1e+6 as i64),
        };

        let sample_interval = PgInterval{
            months: 0,
            days: 0,
            microseconds: params.ps_interval_seconds * (1e+6 as i64),
        };

        let raw_stats = sqlx::query!(
            r#"
            WITH match_combat_logs AS (
                SELECT wce.*
                FROM squadov.wow_combat_log_events AS wce
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = wce.combat_log_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS wma
                    ON wma.combat_log_uuid = wcl.uuid
                WHERE wcl.user_id = $1
                    AND wma.match_uuid = $2
            )
            SELECT
                generate_series AS "xtm",
                mcl.dest->>'guid' AS "guid",
                SUM((mcl.evt->>'amount')::INTEGER) / CAST($7::BIGINT AS DOUBLE PRECISION) AS "amount"
            FROM generate_series(
                $3::TIMESTAMPTZ,
                $4::TIMESTAMPTZ,
                $5::INTERVAL
            )
            CROSS JOIN squadov.wow_match_combatants AS wmc
            LEFT JOIN match_combat_logs AS mcl
                ON mcl.tm BETWEEN (generate_series - $6::INTERVAL) AND (generate_series + $6::INTERVAL)
                    AND mcl.dest->>'guid' = wmc.combatant_guid
            WHERE mcl.evt @> '{"type": "DamageDone"}'
            GROUP BY xtm, guid
            ORDER BY xtm, guid
            "#,
            user_id,
            match_uuid,
            params.start.unwrap(),
            params.end.unwrap(),
            step_interval,
            sample_interval,
            params.ps_interval_seconds * 2
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
                tm: (*tm - params.start.unwrap()).num_milliseconds() as f64 / 1000.0,
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
