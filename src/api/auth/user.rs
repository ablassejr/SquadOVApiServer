use actix_web::{web, HttpResponse, HttpRequest, Result};
use serde::{Deserialize};
use crate::logged_error;
use sqlx;
use sqlx::postgres::PgPool;
use derive_more::{Display};

#[derive(Debug)]
pub struct SquadOVUser {
    pub id: i64,
    pub username: String,
    pub email: String
}

pub struct UserManager {
}

#[derive(Debug, Display)]
pub enum UserError {
    DbError(sqlx::Error)
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

#[derive(Deserialize)]
pub struct RegisterData {
    username: String,
    email: String,
    password: String,
}

async fn logout() -> Result<(), super::AuthError> {
    return Ok(())
}

async fn register(data: RegisterData) -> Result<(), super::AuthError> {
    return Ok(())
}

async fn forgot_pw() -> Result<(), super::AuthError> {
    return Ok(())
}

pub fn is_logged_in(req : &HttpRequest) -> bool {
    return false
}

/// Handles collecting the user data and passing it to FusionAuth for registration.
/// 
/// We expect only three parameters to be passed via the POST body: 
/// * Username
/// * Password
/// * Email
///
/// Possible Responses:
/// * 200 - Registration succeeded.
/// * 400 - If a user is already logged in.
/// * 500 - Registration failed due to other reasons.
pub async fn register_handler(data : web::Json<RegisterData>, req : HttpRequest) -> Result<HttpResponse, super::AuthError> {
    if is_logged_in(&req) {
        return logged_error!(super::AuthError::BadRequest);
    }

    match register(data.into_inner()).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}

/// Logouts the user with SquadOV and FusionAuth.
/// 
/// Possible Responses:
/// * 200 - Logout succeded.
/// * 500 - Logout failed.
pub async fn logout_handler(req: HttpRequest) -> Result<HttpResponse, super::AuthError> {
    if !is_logged_in(&req) {
        // They weren't logged in to begin with so it's OK to just tell them
        // they logged out successfully.
        return Ok(HttpResponse::Ok().finish());
    }

    match logout().await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}

/// Starts the password reset flow. Note that no error is given if
/// the specified user doesn't exist.
/// 
/// Possible Responses:
/// * 200 - Success.
/// * 500 - Internal error (no email sent).
pub async fn forgot_pw_handler() -> Result<HttpResponse, super::AuthError> {
    match forgot_pw().await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}