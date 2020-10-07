use actix_web::{HttpRequest};
use sqlx;
use sqlx::postgres::PgPool;

#[derive(Debug)]
pub struct SquadOVUser {
    pub id: i64,
    pub username: String,
    pub email: String
}

pub struct UserManager {
}

impl UserManager {
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

    pub async fn create_user(&self, user: &SquadOVUser, pool: &PgPool) -> Result<SquadOVUser, sqlx::Error> {
        return sqlx::query_as!(
            SquadOVUser,
            "
            INSERT INTO squadov.users (
                email,
                username
            )
            VALUES (
                $1,
                $2
            )
            RETURNING *
            ",
            user.email,
            user.username,
        ).fetch_one(pool).await;
    }
}


pub fn is_logged_in(req : &HttpRequest) -> bool {
    return false
}