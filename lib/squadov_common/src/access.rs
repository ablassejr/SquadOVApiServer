use crate::{
    SquadOvError,
    encrypt::AESEncryptToken,
    stats::StatPermission,
    encrypt::{
        AESEncryptRequest,
        squadov_encrypt,
    },
};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use sqlx::{Transaction, Executor, Postgres};
use convert_case::{Case, Casing};
use chrono::{DateTime, Utc};

// A full-blown access token system where the access token is self contained
// in the information we need to verify what permissions it has and when it expires.
#[derive(Serialize,Deserialize,Debug,Clone)]
#[serde(rename_all="camelCase")]
pub struct AccessToken {
    pub expires: Option<DateTime<Utc>>,
    // HTTP Methods that we allow this access token to perform. None = all methods. Empty vec = no methods.
    pub methods: Option<Vec<String>>,
    // API Paths that we allow this access token to access. None = all paths. Empty vec = no methods.
    pub paths: Option<Vec<String>>,
    // Who we're using the app as.
    pub user_id: Option<i64>,
}

impl AccessToken {
    pub fn encrypt(&self, key: &str) -> Result<String, SquadOvError> {
        let encryption_request = AESEncryptRequest{
            data: serde_json::to_vec(self)?,
            aad: vec![],
        };

        let encryption_token = squadov_encrypt(encryption_request, key)?;
        Ok(encryption_token.to_string())
    }

    pub fn check_method(&self, method: &str) -> bool {
        if let Some(methods) = self.methods.as_ref() {
            methods.contains(&String::from(method))
        } else {
            true
        }
    }

    pub fn check_path(&self, path: &str) -> bool {
        if let Some(paths) = self.paths.as_ref() {
            for p in paths {
                if path.contains(p) {
                    return true;
                }
            }

            false
        } else {
            true
        }
    }
}

// Some janky system where we can request access to certain aspects of the app using an access token.
#[derive(Serialize,Deserialize,Debug,Clone)]
#[serde(rename_all="camelCase")]
pub struct AccessTokenRequest {
    pub full_path: String,
    pub user_uuid: Uuid,
    pub match_uuid: Option<Uuid>,
    pub video_uuid: Option<Uuid>,
    #[serde(default)]
    pub bulk_video_uuids: Vec<Uuid>,
    pub clip_uuid: Option<Uuid>,
    pub graphql_stats: Option<Vec<StatPermission>>,
}

pub async fn delete_encrypted_access_token_for_match_user<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.share_tokens
        WHERE match_uuid = $1 AND user_id = $2
        ",
        match_uuid,
        user_id,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn find_encrypted_access_token_for_match_user<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<Option<Uuid>, SquadOvError>
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

pub async fn delete_encrypted_access_token_for_clip_user<'a, T>(ex: T, clip_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.share_tokens
        WHERE clip_uuid = $1 AND user_id = $2
        ",
        clip_uuid,
        user_id,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn find_encrypted_access_token_for_clip_user<'a, T>(ex: T, clip_uuid: &Uuid, user_id: i64) -> Result<Option<Uuid>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT id
            FROM squadov.share_tokens
            WHERE clip_uuid = $1 AND user_id = $2
            ",
            clip_uuid,
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

pub async fn find_encrypted_access_token_from_flexible_id<'a, T>(ex: T, id: &str) -> Result<AESEncryptToken, SquadOvError>
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
            WHERE id::VARCHAR = $1 OR friendly_name = $1
            "#,
            id,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_share_url_identifier_for_id<'a, T>(ex: T, id: &Uuid) -> Result<String, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT COALESCE(friendly_name, id::VARCHAR) AS "id!"
            FROM squadov.share_tokens
            WHERE id = $1
            "#,
            id,
        )
            .fetch_one(ex)
            .await?
            .id
    )
}

pub async fn generate_friendly_share_token(tx: &mut Transaction<'_, Postgres>, id: &Uuid) -> Result<Option<String>, SquadOvError> {
    // Try to generate a friendly share token within a few iterations.
    // If it doesn't work then say fuck it and move on. This is for
    // aesthetics and doesn't matter that much.
    for _i in 0i32..5 {
        let nm = petname::Petnames::large().generate_one(3, "-").to_case(Case::Pascal);
        match sqlx::query!(
            "
            UPDATE squadov.share_tokens
            SET friendly_name = $2
            WHERE id = $1
            ",
            id,
            &nm
        )
            .execute(&mut *tx)
            .await {
            Ok(_) => (),
            Err(err) => match SquadOvError::from(err) {
                SquadOvError::Duplicate => continue,
                x => return Err(x),
            }
        }
        return Ok(Some(nm));
    }
    
    Ok(None)
}

pub async fn store_encrypted_access_token_for_match_user<'a, T>(ex: T, match_uuid: &Uuid, video_uuids: &[Uuid], user_id: i64, token: &AESEncryptToken) -> Result<Uuid, SquadOvError>
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
                tag,
                bulk_video_uuids
            ) VALUES (
                gen_random_uuid(),
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7
            )
            RETURNING id
            ",
            match_uuid,
            user_id,
            token.data,
            token.iv,
            token.aad,
            token.tag,
            video_uuids
        )
            .fetch_one(ex)
            .await?
            .id
    )
}

pub async fn store_encrypted_access_token_for_clip_user<'a, T>(ex: T, clip_uuid: &Uuid, user_id: i64, token: &AESEncryptToken) -> Result<Uuid, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            INSERT INTO squadov.share_tokens (
                id,
                clip_uuid,
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
            clip_uuid,
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

pub async fn check_user_has_access_to_match_vod_from_user<'a, T>(ex: T, dest_user_id: i64, source_user_id: Option<i64>, match_uuid: Option<Uuid>, video_uuid: Option<Uuid>) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    if let Some(user_id) = source_user_id {
        if user_id == dest_user_id {
            return Ok(true);
        }
    }

    Ok(
        sqlx::query!(
            r#"
            WITH RECURSIVE access_cte AS (
                SELECT vau.*
                FROM squadov.view_share_connections_access_users AS vau
                WHERE ($3::UUID IS NULL OR vau.match_uuid = $3)
                    AND ($4::UUID IS NULL OR vau.video_uuid = $4)
                    AND vau.user_id = $1
                UNION
                SELECT vau.*
                FROM squadov.view_share_connections_access_users AS vau
                INNER JOIN access_cte AS ac
                    ON ac.parent_connection_id = vau.id
            )
            SELECT EXISTS (
                SELECT 1
                FROM access_cte
                WHERE $2::BIGINT IS NULL OR source_user_id = $2
            ) AS "exists!"
            "#,
            dest_user_id,
            source_user_id,
            match_uuid,
            video_uuid,
        )
            .fetch_one(ex)
            .await?
            .exists
    )
}