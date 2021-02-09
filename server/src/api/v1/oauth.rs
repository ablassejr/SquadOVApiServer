use actix_web::{web, HttpResponse};
use crate::api;
use crate::api::{
    v1::UserResourcePath,
    auth::SquadOVSession,
};
use std::sync::Arc;
use squadov_common::{SquadOvError};
use uuid::Uuid;
use serde::Deserialize;
use chrono::{Utc, Duration};

#[derive(Deserialize)]
pub struct RiotOAuthCallbackData {
    state: String,
    code: String,
}

pub async fn get_user_rso_auth_url_handler(app : web::Data<Arc<api::ApiApplication>>, path : web::Path<UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    // Generate a new session just for OAuth since the web app will need to know how to fire off a valid
    // API request even if the user isn't logged in in the browser.
    let session = SquadOVSession{
        session_id: Uuid::new_v4().to_string(),
        user: app.users.get_stored_user_from_id(path.user_id, &*app.pool).await?.ok_or(SquadOvError::NotFound)?,
        access_token: String::new(),
        refresh_token: String::new(),
        old_session_id: None,
        is_temp: true,
        share_token: None,
    };
    app.session.store_session(&session, &*app.pool).await?;

    let state = squadov_common::generate_csrf_user_oauth_state(&*app.pool, &session.user.uuid, &session.session_id).await?;
    let rso_url = format!(
        "{base}&state={state}",
        base=app.config.riot.rso_url,
        state=&state,
    );
    Ok(HttpResponse::Ok().json(&rso_url))
}

pub async fn handle_riot_oauth_callback_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<RiotOAuthCallbackData>) -> Result<HttpResponse, SquadOvError> {
    let session_id = squadov_common::check_csrf_user_oauth_state(&*app.pool, &data.state).await?;
    let session = app.session.get_session_from_id(&session_id, &*app.pool).await?.ok_or(SquadOvError::Unauthorized)?;

    // Now we can exchange the auth code for an access token from Riot.
    // We can then use this access token to get account information.
    // TODO: When we have production key access for LoL/TFT we need to use this account information to summoner name info too.
    let access_token = squadov_common::riot::rso::exchange_authorization_code_for_access_token(&app.config.riot.rso_client_id, &app.config.riot.rso_client_secret, &data.code).await?;
    app.rso_itf.request_riot_account_from_access_token(&access_token.access_token, &access_token.refresh_token, Utc::now() + Duration::seconds(access_token.expires_in.into()), session.user.id).await?;
    app.session.delete_session(&session_id, &*app.pool).await?;
    Ok(HttpResponse::Ok().finish())
}