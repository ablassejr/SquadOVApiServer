use crate::{
    SquadOvError,
    matches::MatchPlayerPair,
};
use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};
use num_enum::TryFromPrimitive;
use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use std::convert::TryFrom;
use uuid::Uuid;
use super::WoWCombatLogState;

#[derive(Deserialize)]
pub struct WoWEncounterStart {
    #[serde(rename="encounterId")]
    pub encounter_id: i32,
    #[serde(rename="encounterName")]
    pub encounter_name: String,
    pub difficulty: i32,
    #[serde(rename="numPlayers")]
    pub num_players: i32,
    #[serde(rename="instanceId")]
    pub instance_id: i32
}

#[derive(Deserialize)]
pub struct WoWEncounterEnd {
    #[serde(rename="encounterId")]
    pub encounter_id: i32,
    #[serde(rename="encounterName")]
    pub encounter_name: String,
    pub difficulty: i32,
    #[serde(rename="numPlayers")]
    pub num_players: i32,
    pub success: bool
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WowBossStatus {
    pub name: Option<String>,
    pub npc_id: Option<i64>,
    pub current_hp: Option<i64>,
    pub max_hp: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WoWEncounter {
    pub match_uuid: Uuid,
    pub tm: DateTime<Utc>,
    pub combatants_key: String,
    pub encounter_id: i32,
    pub encounter_name: String,
    pub difficulty: i32,
    pub num_players: i32,
    pub instance_id: i32,
    pub finish_time: Option<DateTime<Utc>>,
    pub success: bool,
    pub user_uuid: Uuid,
    pub build: String,
    pub boss: Vec<WowBossStatus>,
    pub pull_number: Option<i64>,
}

#[derive(Deserialize)]
pub struct WoWChallengeStart {
    #[serde(rename="challengeName")]
    pub challenge_name: String,
    #[serde(rename="instanceId")]
    pub instance_id: i32,
    #[serde(rename="keystoneLevel")]
    pub keystone_level: i32,
}

#[derive(Deserialize)]
pub struct WoWChallengeEnd {
    #[serde(rename="instanceId")]
    pub instance_id: i32,
    #[serde(rename="keystoneLevel")]
    pub keystone_level: i32,
    pub success: bool,
    #[serde(rename="timeMs")]
    pub time_ms: i64
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WoWChallenge {
    #[serde(rename="matchUuid")]
    pub match_uuid: Uuid,
    pub tm: DateTime<Utc>,
    #[serde(rename="combatantsKey")]
    pub combatants_key: String,
    #[serde(rename="challengeName")]
    pub challenge_name: String,
    #[serde(rename="instanceId")]
    pub instance_id: i32,
    #[serde(rename="keystoneLevel")]
    pub keystone_level: i32,
    #[serde(rename="finishTime")]
    pub finish_time: Option<DateTime<Utc>>,
    #[serde(rename="timeMs")]
    pub time_ms: i64,
    pub success: bool,
    #[serde(rename="userUuid")]
    pub user_uuid: Uuid,
    pub build: String
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WoWArenaStart {
    pub instance_id: i32,
    #[serde(rename="type")]
    pub arena_type: String,
    pub local_team_id: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WoWArenaEnd {
    pub winning_team_id: i32,
    pub match_duration_seconds: i32,
    pub new_ratings: Vec<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WoWArena {
    pub match_uuid: Uuid,
    pub tm: DateTime<Utc>,
    pub combatants_key: String,
    pub instance_id: i32,
    #[serde(rename="type")]
    pub arena_type: String,
    pub finish_time: Option<DateTime<Utc>>,
    pub winning_team_id: Option<i32>,
    pub match_duration_seconds: Option<i32>,
    pub new_ratings: Option<Vec<i32>>,
    pub user_uuid: Uuid,
    pub success: bool,
    pub build: String
}

#[derive(Copy, Clone, Serialize_repr, Deserialize_repr, Debug, TryFromPrimitive, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WowInstanceType {
    NotInstanced,
    PartyDungeon,
    RaidDungeon,
    PVPBattlefield,
    ArenaBattlefield,
    Scenario,
    Unknown
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WowInstanceData {
    pub id: i64,
    pub name: String,
    pub expansion: i64,
    pub loading_screen_id: i64,
    pub instance_type: WowInstanceType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WowInstance {
    pub match_uuid: Uuid,
    pub tm: DateTime<Utc>,
    pub combatants_key: String,
    pub instance_id: i32,
    pub finish_time: Option<DateTime<Utc>>,
    pub success: bool,
    pub build: String,
    pub instance_type: WowInstanceType,
    pub user_uuid: Uuid,
}

#[derive(Clone)]
pub struct GenericWoWMatchView {
    pub id: Uuid,
    pub alt_id: i64,
    pub user_id: i64,
    pub combat_log_version: String,
    pub advanced_log: bool,
    pub build_version: String,
    pub match_uuid: Option<Uuid>,
    pub combat_log_partition_id: Option<String>,
    pub start_tm: DateTime<Utc>,
    pub end_tm: Option<DateTime<Utc>>,
}

pub async fn get_generic_wow_match_view_from_match_user<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<GenericWoWMatchView, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            GenericWoWMatchView,
            "
            SELECT id, alt_id, user_id, combat_log_version, advanced_log, build_version, match_uuid, combat_log_partition_id, start_tm, end_tm
            FROM squadov.wow_match_view
            WHERE match_uuid = $1 AND user_id = $2
            ",
            match_uuid,
            user_id,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_generic_wow_match_view_from_id<'a, T>(ex: T, id: &Uuid) -> Result<GenericWoWMatchView, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            GenericWoWMatchView,
            "
            SELECT id, alt_id, user_id, combat_log_version, advanced_log, build_version, match_uuid, combat_log_partition_id, start_tm, end_tm
            FROM squadov.wow_match_view
            WHERE id = $1
            ",
            id,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_generic_wow_match_view_from_combat_log_id<'a, T>(ex: T, id: &str) -> Result<GenericWoWMatchView, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            GenericWoWMatchView,
            "
            SELECT id, alt_id, user_id, combat_log_version, advanced_log, build_version, match_uuid, combat_log_partition_id, start_tm, end_tm
            FROM squadov.wow_match_view
            WHERE combat_log_partition_id = $1
            ",
            id,
        )
            .fetch_one(ex)
            .await?
    )
}

impl GenericWoWMatchView {
    pub fn combat_log_state(&self) -> WoWCombatLogState {
        WoWCombatLogState {
            combat_log_version: self.combat_log_version.clone(),
            advanced_log: self.advanced_log,
            build_version: self.build_version.clone(),
        }
    }
}

pub async fn filter_valid_wow_match_player_pairs<'a, T>(ex: T, uuids: &[MatchPlayerPair]) -> Result<(Vec<Uuid>, Vec<i64>), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let match_uuids = uuids.iter().map(|x| { x.match_uuid.clone() }).collect::<Vec<Uuid>>();
    let player_uuids = uuids.iter().map(|x| { x.player_uuid.clone() }).collect::<Vec<Uuid>>();
    
    let final_identifiers = sqlx::query!(
        r#"
        SELECT
            inp.match_uuid AS "match_uuid!",
            u.id AS "user_id!"
        FROM UNNEST($1::UUID[], $2::UUID[]) AS inp(match_uuid, player_uuid)
        INNER JOIN squadov.users AS u
            ON u.uuid = inp.player_uuid
        INNER JOIN squadov.wow_match_view AS wmv
            ON wmv.user_id = u.id
                AND wmv.match_uuid = inp.match_uuid
        "#,
        &match_uuids,
        &player_uuids,
    )
        .fetch_all(ex)
        .await?
        .into_iter()
        .map(|x| {
            (x.match_uuid, x.user_id)
        })
        .collect::<Vec<(Uuid, i64)>>();
    
    let match_uuids = final_identifiers.iter().map(|x| { x.0.clone() }).collect::<Vec<Uuid>>();
    let player_ids = final_identifiers.iter().map(|x| { x.1 }).collect::<Vec<i64>>();
    Ok((match_uuids, player_ids))
}

pub async fn list_wow_encounter_for_uuids<'a, T>(ex: T, uuids: &[MatchPlayerPair]) -> Result<Vec<WoWEncounter>, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    let (match_uuids, user_ids) = filter_valid_wow_match_player_pairs(ex, uuids).await?;

    Ok(
        sqlx::query!(
            r#"
            SELECT *
            FROM (
                SELECT DISTINCT ON (wmv.match_uuid, u.uuid)
                    wmv.match_uuid AS "match_uuid!",
                    wmv.start_tm AS "tm!",
                    wmv.end_tm AS "finish_time", 
                    wmv.build_version AS "build!",
                    u.uuid AS "user_uuid!",
                    wa.combatants_key,
                    wav.encounter_id,
                    wav.encounter_name,
                    wav.difficulty,
                    wav.num_players,
                    wav.instance_id,
                    COALESCE(wav.success, FALSE) AS "success!",
                    MAX(mmc.match_order) AS "pull_number"
                FROM UNNEST($1::UUID[], $2::BIGINT[]) AS inp(match_uuid, user_id)
                INNER JOIN squadov.wow_match_view AS wmv
                    ON wmv.match_uuid = inp.match_uuid
                        AND wmv.user_id = inp.user_id
                INNER JOIN squadov.new_wow_encounters AS wa
                    ON wa.match_uuid = wmv.match_uuid
                INNER JOIN squadov.wow_encounter_view AS wav
                    ON wav.view_id = wmv.id
                INNER JOIN squadov.users AS u
                    ON u.id = wmv.user_id
                LEFT JOIN squadov.match_to_match_collection AS mmc
                    ON mmc.match_uuid = inp.match_uuid
                GROUP BY
                    wmv.match_uuid,
                    wmv.start_tm,
                    wmv.end_tm,
                    wmv.build_version,
                    u.uuid,
                    wa.combatants_key,
                    wav.encounter_id,
                    wav.encounter_name,
                    wav.difficulty,
                    wav.num_players,
                    wav.instance_id,
                    wav.success
                ORDER BY wmv.match_uuid, u.uuid
            ) AS t
            ORDER BY finish_time DESC
            "#,
            &match_uuids,
            &user_ids,
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                WoWEncounter {
                    match_uuid: x.match_uuid,
                    tm: x.tm,
                    combatants_key: x.combatants_key,
                    encounter_id: x.encounter_id,
                    encounter_name: x.encounter_name,
                    difficulty: x.difficulty,
                    num_players: x.num_players,
                    instance_id: x.instance_id,
                    finish_time: x.finish_time,
                    success: x.success,
                    user_uuid: x.user_uuid,
                    build: x.build,
                    boss: vec![],
                    pull_number: x.pull_number,
                }
            })
            .collect::<Vec<WoWEncounter>>()
    )
}

pub async fn list_wow_challenges_for_uuids<'a, T>(ex: T, uuids: &[MatchPlayerPair]) -> Result<Vec<WoWChallenge>, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    let (match_uuids, user_ids) = filter_valid_wow_match_player_pairs(ex, uuids).await?;

