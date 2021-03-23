use crate::api;
use crate::api::auth::SquadOVSession;
use crate::api::v1::RecentMatchQuery;
use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use squadov_common::{SquadOvError, VodSegmentId, SquadOvGames, VodClip, ClipReact, ClipComment};
use std::sync::Arc;
use std::convert::TryFrom;
use serde_qs::actix::QsQuery;
use chrono::{Utc, TimeZone};

#[derive(Deserialize)]
pub struct CreateClipPathInput {
    video_uuid: Uuid,
}

#[derive(Deserialize)]
pub struct ClipPathInput {
    clip_uuid: Uuid,
}

#[derive(Deserialize)]
pub struct ClipBodyInput {
    title: String,
    description: String,
    game: SquadOvGames
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ClipResponse {
    uuid: Uuid,
    upload_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ClipQuery {
    match_uuid: Option<Uuid>
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ClipCommentInput {
    comment: String
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ClipCommentPathInput {
    comment_id: i64,
}

impl api::ApiApplication {
    async fn create_clip_for_vod(&self, vod_uuid: &Uuid, user_id: i64, title: &str, description: &str, game: SquadOvGames) -> Result<ClipResponse, SquadOvError> {
        let clip_uuid = Uuid::new_v4();

        let mut tx = self.pool.begin().await?;
        self.reserve_vod_uuid(&mut tx, &clip_uuid, "mp4", user_id, true).await?;

        sqlx::query!(
            "
            INSERT INTO squadov.vod_clips (
                clip_uuid,
                parent_vod_uuid,
                clip_user_id,
                title,
                description,
                game,
                tm
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                NOW()
            )
            ",
            clip_uuid,
            vod_uuid,
            user_id,
            title,
            description,
            game as i32,
        )
            .execute(&mut tx)
            .await?;
        tx.commit().await?;

        Ok(ClipResponse{
            uuid: clip_uuid.clone(),
            upload_path: self.vod.get_segment_upload_uri(&VodSegmentId{
                video_uuid: clip_uuid.clone(),
                quality: String::from("source"),
                segment_name: String::from("video.mp4"),
            }).await?,
        })
    }

    pub async fn get_vod_clip_from_clip_uuids(&self, uuids: &[Uuid]) -> Result<Vec<VodClip>, SquadOvError> {
        // This is a multi-pass solution to get all the data we need in the most efficient way.
        // First we get all the basic VOD clip data stored in the database. After that, the only thing
        // we need to take care of is grabbing the VodAssociation and VodManifest for each clip.
        let base_data = sqlx::query!(
            r#"
            SELECT
                vc.*,
                u.username AS "clipper",
                COALESCE(rc.count, 0) AS "reacts!",
                COALESCE(cc.count, 0) AS "comments!",
                COALESCE(cv.count, 0) AS "views!"
            FROM squadov.vod_clips AS vc
            INNER JOIN squadov.users AS u
                ON u.id = vc.clip_user_id
            LEFT JOIN squadov.view_clip_react_count AS rc
                ON rc.clip_uuid = vc.clip_uuid
            LEFT JOIN squadov.view_clip_comment_count AS cc
                ON cc.clip_uuid = vc.clip_uuid
            LEFT JOIN squadov.view_clip_view_count AS cv
                ON cv.clip_uuid = vc.clip_uuid
            WHERE vc.clip_uuid = ANY($1)
            ORDER BY vc.tm DESC
            "#,
            uuids,
        )
            .fetch_all(&*self.pool)
            .await?;

        // Don't use the input uuids but rather use the more accurate UUIDs that we collected in base_data
        // as the input may (or may not) have invalid UUIDs.
        let clip_uuids: Vec<Uuid> = base_data.iter().map(|x| { x.clip_uuid.clone() }).collect();
        let mut clip_vod_associations = self.find_vod_associations(&clip_uuids).await?;
        let mut clip_vod_manifests = self.get_vod(&clip_uuids).await?;

        Ok(base_data.into_iter().map(|x| {
            Ok(VodClip {
                clip: clip_vod_associations.remove(&x.clip_uuid).ok_or(SquadOvError::NotFound)?,
                manifest: clip_vod_manifests.remove(&x.clip_uuid).ok_or(SquadOvError::NotFound)?,
                title: x.title,
                description: x.description,
                clipper: x.clipper,
                game: SquadOvGames::try_from(x.game)?,
                tm: x.tm,
                views: x.views,
                reacts: x.reacts,
                comments: x.comments,
            })
        }).collect::<Result<Vec<VodClip>, SquadOvError>>()?)
    }

    async fn list_user_accessible_clips(&self, user_id: i64, start: i64, end: i64, match_uuid: Option<Uuid>, filter: &RecentMatchQuery) -> Result<Vec<VodClip>, SquadOvError> {
        let clips = sqlx::query!(
            "
            SELECT DISTINCT vc.clip_uuid
            FROM squadov.vod_clips AS vc
            INNER JOIN squadov.vods AS v
                ON v.video_uuid = vc.clip_uuid
            INNER JOIN squadov.matches AS m
                ON m.uuid = v.match_uuid
            LEFT JOIN squadov.squad_role_assignments AS sra
                ON sra.user_id = vc.clip_user_id
            LEFT JOIN squadov.squad_role_assignments AS ora
                ON ora.squad_id = sra.squad_id
            INNER JOIN squadov.users AS ou
                ON ou.id = ora.user_id
                    OR ou.uuid = v.user_uuid
            LEFT JOIN squadov.user_favorite_vods AS ufv
                ON ufv.video_uuid = v.video_uuid
                    AND ufv.user_id = ou.id
            LEFT JOIN squadov.user_watchlist_vods AS uwv
                ON uwv.video_uuid = v.video_uuid
                    AND uwv.user_id = ou.id
            WHERE (vc.clip_user_id = $1 OR ora.user_id = $1)
                AND ($4::UUID IS NULL OR m.uuid = $4)
                AND (CARDINALITY($5::INTEGER[]) = 0 OR m.game = ANY($5))
                AND (CARDINALITY($6::BIGINT[]) = 0 OR sra.squad_id = ANY($6))
                AND (CARDINALITY($7::BIGINT[]) = 0 OR ora.user_id = ANY($7))
                AND COALESCE(v.end_time >= $8, TRUE)
                AND COALESCE(v.end_time <= $9, TRUE)
                AND (NOT $10::BOOLEAN OR ufv.video_uuid IS NOT NULL)
                AND (NOT $11::BOOLEAN OR uwv.video_uuid IS NOT NULL)
            LIMIT $2 OFFSET $3
            ",
            user_id,
            end - start,
            start,
            match_uuid,
            &filter.games.as_ref().unwrap_or(&vec![]).iter().map(|x| {
                *x as i32
            }).collect::<Vec<i32>>(),
            &filter.squads.as_ref().unwrap_or(&vec![]).iter().map(|x| { *x }).collect::<Vec<i64>>(),
            &filter.users.as_ref().unwrap_or(&vec![]).iter().map(|x| { *x }).collect::<Vec<i64>>(),
            filter.time_start.map(|x| {
                Utc.timestamp_millis(x)
            }),
            filter.time_end.map(|x| {
                Utc.timestamp_millis(x)
            }),
            filter.only_favorite,
            filter.only_watchlist,
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|x| { x.clip_uuid })
            .collect::<Vec<Uuid>>();
        Ok(self.get_vod_clip_from_clip_uuids(&clips).await?)
    }

    async fn mark_clip_view(&self, clip_uuid: &Uuid, user_id: Option<i64>) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.clip_views (
                clip_uuid,
                user_id,
                tm
            )
            VALUES (
                $1,
                $2,
                NOW()
            )
            ",
            clip_uuid,
            user_id,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    async fn get_clip_reacts_for_user(&self, clip_uuid: &Uuid, user_id: i64) -> Result<Vec<ClipReact>, SquadOvError> {
        let num_reacts = sqlx::query!(
            r#"
            SELECT COALESCE(
                (
                    SELECT 1
                    FROM squadov.clip_reacts AS cr
                    WHERE cr.clip_uuid = $1
                        AND cr.user_id = $2
                ),
                0
            ) AS "num!"
            "#,
            clip_uuid,
            user_id,
        )
            .fetch_one(&*self.pool)
            .await?
            .num;
        Ok(vec![ClipReact{}; num_reacts as usize])
    }

    async fn add_clip_react_for_user(&self, clip_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.clip_reacts (
                clip_uuid,
                user_id,
                tm
            )
            VALUES (
                $1,
                $2,
                NOW()
            )
            ON CONFLICT DO NOTHING
            ",
            clip_uuid,
            user_id,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    async fn remove_clip_react_for_user(&self, clip_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            DELETE FROM squadov.clip_reacts
            WHERE clip_uuid = $1 AND user_id = $2
            ",
            clip_uuid,
            user_id,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    async fn check_user_has_access_to_clip(&self, clip_uuid: &Uuid, user_id: i64) -> Result<bool, SquadOvError> {
        Ok(
            sqlx::query!(
                r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM squadov.vod_clips AS vc
                    LEFT JOIN squadov.squad_role_assignments AS sra
                        ON sra.user_id = vc.clip_user_id
                    LEFT JOIN squadov.squad_role_assignments AS ora
                        ON ora.squad_id = sra.squad_id
                    WHERE vc.clip_uuid = $1
                        AND (vc.clip_user_id = $2 OR ora.user_id = $2)
                    LIMIT 1
                ) AS "exists!"
                "#,
                clip_uuid,
                user_id
            )
                .fetch_one(&*self.pool)
                .await?
                .exists
        )
    }

    async fn get_clip_comments(&self, clip_uuid: &Uuid, start: i64, end: i64) -> Result<Vec<ClipComment>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                ClipComment,
                "
                SELECT
                    cc.id,
                    cc.clip_uuid,
                    u.username,
                    cc.comment,
                    cc.tm
                FROM squadov.clip_comments AS cc
                INNER JOIN squadov.users AS u
                    ON u.id = cc.user_id
                WHERE cc.clip_uuid = $1
                ORDER BY cc.tm DESC
                LIMIT $2 OFFSET $3
                ",
                clip_uuid,
                end - start,
                start
            )
                .fetch_all(&*self.pool)
                .await?
        )
    }

    async fn create_clip_comment(&self, clip_uuid: &Uuid, user_id: i64, comment: &str) -> Result<ClipComment, SquadOvError> {
        Ok(
            sqlx::query_as!(
                ClipComment,
                "
                WITH new_comment AS (
                    INSERT INTO squadov.clip_comments (
                        clip_uuid,
                        user_id,
                        comment,
                        tm
                    )
                    VALUES (
                        $1,
                        $2,
                        $3,
                        NOW()
                    )
                    RETURNING *
                )
                SELECT
                    cc.id,
                    cc.clip_uuid,
                    u.username,
                    cc.comment,
                    cc.tm
                FROM new_comment AS cc
                INNER JOIN squadov.users AS u
                    ON u.id = cc.user_id
                ",
                clip_uuid,
                user_id,
                comment,
            )
                .fetch_one(&*self.pool)
                .await?
        )
    }

    async fn delete_clip_comment(&self, comment_id: i64, user_id: i64) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            DELETE FROM squadov.clip_comments
            WHERE id = $1 AND user_id = $2
            ",
            comment_id,
            user_id,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
}

pub async fn create_clip_for_vod_handler(pth: web::Path<CreateClipPathInput>, data : web::Json<ClipBodyInput>, app : web::Data<Arc<api::ApiApplication>>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    let resp = app.create_clip_for_vod(&pth.video_uuid, session.user.id, &data.title, &data.description, data.game).await?;
    Ok(HttpResponse::Ok().json(&resp))
}

pub async fn list_clips_for_user_handler(app : web::Data<Arc<api::ApiApplication>>, page: web::Query<api::PaginationParameters>,  query: web::Query<ClipQuery>, filter: QsQuery<RecentMatchQuery>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    let clips = app.list_user_accessible_clips(session.user.id, page.start, page.end, query.match_uuid.clone(), &filter).await?;

    let expected_total = page.end - page.start;
    let got_total = clips.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(clips, &request, &page, expected_total == got_total)?)) 
}

