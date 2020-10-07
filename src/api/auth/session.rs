use sqlx;
use sqlx::postgres::PgPool;
use actix_web::{ HttpRequest };

#[derive(Debug)]
pub struct SquadOVSession {
    pub session_id: String,
    pub user: super::SquadOVUser,
    pub access_token: String,
    pub refresh_token: String,
}

pub struct SessionManager {
}

const SESSION_ID_HEADER_KEY : &str = "X-SQUADOV-SESSION-ID";

impl SessionManager {
    pub fn new() -> SessionManager {
        return SessionManager{
        }
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
                u.verified AS \"user_verified\"
            FROM squadov.user_sessions AS us
            INNER JOIN squadov.users AS u
                ON u.id = us.user_id
            WHERE us.id = $1
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
                },
                access_token: x.access_token,
                refresh_token: x.refresh_token,
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

    pub async fn is_logged_in(&self, req : &HttpRequest, pool: &PgPool) -> Result<bool, super::AuthError> {
        match self.get_session_from_request(req, pool).await {
            Ok(x) => match x {
                Some(_) => Ok(true),
                None => Ok(false),
            },
            Err(err) => return Err(super::AuthError::System{
                message: format!("Is Logged In {}", err)
            }),
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
}