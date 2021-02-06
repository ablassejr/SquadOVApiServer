use crate::SquadOvError;
    use serde::{Serialize, Deserialize};
use uuid::Uuid;
use openssl::symm::{encrypt, decrypt, Cipher};
use sha2::{Sha256, Digest};
use sqlx::{Executor, Postgres};

#[derive(Serialize,Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct AccessTokenRequest {
    pub full_path: String,
    pub user_uuid: Uuid,
    pub match_uuid: Uuid,
    pub video_uuid: Option<Uuid>,
}

impl AccessTokenRequest {
    pub fn generate_token(&self, key: &str, iv: &str) -> Result<String, SquadOvError> {
        let cipher = Cipher::aes_256_gcm();
        let data = encrypt(
            cipher,
            &Sha256::digest(key.as_bytes()).as_slice(),
            Some(iv.as_bytes()),
            &serde_json::to_vec(self)?,
        )?;

        Ok(base64::encode_config(&data, base64::URL_SAFE_NO_PAD))
    }
}

pub async fn find_encrypted_access_token<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<Option<(Uuid, String)>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT id, iv
            FROM squadov.share_tokens
            WHERE match_uuid = $1 AND user_id = $2
            ",
            match_uuid,
            user_id,
        )
            .fetch_optional(ex)
            .await?
            .map(|x| {
                (x.id, x.iv)
            })
    )
}

pub async fn store_encrypted_access_token<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64, enc: &str, iv: &str) -> Result<Uuid, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            INSERT INTO squadov.share_tokens (
                id,
                match_uuid,
                user_id,
                encrypted_token,
                iv
            ) VALUES (
                gen_random_uuid(),
                $1,
                $2,
                $3,
                $4
            )
            RETURNING id
            ",
            match_uuid,
            user_id,
            enc,
            iv,
        )
            .fetch_one(ex)
            .await?
            .id
    )
}