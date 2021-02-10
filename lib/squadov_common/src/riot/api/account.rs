use crate::{
    SquadOvError,
    rabbitmq::{RABBITMQ_DEFAULT_PRIORITY},
    riot::RiotAccount
};
use super::RiotApiTask;
use reqwest::{StatusCode};
use crate::riot::db;
use serde::Deserialize;
use chrono::{DateTime, Utc};

impl super::RiotApiHandler {
    pub async fn get_account_by_puuid(&self, puuid: &str) -> Result<RiotAccount, SquadOvError> {
        let client = self.create_http_client()?;
        let endpoint = Self::build_api_endpoint("americas", &format!("riot/account/v1/accounts/by-puuid/{}", puuid));
        self.tick_thresholds().await;

        let resp = client.get(&endpoint)
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::InternalError(format!("Failed to obtain Riot acount by PUUID {} - {}", resp.status().as_u16(), resp.text().await?)));
        }

        Ok(resp.json::<RiotAccount>().await?)
    }

    pub async fn get_active_shard_by_game_for_puuid(&self, game: &str, puuid: &str) -> Result<String, SquadOvError> {
        let client = self.create_http_client()?;
        let endpoint = Self::build_api_endpoint("americas", &format!("riot/account/v1/active-shards/by-game/{game}/by-puuid/{puuid}", game=game, puuid=puuid));
        self.tick_thresholds().await;

        let resp = client.get(&endpoint)
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::InternalError(format!("Failed to get active shard for game by puuid {} - {}", resp.status().as_u16(), resp.text().await?)));
        }

        #[derive(Deserialize)]
        struct ShardInfo {
            #[serde(rename="activeShard")]
            active_shard: String
        }

        let shard = resp.json::<ShardInfo>().await?;
        Ok(shard.active_shard)
    }

    pub async fn get_account_me(&self, access_token: &str) -> Result<RiotAccount, SquadOvError> {
        let client = reqwest::ClientBuilder::new().build()?;
        let endpoint = Self::build_api_endpoint("americas", "riot/account/v1/accounts/me");
        self.tick_thresholds().await;

        let resp = client.get(&endpoint)
            .bearer_auth(access_token)
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::InternalError(format!("Failed to get Riot account using RSO {} - {}", resp.status().as_u16(), resp.text().await?)));
        }

        Ok(resp.json::<RiotAccount>().await?)
    }
}

impl super::RiotApiApplicationInterface {
    pub async fn obtain_riot_account_from_puuid(&self, puuid: &str) -> Result<(), SquadOvError> {
        let account = self.api.get_account_by_puuid(puuid).await?;
        let mut tx = self.db.begin().await?;
        db::store_riot_account(&mut tx, &account).await?;
        tx.commit().await?;
        Ok(())
    }
    
    pub async fn request_riot_account_from_puuid(&self, puuid: &str) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.mqconfig.rso_queue, serde_json::to_vec(&RiotApiTask::Account{puuid: String::from(puuid)})?, RABBITMQ_DEFAULT_PRIORITY).await;
        Ok(())
    }

    pub async fn obtain_riot_account_from_access_token(&self, access_token: &str, refresh_token: &str, expiration: &DateTime<Utc>, user_id: i64) -> Result<(), SquadOvError> {
        log::info!("Obtain Riot Account from Access Token for User: {}", user_id);
        // Check for the expiration of the access token using the passed in expiration date. If it is expired, use the refresh token to obtain a new access token.
        // Note that we use a 1 minute buffer here to guard against potential cases where the access token is valid when we check but no longer valid when we send the request.
        let (access_token, refresh_token, expiration) = if &(Utc::now() + chrono::Duration::minutes(1)) > expiration {
            let new_token = crate::riot::rso::refresh_authorization_code(&self.config.rso_client_id, &self.config.rso_client_secret, refresh_token).await?;
            (new_token.access_token.clone(), new_token.refresh_token.clone(), Utc::now() + chrono::Duration::seconds(new_token.expires_in.into()))
        } else {
            (access_token.to_string(), refresh_token.to_string(), expiration.clone())
        };

        let account = self.api.get_account_me(&access_token).await?;
        log::info!("\t...Storing account: {:?}#{:?} for {}", &account.game_name, &account.tag_line, user_id);
        let mut tx = self.db.begin().await?;
        db::store_riot_account(&mut tx, &account).await?;
        db::link_riot_account_to_user(&mut tx, &account.puuid, user_id).await?;
        db::store_rso_for_riot_account(&mut tx, &account.puuid, user_id, &access_token, &refresh_token, &expiration).await?;
        tx.commit().await?;

        // Also fire off a request for LoL summoner information. Note however that this needs to be done on the correct queue
        // as the RSO queue and the LoL queue use different keys. We can, however, do a publish correctly from whatever queue that
        // we want!
        self.request_lol_summoner_from_puuid(&account.puuid).await?;

        Ok(())
    }
    
    pub async fn request_riot_account_from_access_token(&self, access_token: &str, refresh_token: &str, expiration: DateTime<Utc>, user_id: i64) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.mqconfig.rso_queue, serde_json::to_vec(&RiotApiTask::AccountMe{
            access_token: access_token.to_string(),
            refresh_token: refresh_token.to_string(),
            expiration,
            user_id,
        })?, RABBITMQ_DEFAULT_PRIORITY).await;
        Ok(())
    }
}