use crate::{
    SquadOvError,
    rabbitmq::{RABBITMQ_DEFAULT_PRIORITY, RABBITMQ_HIGH_PRIORITY},
    riot::{
        db,
        games::{
            LolMatchlistDto,
            LolMatchReferenceDto,
            LolMatchDto,
            LolMatchTimelineDto,
        }
    },
};
use super::RiotApiTask;
use chrono::{Utc, Duration};

impl super::RiotApiHandler {
    pub async fn get_lol_matches_for_user(&self, account_id: &str, platform: &str, begin_index: i32, end_index: i32) -> Result<Vec<LolMatchReferenceDto>, SquadOvError> {
        let client = self.create_http_client()?;
        let endpoint = Self::build_api_endpoint(&platform.to_lowercase(), &format!("lol/match/v4/matchlists/by-account/{}?beginIndex={}&endIndex={}", account_id, begin_index, end_index));
        self.tick_thresholds().await;

        let resp = client.get(&endpoint)
            .send()
            .await?;

        let resp = self.check_for_response_error(resp, "Failed to obtain LOL matches for user").await?;
        Ok(resp.json::<LolMatchlistDto>().await?.matches)
    }

    pub async fn get_lol_match(&self, platform: &str, game_id: i64) -> Result<LolMatchDto, SquadOvError> {
        let client = self.create_http_client()?;
        let endpoint = Self::build_api_endpoint(&platform.to_lowercase(), &format!("lol/match/v4/matches/{}", game_id));
        self.tick_thresholds().await;

        let resp = client.get(&endpoint)
            .send()
            .await?;

        let resp = self.check_for_response_error(resp, "Failed to obtain LOL match").await?;
        Ok(resp.json::<LolMatchDto>().await?)
    }

    pub async fn get_lol_match_timeline(&self, platform: &str, game_id: i64) -> Result<LolMatchTimelineDto, SquadOvError> {
        let client = self.create_http_client()?;
        let endpoint = Self::build_api_endpoint(&platform.to_lowercase(), &format!("lol/match/v4/timelines/by-match/{}", game_id));
        self.tick_thresholds().await;

        let resp = client.get(&endpoint)
            .send()
            .await?;

        let resp = self.check_for_response_error(resp, "Failed to obtain LOL match timeline").await?;
        Ok(resp.json::<LolMatchTimelineDto>().await?)
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
        log::info!("Obtain LoL Match Info {} [{}]", game_id, platform);
        if db::check_lol_match_details_exist(&*self.db, platform, game_id).await? {
            return Ok(());
        }

        // One HTTP request to get the match information and another HTTP request to obtain the timeline.
        // Note that not every match is guaranteed to have a timeline.
        let match_info = self.api.get_lol_match(platform, game_id).await?;
        let match_timeline = match self.api.get_lol_match_timeline(platform, game_id).await {
            Ok(x) => Some(x),
            Err(err) => match err {
                SquadOvError::NotFound => None,
                _ => return Err(err)
            }
        };
        
        for _i in 0..2i32 {
            let mut tx = self.db.begin().await?;
            let match_uuid = match db::create_or_get_match_uuid_for_lol_match(&mut tx, platform, game_id, None).await {
                Ok(x) => x,
                Err(err) => match err {
                    SquadOvError::Duplicate => {
                        log::warn!("Caught duplicate LoL match...retrying!");
                        tx.rollback().await?;
                        continue;
                    },
                    _ => return Err(err)
                }
            };

            match db::store_lol_match_info(&mut tx, &match_uuid, &match_info).await {
                Ok(_) => (),
                Err(err) => match err {
                    SquadOvError::Duplicate => {
                        log::warn!("Caught duplicate LoL match details...");
                        tx.rollback().await?;
                        break;
                    },
                    _ => return Err(err)
                }
            };

            if match_timeline.is_some() {
                db::store_lol_match_timeline_info(&mut tx, &match_uuid, &match_timeline.as_ref().unwrap()).await?;
            }
            tx.commit().await?;
            break;
        }

        Ok(())
    }

    pub async fn request_backfill_user_lol_matches(&self, summoner_name: &str, platform: &str, user_id: i64) -> Result<(), SquadOvError> {
        let summoner = db::get_user_riot_summoner_from_name(&*self.db, user_id, summoner_name).await?;
        if summoner.last_backfill_lol_time.is_some() {
            let time_since_backfill = Utc::now() - summoner.last_backfill_lol_time.unwrap();
            if time_since_backfill < Duration::days(3) {
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
        log::info!("Backfill LoL Matches {} [{}]", account_id, platform);
        let matches = self.api.get_lol_matches_for_user(account_id, platform, 0, LOL_BACKFILL_AMOUNT).await?;
        db::tick_riot_account_lol_backfill_time(&*self.db, account_id).await?;

        let backfill_matches = db::get_lol_matches_that_require_backfill(&*self.db, &matches).await?;
        for bm in &backfill_matches {
            self.request_obtain_lol_match_info(&bm.platform_id, bm.game_id, false).await?;
        }
        Ok(())
    }
}