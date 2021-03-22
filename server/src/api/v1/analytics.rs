use squadov_common::SquadOvError;
use actix_web::{web, HttpResponse, HttpRequest};
use std::sync::Arc;
use crate::api;
use crate::api::auth::SquadOVSession;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ReferralQuery {
    #[serde(rename="ref")]
    pub referral_code: Option<String>,
}

impl api::ApiApplication {
    pub async fn get_user_referral_code(&self, user_id: i64) -> Result<Option<String>, SquadOvError> {
        Ok(
            sqlx::query!(
                r#"
                SELECT code AS "code!"
                FROM squadov.referral_codes
                WHERE user_id = $1
                "#,
                user_id,
            )
                .fetch_optional(&*self.pool)
                .await?
                .map(|x| {
                    x.code
                })
        )
    }

    pub async fn create_referral_code(&self, code: &str, user_id: Option<i64>) -> Result<String, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                INSERT INTO squadov.referral_codes (
                    id,
                    code,
                    description,
                    user_id,
                    tm
                )
                VALUES (
                    gen_random_uuid(),
                    $1,
                    '',
                    $2,
                    NOW()
                )
                RETURNING code
                ",
                code.to_lowercase(),
                user_id,
            )
                .fetch_one(&*self.pool)
                .await?
                .code
        )
    }

    pub async fn mark_referral_visit(&self, code: &str) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.referral_visits (
                code,
                tm
            )
            SELECT c.id, NOW()
            FROM squadov.referral_codes AS c
            WHERE c.code = $1
            ",
            code,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    pub async fn mark_referral_download(&self, code: &str) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.referral_downloads (
                code,
                tm
            )
            SELECT c.id, NOW()
            FROM squadov.referral_codes AS c
            WHERE c.code = $1
            ",
            code,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
}

pub async fn public_landing_visit_handler(app : web::Data<Arc<api::ApiApplication>>, query: web::Query<ReferralQuery>) -> Result<HttpResponse, SquadOvError> {
    if query.referral_code.is_some() {
        match app.mark_referral_visit(query.referral_code.as_ref().unwrap()).await {
            Ok(_) => (),
            Err(err) => log::warn!("Failed to mark referral visit: {:?}", err)
        };
    }
    Ok(HttpResponse::NoContent().finish())
}

pub async fn public_landing_download_handler(app : web::Data<Arc<api::ApiApplication>>, query: web::Query<ReferralQuery>) -> Result<HttpResponse, SquadOvError> {
    if query.referral_code.is_some() {
        match app.mark_referral_download(query.referral_code.as_ref().unwrap()).await {
            Ok(_) => (),
            Err(err) => log::warn!("Failed to mark referral download: {:?}", err)
        };
    }
    Ok(HttpResponse::NoContent().finish())
}

pub async fn get_user_me_referral_link_handler(app : web::Data<Arc<api::ApiApplication>>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    let mut code = app.get_user_referral_code(session.user.id).await?;
    if code.is_none() {
        code = Some(app.create_referral_code(&session.user.username, Some(session.user.id)).await?);
    }

    let code = code.unwrap();
    Ok(HttpResponse::Ok().json(format!(
        "{}/?ref={}",
        &app.config.squadov.landing_url,
        code
    )))
}