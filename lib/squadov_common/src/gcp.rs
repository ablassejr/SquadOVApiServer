pub mod gcs;

use crate::{SquadOvError, OAuthAccessToken};
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use jsonwebtoken::{Header, Algorithm, EncodingKey};
use std::io::Read;
use chrono::{DateTime, Utc, NaiveDateTime, Duration};
use reqwest;
use reqwest::header;
use futures::executor::block_on;

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
    access_token: Arc<RwLock<Option<OAuthAccessToken>>>
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
    async fn exchange_jwt_for_access_token(&self, jwt: &str) -> Result<OAuthAccessToken, SquadOvError> {
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

    fn construct_oauth_jwt(&self) -> Result<String, SquadOvError> {
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

    async fn refresh_access_token(&mut self) -> Result<(), SquadOvError> {
        // 1) Create a signed JWT 
        // 2) Send the signed JWT to Google eto get an access token
        let jwt = self.construct_oauth_jwt()?;
        let mut access_token = self.exchange_jwt_for_access_token(&jwt).await?;
        let expire_time = Utc::now().timestamp() + access_token.expires_in as i64;
        access_token.expire_time = Some(DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(expire_time, 0), Utc));
        *self.access_token.write()? = Some(access_token);
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
            access_token: Arc::new(RwLock::new(None)),
        };
        http.refresh_access_token().await.unwrap();

        return http
    }

    fn create_http_client(&self) -> Result<reqwest::Client, SquadOvError> {
        let token = match self.access_token.read() {
            Ok(x) => x,
            Err(err) => return Err(SquadOvError::InternalError(format!("Failed to get access token: {}", err)))
        };

        if (*token).is_none() {
            return Err(SquadOvError::InternalError(String::from("Token doesn't exist.")))
        }

        let ref_token = (*token).as_ref().unwrap();
        if ref_token.is_expired(Duration::seconds(0)) {
            return Err(SquadOvError::InternalError(String::from("Token is expired.")));
        }

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
        let http = Arc::new(RwLock::new(GCPHttpAuthClient::new(config).await));

        // Spawn a thread that takes care of refreshing the HTTP auth client's access token.
        let thread_http = http.clone();
        std::thread::spawn(move || {
            let fn_get_token_sleep_time = || {
                let sleep_duration: Duration;
                let mut refresh_token: bool = false;
                let http = thread_http.read();
                if http.is_err() {
                    return (Duration::milliseconds(500), false);
                }

                let http = http.unwrap();

                let token = http.access_token.read();
                if token.is_err() {
                    return (Duration::milliseconds(500), false);
                }
                let token = token.unwrap();

                if (*token).is_some() {
                    let token = token.as_ref().unwrap();

                    // Give ourselves a 5 minute buffer before the expiration to make sure we're always
                    // rocking a valid token. Note that the GCP tokens lost for ~1 hour.
                    let duration = token.duration_until_expiration(Duration::minutes(5));
                    if duration.is_ok() {
                        sleep_duration = duration.unwrap();
                        refresh_token = true;
                    } else {
                        sleep_duration = Duration::milliseconds(500);
                    }
                } else {
                    sleep_duration = Duration::milliseconds(500);
                }
                (sleep_duration, refresh_token)
            };


            loop {
                // Sleep until when the token is going to be expired. Note that errors should not
                // cause us to exist out of the loop. We should log it and continue and hopefully? recover.
                let (duration, do_refresh) = fn_get_token_sleep_time();
                log::info!("Pending GCP Token Refresh: {:?} - {}", duration, do_refresh);
                std::thread::sleep(duration.to_std().unwrap_or(std::time::Duration::from_millis(500)));

                if do_refresh {
                    log::info!("Perform GCP Token Refresh");
                    let http = thread_http.write();
                    if http.is_err() {
                        log::warn!("Failed to get write lock.");
                        continue;
                    }

                    {
                        let mut http = http.unwrap();
                        let result = block_on(http.refresh_access_token());
                        if result.is_err() {
                            log::warn!("Failed to refresh GCP access token: {:?}", result.err());
                        }
                    }
                }
            }
        });

        return GCPClient{
            gcs_client: gcs::GCSClient::new(http.clone()),
        }
    }

    pub fn gcs(&self) -> &gcs::GCSClient {
        &self.gcs_client
    }
}