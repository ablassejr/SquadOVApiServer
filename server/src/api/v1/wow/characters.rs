use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
    WoWCharacter
};
use uuid::Uuid;

impl api::ApiApplication {
    async fn list_wow_characters_for_match(&self, match_uuid: &Uuid, user_id: i64) -> Result<Vec<WoWCharacter>, SquadOvError> {
        let characters = sqlx::query!(
            r#"
            WITH match_start_stop (start, stop) AS (
                SELECT COALESCE(we.tm, wc.tm, NOW()), COALESCE(we.finish_time, wc.finish_time, NOW())
                FROM squadov.matches AS m
                LEFT JOIN squadov.wow_encounters AS we
                    ON we.match_uuid = m.uuid
                LEFT JOIN squadov.wow_challenges AS wc
                    ON wc.match_uuid = m.uuid
                WHERE m.uuid = $2
            ), match_combat_logs AS (
                SELECT wce.*
                FROM squadov.wow_combat_log_events AS wce
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = wce.combat_log_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS wma
                    ON wma.combat_log_uuid = wcl.uuid
                CROSS JOIN match_start_stop AS mss
                WHERE wcl.user_id = $1
                    AND wma.match_uuid = $2
                    AND wce.tm >= mss.start AND wce.tm <= mss.stop
            ), combat_log_player_flags (guid, name) AS (
                SELECT DISTINCT source->>'guid', source->>'name'
                FROM match_combat_logs
                WHERE source IS NOT NULL
                    AND NOT source @> '{"name": "nil"}'
                UNION
                SELECT DISTINCT dest->>'guid', dest->>'name'
                FROM match_combat_logs
                WHERE dest IS NOT NULL
                    AND NOT dest @> '{"name": "nil"}'
            )
            SELECT DISTINCT ON (mcl.evt->>'guid')
                mcl.evt->>'guid' AS "guid!",
                clpf.name AS "name!",
                (mcl.evt->>'spec_id')::INTEGER AS "spec_id!",
                jsonb_path_query_array(
                    (mcl.evt->>'items')::jsonb,
                    '$[*].ilvl'
                ) AS "items!"
            FROM match_combat_logs AS mcl
            INNER JOIN combat_log_player_flags AS clpf
                ON clpf.guid = mcl.evt->>'guid'
            WHERE mcl.evt @> '{"type": "CombatantInfo"}'
            ORDER BY mcl.evt->>'guid', mcl.tm DESC, clpf.name ASC
            "#,
            user_id,
            match_uuid,
        )
            .fetch_all(&*self.pool)
            .await?;
        
        Ok(
            characters.into_iter().map(|x| {
                let relevant_ilvls: Vec<i32> = serde_json::from_value::<Vec<i32>>(x.items)
                    .unwrap_or(vec![])
                    .into_iter()
                    .filter(|x| { *x > 0 })
                    .collect();

                WoWCharacter{
                    guid: x.guid,
                    name: x.name,
                    ilvl: (relevant_ilvls.iter().sum::<i32>() as f32 / relevant_ilvls.len() as f32).floor() as i32,
                    spec_id: x.spec_id,
                }
            }).collect()          
        )
    }

    async fn list_wow_characters_for_user(&self, user_id: i64) -> Result<Vec<WoWCharacter>, SquadOvError> {
        // Pretty insane query that's probably pretty slow...once we have more logs. 
        // TODO: Optimize, maybe?
        // What this query does is 
        // 1) Pull all the combat log events that this user uploaded (maybe limit this somehow?)
        // 2) For every combat log event, analyze the 'source' and 'dest' fields for the COMBATLOG_FILTER_ME (0x511 = 1297) flag to determine
        //    which player GUID can be considered the "current player".
        // 3) Then given the player GUID, look for the latest relevant COMBATANT_INFO lines to pull their spec and ilvl.
        let characters = sqlx::query!(
            r#"
            WITH user_combat_logs AS (
                SELECT wce.*
                FROM squadov.wow_combat_log_events AS wce
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = wce.combat_log_uuid
                WHERE wcl.user_id = $1
            ), combat_log_player_flags (guid, name) AS (
                SELECT DISTINCT source->>'guid', source->>'name'
                FROM user_combat_logs
                WHERE source IS NOT NULL
                    AND source @> '{"flags": 1297}'
                    AND NOT source @> '{"name": "nil"}'
                UNION
                SELECT DISTINCT dest->>'guid', dest->>'name'
                FROM user_combat_logs
                WHERE dest IS NOT NULL
                    AND dest @> '{"flags": 1297}'
                    AND NOT dest @> '{"name": "nil"}'
            )
            SELECT DISTINCT ON (ucl.evt->>'guid')
                ucl.evt->>'guid' AS "guid!",
                clpf.name AS "name!",
                (ucl.evt->>'spec_id')::INTEGER AS "spec_id!",
                jsonb_path_query_array(
                    (ucl.evt->>'items')::jsonb,
                    '$[*].ilvl'
                ) AS "items!"
            FROM user_combat_logs AS ucl
            INNER JOIN combat_log_player_flags AS clpf
                ON clpf.guid = ucl.evt->>'guid'
            WHERE ucl.evt @> '{"type": "CombatantInfo"}'
            ORDER BY ucl.evt->>'guid', ucl.tm DESC, clpf.name ASC
            "#,
            user_id,
        )
            .fetch_all(&*self.pool)
            .await?;

        Ok(
            characters.into_iter().map(|x| {
                let relevant_ilvls: Vec<i32> = serde_json::from_value::<Vec<i32>>(x.items)
                    .unwrap_or(vec![])
                    .into_iter()
                    .filter(|x| { *x > 0 })
                    .collect();

                WoWCharacter{
                    guid: x.guid,
                    name: x.name,
                    ilvl: (relevant_ilvls.iter().sum::<i32>() as f32 / relevant_ilvls.len() as f32).floor() as i32,
                    spec_id: x.spec_id,
                }
            }).collect()          
        )
    }
}

pub async fn list_wow_characters_for_user_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserPath>) -> Result<HttpResponse, SquadOvError> {
    let chars = app.list_wow_characters_for_user(path.user_id).await?;
    Ok(HttpResponse::Ok().json(chars))
}

pub async fn list_wow_characters_for_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>) -> Result<HttpResponse, SquadOvError> {
    let chars = app.list_wow_characters_for_match(&path.match_uuid, path.user_id).await?;
    Ok(HttpResponse::Ok().json(chars))
}