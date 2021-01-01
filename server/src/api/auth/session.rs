use sqlx;
use sqlx::postgres::PgPool;
use actix_web::{ HttpRequest, FromRequest, dev, Error };
use actix_web::error::ErrorBadRequest;
use futures_util::future::{ok, err, Ready};
use squadov_common;
use crate::api::fusionauth;
use uuid::Uuid;
use crate::logged_error;
use serde::Serialize;
use std::clone::Clone;

#[derive(Debug, Serialize, Clone)]
pub struct SquadOVSession {
    pub session_id: String,
    pub user: super::SquadOVUser,
    pub access_token: String,
    pub refresh_token: String,
    pub old_session_id: Option<String>
}

impl FromRequest for SquadOVSession {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req : &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        let extensions = req.extensions();
        match extensions.get::<SquadOVSession>() {
            Some(x) => ok(x.clone()),
            None => err(ErrorBadRequest("No session available."))
        }
    }
}

pub struct SessionManager {
}

const SESSION_ID_HEADER_KEY : &str = "x-squadov-session-id";

impl SessionManager {
    pub fn new() -> SessionManager {
        return SessionManager{
        }
    }

    pub async fn delete_session(&self, id : &str, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "
            DELETE FROM squadov.user_sessions
            WHERE id = $1 OR old_id = $1
            ",
            id,
        )
            .execute(pool)
            .await?;
        
        return Ok(())
    }

    pub async fn get_session_from_id(&self, id : &str, pool: &PgPool) -> Result<Option<SquadOVSession>, sqlx::Error> {
        let ret = sqlx::query!(
            "
            SELECT
                us.id AS \"session_id\",
                us.access_token,
                us.refresh_token,
                u.id AS \"user_id\",
                u.username AS \"user_username\",
                u.email AS \"user_email\",
                u.verified AS \"user_verified\",
                u.uuid AS \"user_uuid\"
            FROM squadov.user_sessions AS us
            INNER JOIN squadov.users AS u
                ON u.id = us.user_id
            WHERE us.id = $1 OR us.old_id = $1
            ",
            id,
        ).fetch_optional(pool).await?;

        match ret {
            Some(x) => Ok(Some(SquadOVSession{
                session_id: x.session_id,
                user: super::SquadOVUser{
                    id: x.user_id,
                    username: x.user_username,
                    email: x.user_email,
                    verified: x.user_verified,
                    uuid: x.user_uuid,
                },
                access_token: x.access_token,
                refresh_token: x.refresh_token,
                old_session_id: None,
            })),
            None => Ok(None),
        }
    }

    pub async fn get_session_from_request(&self, req : &HttpRequest, pool: &PgPool) -> Result<Option<SquadOVSession>, sqlx::Error> {
        match req.headers().get(SESSION_ID_HEADER_KEY) {
            Some(id) => self.get_session_from_id(id.to_str().unwrap(), pool).await,
            None => Ok(None),
        }
    }

    pub async fn store_session(&self, session: &SquadOVSession, pool: &PgPool) -> Result<(), sqlx::Error> {
        // Store in database
        sqlx::query!(
            "
            INSERT INTO squadov.user_sessions (
                id,
                access_token,
                refresh_token,
                user_id
            )
            VALUES (
                $1,
                $2,
                $3,
                $4
            )
            ",
            session.session_id,
            &session.access_token,
            &session.refresh_token,
            session.user.id
        )
            .execute(pool)
            .await?;
        
        return Ok(())
    }

    pub async fn refresh_session(&self, old_id: &str, session: &SquadOVSession, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "
            UPDATE squadov.user_sessions
            SET id = $1,
                access_token = $2,
                refresh_token = $3,
                old_id = $4
            WHERE id = $4
            ",
            session.session_id,
            &session.access_token,
            &session.refresh_token,
            old_id,
        )
            .execute(pool)
            .await?;
        
        return Ok(())
    }
}

impl crate::api::ApiApplication {
    pub async fn is_session_valid(&self, session: &SquadOVSession) -> Result<bool, squadov_common::SquadOvError> {
        match self.clients.fusionauth.validate_jwt(&session.access_token).await {
            Ok(_) => Ok(true),
            Err(err) => match err {
                fusionauth::FusionAuthValidateJwtError::Invalid => Ok(false),
                _ => Err(squadov_common::SquadOvError::InternalError(format!("Validate JWT {}", err)))
            }
        }
    }

    pub async fn refresh_session_if_necessary(&self, session: SquadOVSession, force: bool) -> Result<SquadOVSession, squadov_common::SquadOvError> {
        // Check if the session is expired (as determined by FusionAuth).
        // If it is expired (or close to it), generate a new session ID and use the refresh token to get a new access token.
        // If it isn't expired, return the session as is.
        let mut session = session;
        let expired = !self.is_session_valid(&session).await?;

        if expired || force {
            let new_token = match self.clients.fusionauth.refresh_jwt(&session.refresh_token).await {
                Ok(t) => t,
                Err(err) => return logged_error!(squadov_common::SquadOvError::InternalError(format!("Refresh JWT {}", err)))
            };

            let old_id = session.session_id;
            session.session_id = Uuid::new_v4().to_string();
            session.access_token = new_token.token;
            session.refresh_token = new_token.refresh_token;

            let mut tx = self.pool.begin().await?;
            squadov_common::analytics::mark_active_user_session(&mut tx, session.user.id).await?;
            tx.commit().await?;

            match self.session.refresh_session(&old_id, &session, &self.pool).await {
                Ok(_) => (),
                Err(err) => return logged_error!(squadov_common::SquadOvError::InternalError(format!("Refresh Session {}", err)))
            };

            session.old_session_id = Some(old_id);
        }

        Ok(session)
    }

    pub async fn refresh_and_obtain_valid_session_from_request(&self, req : &HttpRequest) -> Result<Option<SquadOVSession>, squadov_common::SquadOvError> {
        let session = match self.session.get_session_from_request(req, &self.pool).await {
            Ok(x) => match x {
                Some(y) => y,
                None => return Ok(None),
            },
            Err(err) => return logged_error!(squadov_common::SquadOvError::InternalError(format!("Refresh And Obtain Session {}", err))),
        };

        let session = self.refresh_session_if_necessary(session, false).await?;
        
        return Ok(Some(session));
    }

    pub async fn is_logged_in(&self, req : &HttpRequest) -> Result<bool, squadov_common::SquadOvError> {
        match self.refresh_and_obtain_valid_session_from_request(req).await {
            Ok(x) => match x {
                Some(_) => Ok(true),
                None => Ok(false),
            },
            Err(err) => return Err(squadov_common::SquadOvError::InternalError(format!("Is Logged In Error {}", err))),
        }
    }

    pub async fn logout(&self, session: &SquadOVSession) -> Result<(),  squadov_common::SquadOvError> {
        // Logout from FusionAuth AND delete the session from our database.
        // Both operations should be done regardless of whether the other one is successful.
        let fa_result = self.clients.fusionauth.logout(&session.refresh_token).await;
        let db_result = self.session.delete_session(&session.session_id, &self.pool).await;

        match fa_result {
            Ok(_) => (),
            Err(err) => return Err(squadov_common::SquadOvError::InternalError(format!("Failed to logout (FA): {}", err)))
        };

        match db_result {
            Ok(_) => (),
            Err(err) => return Err(squadov_common::SquadOvError::InternalError(format!("Failed to logout (DB): {}", err)))
        }

        return Ok(());
    }
}