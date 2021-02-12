use sqlx;
use sqlx::postgres::PgPool;
use serde::Serialize;
use uuid::Uuid;
use std::clone::Clone;
use squadov_common::SquadOvError;

#[derive(Debug, Serialize, Clone)]
pub struct SquadOVUser {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub verified: bool,
    pub uuid: Uuid,
    #[serde(skip_serializing)]
    pub is_test: bool,
    #[serde(skip_serializing)]
    pub is_admin: bool
}

pub struct UserManager {
}

impl UserManager {
    pub async fn mark_user_email_verified_from_email(&self, email: &str, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "
            UPDATE squadov.users
            SET verified = TRUE
            WHERE email = $1
            ",
            email
        )
            .execute(pool)
            .await?;
        return Ok(())
    }

    pub async fn get_stored_user_from_email(&self, email : &str, pool: &PgPool) -> Result<Option<SquadOVUser>, sqlx::Error> {
        return sqlx::query_as!(
            SquadOVUser,
            "
            SELECT
                id,
                username,
                email,
                verified,
                uuid,
                is_test,
                is_admin
            FROM squadov.users
            WHERE email = $1
            ",
            email
        ).fetch_optional(pool).await;
    }

    pub async fn get_stored_user_from_id(&self, id : i64, pool: &PgPool) -> Result<Option<SquadOVUser>, sqlx::Error> {
        return sqlx::query_as!(
            SquadOVUser,
            "
            SELECT
                id,
                username,
                email,
                verified,
                uuid,
                is_test,
                is_admin
            FROM squadov.users
            WHERE id = $1
            ",
            id
        ).fetch_optional(pool).await;
    }

    pub async fn get_stored_user_from_uuid(&self, uuid: &Uuid, pool: &PgPool) -> Result<Option<SquadOVUser>, SquadOvError> {
        Ok(sqlx::query_as!(
            SquadOVUser,
            "
            SELECT
                id,
                username,
                email,
                verified,
                uuid,
                is_test,
                is_admin
            FROM squadov.users
            WHERE uuid = $1
            ",
            uuid
        ).fetch_optional(pool).await?)
    }

    pub async fn create_user(&self, user: &SquadOVUser, pool: &PgPool) -> Result<SquadOVUser, sqlx::Error> {
        return sqlx::query_as!(
            SquadOVUser,
            "
            INSERT INTO squadov.users (
                email,
                username,
                verified,
                local_encryption_key
            )
            SELECT $1, $2, $3, encode(digest(gen_random_bytes(16), 'sha256'), 'base64')
            RETURNING
                id,
                username,
                email,
                verified,
                uuid,
                is_test,
                is_admin
            ",
            user.email,
            user.username,
            user.verified,
        ).fetch_one(pool).await;
    }
}