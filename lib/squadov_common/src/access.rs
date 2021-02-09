use crate::{
    SquadOvError,
    encrypt::AESEncryptToken,
    stats::StatPermission,
};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use sqlx::{Executor, Postgres};

#[derive(Serialize,Deserialize,Debug,Clone)]
#[serde(rename_all="camelCase")]
pub struct AccessTokenRequest {
    pub full_path: String,
    pub user_uuid: Uuid,
    pub match_uuid: Uuid,
    pub video_uuid: Option<Uuid>,
    pub graphql_stats: Option<Vec<StatPermission>>,
}

pub async fn find_encrypted_access_token<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<Option<Uuid>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT id
            FROM squadov.share_tokens
            WHERE match_uuid = $1 AND user_id = $2
            ",
            match_uuid,
            user_id,
        )
            .fetch_optional(ex)
            .await?
            .map(|x| {
                x.id
            })
    )
}


pub async fn find_encrypted_access_token_from_id<'a, T>(ex: T, id: &Uuid) -> Result<AESEncryptToken, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            AESEncryptToken,
            r#"
            SELECT
                encrypted_token AS "data",
                iv,
                aad,
                tag
            FROM squadov.share_tokens
            WHERE id = $1
            "#,
            id,
        )
            .fetch_one(ex)
            .await?
    )
}


pub async fn store_encrypted_access_token<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64, token: &AESEncryptToken) -> Result<Uuid, SquadOvError>
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
                iv,
                aad,
                tag
            ) VALUES (
                gen_random_uuid(),
                $1,
                $2,
                $3,
                $4,
                $5,
                $6
            )
            RETURNING id
            ",
            match_uuid,
            user_id,
            token.data,
            token.iv,
            token.aad,
            token.tag
        )
            .fetch_one(ex)
            .await?
            .id
    )
}