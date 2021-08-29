use crate::{
    SquadOvError,
    accounts::{
        TwitchAccount,
        twitch as tvacc
    },
    twitch::{
        TwitchConfig,
        oauth::{
            self,
            TwitchOAuthToken,
        },
    },
};
use serde::{Serialize, Deserialize};
use reqwest::{StatusCode, header};
use std::sync::Arc;
use async_std::sync::RwLock;
use sqlx::PgPool;

const TWITCH_HELIX_URL : &'static str = "https://api.twitch.tv/helix";

#[derive(PartialEq, Copy, Clone)]
pub enum TwitchTokenType {
    User,
    App
}

pub struct TwitchApiClient {
    config: TwitchConfig,
    token: RwLock<TwitchOAuthToken>,
    token_type: TwitchTokenType,
    pool: Arc<PgPool>,
}

#[derive(Clone,Serialize)]
pub struct EventSubCondition {
    pub broadcaster_user_id: String,
}

#[derive(Clone,Serialize)]
pub struct EventSubTransport {
    pub method: String,
    pub callback: String,
    pub secret: String,
}

#[derive(Deserialize)]
pub struct TwitchSubscription {
    pub broadcaster_id: String,
    pub tier: String,
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct TwitchSubscriptionAlt {
    pub broadcaster_user_id: String,
    pub tier: String,
    pub user_id: String,
}

impl TwitchSubscriptionAlt {
    pub fn to_sub(self) -> TwitchSubscription {
        TwitchSubscription {
            broadcaster_id: self.broadcaster_user_id,
            tier: self.tier,
            user_id: self.user_id,
        }
    }
}


impl TwitchApiClient {
    pub fn new(config: TwitchConfig, token: TwitchOAuthToken, typ: TwitchTokenType, pool: Arc<PgPool>) -> Self {
        Self {
            config,
            token: RwLock::new(token),
            token_type: typ,
            pool,
        }
    }

    async fn create_http_client(&self) -> Result<reqwest::Client, SquadOvError> {
        let mut headers = header::HeaderMap::new();
        let access_token = format!("Bearer {}", {
            // App tokens should be validated before we send the request. User tokens can be validated/refreshed afterwards.
            if self.token_type == TwitchTokenType::App {
                let is_valid = {
                    let tk = self.token.read().await;
                    oauth::validate_access_token(&tk.access_token).await?
                };

                if !is_valid {
                    let mut tk = self.token.write().await;
                    let new_token = oauth::get_oauth_client_credentials_token(&self.config.client_id, &self.config.client_secret).await?;
                    tk.copy_from(&new_token);
                }
            }

            let tk = self.token.read().await;
            tk.access_token.clone()
        });
        headers.insert(header::AUTHORIZATION, header::HeaderValue::from_str(&access_token)?);
        headers.insert("Client-Id", header::HeaderValue::from_str(&self.config.client_id)?);
        Ok(reqwest::ClientBuilder::new()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(60))
            .build()?)
    }

    pub async fn refresh_user_access_token(&self) -> Result<(), SquadOvError> {
        let new_token = oauth::refresh_oauth_token(&self.config.client_id, &self.config.client_secret, &{
            let tk = self.token.read().await;
            tk.refresh_token.clone()
        }).await?;

        let mut tk = self.token.write().await;
        let old_access_key = tk.access_token.clone();
        tk.copy_from(&new_token);

        tvacc::update_twitch_access_token(&*self.pool, &old_access_key, &tk).await?;
        Ok(())
    }