    Ok(
        sqlx::query_as!(
            WoWChallenge,
            r#"
            SELECT * FROM (
                SELECT DISTINCT ON (wmv.match_uuid, u.uuid)
                    wmv.match_uuid AS "match_uuid!",
                    wmv.start_tm AS "tm!",
                    wmv.end_tm AS "finish_time", 
                    wmv.build_version AS "build!",
                    u.uuid AS "user_uuid!",
                    wa.combatants_key,
                    wav.challenge_name,
                    wav.instance_id,
                    wav.keystone_level,
                    COALESCE(wav.time_ms, 0) AS "time_ms!",
                    COALESCE(wav.success, FALSE) AS "success!"
                FROM UNNEST($1::UUID[], $2::BIGINT[]) AS inp(match_uuid, user_id)
                INNER JOIN squadov.wow_match_view AS wmv
                    ON wmv.match_uuid = inp.match_uuid
                        AND wmv.user_id = inp.user_id
                INNER JOIN squadov.new_wow_challenges AS wa
                    ON wa.match_uuid = wmv.match_uuid
                INNER JOIN squadov.wow_challenge_view AS wav
                    ON wav.view_id = wmv.id
                INNER JOIN squadov.users AS u
                    ON u.id = wmv.user_id
                ORDER BY wmv.match_uuid, u.uuid
            ) AS t
            ORDER BY finish_time DESC
            "#,
            &match_uuids,
            &user_ids,
        )
            .fetch_all(ex)
            .await?
    )
}

