use serde::{Serialize,Deserialize};
use derive_more::{Display};
use squadov_common::{
    SquadOvError,
    encode::url_encode,
};
use uuid::Uuid;

#[derive(Debug, Serialize,Deserialize)]
pub struct FusionAuthRegistration {
    #[serde(rename = "applicationId")]
    pub application_id: String,
    pub username: Option<String>,
    #[serde(rename = "insertInstant", skip_serializing)]
    pub insert_instant: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct FusionAuthTwoFactorMethod {
    pub id: String,
    pub method: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct FusionAuthTwoFactor {
    pub methods: Option<Vec<FusionAuthTwoFactorMethod>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct FusionAuthUser {
    pub id: Uuid,
    pub email: String,
    pub registrations: Vec<FusionAuthRegistration>,
    pub two_factor: FusionAuthTwoFactor,
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

#[derive(Serialize)]
struct FusionAuthStartForgotPasswordInput<'a> {
    #[serde(rename = "loginId")]
    login_id: &'a str
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct FusionAuthChangePasswordInput<'a> {
    password: &'a str,
    trust_challenge: Option<String>,
    trust_token: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct FusionAuthChangePasswordWithUserInput<'a> {
    password: &'a str,
    current_password: &'a str,
    login_id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct FusionAuthPatchUserData<'a> {
    email: Option<&'a str>,
    username: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct FusionAuthPatchUserInput<'a> {
    user: FusionAuthPatchUserData<'a>
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

    pub async fn find_user_generic(&self, key: &str, value: &str) -> Result<FusionAuthUser, FusionAuthUserError> {
        match self.client.get(self.build_url(format!("/api/user?{}={}", key, url_encode(value)).as_str()).as_str())
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

    pub async fn find_user_from_email_address(&self, email: &str) -> Result<FusionAuthUser, FusionAuthUserError> {
        Ok(self.find_user_generic("email", email).await?)
    }

    pub async fn find_user_from_email_verification_id(&self, id: &str) -> Result<FusionAuthUser, FusionAuthUserError> {
        Ok(self.find_user_generic("verificationId", id).await?)
    }

    pub async fn resend_verify_email(&self, email: &str) -> Result<(), FusionAuthResendVerificationEmailError> {
        match self.client.put(self.build_url(format!("/api/user/verify-email?email={}", url_encode(email)).as_str()).as_str())
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

    pub async fn start_forgot_password_workflow(&self, login_id: &str) -> Result<(), FusionAuthResendVerificationEmailError> {
        match self.client.post(self.build_url("/api/user/forgot-password").as_str())
            .json(&FusionAuthStartForgotPasswordInput{
                login_id: login_id,
            })
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
                    _ => Err(FusionAuthResendVerificationEmailError::Generic(String::from("Unknown start password workflow error."))),
                }
            },
            Err(err) => Err(FusionAuthResendVerificationEmailError::Generic(format!("{}", err))),
        }
    }

    pub async fn change_user_password(&self, change_password_id: &str, password: &str, trust_challenge: Option<String>, trust_token: Option<String>) -> Result<(), FusionAuthUserError> {
        match self.client.post(self.build_url(format!("/api/user/change-password/{}", change_password_id).as_str()).as_str())
            .json(&FusionAuthChangePasswordInput{
                password: password,
                trust_challenge,
                trust_token,
            })
            .send()
            .await
        {
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
                    _ => Err(FusionAuthUserError::Generic(String::from("Unknown change password error."))),
                }
            },
            Err(err) => Err(FusionAuthUserError::Generic(format!("{}", err))),
        }
    }

    pub async fn change_user_password_with_id(&self, current_password: &str, new_password: &str, login_id: &str) -> Result<(), SquadOvError> {
        let resp = self.client.post(self.build_url("/api/user/change-password").as_str())
            .json(&FusionAuthChangePasswordWithUserInput{
                password: new_password,
                current_password: current_password,
                login_id: login_id,
            })
            .send()
            .await?;
        
        match resp.status().as_u16() {
            200 => Ok(()),
            400 => {
                let body = resp.text().await?;
                log::warn!("Failure in request to change user password: {}", body);
                Err(SquadOvError::BadRequest)
            },
            401 => Err(SquadOvError::Unauthorized),
            404 => Err(SquadOvError::NotFound),
            500 => Err(SquadOvError::InternalError(String::from("FA Error"))),
            503 => {
                let body = resp.text().await?;
                Err(SquadOvError::InternalError(body))
            }
            _ => Err(SquadOvError::InternalError(String::from("Unknown change password error."))),
        }
    }

    async fn update_user(&self, user_id: &Uuid, data: FusionAuthPatchUserInput<'_>) -> Result<(), SquadOvError> {
        let resp = self.client.patch(self.build_url(&format!("/api/user/{}", user_id)).as_str())
            .json(&data)
            .send()
            .await?;
        
        match resp.status().as_u16() {
            200 => Ok(()),
            400 => {
                let body = resp.text().await?;
                log::warn!("Failure in request to update user: {}", body);
                Err(SquadOvError::BadRequest)
            },
            401 => Err(SquadOvError::Unauthorized),
            404 => Err(SquadOvError::NotFound),
            500 => Err(SquadOvError::InternalError(String::from("FA Error"))),
            503 => {
                let body = resp.text().await?;
                Err(SquadOvError::InternalError(body))
            }
            _ => Err(SquadOvError::InternalError(String::from("Unknown update user error."))),
        }
    }

    pub async fn update_user_id(&self, user_id: &Uuid, username: &str, email: &str) -> Result<(), SquadOvError> {
        self.update_user(user_id, FusionAuthPatchUserInput{
            user: FusionAuthPatchUserData{
                email: Some(email),
                username: Some(username),
            },
        }).await?;
        Ok(())
    }
}