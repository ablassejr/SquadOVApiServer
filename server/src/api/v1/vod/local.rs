use crate::api::{
    self,
    auth::{SquadOVSession, SquadOvMachineId},
    v1::GenericVodPathInput,
};
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
    vod::{
        db as vdb,
        VodCopyLocation,
    },
};
use actix_web::{web, HttpResponse};
use uuid::Uuid;
use std::collections::HashSet;
use std::iter::FromIterator;

pub async fn sync_local_storage_handler(app : web::Data<Arc<api::ApiApplication>>, session: SquadOVSession, machine_id: web::Header<SquadOvMachineId>, data: web::Json<Vec<Uuid>>) -> Result<HttpResponse, SquadOvError> {
    if session.user.id != machine_id.user_id {
        return Err(SquadOvError::Unauthorized);
    }

    let mut tx = app.pool.begin().await?;

    let new_copies: HashSet<Uuid> = HashSet::from_iter(data.into_inner());
    let existing_copies: HashSet<Uuid> = vdb::get_vod_copies_at_location(&mut tx, VodCopyLocation::Local, &machine_id.id).await?.into_iter().map(|x| { x.video_uuid }).collect();

    let mut to_add: HashSet<Uuid> = HashSet::new();
    let mut to_remove: HashSet<Uuid> = HashSet::new();

    for v in &new_copies {
        if !existing_copies.contains(v) {
            to_add.insert(v.clone());
        }
    }

    for v in &existing_copies {
        if !new_copies.contains(v) {
            to_remove.insert(v.clone());
        }
    }

    let to_add: Vec<_> = to_add.into_iter().collect();
    let to_remove: Vec<_> = to_remove.into_iter().collect();

    vdb::bulk_delete_vod_copies(&mut tx, &to_remove, VodCopyLocation::Local,  &machine_id.id).await?;
    vdb::bulk_sync_vod_copies(&mut tx, &to_add, VodCopyLocation::Local, &machine_id.id).await?;
    tx.commit().await?;

    for v in &to_add {
        app.es_itf.request_update_vod_copies(v.clone()).await?;
    }

    for v in &to_remove {
        app.es_itf.request_update_vod_copies(v.clone()).await?;
    }
    Ok(HttpResponse::NoContent().finish())
}

pub async fn add_local_storage_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericVodPathInput>, session: SquadOVSession, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, SquadOvError> {
    if session.user.id != machine_id.user_id {
        return Err(SquadOvError::Unauthorized);
    }

    vdb::bulk_sync_vod_copies(&*app.pool, &[path.video_uuid], VodCopyLocation::Local,  &machine_id.id).await?;
    app.es_itf.request_update_vod_copies(path.video_uuid.clone()).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn remove_local_storage_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericVodPathInput>, session: SquadOVSession, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, SquadOvError> {
    if session.user.id != machine_id.user_id {
        return Err(SquadOvError::Unauthorized);
    }

    vdb::bulk_delete_vod_copies(&*app.pool, &[path.video_uuid], VodCopyLocation::Local,  &machine_id.id).await?;
    app.es_itf.request_update_vod_copies(path.video_uuid.clone()).await?;
    Ok(HttpResponse::NoContent().finish())
}