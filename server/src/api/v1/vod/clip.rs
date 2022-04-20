use crate::api;
use crate::api::auth::SquadOVSession;
use crate::api::v1::{
    RecentMatchQuery,
    UserProfilePath,
};
use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use squadov_common::{
    SquadOvError,
    VodDestination,
    SquadOvGames,
    VodClip,
    ClipReact,
    ClipComment,
    access::{
        self,
        AccessToken,
    },
    vod::{
        self,
        RawVodTag,
        StagedVodClip,
        VodAssociation,
        VodSegmentId,
        db as vdb,
    },
    elastic::vod::ESVodDocument,
};
use std::sync::Arc;
use std::convert::TryFrom;
use chrono::{Utc, Duration};
use elasticsearch_dsl::{Sort, SortOrder};

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
    destination: VodDestination,
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
    async fn create_clip_for_vod(&self, vod_uuid: &Uuid, user_id: i64, title: &str, description: &str, game: SquadOvGames, accel: bool) -> Result<ClipResponse, SquadOvError> {
        let clip_uuid = Uuid::new_v4();

        let mut tx = self.pool.begin().await?;
        vdb::reserve_vod_uuid(&mut tx, &clip_uuid, "mp4", user_id, true).await?;
        vdb::create_clip(&mut tx, &clip_uuid, vod_uuid, user_id, title, description, game, true).await?;
        tx.commit().await?;

        Ok(ClipResponse{
            uuid: clip_uuid.clone(),
            destination: self.create_vod_destination(&clip_uuid, "mp4", accel).await?,
        })
    }

    pub async fn get_vod_clip_from_clip_uuids(&self, uuids: &[Uuid], user_id: i64) -> Result<Vec<VodClip>, SquadOvError> {
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
                COALESCE(cv.count, 0) AS "views!",
                ufv.reason AS "favorite_reason?",
                uwv.video_uuid IS NOT NULL AS "is_watchlist!",
                COALESCE(JSONB_AGG(vvt.*) FILTER(WHERE vvt.video_uuid IS NOT NULL), '[]'::JSONB)  AS "tags!"
            FROM squadov.vod_clips AS vc
            INNER JOIN squadov.users AS u
                ON u.id = vc.clip_user_id
            LEFT JOIN squadov.view_clip_react_count AS rc
                ON rc.clip_uuid = vc.clip_uuid
            LEFT JOIN squadov.view_clip_comment_count AS cc
                ON cc.clip_uuid = vc.clip_uuid
            LEFT JOIN squadov.view_clip_view_count AS cv
                ON cv.clip_uuid = vc.clip_uuid
            LEFT JOIN squadov.user_favorite_vods AS ufv
                ON ufv.video_uuid = vc.clip_uuid
                    AND ufv.user_id = $2
            LEFT JOIN squadov.user_watchlist_vods AS uwv
                ON uwv.video_uuid = vc.clip_uuid
                    AND uwv.user_id = $2
            LEFT JOIN squadov.view_vod_tags AS vvt
                ON vvt.video_uuid = vc.clip_uuid
            WHERE vc.clip_uuid = ANY($1)
            GROUP BY vc.clip_uuid, vc.parent_vod_uuid, vc.clip_user_id, vc.title, vc.description, vc.game, vc.tm, vc.published, u.username, rc.count, cc.count, cv.count, ufv.reason, uwv.video_uuid
            ORDER BY vc.tm DESC
            "#,
            uuids,
            user_id,
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
                favorite_reason: x.favorite_reason,
                is_watchlist: x.is_watchlist,
                access_token: None,
                tags: vod::condense_raw_vod_tags(serde_json::from_value::<Vec<RawVodTag>>(x.tags)?, user_id),
                published: x.published,
            })
        }).collect::<Result<Vec<VodClip>, SquadOvError>>()?)
    }

    async fn list_user_accessible_clips(&self, user_id: i64, start: i64, end: i64, filter: &RecentMatchQuery) -> Result<Vec<VodClip>, SquadOvError> {
        let es_search = filter.to_es_search(user_id, true)
            .from(start)
            .size(end)
            .sort(vec![
                Sort::new("vod.endTime")
                    .order(SortOrder::Desc)
            ]);

        let documents: Vec<ESVodDocument> = self.es_api.search_documents(&self.config.elasticsearch.vod_index_read, serde_json::to_value(es_search)?).await?;
        let mut clips: Vec<_> = documents.into_iter()
            .map(|x| {
                vod::vod_document_to_vod_clip_for_user(x, user_id)
            })
            .filter(|x| {
                x.is_some()
            })
            .map(|x| {
                x.unwrap()
            })
            .collect();

        // We don't store # of views/reacts/comments in ES - we need to do an additional query to the database to find this information.
        let mut react_stats = vdb::get_vod_clip_react_stats(&*self.pool, &(clips.iter().map(|x| { x.clip.video_uuid }).collect::<Vec<Uuid>>()) ).await?;
        for c in &mut clips {
            if let Some(stats) = react_stats.remove(&c.clip.video_uuid) {
                c.views = stats.views;
                c.reacts = stats.reacts;
                c.comments = stats.comments;
            }
        }

        Ok(clips)
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
        let owner_id = self.get_vod_owner_user_id(clip_uuid).await?;
        if owner_id == user_id {
            return Ok(true);
        } else {
            return Ok(access::check_user_has_access_to_match_vod_from_user(&*self.pool, user_id, None, None, Some(clip_uuid.clone())).await?);
        }
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
                r#"
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
                    cc.id AS "id!",
                    cc.clip_uuid AS "clip_uuid!",
                    u.username AS "username!",
                    cc.comment AS "comment!",
                    cc.tm AS "tm!"
                FROM new_comment AS cc
                INNER JOIN squadov.users AS u
                    ON u.id = cc.user_id
                "#,
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

    fn generate_access_token_for_vod_clip(&self, user_id: Option<i64>, id: &Uuid) -> Result<String, SquadOvError> {
        Ok(
            AccessToken{
                // Ideally we'd refresh this somehow instead of just granting access for such a large chunk of time.
                expires: Some(Utc::now() + Duration::hours(6)),
                methods: Some(vec![String::from("GET")]),
                paths: Some(vec![
                    format!("/v1/vod/{}", id),
                    format!("/v1/clip/{}", id),
                ]),
                user_id,
            }.encrypt(&self.config.squadov.access_key)?
        )
    }
}

