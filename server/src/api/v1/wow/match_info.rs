use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
    SerializedWoWDeath,
    SerializedWoWAura,
    SerializedWowEncounter,
    SerializedWoWResurrection,
    WoWSpellAuraType
};
use uuid::Uuid;
use serde::Serialize;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use std::str::FromStr;

impl api::ApiApplication {
    async fn get_wow_match_subencounters(&self, view_uuid: &Uuid) -> Result<Vec<SerializedWowEncounter>, SquadOvError> {
        let subencounter_events = sqlx::query!(
            r#"
            SELECT
                wve.tm,
                see.encounter_name,
                see.is_start
            FROM squadov.wow_match_view_subencounter_events AS see
            INNER JOIN squadov.wow_match_view_events AS wve
                ON wve.event_id = see.event_id
            INNER JOIN squadov.wow_match_view AS wmv
                ON wmv.alt_id = wve.view_id
            WHERE wmv.id = $1
            ORDER BY wve.tm ASC
            "#,
            view_uuid
        )
            .fetch_all(&*self.heavy_pool)
            .await?;

        let raw_starts: Vec<_> = subencounter_events.iter().filter(|x| {
            x.is_start
        }).collect();

        let raw_ends: Vec<_> = subencounter_events.iter().filter(|x| {
            !x.is_start
        }).collect();

        // For each encounter name, we get a list of when that encounter ends.
        // Note that an encounter can appear multiple times (theoretically but probably not practically).
        let mut encounter_end_hashmap: HashMap<String, Vec<DateTime<Utc>>> = HashMap::new();
        for end in &raw_ends {
            if !encounter_end_hashmap.contains_key(&end.encounter_name) {
                encounter_end_hashmap.insert(end.encounter_name.clone(), vec![]);
            }

            let inner_vec = encounter_end_hashmap.get_mut(&end.encounter_name).unwrap();
            inner_vec.push(end.tm.clone());
        }

        Ok(
            raw_starts.into_iter()
                .map(|x| {
                    let mut end_tm: DateTime<Utc> = Utc::now();
                    if encounter_end_hashmap.contains_key(&x.encounter_name) {
                        let inner_vec = encounter_end_hashmap.get(&x.encounter_name).unwrap();
                        let idx = match inner_vec.binary_search(&x.tm) {
                            Ok(x) => x,
                            Err(x) => x
                        };

                        if idx < inner_vec.len() {
                            end_tm = inner_vec[idx];
                        }
                    }

                    SerializedWowEncounter{
                        encounter_name: x.encounter_name.clone(),
                        start_tm: x.tm.clone(),
                        end_tm,
                    }
                })
                .collect()
        )
    }

    async fn get_wow_match_aura_events(&self, view_uuid: &Uuid) -> Result<Vec<SerializedWoWAura>, SquadOvError> {
        let raw_auras = sqlx::query!(
            r#"
            SELECT
                dest.unit_guid AS "target_guid",
                COALESCE(dest.unit_name, dest.unit_guid) AS "target_name!",
                wae.spell_id,
                wae.aura_type,
                wae.applied,
                wve.tm
            FROM squadov.wow_match_view_aura_events AS wae
            INNER JOIN squadov.wow_match_view_events AS wve
                ON wve.event_id = wae.event_id
            INNER JOIN squadov.wow_match_view_character_presence AS dest
                ON dest.character_id = wve.dest_char
            INNER JOIN squadov.wow_match_view AS wmv
                ON wmv.alt_id = wve.view_id
            WHERE wmv.id = $1
            ORDER BY wve.tm ASC
            "#,
            view_uuid,
        )
            .fetch_all(&*self.heavy_pool)
            .await?;

        // We need to match the end of an aura with the start of an aura. We do this based on spell id and the target
        // as we assume that the player will only have one instance of an aura running at any time.
        let mut removed_aura_hashmap: HashMap<String, HashMap<i64, Vec<DateTime<Utc>>>> = HashMap::new();
        for aura in &raw_auras {
            if aura.applied {
                continue;
            }

            if !removed_aura_hashmap.contains_key(&aura.target_guid) {
                removed_aura_hashmap.insert(aura.target_guid.clone(), HashMap::new());
            }

            let spell_id_hashmap = removed_aura_hashmap.get_mut(&aura.target_guid).unwrap();
            if !spell_id_hashmap.contains_key(&aura.spell_id) {
                spell_id_hashmap.insert(aura.spell_id, vec![]);
            }

            let removed_tms = spell_id_hashmap.get_mut(&aura.spell_id).unwrap();
            removed_tms.push(aura.tm);
        }

        Ok(
            raw_auras.into_iter()
                .filter(|x| {
                    x.applied
                })
                .map(|x| {
                    let mut removed_tm: DateTime<Utc> = Utc::now();
                    if removed_aura_hashmap.contains_key(&x.target_guid) {
                        let spell_id_hashmap = removed_aura_hashmap.get(&x.target_guid).unwrap();
                        if spell_id_hashmap.contains_key(&x.spell_id) {
                            let inner_vec = spell_id_hashmap.get(&x.spell_id).unwrap();
                            let idx = match inner_vec.binary_search(&x.tm) {
                                Ok(x) => x,
                                Err(x) => x
                            };

                            if idx < inner_vec.len() {
                                removed_tm = inner_vec[idx];
                            }
                        }
                    }

                    SerializedWoWAura{
                        target_guid: x.target_guid,
                        target_name: x.target_name,
                        spell_id: x.spell_id,
                        aura_type: WoWSpellAuraType::from_str(&x.aura_type).map_or(WoWSpellAuraType::Unknown, |x| { x }),
                        applied_tm: x.tm,
                        removed_tm,
                    }
                })
                .collect()
        )
    }

