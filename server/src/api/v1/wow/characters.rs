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
use serde::Deserialize;

#[derive(Deserialize)]
pub struct WowCharacterPathInput {
    character_name: String,
}

fn compute_wow_character_ilvl(items: &[i32]) -> i32 {
    let mut relevant_ilvls: Vec<i32> = items.iter().map(|x| { *x }).collect();

    if relevant_ilvls.len() == 18 {
        // We need to filter out shirts and tabards from the ilvl of the character.
        // In the case where the character has a 2-handed weapon equipped, that weapon needs to
        // count for double. Right now we have no way of determining the type of any particular item
        // so we do our best guesses on how to best filter this stuff.
        // There's 18 item slots and item index 15 is the primary weapon and index 16 is the off-hand weapon.
        // If the off-hand weapon has an ilvl of 0 then we assume that the user is using a two-handed.
        if relevant_ilvls[15] > 0 && relevant_ilvls[16] == 0 {
            relevant_ilvls[15] = relevant_ilvls[15] * 2;
        }
    }

    let relevant_ilvls: Vec<i32> = relevant_ilvls.into_iter().filter(|x| {
        *x > 1
    }).collect();
    
    (relevant_ilvls.iter().sum::<i32>() as f32 / 16.0).floor() as i32
}

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
                    .collect();

                let nm = guid_to_name.get(&x.guid).unwrap_or(&String::from("<Unknown>")).clone();
                WoWCharacter{
                    guid: x.guid,
                    name: nm,
                    ilvl: compute_wow_character_ilvl(&relevant_ilvls) as i32,
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
                        .collect();

                    WoWCharacter {
                        guid: x.guid,
                        name: x.name.unwrap_or(String::from("<Unknown>")),
                        ilvl: compute_wow_character_ilvl(&relevant_ilvls),
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

    async fn get_wow_realm_region(&self, realm: &str) -> Result<Vec<String>, SquadOvError> {
        Ok(
            // I have no idea if the incoming name is going to be the name or the slug SOOO...
            sqlx::query!(
                "
                SELECT region
                FROM squadov.wow_realms
                WHERE LOWER(name) = LOWER($1) OR LOWER(slug) = LOWER($1)
                ",
                realm,
            )
                .fetch_all(&*self.pool)
                .await?
                .into_iter()
                .map(|x| {
                    x.region
                })
                .collect()
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

pub async fn get_wow_armory_link_for_character_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<WowCharacterPathInput>) -> Result<HttpResponse, SquadOvError> {
    // Get character name for this GUID which is composed of the CHARACTER NAME-SERVER NAME.
    // Next, obtain the region for the extracted server name.
    let name_parts: Vec<&str> = path.character_name.split("-").into_iter().collect();
    if name_parts.len() != 2 {
        return Err(SquadOvError::InternalError(format!("Unexpected WoW name: {}", &path.character_name)));
    }

    let char_name = name_parts[0];
    let server_name = name_parts[1];
    let regions= app.get_wow_realm_region(&server_name).await?;

    // Finally compose the WoW armory link: 
    // https://worldofwarcraft.com/en-us/character/REGION/SERVER NAME/CHARACTER NAME
    // There's going to be multiple possible armory links for this character depending on the server name...not sure which one to choose.
    // Have client open all of them? LOL.
    Ok(HttpResponse::Ok().json(regions.into_iter().map(|r| {
        format!(
            "https://worldofwarcraft.com/en-us/character/{region}/{server}/{character}",
            region=r,
            server=server_name,
            character=char_name,
        )
    }).collect::<Vec<String>>()))
}