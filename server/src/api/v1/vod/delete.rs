use squadov_common::{
    SquadOvError,
    vod::{
        db,
        VodCopyLocation,
    },
};
use crate::api;
use crate::api::auth::{SquadOvMachineId, SquadOVSession};
use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use std::sync::Arc;
use serde::{Deserialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct VodDeleteFromUuid {
    video_uuid: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct BulkDeleteVodData {
    vods: Vec<Uuid>,
    #[serde(default)]
    local_only: bool,
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct DeleteVodQuery {
    #[serde(default)]
    local_only: bool
}

async fn delete_vod_helper(app: Arc<api::ApiApplication>, video_uuids: &[Uuid], user_id: i64, from_machine: Option<String>) -> Result<(), SquadOvError> {
    // Security measure to make sure user is only ever able to delete their own VODs.
    let video_uuids = db::get_video_uuids_owned_by_user(&*app.pool, video_uuids, user_id).await?;

    if let Some(machine_id) = from_machine {
        db::bulk_delete_vod_copies(&*app.pool, &video_uuids, VodCopyLocation::Local, &machine_id).await?;
    } else {
        for v in &video_uuids {
            app.vod_itf.request_delete_vod(v).await?;
        }
    }

    for v in video_uuids {
        app.es_itf.request_update_vod_copies(v).await?;
    }
    
    Ok(())
}

pub async fn delete_vod_handler(data : web::Path<VodDeleteFromUuid>, app : web::Data<Arc<api::ApiApplication>>, query: web::Query<DeleteVodQuery>, machine_id: web::Header<SquadOvMachineId>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    delete_vod_helper(app.get_ref().clone(), &[data.video_uuid.clone()], session.user.id, if query.local_only { Some(machine_id.id.clone()) } else { None }).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn bulk_delete_vods_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<BulkDeleteVodData>, machine_id: web::Header<SquadOvMachineId>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    delete_vod_helper(app.get_ref().clone(), &data.vods, session.user.id, if data.local_only { Some(machine_id.id.clone()) } else { None }).await?;
    Ok(HttpResponse::NoContent().finish())
}