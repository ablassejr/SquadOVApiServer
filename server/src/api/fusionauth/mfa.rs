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