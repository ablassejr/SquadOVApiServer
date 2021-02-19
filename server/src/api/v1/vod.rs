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

pub async fn create_clip_share_signature_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericClipPathInput>, data: web::Json<ClipShareSignatureData>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    // Only the owner of the clip can share.
    let clips = app.get_vod_clip_from_clip_uuids(&[path.clip_uuid.clone()]).await?;
    if clips.is_empty() {
        return Err(SquadOvError::BadRequest);
    }

    if clips[0].clip.user_uuid.is_none() {
        return Err(SquadOvError::Unauthorized);
    }

    if clips[0].clip.user_uuid.as_ref().unwrap() != &session.user.uuid {
        return Err(SquadOvError::Unauthorized);
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

    let token = token.ok_or(SquadOvError::InternalError(String::from("Failed to obtain/generate share token.")))?;

    // It could be neat to store some sort of access token ID in our database and allow users to track how
    // many times it was used and be able to revoke it and stuff but I don't think the gains are worth it at
    // the moment. I'd rather have a more distributed version where we toss a URL out there and just let it be
    // valid.
    Ok(HttpResponse::Ok().json(&format!(
        "{}/share/{}",
        &app.config.cors.domain,
        &token,
    )))
}