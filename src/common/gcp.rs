pub mod gcs;

use crate::common;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use jsonwebtoken::{Header, Algorithm, EncodingKey};
use std::io::Read;
use chrono::{DateTime, Utc, NaiveDateTime};
use reqwest;
use reqwest::header;

#[derive(Deserialize,Debug,Clone)]
pub struct GCPConfig {
    pub enabled: bool,
    pub service_account_key: String
}

#[derive(Deserialize)]
pub struct GCPServiceAccountJson {
/*
    r#type: String,
    project_id: String,
    private_key_id: String,
*/
    private_key: String,
    client_email: String,
/*
    client_id: String,
    auth_uri: String,
    token_uri: String,
    auth_provider_x509_cert_url: String,
    client_x509_cert_url: String
*/
}

#[derive(Serialize)]
struct GCPTokenRequest {
    grant_type: String,
    assertion: String
}

pub struct GCPHttpAuthClient {
    pub credentials: GCPServiceAccountJson,
    access_token: RwLock<Option<common::OAuthAccessToken>>
}

#[derive(Serialize)]
struct GCPJwtClaims {
    iss: String,
    scope: String,
    aud: String,
    exp: i64,
    iat: i64
}

impl GCPHttpAuthClient {
    async fn exchange_jwt_for_access_token(&self, jwt: &str) -> Result<common::OAuthAccessToken, common::SquadOvError> {
        let client = reqwest::Client::new();
        Ok(
            client.post("https://oauth2.googleapis.com/token")
                .json(&GCPTokenRequest{
                    grant_type: String::from("urn:ietf:params:oauth:grant-type:jwt-bearer"),
                    assertion: String::from(jwt),
                })
                .send()
                .await?
                .json()
                .await?
        )
    }

    fn construct_oauth_jwt(&self) -> Result<String, common::SquadOvError> {
        let current_unix_time = Utc::now().timestamp();
        let expire_unix_time = current_unix_time + 3600;

        let claims = GCPJwtClaims{
            iss: self.credentials.client_email.clone(),
            scope: String::from("https://www.googleapis.com/auth/devstorage.read_write"),
            aud: String::from("https://oauth2.googleapis.com/token"),
            exp: expire_unix_time,
            iat: current_unix_time
        };
        let header = Header::new(Algorithm::RS256);
        let key = EncodingKey::from_rsa_pem(self.credentials.private_key.as_bytes())?;
        Ok(jsonwebtoken::encode(&header, &claims, &key)?)
    }

    async fn refresh_access_token(&mut self) -> Result<(), common::SquadOvError> {
        let mut token = match self.access_token.write() {
            Ok(x) => x,
            Err(err) => return Err(common::SquadOvError::InternalError(format!("Refresh access token failed to obtain lock: {}", err)))
        };

        // 1) Create a signed JWT 
        // 2) Send the signed JWT to Google eto get an access token
        let jwt = self.construct_oauth_jwt()?;
        let mut access_token = self.exchange_jwt_for_access_token(&jwt).await?;
        let expire_time = Utc::now().timestamp() + access_token.expires_in as i64;
        access_token.expire_time = Some(DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(expire_time, 0), Utc));
        *token = Some(access_token);
        Ok(())
    }

    async fn new(config: &GCPConfig) -> GCPHttpAuthClient {
        // Load service account credentials.
        let mut file = std::fs::File::open(&config.service_account_key).unwrap();
        let mut key_data = String::new();
        file.read_to_string(&mut key_data).unwrap();
        let credentials: GCPServiceAccountJson = serde_json::from_str(&key_data).unwrap();

        // Need to create an HTTP client that is properly authenticated with
        // Google Cloud's APIs.
        let mut http = GCPHttpAuthClient{
            credentials: credentials,
            access_token: RwLock::new(None),
        };
        http.refresh_access_token().await.unwrap();
        return http
    }

    fn create_http_client(&self) -> Result<reqwest::Client, common::SquadOvError> {
        let token = match self.access_token.read() {
            Ok(x) => x,
            Err(err) => return Err(common::SquadOvError::InternalError(format!("Failed to get access token: {}", err)))
        };

        if (*token).is_none() {
            return Err(common::SquadOvError::InternalError(String::from("Token doesn't exist.")))
        }

        let ref_token = (*token).as_ref().unwrap();
        let mut headers = header::HeaderMap::new();
        let access_token = format!("{} {}", ref_token.token_type, ref_token.access_token);
        headers.insert(header::AUTHORIZATION, header::HeaderValue::from_str(&access_token)?);

        Ok(reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()?)
    }
}

pub struct GCPClient {
    gcs_client: gcs::GCSClient
}

impl GCPClient {
    pub async fn new(config: &GCPConfig) -> GCPClient {
        let http = Arc::new(GCPHttpAuthClient::new(config).await);
        return GCPClient{
            gcs_client: gcs::GCSClient::new(http.clone()),
        }
    }

    pub fn gcs(&self) -> &gcs::GCSClient {
        &self.gcs_client
    }
}