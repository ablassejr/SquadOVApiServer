use crate::{
    SquadOvError
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TwitchOAuthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
}

#[derive(Deserialize)]
pub struct TwitchIdToken {
    sub: String,
}

const TOKEN_URL: &'static str = "https://id.twitch.tv/oauth2/token";

pub async fn exchange_authorization_code_for_access_token(client_id: &str, client_secret: &str, redirect_url: &str, code: &str) -> Result<TwitchOAuthToken, SquadOvError> {
    let client = reqwest::ClientBuilder::new().build()?;
    log::info!("exchange: {} {} {} {}", client_id, client_secret, redirect_url, code);
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

pub fn extract_twitch_user_id_from_id_token(token: &str) -> Result<i64, SquadOvError> {
    let jwt = jsonwebtoken::dangerous_insecure_decode::<TwitchIdToken>(token)?;
    Ok(jwt.claims.sub.parse::<i64>()?)
}