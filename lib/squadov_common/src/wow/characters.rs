use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::{Executor, Postgres};
use crate::{SquadOvError, SquadOvWowRelease, games};

#[derive(Deserialize, Serialize, Debug)]
pub struct WoWCharacter {
    pub guid: String,
    pub name: String,
    pub ilvl: i32,
    #[serde(rename="specId")]
    pub spec_id: i32,
    pub team: i32,
    pub rating: i32,
    #[serde(rename="classId")]
    pub class_id: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct WoWCharacterUserAssociation {
    #[serde(rename="userId")]
    pub user_id: i64,
    pub guid: String
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowItem {
    pub item_id: i64,
    pub ilvl: i32,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowCovenant {
    pub covenant_id: i32,
    pub soulbind_id: i32,
    pub soulbind_traits: Vec<i32>,
    pub conduits: Vec<WowItem>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowFullCharacter {
    pub items: Vec<WowItem>,
    pub covenant: Option<WowCovenant>,
    pub talents: Vec<i32>,
    pub pvp_talents: Vec<i32>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowCharacterWrapper {
    pub data: WoWCharacter,
    pub traits: WowFullCharacter,
}

pub fn compute_wow_character_ilvl(items: &[i32]) -> i32 {
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

pub async fn list_wow_characters_for_user<'a, T>(ex: T, user_id: i64, release: Option<SquadOvWowRelease>) -> Result<Vec<WoWCharacter>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    // We can afford to only list combatant info-validated here as we expect the issue where combatant info doesn't show up
    // to be a rare occurence.
    Ok(
        sqlx::query!(
            r#"
            SELECT
                wucc.unit_guid AS "guid",
                COALESCE(wucc.unit_name, '') AS "name!",
                COALESCE(wucc.items, ARRAY[]::INTEGER[]) AS "items!",
                COALESCE(wucc.spec_id, -1) AS "spec_id!",
                wucc.class_id
            FROM squadov.wow_user_character_cache AS wucc
            WHERE wucc.user_id = $1
                AND wucc.build_version SIMILAR TO $2::VARCHAR
            "#,
            user_id,
            release.map(|x| {
                games::wow_release_to_db_build_expression(x)
            }).unwrap_or("%"),
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                WoWCharacter {
                    guid: x.guid,
                    name: x.name,
                    ilvl: compute_wow_character_ilvl(&x.items),
                    spec_id: x.spec_id,
                    team: 0,
                    rating: 0,
                    class_id: x.class_id.map(|x| { x as i64 } ),
                }
            })
            .collect()
    )
}

pub async fn list_wow_characters_for_match<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<Vec<WoWCharacter>, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    // So for this it's gonna be slightly tricky as we won't necessarily have combatant info in this match since WoW can be f'd up sometimes.
    // In that case, we want to fallback to trying to find the latest combatant info. So to do this all efficiently, we just look up the guids
    // of players w/ combatant info in this match. If that's empty, then we assume the worst, and look for players with the player flag set.
    let combatant_guids = sqlx::query!(
        "
        SELECT wcp.unit_guid
        FROM squadov.wow_match_view AS wmv
        INNER JOIN squadov.wow_match_view_character_presence AS wcp
            ON wcp.view_id = wmv.id
        WHERE wmv.match_uuid = $1
            AND wmv.user_id = $2
            AND wcp.has_combatant_info
        ",
        match_uuid,
        user_id,
    )
        .fetch_all(ex)
        .await?;

    if combatant_guids.is_empty() {
        // If combatant guids don't exist, then combatant info does not exist in this match and we thus
        // will need to fill in filler data for characters. Note that we don't want to send people with
        // null class ids back to the client since we can't say for certain whether or not that user is
        // actually in that match or the combat log picked them up by accident.
        Ok(
            sqlx::query!(
                r#"
                SELECT
                    wcp.unit_guid,
                    COALESCE(wcp.unit_name, '') AS "unit_name!",
                    wcp.class_id
                FROM squadov.wow_match_view AS wmv
                INNER JOIN squadov.wow_match_view_character_presence AS wcp
                    ON wcp.view_id = wmv.id
                WHERE wmv.match_uuid = $1
                    AND wmv.user_id = $2
                    AND (wcp.flags & x'100'::BIGINT) > 0
                    AND (wcp.flags & x'400'::BIGINT) > 0
                    AND wcp.class_id IS NOT NULL
                "#,
                match_uuid,
                user_id
            )
                .fetch_all(ex)
                .await?
                .into_iter()
                .map(|x| {
                    WoWCharacter {
                        guid: x.unit_guid,
                        name: x.unit_name,
                        ilvl: -1,
                        spec_id: -1,
                        team: -1,
                        rating: -1,
                        class_id: x.class_id.map(|x| { x as i64 }),
                    }
                })
                .collect()
        )
    } else {
        // If combatant guids exist, then we know we can look up combatant info in this match.
        // However, in the case of keystones, there could be multiple combatant info logs within the
        // same match. In that case we'll just take the first one since we'll just assume they're
        // all the same.
        Ok(
            sqlx::query!(
                r#"
                SELECT DISTINCT ON (wcp.unit_guid)
                    wcp.unit_guid AS "guid",
                    COALESCE(wcp.unit_name, '') AS "name!",
                    COALESCE(ARRAY_AGG(wci.ilvl ORDER BY wci.idx ASC), ARRAY[]::INTEGER[]) AS "items!",
                    wvc.spec_id,
                    wvc.team,
                    wvc.rating,
                    wvc.class_id
                FROM squadov.wow_match_view AS wmv
                INNER JOIN squadov.wow_match_view_character_presence AS wcp
                    ON wcp.view_id = wmv.id
                        AND wcp.has_combatant_info
                INNER JOIN squadov.wow_match_view_combatants AS wvc
                    ON wvc.character_id = wcp.character_id
                LEFT JOIN squadov.wow_match_view_combatant_items AS wci
                    ON wci.event_id = wvc.event_id
                WHERE wmv.match_uuid = $1
                    AND wmv.user_id = $2
                GROUP BY wcp.unit_guid, wcp.unit_name, wvc.spec_id, wvc.team, wvc.event_id, wvc.rating, wvc.class_id
                ORDER BY wcp.unit_guid, wvc.class_id
                "#,
                match_uuid,
                user_id
            )
                .fetch_all(ex)
                .await?
                .into_iter()
                .map(|x| {
                    WoWCharacter {
                        guid: x.guid,
                        name: x.name,
                        ilvl: compute_wow_character_ilvl(&x.items),
                        spec_id: x.spec_id,
                        team: x.team,
                        rating: x.rating,
                        class_id: x.class_id,
                    }
                })
                .collect()
        )
    }
}

pub async fn get_wow_character_covenant<'a, T>(ex: T,  view_uuid: &Uuid, guid: &str) -> Result<Option<WowCovenant>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT DISTINCT
                wcc.covenant_id,
                wcc.soulbind_id,
                wcc.soulbind_traits,
                wcc.conduit_item_ids,
                wcc.conduit_item_ilvls
            FROM squadov.wow_match_view_character_presence AS wcp
            INNER JOIN squadov.wow_match_view_combatant_covenants AS wcc
                ON wcc.character_id = wcp.character_id
            WHERE wcp.view_id = $1
                AND wcp.unit_guid = $2
            ",
            view_uuid,
            guid
        )
            .fetch_optional(ex)
            .await?
            .map(|x| {
                WowCovenant {
                    covenant_id: x.covenant_id,
                    soulbind_id: x.soulbind_id,
                    soulbind_traits: x.soulbind_traits,
                    conduits: x.conduit_item_ids.iter().zip(x.conduit_item_ilvls.iter()).map(|(item_id, ilvl)| {
                        WowItem{
                            item_id: *item_id,
                            ilvl: *ilvl,
                        }
                    }).collect(),
                }
            })
    )
}

