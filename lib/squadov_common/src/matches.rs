use uuid::Uuid;
use sqlx::{Executor, Postgres};
use crate::{
    SquadOvError,
    games::SquadOvGames,
    riot::games::{
        LolPlayerMatchSummary,
        TftPlayerMatchSummary,
        ValorantPlayerMatchSummary,
    },
    aimlab::AimlabTask,
    vod::VodManifest,
    wow::{
        WoWEncounter,
        WoWChallenge,
    },
};
use chrono::{DateTime, Utc};
use serde::Serialize;

pub struct MatchPlayerPair {
    pub match_uuid: Uuid,
    pub player_uuid: Uuid,
}

pub async fn create_new_match<'a, T>(ex: T) -> Result<Uuid, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let uuid = Uuid::new_v4();
    sqlx::query!(
        "
        INSERT INTO squadov.matches (uuid)
        VALUES ($1)
        ",
        &uuid,
    )
        .execute(ex)
        .await?;

    Ok(uuid)
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct BaseRecentMatch {
    pub match_uuid: Uuid,
    pub tm: DateTime<Utc>,
    pub game: SquadOvGames,
    pub vod: VodManifest,
    pub username: String,
    pub user_id: i64,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct RecentMatch {
    pub base: BaseRecentMatch,

    pub aimlab_task: Option<AimlabTask>,
    pub lol_match: Option<LolPlayerMatchSummary>,
    pub tft_match: Option<TftPlayerMatchSummary>,
    pub valorant_match: Option<ValorantPlayerMatchSummary>,
    pub wow_challenge: Option<WoWChallenge>,
    pub wow_encounter: Option<WoWEncounter>,
}