#[derive(Deserialize)]
pub struct ClipCreateQuery {
    #[serde(default)]
    accel: i64,
}

pub async fn create_clip_for_vod_handler(pth: web::Path<CreateClipPathInput>, data : web::Json<ClipBodyInput>, app : web::Data<Arc<api::ApiApplication>>, query: web::Query<ClipCreateQuery>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    let resp = app.create_clip_for_vod(&pth.video_uuid, session.user.id, &data.title, &data.description, data.game, query.accel == 1).await?;
    Ok(HttpResponse::Ok().json(&resp))
}

#[derive(Deserialize)]
pub struct StagedClipInput {
    start: i64,
    end: i64,
    execute: bool,
}

pub async fn create_staged_clip_for_vod_handler(pth: web::Path<CreateClipPathInput>, data: web::Json<StagedClipInput>, app: web::Data<Arc<api::ApiApplication>>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    // Must be less than 3 minutes. Give it a 4 second buffer to account for rounding and the extra second we tack onto the end of clips.
    if (data.end - data.start - 4000) >= (3 * 60 * 1000) {
        return Err(SquadOvError::BadRequest);
    }

    // Must be some valid clipping time.
    if data.start >= data.end {
        return Err(SquadOvError::BadRequest);
    }

    // Can't be before the start of the VOD.
    if data.start < 0 || data.end < 0 { 
        return Err(SquadOvError::BadRequest);
    }

    let can_instant_clip = sqlx::query!(
        "
        SELECT can_instant_clip
        FROM squadov.user_feature_flags
        WHERE user_id = $1
        ",
        session.user.id,
    )
        .fetch_one(&*app.pool)
        .await?
        .can_instant_clip;

    // Returns a 200 to prevent client crashing.
    if !data.execute && !can_instant_clip {
        return Ok(HttpResponse::Ok().finish())
    }

    let svc = sqlx::query_as!(
        StagedVodClip,
        r#"
        INSERT INTO squadov.staged_clips (
            video_uuid,
            user_id,
            start_offset_ms,
            end_offset_ms,
            create_time
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            NOW()
        )
        RETURNING *
        "#,
        &pth.video_uuid,
        session.user.id,
        data.start,
        data.end,
    )
        .fetch_one(&*app.pool)
        .await?;

    if data.execute {
        app.vod_itf.request_generate_staged_clip(&svc).await?;
    }

    Ok(HttpResponse::Ok().json(&svc.id))
}

