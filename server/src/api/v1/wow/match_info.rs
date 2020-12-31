use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
    SerializedWoWDeath,
    SerializedWoWAura,
    SerializedWowEncounter,
    WoWSpellAuraType
};
use uuid::Uuid;
use serde::Serialize;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

impl api::ApiApplication {
    async fn get_wow_match_subencounters(&self, match_uuid: &Uuid, user_id: i64) -> Result<Vec<SerializedWowEncounter>, SquadOvError> {
        let raw_starts = sqlx::query!(
            r#"
            WITH match_start_stop (start, stop) AS (
                SELECT COALESCE(we.tm, wc.tm, NOW()), COALESCE(we.finish_time, wc.finish_time, NOW())
                FROM squadov.matches AS m
                LEFT JOIN squadov.wow_encounters AS we
                    ON we.match_uuid = m.uuid
                LEFT JOIN squadov.wow_challenges AS wc
                    ON wc.match_uuid = m.uuid
                WHERE m.uuid = $1
            )
            SELECT
                wce.tm,
                wce.evt#>>'{encounter_name}' AS "encounter_name"
            FROM squadov.wow_combat_log_events AS wce
            INNER JOIN squadov.wow_combat_logs AS wcl
                ON wcl.uuid = wce.combat_log_uuid
            INNER JOIN squadov.wow_match_combat_log_association AS wma
                ON wma.combat_log_uuid = wcl.uuid
            CROSS JOIN match_start_stop AS mss
            WHERE wma.match_uuid = $1
                AND wcl.user_id = $2
                AND wce.tm >= mss.start AND wce.tm <= mss.stop
                AND wce.evt @> '{"type": "EncounterStart"}'
            ORDER BY wce.tm ASC
            "#,
            match_uuid,
            user_id
        )
            .fetch_all(&*self.pool)
            .await?;

        let raw_ends = sqlx::query!(
            r#"
            WITH match_start_stop (start, stop) AS (
                SELECT COALESCE(we.tm, wc.tm, NOW()), COALESCE(we.finish_time, wc.finish_time, NOW())
                FROM squadov.matches AS m
                LEFT JOIN squadov.wow_encounters AS we
                    ON we.match_uuid = m.uuid
                LEFT JOIN squadov.wow_challenges AS wc
                    ON wc.match_uuid = m.uuid
                WHERE m.uuid = $1
            )
            SELECT
                wce.tm,
                wce.evt#>>'{encounter_name}' AS "encounter_name"
            FROM squadov.wow_combat_log_events AS wce
            INNER JOIN squadov.wow_combat_logs AS wcl
                ON wcl.uuid = wce.combat_log_uuid
            INNER JOIN squadov.wow_match_combat_log_association AS wma
                ON wma.combat_log_uuid = wcl.uuid
            CROSS JOIN match_start_stop AS mss
            WHERE wma.match_uuid = $1
                AND wcl.user_id = $2
                AND wce.tm >= mss.start AND wce.tm <= mss.stop
                AND wce.evt @> '{"type": "EncounterEnd"}'
            ORDER BY wce.tm ASC
            "#,
            match_uuid,
            user_id
        )
            .fetch_all(&*self.pool)
            .await?;

        // Same logic as the aura matching but we only need to match on the encounter name.
        let mut encounter_end_hashmap: HashMap<String, Vec<DateTime<Utc>>> = HashMap::new();
        for end in &raw_ends {
            if end.encounter_name.is_none() {
                continue;
            }

            let encounter_name = end.encounter_name.as_ref().unwrap();
            if !encounter_end_hashmap.contains_key(encounter_name) {
                encounter_end_hashmap.insert(encounter_name.clone(), vec![]);
            }

            let inner_vec = encounter_end_hashmap.get_mut(encounter_name).unwrap();
            inner_vec.push(end.tm);
        }

        Ok(
            raw_starts.into_iter()
                .filter(|x| {
                    return x.encounter_name.is_some()
                })
                .map(|x| {
                    let encounter_name = x.encounter_name.unwrap();

                    let mut end_tm: DateTime<Utc> = Utc::now();
                    if encounter_end_hashmap.contains_key(&encounter_name) {
                        let inner_vec = encounter_end_hashmap.get(&encounter_name).unwrap();
                        let idx = match inner_vec.binary_search(&x.tm) {
                            Ok(x) => x,
                            Err(x) => x
                        };

                        if idx < inner_vec.len() {
                            end_tm = inner_vec[idx];
                        }
                    }

                    SerializedWowEncounter{
                        encounter_name,
                        start_tm: x.tm,
                        end_tm,
                    }
                })
                .collect()
        )
    }

