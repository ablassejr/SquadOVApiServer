use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
};
use uuid::Uuid;
use serde::Deserialize;
use futures_util::StreamExt;

#[derive(Deserialize)]
pub struct WowUserViewPath {
    view_uuid: Uuid,
}

async fn bulk_upload_combatlog_compressed(app: Arc<api::ApiApplication>, view_uuid: &Uuid, data: Vec<u8>) -> Result<(), SquadOvError> {
    app.wow_itf.request_bulk_combat_log_payload_processing(&view_uuid, data).await?;
    Ok(())
}

pub async fn bulk_upload_combatlog_handler(mut body : web::Payload,  app : web::Data<Arc<api::ApiApplication>>, path: web::Path<WowUserViewPath>) -> Result<HttpResponse, SquadOvError> {
    let mut data = web::BytesMut::new();
    while let Some(item) = body.next().await {
        data.extend_from_slice(&item?);
    }

    let view_uuid = path.view_uuid.clone();
    let app = app.clone();
    // Putting this in a separate thread just in case it takes a bit to decompress.
    tokio::task::spawn(async move {
        match bulk_upload_combatlog_compressed(app.get_ref().clone(), &view_uuid, (&data).to_vec()).await {
            Ok(_) => Ok(()),
            Err(err) => {
                log::error!("Failed to forward bulk WoW combat log data: {:?}", err);
                Err(err)
            }
        }
    });

    Ok(HttpResponse::NoContent().finish())
}