use squadov_common::{
    SquadOvError,
    zendesk::{
        ZendeskTicket,
        ZendeskTicketComment,
    },
    SquadOVUser,
};
use actix_web::{web, web::BufMut, HttpResponse, HttpRequest};
use actix_multipart::Multipart;
use crate::api;
use crate::api::auth::SquadOVSession;
use std::sync::Arc;
use futures::{StreamExt, TryStreamExt};
use serde::{Serialize, Deserialize};
use chrono::Utc;
use reqwest::header;

#[derive(Deserialize)]
struct GitlabUploadFileResult {
    markdown: String
}

impl api::ApiApplication {
    async fn submit_bug_report(&self, title: &str, description: &str, log_bytes: bytes::Bytes, user: &SquadOVUser) -> Result<(), SquadOvError> {
        let fname = format!("logs-{}-{}.zip", user.id, &timestamp);
        let attachment_id = self.zendesk.upload_attachment(fname, log_bytes).await?;

        self.zendesk.create_ticket(
            ZendeskTicket::new(
                String::from(title),
                ZendeskTicketComment::new(String::from(description))
                    .add_upload(attachment_id),
                user.username.clone(),
                user.email.clone(),
            ),
        ).await?;

        Ok(())
    }
}

pub async fn create_bug_report_handler(app : web::Data<Arc<api::ApiApplication>>, mut payload: Multipart, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(SquadOvError::BadRequest)
    };
    
    let mut title = web::BytesMut::new();
    let mut description = web::BytesMut::new();
    let mut logs = bytes::BytesMut::new();

    while let Some(mut field) = payload.try_next().await? {
        let content_type = field.content_disposition();
        if content_type.is_none() {
            return Err(SquadOvError::BadRequest);
        }
        let content_type = content_type.unwrap();

        let field_name = content_type.get_name();
        if field_name.is_none() {
            return Err(SquadOvError::BadRequest);
        }
        let field_name = field_name.unwrap();
        
        while let Some(Ok(chunk)) = field.next().await {
            match field_name {
                "title" => title.put(&*chunk),
                "description" => description.put(&*chunk),
                "logs" => logs.put(&*chunk),
                _ => return Err(SquadOvError::BadRequest),
            }
        }
    }

    app.submit_bug_report(
        std::str::from_utf8(&*title)?,
        std::str::from_utf8(&*description)?,
        logs.freeze(),
        session.user.id,
    ).await?;
    Ok(HttpResponse::Ok().finish())
}