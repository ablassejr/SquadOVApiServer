use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
    WoWCharacter,
    WoWCharacterUserAssociation
};
use uuid::Uuid;
use std::collections::HashMap;

impl api::ApiApplication {
    pub async fn list_wow_characters_for_match(&self, match_uuid: &Uuid, user_id: i64) -> Result<Vec<WoWCharacter>, SquadOvError> {
        let mss = self.get_wow_match_start_stop(match_uuid).await?;
        // There's two queries that need to be done here: get a list if the COMBATANT_INFO events that tell us every character that's
        // supposed to be in the match along with their character spec and item level. Additionally, we need to retrieve a log line
        // with that particular character as the source to get the character's name (for display purposes). We combine these two separate
        // queries to generate our vector of WoWCharacter objects.
        let characters = sqlx::query!(
            r#"
            SELECT DISTINCT ON (mcl.evt->>'guid')
                mcl.evt->>'guid' AS "guid!",
                (mcl.evt->>'spec_id')::INTEGER AS "spec_id!",
                jsonb_path_query_array(
                    (mcl.evt->>'items')::jsonb,
                    '$[*].ilvl'
                ) AS "items!",
                COALESCE((mcl.evt->>'team')::INTEGER, 0) AS "team!"
            FROM (
                SELECT wce.*
                FROM squadov.wow_combat_log_events AS wce
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = wce.combat_log_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS wma
                    ON wma.combat_log_uuid = wcl.uuid
                WHERE wcl.user_id = $1
                    AND wma.match_uuid = $2
                    AND wce.tm BETWEEN $3 AND $4
                    AND wce.evt @> '{"type": "CombatantInfo"}'
            ) AS mcl
            ORDER BY mcl.evt->>'guid', mcl.tm DESC
            "#,
            user_id,
            match_uuid,
            &mss.start,
            &mss.end,
        )
            .fetch_all(&*self.heavy_pool)
            .await?;

        let guids: Vec<String> = characters.iter().map(|x| { x.guid.clone() }).collect();

        // It may be tempting to do an ORDER BY wce.tm DESC here but that 1) is unnecessary
        // and 2) makes the query MUCH slower. It's unnecessary since the user's name SHOULD
        // NOT change throughout the duration of a match.
        let guid_to_name: HashMap<String, String> = sqlx::query!(
            r#"
            SELECT pl.guid AS "guid!", t1.name AS "name"
            FROM UNNEST($1::VARCHAR[]) AS pl(guid)
            CROSS JOIN LATERAL (
                SELECT wce.source->>'name'
                FROM squadov.wow_combat_log_events AS wce
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = wce.combat_log_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS wma
                    ON wma.combat_log_uuid = wcl.uuid
                WHERE wcl.user_id = $2
                    AND wma.match_uuid = $3
                    AND wce.source->>'guid' = pl.guid
                LIMIT 1
            ) AS t1(name)
            "#,
            &guids,
            user_id,
            match_uuid
        )
            .fetch_all(&*self.heavy_pool)
            .await?
            .into_iter()
            .map(|x| {
                (x.guid, x.name.unwrap_or(String::from("<Unknown>")))
            })
            .collect();
        
        Ok(
            characters.into_iter().map(|x| {
                let relevant_ilvls: Vec<i32> = serde_json::from_value::<Vec<i32>>(x.items)
                    .unwrap_or(vec![])
                    .into_iter()
                    .filter(|x| { *x > 0 })
                    .collect();

                let nm = guid_to_name.get(&x.guid).unwrap_or(&String::from("<Unknown>")).clone();
                WoWCharacter{
                    guid: x.guid,
                    name: nm,
                    ilvl: (relevant_ilvls.iter().sum::<i32>() as f32 / relevant_ilvls.len() as f32).floor() as i32,
                    spec_id: x.spec_id,
                    team: x.team,
                }
            }).collect()          
        )
    }