pub async fn get_wow_full_character<'a, T>(ex: T, view_uuid: &Uuid, guid: &str) -> Result<WowFullCharacter, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    let items: Vec<WowItem> = sqlx::query_as!(
        WowItem,
        r#"
        SELECT
            MAX(wci.item_id) AS "item_id!",
            MAX(wci.ilvl) AS "ilvl!"
        FROM squadov.wow_match_view_character_presence AS wcp
        LEFT JOIN squadov.wow_match_view_combatant_items AS wci
            ON wci.character_id = wcp.character_id
        WHERE wcp.view_id = $1
            AND wcp.unit_guid = $2
        GROUP BY wcp.unit_guid, wci.idx
        ORDER BY wci.idx ASC
        "#,
        view_uuid,
        guid,
    )
        .fetch_all(ex)
        .await?;

    let talents = sqlx::query!(
        r#"
        SELECT DISTINCT wct.talent_id, wct.is_pvp
        FROM squadov.wow_match_view_character_presence AS wcp
        INNER JOIN squadov.wow_match_view_combatant_talents AS wct
            ON wct.character_id = wcp.character_id
        WHERE wcp.view_id = $1
            AND wcp.unit_guid = $2
        "#,
        view_uuid,
        guid,
    )
        .fetch_all(ex)
        .await?;
    
    Ok(WowFullCharacter {
        items,
        covenant: get_wow_character_covenant(ex, view_uuid, guid).await?,
        talents: talents.iter().filter(|x| { !x.is_pvp }).map(|x| { x.talent_id }).collect(),
        pvp_talents: talents.iter().filter(|x| { x.is_pvp }).map(|x| { x.talent_id }).collect(),
    })
}