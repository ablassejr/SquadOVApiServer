use serde::{Deserialize, Serialize};
use sqlx::{Executor, Postgres};
use crate::{
    SquadOvError,
    SquadOvWowRelease,
    games,
    reports::characters::{
        WowCombatantReport,
    },
    wow::matches::WowBossStatus,
};

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

impl From<WowCombatantReport> for WoWCharacter {
    fn from(x: WowCombatantReport) -> Self {
        Self {
            guid: x.unit_guid,
            name: x.unit_name,
            ilvl: x.ilvl,
            spec_id: x.spec_id,
            team: x.team,
            rating: x.rating,
            class_id: x.class_id,
        }
    }
}

impl From<WoWCharacter> for WowCombatantReport {
    fn from(x: WoWCharacter) -> Self {
        Self {
            unit_guid: x.guid,
            unit_name: x.name,
            ilvl: x.ilvl,
            spec_id: x.spec_id,
            team: x.team,
            rating: x.rating,
            class_id: x.class_id,
        }
    }
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

pub async fn list_wow_encounter_bosses<'a, T>(ex: T, encounter_id: i64) -> Result<Vec<WowBossStatus>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT npc_id, name
            FROM squadov.wow_encounter_bosses
            WHERE encounter_id = $1
            ",
            encounter_id
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                WowBossStatus{
                    name: Some(x.name),
                    npc_id: Some(x.npc_id),
                    current_hp: None,
                    max_hp: None,
                }
            })
            .collect()
    )
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