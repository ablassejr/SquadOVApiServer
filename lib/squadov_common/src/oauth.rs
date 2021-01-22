use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Duration};
use crate::SquadOvError;
use sqlx::{Executor, Postgres, PgPool};
use uuid::Uuid;
use openssl::{
    symm::{encrypt_aead, decrypt_aead, Cipher},
    rand::rand_bytes,
};
use sha2::{Sha256, Digest};

#[derive(Deserialize)]
pub struct OAuthAccessToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i32,
    pub expire_time: Option<DateTime<Utc>>
}

impl OAuthAccessToken {
    pub fn is_expired(&self, buffer_duration: Duration) -> bool {
        if self.expire_time.is_none() {
            true
        } else {
            let expire_time = self.expire_time.unwrap();
            Utc::now() + buffer_duration > expire_time
        }
    }

    pub fn duration_until_expiration(&self, buffer_duration: Duration) -> Result<Duration, SquadOvError> {
        if self.expire_time.is_none() {
            Err(SquadOvError::NotFound)
        } else {
            let target = self.expire_time.unwrap() - buffer_duration;
            let now = Utc::now();
            Ok(if target < now {
                Duration::milliseconds(0)
            } else {
                target - now
            })
        }
    }
}

#[derive(Serialize, Deserialize)]
struct OAuthAuthorizeState {
    session_id: String,
    tm: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
struct OAuthEncryptedState {
    iv: String,
    data: String,
    tag: String,
    aad: String,
}

pub async fn generate_csrf_user_oauth_state<'a, T>(ex: T, uuid: &Uuid, session_id: &str) -> Result<String, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let data = sqlx::query!(
        "
        SELECT local_encryption_key
        FROM squadov.users
        WHERE uuid = $1
        ",
        uuid
    )
        .fetch_one(ex)
        .await?;

    let state = OAuthAuthorizeState{
        session_id: String::from(session_id),
        tm: Utc::now(),
    };

    encrypt_oauth_authorize_state(&state, &uuid.to_string(), &data.local_encryption_key)
}

fn encrypt_oauth_authorize_state(st: &OAuthAuthorizeState, aad: &str, key: &str) -> Result<String, SquadOvError> {
    let raw_unencrypted = serde_json::to_string(st)?;
    
    let mut iv = [0; 256];
    rand_bytes(&mut iv)?;

    let mut tag: Vec<u8> = vec![0; 16];
    let cipher = Cipher::aes_256_gcm();
    let data = encrypt_aead(
        cipher,
        &Sha256::digest(key.as_bytes()).as_slice(),
        Some(&iv),
        aad.as_bytes(),
        raw_unencrypted.as_bytes(),
        &mut tag
    )?;

    let state = OAuthEncryptedState {
        iv: base64::encode(&iv.to_vec()),
        data: base64::encode(&data),
        aad: aad.to_string(),
        tag: base64::encode(&tag),
    };
    
    Ok(base64::encode(&serde_json::to_vec(&state)?))
}

async fn decrypt_oauth_authorize_state(ex: &PgPool, state: &str) -> Result<OAuthAuthorizeState, SquadOvError> {
    let enc_state: OAuthEncryptedState = serde_json::from_slice(&base64::decode(state.as_bytes())?)?;

    let data = sqlx::query!(
        "
        SELECT local_encryption_key
        FROM squadov.users
        WHERE uuid = $1
        ",
        Uuid::parse_str(&enc_state.aad)?
    )
        .fetch_one(ex)
        .await?;

    let cipher = Cipher::aes_256_gcm();
    let unencrypted = decrypt_aead(
        cipher,
        &Sha256::digest(data.local_encryption_key.as_bytes()).as_slice(),
        Some(&base64::decode(enc_state.iv.as_bytes())?),
        enc_state.aad.as_bytes(),
        &base64::decode(&enc_state.data)?,
        &base64::decode(&enc_state.tag)?,
    )?;

    Ok(serde_json::from_slice(&unencrypted)?)
}

// Returns session id
pub async fn check_csrf_user_oauth_state(ex: &PgPool, state: &str) -> Result<String, SquadOvError> {
    let state = decrypt_oauth_authorize_state(&*ex, state).await?;

    // We must check that it's been a reasonable amount of time since the request was made (aka not like 2 days later or something).
    if Utc::now() > (state.tm + chrono::Duration::minutes(30)) {
        return Err(SquadOvError::Unauthorized);
    }

    // Also check that the session is valid.
    let exists = super::is_valid_temporary_squadov_session(&*ex, &state.session_id).await?;
    if !exists {
        return Err(SquadOvError::Unauthorized);
    }

    Ok(state.session_id)
}