    async fn get_wow_match_death_events(&self, view_uuid: &Uuid) -> Result<Vec<SerializedWoWDeath>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                SerializedWoWDeath,
                r#"
                SELECT
                    dest.unit_guid AS "guid",
                    COALESCE(dest.unit_name, dest.unit_guid) AS "name!",
                    dest.flags,
                    wve.tm
                FROM squadov.wow_match_view_death_events AS wde
                INNER JOIN squadov.wow_match_view_events AS wve
                    ON wve.event_id = wde.event_id
                INNER JOIN squadov.wow_match_view_character_presence AS dest
                    ON dest.character_id = wve.dest_char
                INNER JOIN squadov.wow_match_view AS wmv
                    ON wmv.alt_id = wve.view_id
                WHERE wmv.id = $1
                ORDER BY wve.tm ASC
                "#,
                view_uuid
            )
                .fetch_all(&*self.pool)
                .await?
        )
    }

    async fn get_wow_match_resurrection_events(&self, view_uuid: &Uuid) -> Result<Vec<SerializedWoWResurrection>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                SerializedWoWResurrection,
                r#"
                SELECT
                    dest.unit_guid AS "guid",
                    COALESCE(dest.unit_name, dest.unit_guid) AS "name!",
                    dest.flags,
                    wve.tm
                FROM squadov.wow_match_view_resurrect_events AS wre
                INNER JOIN squadov.wow_match_view_events AS wve
                    ON wve.event_id = wre.event_id
                INNER JOIN squadov.wow_match_view_character_presence AS dest
                    ON dest.character_id = wve.dest_char
                INNER JOIN squadov.wow_match_view AS wmv
                    ON wmv.alt_id = wve.view_id
                WHERE wmv.id = $1
                ORDER BY wve.tm ASC
                "#,
                view_uuid
            )
                .fetch_all(&*self.pool)
                .await?
        )
    }
}

pub async fn list_wow_events_for_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>) -> Result<HttpResponse, SquadOvError> {
    #[derive(Serialize)]
    struct Response {
        deaths: Vec<SerializedWoWDeath>,
        auras: Vec<SerializedWoWAura>,
        encounters: Vec<SerializedWowEncounter>,
        resurrections: Vec<SerializedWoWResurrection>
    }

    let view_uuid = app.get_wow_match_view_for_user_match(path.user_id, &path.match_uuid).await?.ok_or(SquadOvError::NotFound)?;
    Ok(HttpResponse::Ok().json(Response{
        deaths: app.get_wow_match_death_events(&view_uuid).await?,
        auras: app.get_wow_match_aura_events(&view_uuid).await?,
        encounters: app.get_wow_match_subencounters(&view_uuid).await?,
        resurrections: app.get_wow_match_resurrection_events(&view_uuid).await?,
    }))
}