    async fn get_wow_match_aura_events(&self, match_uuid: &Uuid, user_id: i64) -> Result<Vec<SerializedWoWAura>, SquadOvError> {
        let raw_applied_auras = sqlx::query!(
            r#"
            WITH match_start_stop (start, stop) AS (
                SELECT COALESCE(we.tm, wc.tm, NOW()), COALESCE(we.finish_time, wc.finish_time, NOW())
                FROM squadov.matches AS m
                LEFT JOIN squadov.wow_encounters AS we
                    ON we.match_uuid = m.uuid
                LEFT JOIN squadov.wow_challenges AS wc
                    ON wc.match_uuid = m.uuid
                WHERE m.uuid = $1
            )
            SELECT
                wce.tm,
                wce.dest->>'guid' AS "guid",
                wce.dest->>'name' AS "name",
                (wce.evt#>>'{spell, id}')::BIGINT AS "spell_id",
                (wce.evt#>'{aura_type}') AS "aura",
                wce.evt#>>'{spell, name}' AS "spell_name"
            FROM squadov.wow_combat_log_events AS wce
            INNER JOIN squadov.wow_combat_logs AS wcl
                ON wcl.uuid = wce.combat_log_uuid
            INNER JOIN squadov.wow_match_combat_log_association AS wma
                ON wma.combat_log_uuid = wcl.uuid
            CROSS JOIN match_start_stop AS mss
            WHERE wma.match_uuid = $1
                AND wcl.user_id = $2
                AND wce.tm >= mss.start AND wce.tm <= mss.stop
                AND wce.evt @> '{"type": "SpellAura"}'
                AND wce.evt @> '{"applied": true}'
            ORDER BY wce.tm ASC
            "#,
            match_uuid,
            user_id
        )
            .fetch_all(&*self.pool)
            .await?;

        let raw_removed_auras = sqlx::query!(
            r#"
            WITH match_start_stop (start, stop) AS (
                SELECT COALESCE(we.tm, wc.tm, NOW()), COALESCE(we.finish_time, wc.finish_time, NOW())
                FROM squadov.matches AS m
                LEFT JOIN squadov.wow_encounters AS we
                    ON we.match_uuid = m.uuid
                LEFT JOIN squadov.wow_challenges AS wc
                    ON wc.match_uuid = m.uuid
                WHERE m.uuid = $1
            )
            SELECT
                wce.tm,
                wce.dest->>'guid' AS "guid",
                (wce.evt#>>'{spell, id}')::BIGINT AS "spell_id"
            FROM squadov.wow_combat_log_events AS wce
            INNER JOIN squadov.wow_combat_logs AS wcl
                ON wcl.uuid = wce.combat_log_uuid
            INNER JOIN squadov.wow_match_combat_log_association AS wma
                ON wma.combat_log_uuid = wcl.uuid
            CROSS JOIN match_start_stop AS mss
            WHERE wma.match_uuid = $1
                AND wcl.user_id = $2
                AND wce.tm >= mss.start AND wce.tm <= mss.stop
                AND wce.evt @> '{"type": "SpellAura"}'
                AND wce.evt @> '{"applied": false}'
            ORDER BY wce.tm ASC
            "#,
            match_uuid,
            user_id
        )
            .fetch_all(&*self.pool)
            .await?;

        // We need to go through every applied aura and match it up with the first matching
        // removed aura. Note that auras can be of different lengths so we can't just have two pointers
        // incrementing like one would do in a merge sort (one for the applied auras vector and one
        // for the removed auras vector); instead, for each applied aura, we need to search
        // through the removed auras to find the corresponding event. The corresponding event is the
        // removed aura event that first event (earlier timestamp) that satisifies
        // 1) The spell ID is identical
        // 2) The destination character GUID is identical.
        // Assuming we have N applied auras and M removed auras, the naive case runtime is N*M which is
        // probably unacceptable for the scale of N and M we expect to have. We thus would want to have
        // a Big-Oh N log M runtime which suggests a binary search of sorts through the array of removed auras.
        // To facilliate this log M search, we need to do a O(M) operation to build a two-layer hashmap of the
        // removed auras indexed on the destination GUID and spell ID. The hashmap would be composed of
        // vectors ordered by the timestamp of the removed aura event. Thus, for every applied aura,
        // we'd only need to do amortized constant time indexing by destination GUID and spell ID and then do
        // a binary search to find the first timestamp greater than the applied aura timestamp.
        let mut removed_aura_hashmap: HashMap<String, HashMap<i64, Vec<DateTime<Utc>>>> = HashMap::new();
        for aura in &raw_removed_auras {
            if aura.guid.is_none() || aura.spell_id.is_none() {
                continue;
            }

            let guid = aura.guid.as_ref().unwrap();
            let spell_id = aura.spell_id.as_ref().unwrap();

            if !removed_aura_hashmap.contains_key(guid) {
                removed_aura_hashmap.insert(guid.clone(), HashMap::new());
            }

            let spell_id_hashmap = removed_aura_hashmap.get_mut(guid).unwrap();
            if !spell_id_hashmap.contains_key(spell_id) {
                spell_id_hashmap.insert(*spell_id, vec![]);
            }

            let removed_tms = spell_id_hashmap.get_mut(spell_id).unwrap();
            removed_tms.push(aura.tm);
        }

        Ok(
            raw_applied_auras.into_iter()
                .filter(|x| {
                    return x.guid.is_some() &&
                        x.name.is_some() &&
                        x.spell_id.is_some() &&
                        x.spell_name.is_some() &&
                        x.aura.is_some()
                })
                .map(|x| {
                    let guid = x.guid.unwrap();
                    let spell_id = x.spell_id.unwrap();
                    let spell_name = x.spell_name.unwrap();

                    let mut removed_tm: DateTime<Utc> = Utc::now();
                    if removed_aura_hashmap.contains_key(&guid) {
                        let spell_id_hashmap = removed_aura_hashmap.get(&guid).unwrap();
                        if spell_id_hashmap.contains_key(&spell_id) {
                            let inner_vec = spell_id_hashmap.get(&spell_id).unwrap();
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
                        target_guid: guid,
                        target_name: x.name.unwrap(),
                        spell_id: spell_id,
                        spell_name: spell_name,
                        aura_type: serde_json::from_value(x.aura.unwrap()).unwrap_or(WoWSpellAuraType::Unknown),
                        applied_tm: x.tm,
                        removed_tm,
                    }
                })
                .collect()
        )
    }

    async fn get_wow_match_death_events(&self, match_uuid: &Uuid, user_id: i64) -> Result<Vec<SerializedWoWDeath>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                SerializedWoWDeath,
                r#"
                WITH match_start_stop (start, stop) AS (
                    SELECT COALESCE(we.tm, wc.tm, NOW()), COALESCE(we.finish_time, wc.finish_time, NOW())
                    FROM squadov.matches AS m
                    LEFT JOIN squadov.wow_encounters AS we
                        ON we.match_uuid = m.uuid
                    LEFT JOIN squadov.wow_challenges AS wc
                        ON wc.match_uuid = m.uuid
                    WHERE m.uuid = $1
                ), match_combat_logs AS (
                    SELECT wce.*
                    FROM squadov.wow_combat_log_events AS wce
                    INNER JOIN squadov.wow_combat_logs AS wcl
                        ON wcl.uuid = wce.combat_log_uuid
                    INNER JOIN squadov.wow_match_combat_log_association AS wma
                        ON wma.combat_log_uuid = wcl.uuid
                    CROSS JOIN match_start_stop AS mss
                    WHERE wma.match_uuid = $1
                        AND wcl.user_id = $2
                        AND wce.tm >= mss.start AND wce.tm <= mss.stop
                )
                SELECT
                    mcl.dest->>'guid' AS "guid!",
                    mcl.dest->>'name' AS "name!",
                    (mcl.dest->>'flags')::BIGINT AS "flags!",
                    mcl.tm AS "tm!"
                FROM match_combat_logs AS mcl
                WHERE mcl.evt @> '{"type": "UnitDied"}'
                ORDER BY mcl.tm ASC
                "#,
                match_uuid,
                user_id,
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
        encounters: Vec<SerializedWowEncounter>
    }

    Ok(HttpResponse::Ok().json(Response{
        deaths: app.get_wow_match_death_events(&path.match_uuid, path.user_id).await?,
        auras: app.get_wow_match_aura_events(&path.match_uuid, path.user_id).await?,
        encounters: app.get_wow_match_subencounters(&path.match_uuid, path.user_id).await?,
    }))
}