use sqlx;
use sqlx::postgres::PgPool;
use serde::Serialize;
use uuid::Uuid;
use std::clone::Clone;

#[derive(Debug, Serialize, Clone)]
pub struct SquadOVUser {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub verified: bool,
    pub uuid: Uuid,
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
            SELECT *
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
            SELECT *
            FROM squadov.users
            WHERE id = $1
            ",
            id
        ).fetch_optional(pool).await;
    }

    pub async fn get_stored_user_from_uuid(&self, uuid: &Uuid, pool: &PgPool) -> Result<Option<SquadOVUser>, sqlx::Error> {
        return sqlx::query_as!(
            SquadOVUser,
            "
            SELECT *
            FROM squadov.users
            WHERE uuid = $1
            ",
            uuid
        ).fetch_optional(pool).await;
    }

    pub async fn create_user(&self, user: &SquadOVUser, pool: &PgPool) -> Result<SquadOVUser, sqlx::Error> {
        return sqlx::query_as!(
            SquadOVUser,
            "
            INSERT INTO squadov.users (
                email,
                username,
                verified
            )
            VALUES (
                $1,
                $2,
                $3
            )
            RETURNING *
            ",
            user.email,
            user.username,
            user.verified,
        ).fetch_one(pool).await;
    }
}