use crate::{
    SquadOvError,
    rabbitmq::{RABBITMQ_DEFAULT_PRIORITY, RABBITMQ_HIGH_PRIORITY},
    riot::games::valorant::{
        ValorantMatchlistDto,
        ValorantMatchDto
    }
};
use reqwest::{StatusCode};
use super::RiotApiTask;
use crate::riot::db;

impl super::RiotApiHandler {
    pub async fn get_valorant_matches_for_user(&self, puuid: &str) -> Result<ValorantMatchlistDto, SquadOvError> {
        let client = self.create_http_client()?;
        let endpoint = Self::build_api_endpoint("na", &format!("val/match/v1/matchlists/by-puuid/{}", puuid));
        self.tick_thresholds().await;

        let resp = client.get(&endpoint)
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::InternalError(format!("Failed to obtain Valorant matches for user {} - {}", resp.status().as_u16(), resp.text().await?)));
        }

        Ok(resp.json::<ValorantMatchlistDto>().await?)
    }

    pub async fn get_valorant_match(&self, match_id: &str) -> Result<ValorantMatchDto, SquadOvError> {
        let client = self.create_http_client()?;
        let endpoint = Self::build_api_endpoint("na", &format!("val/match/v1/matches/{}", match_id));
        self.tick_thresholds().await;

        let resp = client.get(&endpoint)
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::InternalError(format!("Failed to obtain Valorant match {} - {}", resp.status().as_u16(), resp.text().await?)));
        }

        Ok(resp.json::<ValorantMatchDto>().await?)
    }
}

impl super::RiotApiApplicationInterface {
    pub async fn backfill_user_valorant_matches(&self, puuid: &str) -> Result<(), SquadOvError> {
        // Obtain a list of matches that the user played from the VALORANT API and then cross check that
        // with the matches we have stored. If the match doesn't exist then go ahead and request a low
        // priority match retrieval for that particular match.
        let api_matches = self.api.get_valorant_matches_for_user(puuid).await?;
        let match_ids: Vec<String> = api_matches.history.into_iter().map(|x| { x.match_id }).collect();
        let backfill_ids = db::get_valorant_matches_that_require_backfill(&*self.db, &match_ids).await?;
        for mid in &backfill_ids {
            self.request_obtain_valorant_match_info(&mid, false).await?;
        }
        Ok(())
    }

    pub async fn request_backfill_user_valorant_matches(&self, puuid: &str) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.queue, serde_json::to_vec(&RiotApiTask::ValorantBackfill(String::from(puuid)))?, RABBITMQ_DEFAULT_PRIORITY).await;
        Ok(())
    }

    pub async fn obtain_valorant_match_info(&self, match_id: &str) -> Result<(), SquadOvError> {
        let valorant_match = self.api.get_valorant_match(match_id).await?;
        let mut tx = self.db.begin().await?;
        db::store_valorant_match_dto(&mut tx, &valorant_match).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn request_obtain_valorant_match_info(&self, match_id: &str, priority: bool) -> Result<(), SquadOvError> {
        let priority = if priority {
            RABBITMQ_HIGH_PRIORITY
        } else {
            RABBITMQ_DEFAULT_PRIORITY
        };

        self.rmq.publish(&self.queue, serde_json::to_vec(&RiotApiTask::ValorantMatch(String::from(match_id)))?, priority).await;
        Ok(())
    }
}