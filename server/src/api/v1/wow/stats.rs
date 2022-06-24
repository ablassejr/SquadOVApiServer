use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
    wow::{
        reports::{
            WowReportTypes,
            stats::{
                WowUnitTimelineEntry,
                WowUnitStatSummary,
            },
        },
    },
};
use serde::{Serialize};
use std::collections::HashMap;

#[derive(Serialize)]
pub struct WowStatDatum {
    pub tm: f64,
    pub value: f64
}

#[derive(Serialize)]
pub struct WowStatItem {
    pub guid: String,
    pub value: i64,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct WowMatchStatSummaryData {
    pub damage_dealt: Vec<WowStatItem>,
    pub damage_received: Vec<WowStatItem>,
    pub heals: Vec<WowStatItem>,
}

pub async fn get_wow_match_dps_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>) -> Result<HttpResponse, SquadOvError> {
    let match_view = squadov_common::wow::matches::get_generic_wow_match_view_from_match_user(&*app.pool, &path.match_uuid, path.user_id).await?;
    let stats = if let Some(combat_log_partition_id) = match_view.combat_log_partition_id.as_ref() {
        let reports: Vec<_> = app.cl_itf.get_report_avro::<WowUnitTimelineEntry>(&combat_log_partition_id, WowReportTypes::Stats as i32, "dps.avro").await?;
        let mut ret: HashMap<String, Vec<WowStatDatum>> = HashMap::new();

        for x in reports {
            let datum = WowStatDatum{
                tm: x.tm as f64,
                value: x.value,
            };

            if let Some(v) = ret.get_mut(&x.guid) {
                v.push(datum);
            } else {
                ret.insert(x.guid.clone(), vec![datum]);
            }
        }

        ret
    } else {
        return Err(SquadOvError::BadRequest);
    };
    Ok(HttpResponse::Ok().json(stats))
}

pub async fn get_wow_match_heals_per_second_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>) -> Result<HttpResponse, SquadOvError> {
    let match_view = squadov_common::wow::matches::get_generic_wow_match_view_from_match_user(&*app.pool, &path.match_uuid, path.user_id).await?;
    let stats = if let Some(combat_log_partition_id) = match_view.combat_log_partition_id.as_ref() {
        let reports: Vec<_> = app.cl_itf.get_report_avro::<WowUnitTimelineEntry>(&combat_log_partition_id, WowReportTypes::Stats as i32, "hps.avro").await?;
        let mut ret: HashMap<String, Vec<WowStatDatum>> = HashMap::new();

        for x in reports {
            let datum = WowStatDatum{
                tm: x.tm as f64,
                value: x.value,
            };

            if let Some(v) = ret.get_mut(&x.guid) {
                v.push(datum);
            } else {
                ret.insert(x.guid.clone(), vec![datum]);
            }
        }

        ret
    } else {
        return Err(SquadOvError::BadRequest);
    };
    Ok(HttpResponse::Ok().json(stats))
}

pub async fn get_wow_match_damage_received_per_second_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>) -> Result<HttpResponse, SquadOvError> {
    let match_view = squadov_common::wow::matches::get_generic_wow_match_view_from_match_user(&*app.pool, &path.match_uuid, path.user_id).await?;
    let stats = if let Some(combat_log_partition_id) = match_view.combat_log_partition_id.as_ref() {
        let reports: Vec<_> = app.cl_itf.get_report_avro::<WowUnitTimelineEntry>(&combat_log_partition_id, WowReportTypes::Stats as i32, "drps.avro").await?;
        let mut ret: HashMap<String, Vec<WowStatDatum>> = HashMap::new();

        for x in reports {
            let datum = WowStatDatum{
                tm: x.tm as f64,
                value: x.value,
            };

            if let Some(v) = ret.get_mut(&x.guid) {
                v.push(datum);
            } else {
                ret.insert(x.guid.clone(), vec![datum]);
            }
        }

        ret
    } else {
        return Err(SquadOvError::BadRequest);
    };
    Ok(HttpResponse::Ok().json(stats))
}

pub async fn get_wow_match_stat_summary_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>) -> Result<HttpResponse, SquadOvError> {
    let match_view = squadov_common::wow::matches::get_generic_wow_match_view_from_match_user(&*app.pool, &path.match_uuid, path.user_id).await?;
    let summary = if let Some(combat_log_partition_id) = match_view.combat_log_partition_id.as_ref() {
        let mut ret = WowMatchStatSummaryData{
            damage_dealt: vec![],
            damage_received: vec![],
            heals: vec![],
        };

        let reports: Vec<_> = app.cl_itf.get_report_avro::<WowUnitStatSummary>(&combat_log_partition_id, WowReportTypes::Stats as i32, "summary.avro").await?;
        for r in reports {
            ret.damage_dealt.push(WowStatItem{
                guid: r.guid.clone(),
                value: r.damage_dealt,
            });

            ret.damage_received.push(WowStatItem{
                guid: r.guid.clone(),
                value: r.damage_received,
            });

            ret.heals.push(WowStatItem{
                guid: r.guid.clone(),
                value: r.heals,
            });
        }

        ret
    } else {
        return Err(SquadOvError::BadRequest);
    };
    Ok(
        HttpResponse::Ok().json(&summary)
    )
}