pub async fn get_clip_handler(app : web::Data<Arc<api::ApiApplication>>, pth: web::Path<ClipPathInput>) -> Result<HttpResponse, SquadOvError> {
    let clips = app.get_vod_clip_from_clip_uuids(&[pth.clip_uuid.clone()]).await?;

    if clips.is_empty() {
        Err(SquadOvError::NotFound)
    } else {
        Ok(HttpResponse::Ok().json(&clips[0]))
    }
}

// REACTS
pub async fn get_clip_reacts_handler(app : web::Data<Arc<api::ApiApplication>>, pth: web::Path<ClipPathInput>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    if !app.check_user_has_access_to_clip(&pth.clip_uuid, session.user.id).await? {
        return Err(SquadOvError::Unauthorized);
    }

    let reacts = app.get_clip_reacts_for_user(&pth.clip_uuid, session.user.id).await?;
    Ok(HttpResponse::Ok().json(&reacts))
}

pub async fn add_react_to_clip_handler(app : web::Data<Arc<api::ApiApplication>>, pth: web::Path<ClipPathInput>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    if !app.check_user_has_access_to_clip(&pth.clip_uuid, session.user.id).await? {
        return Err(SquadOvError::Unauthorized);
    }

    app.add_clip_react_for_user(&pth.clip_uuid, session.user.id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn delete_react_from_clip_handler(app : web::Data<Arc<api::ApiApplication>>, pth: web::Path<ClipPathInput>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    if !app.check_user_has_access_to_clip(&pth.clip_uuid, session.user.id).await? {
        return Err(SquadOvError::Unauthorized);
    }

    app.remove_clip_react_for_user(&pth.clip_uuid, session.user.id).await?;
    Ok(HttpResponse::NoContent().finish())
}

// VIEWS
pub async fn mark_clip_view_handler(app : web::Data<Arc<api::ApiApplication>>, pth: web::Path<ClipPathInput>,  request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    app.mark_clip_view(&pth.clip_uuid, if session.share_token.is_some() { None } else { Some(session.user.id) }).await?;
    Ok(HttpResponse::NoContent().finish())
}

// COMMENTS
pub async fn get_clip_comments_handler(app : web::Data<Arc<api::ApiApplication>>, page: web::Query<api::PaginationParameters>, pth: web::Path<ClipPathInput>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    if !app.check_user_has_access_to_clip(&pth.clip_uuid, session.user.id).await? {
        return Err(SquadOvError::Unauthorized);
    }

    let comments = app.get_clip_comments(&pth.clip_uuid, page.start, page.end).await?;
    let expected_total = page.end - page.start;
    let got_total = comments.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(comments, &request, &page, expected_total == got_total)?)) 
}

pub async fn create_clip_comment_handler(app : web::Data<Arc<api::ApiApplication>>, pth: web::Path<ClipPathInput>, comment: web::Json<ClipCommentInput>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    if !app.check_user_has_access_to_clip(&pth.clip_uuid, session.user.id).await? {
        return Err(SquadOvError::Unauthorized);
    }

    Ok(HttpResponse::Ok().json(
        &app.create_clip_comment(&pth.clip_uuid, session.user.id, &comment.comment).await?
    ))
}

pub async fn delete_clip_comment_handler(app : web::Data<Arc<api::ApiApplication>>, pth: web::Path<ClipCommentPathInput>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };
    
    app.delete_clip_comment(pth.comment_id, session.user.id).await?;    
    Ok(HttpResponse::NoContent().finish())
}