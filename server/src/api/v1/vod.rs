mod create;
mod delete;
mod find;
mod get;
mod clip;

pub use create::*;
pub use delete::*;
pub use find::*;
pub use get::*;
pub use clip::*;

use crate::api;
use crate::api::auth::SquadOVSession;
use crate::api::v1::FavoriteResponse;
use uuid::Uuid;
use serde::Deserialize;
use std::sync::Arc;
use actix_web::{web, HttpResponse, HttpRequest};
use squadov_common::{
    SquadOvError,
    access::AccessTokenRequest,
    encrypt::{
        AESEncryptRequest,
        squadov_encrypt,
    },
    share,
    share::{
        LinkShareData,
    },
};

#[derive(Deserialize)]
pub struct GenericClipPathInput {
    clip_uuid: Uuid
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct ClipShareSignatureData {
    full_path: String,
}

pub async fn get_clip_share_connections_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericClipPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;
    Ok(
        HttpResponse::Ok().json(
            share::get_match_vod_share_connections_for_user(&*app.pool, None, Some(&path.clip_uuid), session.user.id).await?
        )
    )
}

pub async fn get_clip_share_signature_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericClipPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;
    let token = squadov_common::access::find_encrypted_access_token_for_clip_user(&*app.pool, &path.clip_uuid, session.user.id).await?;

    Ok(
        HttpResponse::Ok().json(
            LinkShareData{
                is_link_shared: token.is_some(),
                share_url: token.map(|x| {
                    format!(
                        "{}/share/{}",
                        &app.config.cors.domain,
                        &x,
                    )
                }),
            }
        )
    )
}

pub async fn delete_clip_share_signature_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericClipPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;
    squadov_common::access::delete_encrypted_access_token_for_clip_user(&*app.pool, &path.clip_uuid, session.user.id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn create_clip_share_signature_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericClipPathInput>, data: web::Json<ClipShareSignatureData>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    // Only the owner of the clip and those with the share permission can share.
    let clips = app.get_vod_clip_from_clip_uuids(&[path.clip_uuid.clone()], session.user.id).await?;
    if clips.is_empty() {
        return Err(SquadOvError::BadRequest);
    }

    if clips[0].clip.user_uuid.is_none() {
        return Err(SquadOvError::Unauthorized);
    }

    if clips[0].clip.user_uuid.as_ref().unwrap() != &session.user.uuid {
        let permissions = share::get_match_vod_share_permissions_for_user(&*app.pool, None, Some(&path.clip_uuid), session.user.id).await?;
        if !permissions.can_share {
            return Err(SquadOvError::Unauthorized);
        }
    }
    
    // If the user already shared this match, reuse that token so we don't fill up our databases with a bunch of useless tokens.
    let mut token = squadov_common::access::find_encrypted_access_token_for_clip_user(&*app.pool, &path.clip_uuid, session.user.id).await?;

    if token.is_none() {
        // Now that we've verified all these things we can go ahead and return to the user a fully fleshed out
        // URL that can be shared. We enable this by generating an encrypted access token that can be used to imitate 
        // access as this session's user to ONLY this current match UUID (along with an optional VOD UUID if one exists).
        let access_request = AccessTokenRequest{
            full_path: data.full_path.clone(),
            user_uuid: session.user.uuid.clone(),
            match_uuid: None,
            video_uuid: Some(path.clip_uuid.clone()),
            clip_uuid: Some(path.clip_uuid.clone()),
            graphql_stats: None,
        };

        let encryption_request = AESEncryptRequest{
            data: serde_json::to_vec(&access_request)?,
            aad: session.user.uuid.as_bytes().to_vec(),
        };

        let encryption_token = squadov_encrypt(encryption_request, &app.config.squadov.share_key)?;

        // Store the encrypted token in our database and return to the user a URL with the unique ID and the IV.
        // This way we get a (relatively) shorter URL instead of a giant encrypted blob.
        let mut tx = app.pool.begin().await?;
        let token_id = squadov_common::access::store_encrypted_access_token_for_clip_user(&mut tx, &path.clip_uuid, session.user.id, &encryption_token).await?;
        tx.commit().await?;

        token = Some(token_id);
    }

    // Make the VOD public - we need to keep track of its public setting in our database as well as configure the backend
    // to enable it to be served publically.
    app.make_vod_public(&path.clip_uuid).await?;

    let token = token.ok_or(SquadOvError::InternalError(String::from("Failed to obtain/generate share token.")))?;

    // It could be neat to store some sort of access token ID in our database and allow users to track how
    // many times it was used and be able to revoke it and stuff but I don't think the gains are worth it at
    // the moment. I'd rather have a more distributed version where we toss a URL out there and just let it be
    // valid.
    Ok(
        HttpResponse::Ok().json(
            LinkShareData{
                is_link_shared: true,
                share_url: Some(
                    format!(
                        "{}/share/{}",
                        &app.config.cors.domain,
                        &token,
                    )
                ),
            }
        )
    )
}

