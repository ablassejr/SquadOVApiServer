use squadov_common::SquadOvError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Deserialize)]
struct InternalFusionAuthRegistrationError {
    code: String,
}

impl InternalFusionAuthRegistrationError {
    fn is_duplicate(&self) -> bool {
        self.code.contains("[duplicate]")
    }
}

#[derive(Deserialize)]
struct InternalFusionAuthRegistrationErrorResponse {
    #[serde(rename="fieldErrors")]
    field_errors: HashMap<String, Vec<InternalFusionAuthRegistrationError>>
}

impl InternalFusionAuthRegistrationErrorResponse {
    fn has_duplicate(&self) -> bool {
        for (_, arr) in &self.field_errors {
            for v in arr {
                if v.is_duplicate() {
                    return true;
                }
            }
        }

        return false;
    }
}

impl super::FusionAuthClient {
    pub fn build_register_input(&self, username : String, email : String, password: String) -> FusionAuthRegisterInput {
        return FusionAuthRegisterInput{
            registration: super::FusionAuthRegistration{
                application_id: self.cfg.application_id.clone(),
                username: Some(username.clone()),
                insert_instant: 0,
            },
            user: super::FusionSingleAppAuthUser{
                email: email,
                password: Some(password),
                username: username.clone(),
            },
        }
    }

    pub async fn register(&self, input : FusionAuthRegisterInput) -> Result<FusionAuthRegisterResult, SquadOvError> {
        let res = self.client.post(self.build_url("/api/user/registration").as_str())
            .json(&input)
            .send()
            .await?;

        let status = res.status();
        match status.as_u16() {
            200 => Ok(res.json::<FusionAuthRegisterResult>().await?),
            400 => {
                let data = res.json::<InternalFusionAuthRegistrationErrorResponse>().await?;
                if data.has_duplicate() {
                    Err(SquadOvError::Duplicate)
                } else {
                    Err(SquadOvError::BadRequest)
                }
            },
            401 => Err(SquadOvError::Unauthorized),
            500 | 503 => Err(SquadOvError::InternalError(String::from("FusionAuth Internval Error"))),
            _ => Err(SquadOvError::InternalError(String::from("FusionAuth Internval Error"))),
        }
    }
}