    async fn list_wow_characters_for_user(&self, user_id: i64) -> Result<Vec<WoWCharacter>, SquadOvError> {
        // There's three queries here:
        // 1) Query what characters the user owns in the wow_user_character_association table.
        // 2) Determine that character's latest name by finding the latest name in the combat logs.
        // 3) Determine that character's latest spec ID and ilvl.
        // TODO: I have a feeling that these two queries can be trivially merged together with no performance loss.
        let owned_characters = sqlx::query!(
            r#"
            SELECT *
            FROM squadov.wow_user_character_association
            WHERE user_id = $1
            "#,
            user_id,
        )
            .fetch_all(&*self.pool)
            .await?;

        let guids: Vec<String> = owned_characters.iter().map(|x| { x.guid.clone() }).collect();

        // Unlike the similar query in list_wow_characters_for_match, we need to filter for the combat log events
        // based on time. However, it's expensive to sort based on combat log events time (lots of rows). So instead, we
        // assume that the user's name doesn't change within each combat log and order based combat log time. Note that this
        // query also handles retrieving spec ID and ilvl because it can for cheap.
        // NOTE: If there ever comes a time where this query becomes too expensive we should note that wce.source->>'guid' and wce.evt->>'guid' dont have indices on them.
        Ok(
            sqlx::query!(
                r#"
                SELECT
                    pl.guid AS "guid!",
                    t1.name AS "name",
                    (wce.evt->>'spec_id')::INTEGER AS "spec_id!",
                    jsonb_path_query_array(
                        (wce.evt->>'items')::jsonb,
                        '$[*].ilvl'
                    ) AS "items!"
                FROM UNNEST($1::VARCHAR[]) AS pl(guid)
                CROSS JOIN LATERAL (
                    SELECT wcl.uuid
                    FROM squadov.wow_combat_logs AS wcl
                    INNER JOIN squadov.wow_combat_log_character_presence AS wclcp
                        ON wclcp.combat_log_uuid = wcl.uuid
                    WHERE wclcp.guid = pl.guid
                        AND wcl.user_id = $2
                    ORDER BY wcl.tm DESC
                    LIMIT 1
                ) AS cl(id)
                CROSS JOIN LATERAL (
                    SELECT wce.source->>'name'
                    FROM squadov.wow_combat_log_events AS wce
                    INNER JOIN squadov.wow_combat_logs AS wcl
                        ON wcl.uuid = wce.combat_log_uuid
                    WHERE wcl.user_id = $2
                        AND wcl.uuid = cl.id
                        AND wce.source->>'guid' = pl.guid
                    LIMIT 1
                ) AS t1(name)
                CROSS JOIN LATERAL (
                    SELECT wce.*
                    FROM squadov.wow_combat_log_events AS wce
                    WHERE wce.combat_log_uuid = cl.id
                        AND wce.evt->>'type' = 'CombatantInfo'
                        AND wce.evt->>'guid' = pl.guid
                    LIMIT 1
                ) AS wce
                "#,
                &guids,
                user_id,
            )
                .fetch_all(&*self.heavy_pool)
                .await?
                .into_iter()
                .map(|x| {
                    let relevant_ilvls: Vec<i32> = serde_json::from_value::<Vec<i32>>(x.items)
                        .unwrap_or(vec![])
                        .into_iter()
                        .filter(|x| { *x > 0 })
                        .collect();

                    WoWCharacter {
                        guid: x.guid,
                        name: x.name.unwrap_or(String::from("<Unknown>")),
                        ilvl: (relevant_ilvls.iter().sum::<i32>() as f32 / relevant_ilvls.len() as f32).floor() as i32,
                        spec_id: x.spec_id,
                        team: 0,
                    }
                })
                .collect()
        )
    }

    async fn list_wow_characters_association_for_squad_in_match(&self, match_uuid: &Uuid, user_id: i64) -> Result<Vec<WoWCharacterUserAssociation>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                WoWCharacterUserAssociation,
                r#"
                SELECT DISTINCT wuca.*
                FROM squadov.wow_user_character_association AS wuca
                INNER JOIN squadov.wow_match_combatants AS wmc
                    ON wmc.combatant_guid = wuca.guid
                LEFT JOIN squadov.squad_role_assignments AS sra
                    ON sra.user_id = wuca.user_id
                LEFT JOIN squadov.squad_role_assignments AS ora
                    ON ora.squad_id = sra.squad_id
                WHERE wmc.match_uuid = $1 
                    AND (wuca.user_id = $2 OR ora.user_id = $2)
                "#,
                match_uuid,
                user_id,
            )
                .fetch_all(&*self.pool)
                .await?
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

pub async fn list_wow_characters_association_for_squad_in_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>) -> Result<HttpResponse, SquadOvError> {
    let chars = app.list_wow_characters_association_for_squad_in_match(&path.match_uuid, path.user_id).await?;
    Ok(HttpResponse::Ok().json(chars))
}