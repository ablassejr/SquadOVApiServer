use serde::{Serialize,Deserialize};
use derive_more::{Display};

#[derive(Serialize,Deserialize)]
pub struct FusionAuthRegistration {
    #[serde(rename = "applicationId")]
    pub application_id: String,
    pub username: Option<String>
}

#[derive(Deserialize)]
pub struct FusionAuthUser {
    pub email: String,
    pub registrations: Vec<FusionAuthRegistration>
}

#[derive(Serialize,Deserialize)]
pub struct FusionSingleAppAuthUser {
    pub email: String,
    pub username: String,
    pub password: Option<String>
}

#[derive(Debug, Display)]
pub enum FusionAuthVerificationEmailError {
    InvalidRequest(String),
    Auth,
    DoesNotExist,
    InternalError,
    Search(String),
    Generic(String)
}

impl super::FusionAuthClient {
    pub fn find_auth_registration<'a>(&self, u: &'a FusionAuthUser) -> Option<&'a FusionAuthRegistration> {
        for reg in &u.registrations {
            if reg.application_id == self.cfg.application_id {
                return Some(&reg)
            }
        }
        return None
    }

    pub async fn verify_email(&self, verification_id: &str) -> Result<(), FusionAuthVerificationEmailError> {
        match self.client.post(self.build_url(format!("/api/user/verify-email/{}", &verification_id).as_str()).as_str())
            .send()
            .await {
            Ok(resp) => {
                match resp.status().as_u16() {
                    200 => Ok(()),
                    400 => {
                        let body = resp.text().await;
                        match body {
                            Ok(j) => Err(FusionAuthVerificationEmailError::InvalidRequest(j)),
                            Err(err) => Err(FusionAuthVerificationEmailError::Generic(format!("{}", err))),
                        }
                    },
                    401 => Err(FusionAuthVerificationEmailError::Auth),
                    404 => Err(FusionAuthVerificationEmailError::DoesNotExist),
                    500 => Err(FusionAuthVerificationEmailError::InternalError),
                    503 => {
                        let body = resp.text().await;
                        match body {
                            Ok(j) => Err(FusionAuthVerificationEmailError::Search(j)),
                            Err(err) => Err(FusionAuthVerificationEmailError::Generic(format!("{}", err))),
                        }
                    }
                    _ => Err(FusionAuthVerificationEmailError::Generic(String::from("Unknown verification error."))),
                }
            },
            Err(err) => Err(FusionAuthVerificationEmailError::Generic(format!("{}", err))),
        }
    }
}