use crate::{
    SquadOvError,
    accounts::TwitchAccount,
};
use serde::Deserialize;
use reqwest::{StatusCode, header};

const TWITCH_HELIX_URL : &'static str = "https://api.twitch.tv/helix";

pub struct TwitchApiClient<'a> {
    client_id: &'a str,
    access_token: &'a str,
}

impl<'a> TwitchApiClient<'a> {
    pub fn new(client_id: &'a str, access_token: &'a str) -> Self {
        Self {
            client_id,
            access_token,
        }
    }

    fn create_http_client(&self) -> Result<reqwest::Client, SquadOvError> {
        let mut headers = header::HeaderMap::new();
        let access_token = format!("Bearer {}", self.access_token);
        headers.insert(header::AUTHORIZATION, header::HeaderValue::from_str(&access_token)?);
        headers.insert("Client-Id", header::HeaderValue::from_str(self.client_id)?);
        Ok(reqwest::ClientBuilder::new()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(60))
            .build()?)
    }

    pub async fn get_basic_account_info(&self, broadcaster_id: i64) -> Result<TwitchAccount, SquadOvError> {
        let client = self.create_http_client()?;
        let resp = client.get(
            &format!(
                "{}/channels?broadcaster_id={}",
                TWITCH_HELIX_URL,
                broadcaster_id,
            ))
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::NotFound);
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
                twitch_user_id: broadcaster_id,
                twitch_name: data.data[0].broadcaster_name.clone(),
            })
        }
    }
}