pub async fn list_wow_arenas_for_uuids<'a, T>(ex: T, uuids: &[MatchPlayerPair]) -> Result<Vec<WoWArena>, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    let (match_uuids, user_ids) = filter_valid_wow_match_player_pairs(ex, uuids).await?;

    Ok(
        sqlx::query_as!(
            WoWArena,
            r#"
            SELECT * FROM (
                SELECT DISTINCT ON (wmv.match_uuid, u.uuid)
                    wmv.match_uuid AS "match_uuid!",
                    wmv.start_tm AS "tm!",
                    wmv.end_tm AS "finish_time", 
                    wmv.build_version AS "build!",
                    wa.combatants_key,
                    wav.instance_id,
                    wav.arena_type,
                    wav.winning_team_id,
                    wav.match_duration_seconds,
                    wav.new_ratings,
                    u.uuid AS "user_uuid",
                    FALSE AS "success!"
                FROM UNNEST($1::UUID[], $2::BIGINT[]) AS inp(match_uuid, user_id)
                INNER JOIN squadov.wow_match_view AS wmv
                    ON wmv.match_uuid = inp.match_uuid
                        AND wmv.user_id = inp.user_id
                INNER JOIN squadov.new_wow_arenas AS wa
                    ON wa.match_uuid = wmv.match_uuid
                INNER JOIN squadov.wow_arena_view AS wav
                    ON wav.view_id = wmv.id
                INNER JOIN squadov.users AS u
                    ON u.id = wmv.user_id
                ORDER BY wmv.match_uuid, u.uuid
            ) AS t
            ORDER BY finish_time DESC
            "#,
            &match_uuids,
            &user_ids,
        )
            .fetch_all(ex)
            .await?
    )
}

