use crate::{
    SquadOvError,
    rabbitmq::{RABBITMQ_DEFAULT_PRIORITY, RABBITMQ_HIGH_PRIORITY},
    riot::{
        db,
        games::{
            LolMatchlistDto,
            LolMatchReferenceDto,
            LOL_SHORTHAND,
        }
    },
};
use super::RiotApiTask;
use chrono::{Utc, Duration};
use reqwest::{StatusCode};

impl super::RiotApiHandler {
    pub async fn get_lol_matches_for_user(&self, account_id: &str, platform: &str, begin_index: i32, end_index: i32) -> Result<Vec<LolMatchReferenceDto>, SquadOvError> {
        let client = self.create_http_client()?;
        let endpoint = Self::build_api_endpoint(&platform.to_lowercase(), &format!("lol/match/v4/matchlists/by-account/{}?beginIndex={}&endIndex={}", account_id, begin_index, end_index));
        self.tick_thresholds().await;

        let resp = client.get(&endpoint)
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::InternalError(format!("Failed to obtain LOL matches for user {} - {}", resp.status().as_u16(), resp.text().await?)));
        }

        Ok(resp.json::<LolMatchlistDto>().await?.matches)
    }
}

const LOL_BACKFILL_AMOUNT: i32 = 100;

impl super::RiotApiApplicationInterface {
    pub async fn request_obtain_lol_match_info(&self, platform: &str, game_id: i64, priority: bool) -> Result<(), SquadOvError> {
        let priority = if priority {
            RABBITMQ_HIGH_PRIORITY
        } else {
            RABBITMQ_DEFAULT_PRIORITY
        };

        self.rmq.publish(&self.queue, serde_json::to_vec(&RiotApiTask::LolMatch{
            platform: String::from(platform),
            game_id,
        })?, priority).await;
        Ok(())
    }

    pub async fn obtain_lol_match_info(&self, platform: &str, game_id: i64) -> Result<(), SquadOvError> {
        Ok(())
    }

    pub async fn request_backfill_user_lol_matches(&self, summoner_name: &str, platform: &str, user_id: i64) -> Result<(), SquadOvError> {
        let summoner = db::get_user_riot_summoner_from_name(&*self.db, user_id, summoner_name, &self.game).await?;
        if summoner.last_backfill_time.is_some() {
            let time_since_backfill = Utc::now() - summoner.last_backfill_time.unwrap();
            if time_since_backfill > Duration::days(3) {
                return Ok(());
            }
        }

        let account_id = summoner.account_id.as_ref().ok_or(SquadOvError::NotFound)?.clone();
        self.rmq.publish(&self.queue, serde_json::to_vec(&RiotApiTask::LolBackfill{
            account_id,
            platform: String::from(platform),
        })?, RABBITMQ_DEFAULT_PRIORITY).await;
        Ok(())
    }

    pub async fn backfill_user_lol_matches(&self, account_id: &str, platform: &str) -> Result<(), SquadOvError> {
        let matches = self.api.get_lol_matches_for_user(account_id, platform, 0, LOL_BACKFILL_AMOUNT).await?;
        db::tick_riot_account_backfill_time(&*self.db, account_id, LOL_SHORTHAND).await?;

        let backfill_matches = db::get_lol_matches_that_require_backfill(&*self.db, &matches).await?;
        for bm in &backfill_matches {
            self.request_obtain_lol_match_info(&bm.platform_id, bm.game_id, false).await?;
        }
        Ok(())
    }
}