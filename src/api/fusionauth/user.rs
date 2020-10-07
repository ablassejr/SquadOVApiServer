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
    pub registrations: Vec<FusionAuthRegistration>,
    pub verified: bool,
}

#[derive(Serialize,Deserialize)]
pub struct FusionSingleAppAuthUser {
    pub email: String,
    pub username: String,
    pub password: Option<String>
}

#[derive(Debug, Display)]
pub enum FusionAuthUserError {
    InvalidRequest(String),
    Auth,
    DoesNotExist,
    InternalError,
    Search(String),
    Generic(String)
}

#[derive(Deserialize)]
pub struct FusionAuthGetUserResult {
    pub user: super::FusionAuthUser,
}

#[derive(Debug, Display)]
pub enum FusionAuthResendVerificationEmailError {
    InvalidRequest(String),
    Auth,
    Disabled,
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

    pub async fn find_user_from_email_verification_id(&self, id: &str) -> Result<FusionAuthUser, FusionAuthUserError> {
        match self.client.get(self.build_url(format!("/api/user?verificationId={}", id).as_str()).as_str())
            .send()
            .await {
            Ok(resp) => {
                match resp.status().as_u16() {
                    200 => {
                        let body = resp.json::<FusionAuthGetUserResult>().await;
                        match body {
                            Ok(j) => Ok(j.user),
                            Err(err) => Err(FusionAuthUserError::Generic(format!("{}", err))),
                        }
                    },
                    400 => {
                        let body = resp.text().await;
                        match body {
                            Ok(j) => Err(FusionAuthUserError::InvalidRequest(j)),
                            Err(err) => Err(FusionAuthUserError::Generic(format!("{}", err))),
                        }
                    },
                    401 => Err(FusionAuthUserError::Auth),
                    404 => Err(FusionAuthUserError::DoesNotExist),
                    500 => Err(FusionAuthUserError::InternalError),
                    503 => {
                        let body = resp.text().await;
                        match body {
                            Ok(j) => Err(FusionAuthUserError::Search(j)),
                            Err(err) => Err(FusionAuthUserError::Generic(format!("{}", err))),
                        }
                    }
                    _ => Err(FusionAuthUserError::Generic(String::from("Unknown verification error."))),
                }
            },
            Err(err) => Err(FusionAuthUserError::Generic(format!("{}", err))),
        }
    }

    pub async fn resend_verify_email(&self, email: &str) -> Result<(), FusionAuthResendVerificationEmailError> {
        match self.client.put(self.build_url(format!("/api/user/verify-email?email={}", &email).as_str()).as_str())
            .send()
            .await {
            Ok(resp) => {
                match resp.status().as_u16() {
                    200 => Ok(()),
                    400 => {
                        let body = resp.text().await;
                        match body {
                            Ok(j) => Err(FusionAuthResendVerificationEmailError::InvalidRequest(j)),
                            Err(err) => Err(FusionAuthResendVerificationEmailError::Generic(format!("{}", err))),
                        }
                    },
                    401 => Err(FusionAuthResendVerificationEmailError::Auth),
                    403 => Err(FusionAuthResendVerificationEmailError::Disabled),
                    404 => Err(FusionAuthResendVerificationEmailError::DoesNotExist),
                    500 => Err(FusionAuthResendVerificationEmailError::InternalError),
                    503 => {
                        let body = resp.text().await;
                        match body {
                            Ok(j) => Err(FusionAuthResendVerificationEmailError::Search(j)),
                            Err(err) => Err(FusionAuthResendVerificationEmailError::Generic(format!("{}", err))),
                        }
                    }
                    _ => Err(FusionAuthResendVerificationEmailError::Generic(String::from("Unknown verification error."))),
                }
            },
            Err(err) => Err(FusionAuthResendVerificationEmailError::Generic(format!("{}", err))),
        }
    }

    pub async fn verify_email(&self, verification_id: &str) -> Result<(), FusionAuthUserError> {
        match self.client.post(self.build_url(format!("/api/user/verify-email/{}", &verification_id).as_str()).as_str())
            .send()
            .await {
            Ok(resp) => {
                match resp.status().as_u16() {
                    200 => Ok(()),
                    400 => {
                        let body = resp.text().await;
                        match body {
                            Ok(j) => Err(FusionAuthUserError::InvalidRequest(j)),
                            Err(err) => Err(FusionAuthUserError::Generic(format!("{}", err))),
                        }
                    },
                    401 => Err(FusionAuthUserError::Auth),
                    404 => Err(FusionAuthUserError::DoesNotExist),
                    500 => Err(FusionAuthUserError::InternalError),
                    503 => {
                        let body = resp.text().await;
                        match body {
                            Ok(j) => Err(FusionAuthUserError::Search(j)),
                            Err(err) => Err(FusionAuthUserError::Generic(format!("{}", err))),
                        }
                    }
                    _ => Err(FusionAuthUserError::Generic(String::from("Unknown verification error."))),
                }
            },
            Err(err) => Err(FusionAuthUserError::Generic(format!("{}", err))),
        }
    }
}