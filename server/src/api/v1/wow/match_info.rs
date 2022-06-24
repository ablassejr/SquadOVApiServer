use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
    SerializedWoWDeath,
    SerializedWoWAura,
    SerializedWowEncounter,
    SerializedWoWResurrection,
    SerializedWoWSpellCast,
    SerializedWoWAuraBreak,
    WowDeathRecap,
    wow::reports::{
        events::{
            aura_breaks::WowAuraBreakEventReport,
            auras::WowAuraEventReport,
            deaths::{WowDeathRecapHpEvent, WowDeathEventReport},
            encounters::WowEncounterEventReport,
            resurrections::WowResurrectionEventReport,
            spell_casts::WowSpellCastEventReport,
        },
        WowReportTypes,
    },
};
use serde::{Deserialize, Serialize};

pub async fn list_wow_events_for_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>) -> Result<HttpResponse, SquadOvError> {
    #[derive(Serialize)]
    #[serde(rename_all="camelCase")]
    struct Response {
        deaths: Vec<SerializedWoWDeath>,
        auras: Vec<SerializedWoWAura>,
        encounters: Vec<SerializedWowEncounter>,
        resurrections: Vec<SerializedWoWResurrection>,
        aura_breaks: Vec<SerializedWoWAuraBreak>,
        spell_casts: Vec<SerializedWoWSpellCast>,
    }

    let match_view = squadov_common::wow::matches::get_generic_wow_match_view_from_match_user(&*app.pool, &path.match_uuid, path.user_id).await?;
    Ok(HttpResponse::Ok().json(
        if let Some(combat_log_partition_id) = match_view.combat_log_partition_id {
            Response{
                deaths: app.cl_itf.get_report_avro::<WowDeathEventReport>(&combat_log_partition_id, WowReportTypes::Events as i32, "deaths.avro").await?.into_iter().map(|x| {
                    x.into()
                }).collect(),
                auras: app.cl_itf.get_report_avro::<WowAuraEventReport>(&combat_log_partition_id, WowReportTypes::Events as i32, "auras.avro").await?.into_iter().map(|x| {
                    x.into()
                }).collect(),
                encounters: app.cl_itf.get_report_avro::<WowEncounterEventReport>(&combat_log_partition_id, WowReportTypes::Events as i32, "encounters.avro").await?.into_iter().map(|x| {
                    x.into()
                }).collect(),
                resurrections: app.cl_itf.get_report_avro::<WowResurrectionEventReport>(&combat_log_partition_id, WowReportTypes::Events as i32, "resurrections.avro").await?.into_iter().map(|x| {
                    x.into()
                }).collect(),
                aura_breaks: app.cl_itf.get_report_avro::<WowAuraBreakEventReport>(&combat_log_partition_id, WowReportTypes::Events as i32, "aura_breaks.avro").await?.into_iter().map(|x| {
                    x.into()
                }).collect(),
                spell_casts: app.cl_itf.get_report_avro::<WowSpellCastEventReport>(&combat_log_partition_id, WowReportTypes::Events as i32, "spell_casts.avro").await?.into_iter().map(|x| {
                    x.into()
                }).collect(),
            }
        } else {
            return Err(SquadOvError::BadRequest);
        }
    ))
}

#[derive(Deserialize)]
pub struct WowEventIdPath {
    event_id: i64
}

pub async fn get_death_recap_handler(app : web::Data<Arc<api::ApiApplication>>, match_path: web::Path<super::WoWUserMatchPath>, event_path: web::Path<WowEventIdPath>) -> Result<HttpResponse, SquadOvError> {
    let match_view = squadov_common::wow::matches::get_generic_wow_match_view_from_match_user(&*app.pool, &match_path.match_uuid, match_path.user_id).await?;
    Ok(HttpResponse::Ok().json(if let Some(combat_log_partition_id) = match_view.combat_log_partition_id {
        let reports = app.cl_itf.get_report_avro::<WowDeathRecapHpEvent>(&combat_log_partition_id, WowReportTypes::DeathRecap as i32, &format!("{}.avro", event_path.event_id)).await?;
        WowDeathRecap{
            // The output should be newest event first - we store oldest event first.
            hp_events: reports.into_iter().map(|x| { x.into() }).rev().collect(),
        }
    } else {
        return Err(SquadOvError::BadRequest);
    }))
}