use serde::{Serialize, Deserialize};
use derive_more::{Display};

#[derive(Debug, Display)]
pub enum FusionAuthValidateJwtError {
    Invalid,
    InternalError,
    Generic(String)
}

#[derive(Debug, Display)]
pub enum FusionAuthRefreshJwtError {
    Invalid,
    DoesNotExist,
    InternalError,
    SearchIndex,
    Generic(String)
}

#[derive(Serialize)]
pub struct FusionAuthRefreshJwtRequest<'a> {
    #[serde(rename = "refreshToken")]
    pub refresh_token: &'a str
}

#[derive(Deserialize)]
pub struct FusionAuthRefreshJwtResult {
    pub token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String
}

impl super::FusionAuthClient {
    /// Returns Ok(()) if the access token (JWT) is valid. Returns a FusionAuthValidateJwtError otherwise
    /// or if the JWT can't be validated for any reason.
    pub async fn validate_jwt(&self, access_token: &str) -> Result<(), FusionAuthValidateJwtError> {
        let res = self.client.get(self.build_url("/api/jwt/validate").as_str())
            .header("Authorization", format!("JWT {}", access_token).as_str())
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await;
        match res {
            Ok(resp) => {
                let status = resp.status();

                match status.as_u16() {
                    200 => Ok(()),
                    401 => Err(FusionAuthValidateJwtError::Invalid),
                    500 => Err(FusionAuthValidateJwtError::InternalError),
                    x => Err(FusionAuthValidateJwtError::Generic(format!("Validate JWT Unhandled error: {}", x))),
                }
            }
            Err(err) => Err(FusionAuthValidateJwtError::Generic(format!("Validate JWT {}", err))),
        }
    }

    /// Returns a new access token and refresh token if successful.
    pub async fn refresh_jwt(&self, refresh_token: &str) -> Result<FusionAuthRefreshJwtResult, FusionAuthRefreshJwtError> {
        let res = self.client.post(self.build_url("/api/jwt/refresh").as_str())
            .json(&FusionAuthRefreshJwtRequest{
                refresh_token,
            })
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await;
        
        match res {
            Ok(resp) => {
                let status = resp.status();

                match status.as_u16() {
                    200 => {
                        let body = resp.json::<FusionAuthRefreshJwtResult>().await;
                        match body {
                            Ok(j) => Ok(j),
                            Err(err) => Err(FusionAuthRefreshJwtError::Generic(format!("{}", err))),
                        }
                    },
                    400 | 401 => Err(FusionAuthRefreshJwtError::Invalid),
                    404 => Err(FusionAuthRefreshJwtError::DoesNotExist),
                    500 => Err(FusionAuthRefreshJwtError::InternalError),
                    503 => Err(FusionAuthRefreshJwtError::SearchIndex),
                    x => Err(FusionAuthRefreshJwtError::Generic(format!("Refresh JWT Unhandled error: {}", x))),
                }
            }
            Err(err) => Err(FusionAuthRefreshJwtError::Generic(format!("Refresh JWT {}", err))),            
        }
    }
}