use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use crate::api::{
    auth::{SquadOVSession, SquadOvMachineId},
    ApiApplication,
    v1::GenericMatchPathInput,
};
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
    vod::db as vdb,
};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct CreateCustomEventHandler {
    pub match_uuid: Option<Uuid>,
    pub video_uuid: Option<Uuid>,
    pub tm: DateTime<Utc>,
    pub label: Option<String>,
    pub icon: Option<String>,
}

#[derive(Deserialize)]
pub struct EventIdPath {
    pub event_id: i64,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct CustomEvent {
    pub match_uuid: Uuid,
    pub user_id: i64,
    pub username: String,
    pub tm: DateTime<Utc>,
    pub label: String,
    pub icon: String,
    pub event_id: i64,
}

pub async fn create_new_custom_match_event_handler(app : web::Data<Arc<ApiApplication>>, data: web::Json<CreateCustomEventHandler>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    if data.match_uuid.is_none() && data.video_uuid.is_none() {
        return Err(SquadOvError::BadRequest);
    }

    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    // If video_uuid is set, we're creating a temporary event that we need to *eventually* associate with a match (done with a DB trigger for simplicity).
    // If match_uuid is set, we're creating an event that is immediately associated with a match.
    sqlx::query!(
        r#"
        INSERT INTO squadov.match_custom_events (
            match_uuid,
            video_uuid,
            user_id,
            tm,
            label,
            icon
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            COALESCE($5, 'Event ' || (SELECT COUNT(*) FROM squadov.match_custom_events WHERE match_uuid = $1 OR video_uuid = $2)),
            COALESCE($6, 'default')
        )
        "#,
        data.match_uuid,
        data.video_uuid,
        session.user.id,
        data.tm,
        data.label,
        data.icon,
    )
        .execute(&*app.pool)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

pub async fn get_accessible_match_custom_events_handler(app : web::Data<Arc<ApiApplication>>, match_path: web::Path<GenericMatchPathInput>, req: HttpRequest, machine_id: Option<web::Header<SquadOvMachineId>>) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    // Use the 'find_accessible_vods_in_match_for_user' since that already takes care of all the sharing stuff.
    // This way sharing VODs is equivalent to sharing custom events (for better or worse).
    let vods = vdb::find_accessible_vods_in_match_for_user(&*app.pool, &match_path.match_uuid, session.user.id, machine_id.map(|x| { x.id.clone() }).unwrap_or(String::new()).as_str()).await?;
    let user_uuids: Vec<Uuid> = vods.iter()
        .filter(|x| { x.user_uuid.is_some() })
        .map(|x| { x.user_uuid.unwrap().clone() })
        .collect();

    Ok(HttpResponse::Ok().json(
        sqlx::query_as!(
            CustomEvent,
            r#"
            SELECT
                mce.match_uuid AS "match_uuid!",
                u.id AS "user_id!",
                u.username AS "username!",
                mce.tm,
                mce.label,
                mce.icon,
                mce.event_id
            FROM squadov.match_custom_events AS mce
            INNER JOIN squadov.users AS u
                ON u.id = mce.user_id
            WHERE mce.match_uuid = $1
                AND u.uuid = ANY($2)
            "#,
            &match_path.match_uuid,
            &user_uuids,
        )
            .fetch_all(&*app.pool)
            .await?
    ))
}

impl ApiApplication {
    pub async fn get_custom_event_owner_user_id(&self, event_id: i64) -> Result<i64, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT user_id
                FROM squadov.match_custom_events
                WHERE event_id = $1
                ",
                event_id
            )
                .fetch_one(&*self.pool)
                .await?
                .user_id
        )
    }

}

pub async fn edit_match_custom_event_handler(app : web::Data<Arc<ApiApplication>>, event_path: web::Path<EventIdPath>, data: web::Json<CreateCustomEventHandler>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;
    let event_owner_user_id = app.get_custom_event_owner_user_id(event_path.event_id).await?;
    if event_owner_user_id != session.user.id {
        return Err(SquadOvError::Unauthorized);
    }

    sqlx::query!(
        "
        UPDATE squadov.match_custom_events
        SET tm = $2,
            label = COALESCE($3, label),
            icon = COALESCE($4, icon)
        WHERE event_id = $1
        ",
        event_path.event_id,
        data.tm,
        data.label,
        data.icon,
    )
        .execute(&*app.pool)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

pub async fn delete_match_custom_event_handler(app : web::Data<Arc<ApiApplication>>, event_path: web::Path<EventIdPath>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;
    let event_owner_user_id = app.get_custom_event_owner_user_id(event_path.event_id).await?;
    if event_owner_user_id != session.user.id {
        return Err(SquadOvError::Unauthorized);
    }

    sqlx::query!(
        "
        DELETE FROM squadov.match_custom_events
        WHERE event_id = $1
        ",
        event_path.event_id,
    )
        .execute(&*app.pool)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}