pub async fn list_wow_instances_for_uuids<'a, T>(ex: T, uuids: &[MatchPlayerPair]) -> Result<Vec<WowInstance>, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    let (match_uuids, user_ids) = filter_valid_wow_match_player_pairs(ex, uuids).await?;

    Ok(
        sqlx::query!(
            r#"
            SELECT * FROM (
                SELECT DISTINCT ON (wmv.match_uuid, wmv.user_id)
                    wmv.match_uuid AS "match_uuid!",
                    wmv.start_tm AS "tm!",
                    wmv.end_tm AS "finish_time", 
                    wmv.build_version AS "build!",
                    '' AS "combatants_key!",
                    FALSE AS "success!",
                    nwi.instance_id,
                    nwi.instance_type,
                    u.uuid AS "user_uuid!"
                FROM UNNEST($1::UUID[], $2::BIGINT[]) AS inp(match_uuid, user_id)
                INNER JOIN squadov.wow_match_view AS wmv
                    ON wmv.match_uuid = inp.match_uuid
                        AND wmv.user_id = inp.user_id
                INNER JOIN squadov.new_wow_instances AS nwi
                    ON nwi.match_uuid = wmv.match_uuid
                INNER JOIN squadov.users AS u
                    ON u.id = inp.user_id
                ORDER BY wmv.match_uuid, wmv.user_id
            ) AS t
            ORDER BY finish_time DESC
            "#,
            &match_uuids,
            &user_ids,
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                Ok(WowInstance{
                    match_uuid: x.match_uuid,
                    tm: x.tm,
                    finish_time: x.finish_time,
                    build: x.build,
                    combatants_key: x.combatants_key,
                    success: x.success,
                    instance_id: x.instance_id,
                    instance_type: WowInstanceType::try_from(x.instance_type)?,
                    user_uuid: x.user_uuid,
                })
            })
            .collect::<Result<Vec<WowInstance>, SquadOvError>>()?
    )
}