#[derive(Deserialize)]
pub struct GenericVodPathInput {
    video_uuid: Uuid
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct VodFavoriteData {
    reason: String,
}

impl api::ApiApplication {
    pub async fn make_vod_public(&self, video_uuid: &Uuid) -> Result<(), SquadOvError> {
        // Get all the segments that exist for this VOD.
        let quality_options = self.get_vod_quality_options(&[video_uuid.clone()]).await?;
        let assocs = self.find_vod_associations(&[video_uuid.clone()]).await?;

        if let Some(quality_arr) = quality_options.get(video_uuid) {
            if let Some(vod) = assocs.get(video_uuid) {
                let raw_extension = squadov_common::container_format_to_extension(&vod.raw_container_format);
                for quality in quality_arr {
                    // We only need to make one segment public since only one or the other will ever exist
                    // at a given point in time.
                    let base_segment = if quality.has_fastify {
                        squadov_common::VodSegmentId{
                            video_uuid: video_uuid.clone(),
                            quality: quality.id.clone(),
                            segment_name: String::from("fastify.mp4"),
                        }
                    } else {
                        squadov_common::VodSegmentId{
                            video_uuid: video_uuid.clone(),
                            quality: quality.id.clone(),
                            segment_name: format!("video.{}", &raw_extension),
                        }
                    };

                    self.vod.make_segment_public(&base_segment).await?;
                }
            }
        }

        Ok(())
    }

    async fn add_vod_favorite_for_user(&self, video_uuid: &Uuid, user_id: i64, reason: &str) -> Result<(), SquadOvError> {
        sqlx::query!(
            r#"
            INSERT INTO squadov.user_favorite_vods (
                video_uuid,
                user_id,
                reason
            )
            VALUES (
                $1,
                $2,
                $3
            )
            ON CONFLICT DO NOTHING
            "#,
            video_uuid,
            user_id,
            reason,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    async fn remove_vod_favorite_for_user(&self, video_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError> {
        sqlx::query!(
            r#"
            DELETE FROM squadov.user_favorite_vods
            WHERE video_uuid = $1 AND user_id = $2
            "#,
            video_uuid,
            user_id,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    async fn is_vod_favorite_for_user(&self, video_uuid: &Uuid, user_id: i64) -> Result<Option<String>, SquadOvError> {
        Ok(
            sqlx::query!(
                r#"
                SELECT reason
                FROM squadov.user_favorite_vods
                WHERE video_uuid = $1
                    AND user_id = $2
                "#,
                video_uuid,
                user_id,
            )
                .fetch_optional(&*self.pool)
                .await?
                .map(|x| { x.reason })
        )
    }

    async fn add_vod_watchlist_for_user(&self, video_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError> {
        sqlx::query!(
            r#"
            INSERT INTO squadov.user_watchlist_vods (
                video_uuid,
                user_id
            )
            VALUES (
                $1,
                $2
            )
            ON CONFLICT DO NOTHING
            "#,
            video_uuid,
            user_id,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    async fn remove_vod_watchlist_for_user(&self, video_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError> {
        sqlx::query!(
            r#"
            DELETE FROM squadov.user_watchlist_vods
            WHERE video_uuid = $1 AND user_id = $2
            "#,
            video_uuid,
            user_id,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    async fn is_vod_watchlist_for_user(&self, video_uuid: &Uuid, user_id: i64) -> Result<bool, SquadOvError> {
        Ok(
            sqlx::query!(
                r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM squadov.user_watchlist_vods
                    WHERE video_uuid = $1
                        AND user_id = $2
                ) AS "exists!"
                "#,
                video_uuid,
                user_id,
            )
                .fetch_one(&*self.pool)
                .await?
                .exists
        )
    }
}

pub async fn favorite_vod_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericVodPathInput>, data: web::Json<VodFavoriteData>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    app.add_vod_favorite_for_user(&path.video_uuid, session.user.id, &data.reason).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn remove_favorite_vod_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericVodPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    app.remove_vod_favorite_for_user(&path.video_uuid, session.user.id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn check_favorite_vod_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericVodPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    let reason = app.is_vod_favorite_for_user(&path.video_uuid, session.user.id).await?;
    Ok(HttpResponse::Ok().json(
        FavoriteResponse{
            favorite: reason.is_some(),
            reason,
        }
    ))
}

pub async fn watchlist_vod_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericVodPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    app.add_vod_watchlist_for_user(&path.video_uuid, session.user.id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn remove_watchlist_vod_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericVodPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    app.remove_vod_watchlist_for_user(&path.video_uuid, session.user.id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn check_watchlist_vod_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericVodPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };
    Ok(HttpResponse::Ok().json(
        app.is_vod_watchlist_for_user(&path.video_uuid, session.user.id).await?
    ))
}