async fn get_recent_clips_for_user(user_id: i64, app : web::Data<Arc<api::ApiApplication>>, req: &HttpRequest, page: web::Query<api::PaginationParameters>, query: web::Query<ClipQuery>, mut filter: web::Json<RecentMatchQuery>, needs_profile: bool) -> Result<HttpResponse, SquadOvError> {
    if needs_profile {
        filter.users = Some(vec![user_id]);
        filter.only_profile = true;
    }

    if let Some(match_uuid) = query.match_uuid.as_ref() {
        filter.matches = Some(vec![match_uuid.clone()]);
    }
    
    let mut clips = app.list_user_accessible_clips(user_id, page.start, page.end, &filter).await?;

    if needs_profile {
        for c in &mut clips {
            c.access_token = Some(app.generate_access_token_for_vod_clip(if let Some(user_uuid) = &c.clip.user_uuid {
                if let Some(u) = app.users.get_stored_user_from_uuid(user_uuid, &*app.pool).await? {
                    Some(u.id)
                } else {
                    None
                }
            } else {
                None
            }, &c.clip.video_uuid)?);
        }
    }

    let expected_total = page.end - page.start;
    let got_total = clips.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(clips, req, &page, expected_total == got_total)?)) 
}

pub async fn list_clips_for_user_handler(app : web::Data<Arc<api::ApiApplication>>, page: web::Query<api::PaginationParameters>,  query: web::Query<ClipQuery>, filter: web::Json<RecentMatchQuery>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    get_recent_clips_for_user(session.user.id, app, &request, page, query, filter, false).await
}

pub async fn get_profile_clips_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<UserProfilePath>, page: web::Query<api::PaginationParameters>,  query: web::Query<ClipQuery>, filter: web::Json<RecentMatchQuery>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    get_recent_clips_for_user(path.profile_id, app, &request, page, query, filter, true).await
}

pub async fn get_clip_handler(app : web::Data<Arc<api::ApiApplication>>, pth: web::Path<ClipPathInput>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    let clips = app.get_vod_clip_from_clip_uuids(&[pth.clip_uuid.clone()], session.user.id).await?;

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


#[derive(Deserialize)]
pub struct StagedClipPath {
    stage_id: i64,
}

pub async fn get_staged_clip_status_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<StagedClipPath>) -> Result<HttpResponse, SquadOvError> {
    let some_clip_uuid = sqlx::query!(
        r#"
        SELECT clip_uuid AS "clip_uuid!"
        FROM squadov.staged_clips
        WHERE id = $1
            AND execute_time IS NOT NULL
            AND clip_uuid IS NOT NULL
        "#,
        path.stage_id,
    )
        .fetch_optional(&*app.pool)
        .await?
        .map(|x| { x.clip_uuid });

    if let Some(clip_uuid) = some_clip_uuid {
        #[derive(Serialize)]
        pub struct Status {
            url: String,
            uuid: Uuid,
        }

        let metadata = vdb::get_vod_metadata(&*app.pool, &clip_uuid, "source").await?;
        let manager = app.get_vod_manager(&metadata.bucket).await?;

        Ok(HttpResponse::Ok().json(Status{
            url: manager.get_segment_redirect_uri(&VodSegmentId{
                video_uuid: clip_uuid.clone(),
                quality: String::from("source"),
                segment_name: String::from("fastify.mp4"),
            }).await?.0,
            uuid: clip_uuid.clone(),
        }))
    } else {
        Ok(HttpResponse::Ok().json(serde_json::value::Value::Null))
    }
}

#[derive(Deserialize)]
pub struct PublishClipInput {
    title: Option<String>,
    description: Option<String>,
}

pub async fn publish_clip_handler(app : web::Data<Arc<api::ApiApplication>>, pth: web::Path<ClipPathInput>, data: web::Json<PublishClipInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    let mut tx = app.pool.begin().await?;
    sqlx::query!(
        "
        UPDATE squadov.vod_clips
        SET published = TRUE,
            title = COALESCE($2, title),
            description = COALESCE($3, description)
        WHERE clip_uuid = $1
        ",
        pth.clip_uuid,
        data.title,
        data.description,
    )
        .execute(&mut tx)
        .await?;
    
    app.handle_vod_share(&mut tx, session.user.id, &VodAssociation{
        video_uuid: pth.clip_uuid.clone(),
        is_clip: true,
        ..VodAssociation::default()
    }).await?;
    tx.commit().await?;

    app.es_itf.request_update_vod_sharing(pth.clip_uuid).await?;
    app.es_itf.request_update_vod_data(pth.clip_uuid).await?;
    app.es_itf.request_update_vod_clip(pth.clip_uuid).await?;
    Ok(HttpResponse::NoContent().finish())
}