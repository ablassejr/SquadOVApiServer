use squadov_common::SquadOvError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct FusionAuthMfaSecret {
    pub secret: String,
    pub secret_base32_encoded: String,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct FusionAuthEnableMfaInput<'a> {
    code: &'a str,
    method: &'a str,
    secret_base32_encoded: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct FusionAuthRecoveryCodes {
    recovery_codes: Vec<String>,
}

impl super::FusionAuthClient {
    pub async fn complete_mfa(&self, code: &str, two_factor_id: &str) -> Result<String, SquadOvError> {
        #[derive(Serialize)]
        #[serde(rename_all="camelCase")]
        struct Input {
            code: String,
            two_factor_id: String,
        }

        let resp = self.client.post(self.build_url("/api/two-factor/login").as_str())
            .json(&Input{
                code: code.to_string(),
                two_factor_id: two_factor_id.to_string(),
            })
            .send()
            .await?;

        #[derive(Deserialize)]
        #[serde(rename_all="camelCase")]
        struct Output {
            trust_token: String,
        }
        
        match resp.status().as_u16() {
            200 => Ok(resp.json::<Output>().await?.trust_token),
            401 => Err(SquadOvError::Unauthorized),
            _ => Err(SquadOvError::InternalError(format!(
                "Complete MFA: {}",
                resp.text().await?,
            )))
        }
    }

    pub async fn start_mfa(&self, challenge: &str, user_id: Option<&str>, login_id: Option<&str>) -> Result<String, SquadOvError> {
        #[derive(Serialize)]
        #[serde(rename_all="camelCase")]
        struct Input {
            user_id: Option<String>,
            login_id: Option<String>,
            trust_challenge: String,
        }

        let resp = self.client.post(self.build_url("/api/two-factor/start").as_str())
            .json(&Input{
                user_id: user_id.map(|x| { x.to_string() }),
                login_id: login_id.map(|x| { x.to_string() }),
                trust_challenge: challenge.to_string(),
            })
            .send()
            .await?;

        #[derive(Deserialize)]
        #[serde(rename_all="camelCase")]
        struct Output {
            two_factor_id: String,
        }
        
        match resp.status().as_u16() {
            200 => Ok(resp.json::<Output>().await?.two_factor_id),
            401 => Err(SquadOvError::Unauthorized),
            _ => Err(SquadOvError::InternalError(format!(
                "Start MFA: {}",
                resp.text().await?,
            )))
        }
    }

    pub async fn generate_mfa_secret(&self) -> Result<FusionAuthMfaSecret, SquadOvError> {
        let resp = self.client.get(self.build_url("/api/two-factor/secret").as_str())
            .send()
            .await?;
        
        match resp.status().as_u16() {
            200 => Ok(resp.json::<FusionAuthMfaSecret>().await?),
            401 => Err(SquadOvError::Unauthorized),
            _ => Err(SquadOvError::InternalError(format!(
                "Generate MFA Secret Error: {}",
                resp.text().await?,
            )))
        }
    }

    pub async fn enable_mfa(&self, id: &Uuid, code: &str, secret: &str) -> Result<Vec<String>, SquadOvError> {
        let resp = self.client.post(self.build_url(format!("/api/user/two-factor/{}", id).as_str()).as_str())
            .json(&FusionAuthEnableMfaInput{
                code: code,
                method: "authenticator",
                secret_base32_encoded: secret,
            })
            .send()
            .await?;
        match resp.status().as_u16() {
            200 => Ok(resp.json::<FusionAuthRecoveryCodes>().await?.recovery_codes),
            400 => Err(SquadOvError::BadRequest),
            401 => Err(SquadOvError::Unauthorized),
            404 => Err(SquadOvError::NotFound),
            _ => Err(SquadOvError::InternalError(format!(
                "Enable MFA Error: {}",
                resp.text().await?,
            )))
        }
    }

    pub async fn disable_mfa(&self, id: &Uuid, code: &str, method: &str) -> Result<(), SquadOvError> {
        let resp = self.client.delete(self.build_url(format!("/api/user/two-factor/{}?code={}&methodId={}", id, code, method).as_str()).as_str())
            .send()
            .await?;
        match resp.status().as_u16() {
            200 => Ok(()),
            400 => {
                log::warn!("Disable MFA Bad Request: {}", resp.text().await?);
                Err(SquadOvError::BadRequest)
            },
            401 => Err(SquadOvError::Unauthorized),
            404 => Err(SquadOvError::NotFound),
            _ => Err(SquadOvError::InternalError(format!(
                "Disable MFA Error: {}",
                resp.text().await?,
            )))
        }
    }

    /*
    pub async fn get_mfa_recovery_codes(&self, id: &Uuid) -> Result<Vec<String>, SquadOvError> {
        let resp = self.client.get(self.build_url(format!("/api/recovery-codes/{}", id).as_str()).as_str())
            .send()
            .await?;
        match resp.status().as_u16() {
            200 => Ok(resp.json::<FusionAuthRecoveryCodes>().await?.recovery_codes),
            400 => Err(SquadOvError::BadRequest),
            401 => Err(SquadOvError::Unauthorized),
            404 => Err(SquadOvError::NotFound),
            _ => Err(SquadOvError::InternalError(format!(
                "Get Recovery Codes Error: {}",
                resp.text().await?,
            )))
        }
    }
    */
}