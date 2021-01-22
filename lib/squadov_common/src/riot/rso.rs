use crate::SquadOvError;
use serde::{Serialize, Deserialize};

#[derive(Deserialize, Debug)]
pub struct RsoOAuthAccessToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i32,
}

#[derive(Serialize)]
struct RsoTokenRequest {
    grant_type: String,
    code: String,
    redirect_uri: String
}

#[derive(Serialize)]
struct RsoRefreshRequest {
    grant_type: String,
    refresh_token: String,
}

const RSO_TOKEN_URL: &'static str = "https://auth.riotgames.com/token";
const RSO_REDIRECT_URI: &'static str = "https://app.squadov.gg/riot/oauth-callback";

pub async fn exchange_authorization_code_for_access_token(client_id: &str, client_secret: &str, code: &str) -> Result<RsoOAuthAccessToken, SquadOvError> {
    let client = reqwest::ClientBuilder::new().build()?;
    let result = client
        .post(RSO_TOKEN_URL)
        .form(&RsoTokenRequest{
            grant_type: String::from("authorization_code"),
            code: crate::encode::url_decode(code)?,
            redirect_uri: String::from(RSO_REDIRECT_URI),
        })
        .basic_auth(client_id, Some(client_secret))
        .send()
        .await?;
    
    let status = result.status().as_u16();
    if status != 200 {
        return Err(SquadOvError::InternalError(format!("Failed to exchange auth code RSO [{}]: {}", status, result.text().await?)));
    }

    Ok(result.json::<RsoOAuthAccessToken>().await?)
}

pub async fn refresh_authorization_code(client_id: &str, client_secret: &str, refresh_token: &str) -> Result<RsoOAuthAccessToken, SquadOvError> {
    let client = reqwest::ClientBuilder::new().build()?;
    let result = client
        .post(RSO_TOKEN_URL)
        .form(&RsoRefreshRequest{
            grant_type: String::from("refresh_token"),
            refresh_token: refresh_token.to_string(),
        })
        .basic_auth(client_id, Some(client_secret))
        .send()
        .await?;
    
    let status = result.status().as_u16();
    if status != 200 {
        return Err(SquadOvError::InternalError(format!("Failed to refresh auth code RSO [{}]: {}", status, result.text().await?)));
    }

    Ok(result.json::<RsoOAuthAccessToken>().await?)
}