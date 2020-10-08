use serde::{Deserialize, Serialize};
use derive_more::{Display};
#[derive(Serialize)]
pub struct FusionAuthRegisterInput {
    registration: super::FusionAuthRegistration,
    user: super::FusionSingleAppAuthUser,
}

#[derive(Deserialize)]
pub struct FusionAuthRegisterResult {
    pub user: super::FusionSingleAppAuthUser,
    pub token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String
}

#[derive(Debug, Display)]
pub enum FusionAuthRegisterError {
    InvalidRequest(String),
    ServerAuth,
    InternalError,
    Search(String),
    Generic(String)
}

impl super::FusionAuthClient {
    pub fn build_register_input(&self, username : String, email : String, password: String) -> FusionAuthRegisterInput {
        return FusionAuthRegisterInput{
            registration: super::FusionAuthRegistration{
                application_id: self.cfg.application_id.clone(),
                username: Some(username.clone()),
            },
            user: super::FusionSingleAppAuthUser{
                email: email,
                password: Some(password),
                username: username.clone(),
            },
        }
    }

    pub async fn register(&self, input : FusionAuthRegisterInput) -> Result<FusionAuthRegisterResult, FusionAuthRegisterError> {
        let res = self.client.post(self.build_url("/api/user/registration").as_str())
            .json(&input)
            .send()
            .await;

        match res {
            Ok(resp) => {
                let status = resp.status();
                match status.as_u16() {
                    200 => {
                        let body = resp.json::<FusionAuthRegisterResult>().await;
                        match body {
                            Ok(j) => Ok(j),
                            Err(err) => Err(FusionAuthRegisterError::Generic(format!("{}", err))),
                        }
                    },
                    400 => {
                        let body = resp.text().await;
                        match body {
                            Ok(j) => Err(FusionAuthRegisterError::InvalidRequest(j)),
                            Err(err) => Err(FusionAuthRegisterError::Generic(format!("{}", err))),
                        }
                    }
                    401 => Err(FusionAuthRegisterError::ServerAuth),
                    500 => Err(FusionAuthRegisterError::InternalError),
                    503 => {
                        let body = resp.text().await;
                        match body {
                            Ok(j) => Err(FusionAuthRegisterError::Search(j)),
                            Err(err) => Err(FusionAuthRegisterError::Generic(format!("{}", err))),
                        }
                    },
                    _ => Err(FusionAuthRegisterError::Generic(format!("Unknown Fusion Auth Error: {}", status.as_u16())))
                }
            },
            Err(err) => Err(FusionAuthRegisterError::Generic(format!("{}", err))),
        }
    }
}