    async fn execute(&self, f: Arc<dyn Fn(&reqwest::Client) -> reqwest::RequestBuilder + Send + Sync>) -> Result<reqwest::Response, SquadOvError> {
        let mut client = self.create_http_client().await?;
        let mut resp = f(&client).send().await?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            if self.token_type == TwitchTokenType::User {
                log::info!("Unauthorized Twitch Access [User] - Refreshing {}", resp.text().await?);
                // If the access token fails then this Twitch account is no longer valid.
                match self.refresh_user_access_token().await {
                    Ok(_) => (),
                    Err(err) => {
                        log::warn!("Failed to refresh Twitch access token: {:?}", err);
                        let tk = self.token.read().await;
                        tvacc::delete_twitch_account(&*self.pool, &tk.access_token).await?;
                        return Err(SquadOvError::Unauthorized);
                    }
                }
                
                client = self.create_http_client().await?;
                resp = f(&client).send().await?;
            }
        }
        Ok(resp)
    }

    pub async fn get_basic_account_info(&self, broadcaster_id: &str) -> Result<TwitchAccount, SquadOvError> {
        let resp = {
            let bid = broadcaster_id.to_string();
            self.execute(Arc::new(move |client: &reqwest::Client| {
                client.get(
                    &format!(
                        "{}/channels?broadcaster_id={}",
                        TWITCH_HELIX_URL,
                        bid.clone(),
                    )
                )
            })).await?
        };

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::InternalError(format!("Basic Account Info Error Twitch: {} - {}", resp.status().as_u16(), resp.text().await?)));
        }

        #[derive(Deserialize)]
        struct ChannelInfo {
            broadcaster_name: String,
        }

        #[derive(Deserialize)]
        struct Response {
            data: Vec<ChannelInfo>
        }

        let data = resp.json::<Response>().await?;
        if data.data.is_empty() {
            Err(SquadOvError::NotFound)
        } else {
            Ok(TwitchAccount{
                twitch_user_id: String::from(broadcaster_id.clone()),
                twitch_name: data.data[0].broadcaster_name.clone(),
            })
        }
    }

    pub async fn register_eventsub_subscription(&self, sub: &str, condition: EventSubCondition, transport: EventSubTransport) -> Result<(), SquadOvError> {
        #[derive(Serialize)]
        struct Request<'a> {
            #[serde(rename="type")]
            sub_type: String,
            version: &'a str,
            condition: EventSubCondition,
            transport: EventSubTransport,
        }

        let resp = {
            let sub = sub.to_string();
            let condition = condition.clone();
            let transport = transport.clone();

            self.execute(Arc::new(move |client: &reqwest::Client| {
                client.post(
                    &format!(
                        "{}/eventsub/subscriptions",
                        TWITCH_HELIX_URL,
                    )
                )
                .json(&Request{
                    sub_type: sub.clone(),
                    version: "1",
                    condition: condition.clone(),
                    transport: transport.clone(),
                })
            })).await?
        };

        if resp.status() != StatusCode::ACCEPTED {
            return Err(SquadOvError::InternalError(format!("Register Eventsub Twitch: {} - {}", resp.status().as_u16(), resp.text().await?)));
        }

        Ok(())
    }

    pub async fn get_broadcaster_subscriptions(&self, broadcaster_id: &str, cursor: Option<String>) -> Result<(Vec<TwitchSubscription>, Option<String>), SquadOvError> {
        let resp = {
            let bid = broadcaster_id.to_string();
            let cursor = cursor.clone();
            self.execute(Arc::new(move |client: &reqwest::Client| {
                let base = format!(
                    "{}/subscriptions?broadcaster_id={}",
                    TWITCH_HELIX_URL,
                    bid.clone(),
                );

                client.get(&if let Some(cursor) = cursor.clone() {
                    format!("{}&after={}", base, cursor)
                } else {
                    base
                })
            })).await?
        };

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::InternalError(format!("Get Subs Twitch: {} - {}", resp.status().as_u16(), resp.text().await?)));
        }

        #[derive(Deserialize)]
        struct Pagination {
            cursor: Option<String>,  
        }

        #[derive(Deserialize)]
        struct Response {
            data: Vec<TwitchSubscription>,
            pagination: Option<Pagination>,
        }

        let data = resp.json::<Response>().await?;
        Ok((data.data, if let Some(page) = data.pagination {
            page.cursor
        } else {
            None
        }))
    }
}