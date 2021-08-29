use crate::{
    SquadOvError
};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Duration};
use reqwest::header;

#[derive(Deserialize, Clone)]
pub struct TwitchOAuthToken {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    pub id_token: Option<String>,
    pub expires_in: i32,
}

impl TwitchOAuthToken {
    pub fn copy_from(&mut self, other: &TwitchOAuthToken) {
        self.access_token = other.access_token.clone();
        self.refresh_token = other.refresh_token.clone();
        self.expires_in = other.expires_in;
    }
}

impl TwitchOAuthToken {
    pub fn expiration_time(&self) -> DateTime<Utc> {
        // Subtract a couple minutes to give us a buffer to ensure there isn't some weird timing issue
        // where we technically used more than 0 seconds between when the token was issued to now causing
        // there technically to be less than 3600 (for example) seconds left. 
        Utc::now() + Duration::seconds(self.expires_in as i64 - 120)
    }
}

#[derive(Deserialize)]
pub struct TwitchIdToken {
    sub: String,
}

const TOKEN_URL: &'static str = "https://id.twitch.tv/oauth2/token";

pub async fn exchange_authorization_code_for_access_token(client_id: &str, client_secret: &str, redirect_url: &str, code: &str) -> Result<TwitchOAuthToken, SquadOvError> {
    let client = reqwest::ClientBuilder::new().build()?;
    let result = client
        .post(
            &format!(
                "{base}?client_id={client_id}&client_secret={client_secret}&code={code}&grant_type=authorization_code&redirect_uri={redirect}",
                base=TOKEN_URL,
                client_id=client_id,
                client_secret=client_secret,
                code=code,
                redirect=redirect_url,
            )
        )
        .send()
        .await?;
    
    let status = result.status().as_u16();
    if status != 200 {
        return Err(SquadOvError::InternalError(format!("Failed to exchange auth code Twitch [{}]: {}", status, result.text().await?)));
    }

    Ok(result.json::<TwitchOAuthToken>().await?)
}

pub async fn refresh_oauth_token(client_id: &str, client_secret: &str, refresh_token: &str) -> Result<TwitchOAuthToken, SquadOvError> {        
    #[derive(Serialize)]
    pub struct Body<'a> {
        grant_type: &'a str,
        refresh_token: &'a str,
        client_id: &'a str,
        client_secret: &'a str,
    }

    let client = reqwest::ClientBuilder::new().build()?;
    let result = client
        .post(TOKEN_URL)
        .form(&Body{
            grant_type: "refresh_token",
            refresh_token,
            client_id,
            client_secret
        })
        .send()
        .await?;
    
    let status = result.status().as_u16();
    if status != 200 {
        return Err(SquadOvError::InternalError(format!("Failed to exchange refresh oauth token Twitch [{}]: {}", status, result.text().await?)));
    }

    Ok(result.json::<TwitchOAuthToken>().await?)
}

pub async fn get_oauth_client_credentials_token(client_id: &str, client_secret: &str) -> Result<TwitchOAuthToken, SquadOvError> {
    let client = reqwest::ClientBuilder::new().build()?;
    let result = client
        .post(
            &format!(
                "{base}?client_id={client_id}&client_secret={client_secret}&grant_type=client_credentials",
                base=TOKEN_URL,
                client_id=client_id,
                client_secret=client_secret,
            )
        )
        .send()
        .await?;
    
    let status = result.status().as_u16();
    if status != 200 {
        return Err(SquadOvError::InternalError(format!("Failed to to do client credentials flow Twitch [{}]: {}", status, result.text().await?)));
    }

    Ok(result.json::<TwitchOAuthToken>().await?)
}

pub async fn validate_access_token(access_token: &str) -> Result<bool, SquadOvError> {
    let access_token = format!("Bearer {}", access_token);
    let client = reqwest::ClientBuilder::new().build()?;
    let result = client
        .get("https://id.twitch.tv/oauth2/validate")
        .header(header::AUTHORIZATION, header::HeaderValue::from_str(&access_token)?)
        .send()
        .await?;
    
    let status = result.status().as_u16();
    let success = status == 200;

    if !success {
        log::warn!("Failed to validate Twitch access token: {} -- {}", status, result.text().await?);
    }

    Ok(success)
}

pub fn extract_twitch_user_id_from_id_token(token: &str) -> Result<String, SquadOvError> {
    let jwt = jsonwebtoken::dangerous_insecure_decode::<TwitchIdToken>(token)?;
    Ok(jwt.claims.sub)
}