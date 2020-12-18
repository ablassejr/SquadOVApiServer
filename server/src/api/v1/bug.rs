use squadov_common::SquadOvError;
use actix_web::{web, web::BufMut, HttpResponse, HttpRequest};
use actix_multipart::Multipart;
use crate::api;
use crate::api::auth::SquadOVSession;
use std::sync::Arc;
use futures::{StreamExt, TryStreamExt};
use serde::Deserialize;
use chrono::Utc;
use reqwest::header;

#[derive(Deserialize)]
struct GitlabUploadFileResult {
    markdown: String
}

impl api::ApiApplication {
    async fn submit_bug_report(&self, title: &str, description: &str, log_bytes: web::Bytes, user_id: i64) -> Result<(), SquadOvError> {
        let mut headers = header::HeaderMap::new();
        headers.insert("PRIVATE-TOKEN", header::HeaderValue::from_str(&self.config.gitlab.access_token)?);

        // Upload the file to gitlab. Put the markdown of the upload into the description that the user gave us.
        let gitlab_client = reqwest::Client::builder().default_headers(headers).build()?;
        let timestamp = Utc::now().to_rfc3339();
        let fname = format!("logs-{}-{}.zip", user_id, &timestamp);
        let form = reqwest::multipart::Form::new()
            .part(
                "file",
                reqwest::multipart::Part::stream(log_bytes)
                    .file_name(fname.clone())
            );

        let file_upload_result = gitlab_client
            .post(&format!("https://gitlab.com/api/v4/projects/{}/uploads", self.config.gitlab.project_id))
            .multipart(form)
            .send()
            .await?;

        let status = file_upload_result.status().as_u16();
        if status != 201 {
            return Err(SquadOvError::InternalError(format!("Failed to upload Gitlab logs [{}]: {}", status, file_upload_result.text().await?)));
        }

        let file_upload_result = file_upload_result
            .json::<GitlabUploadFileResult>()
            .await?;

        let issue_result = gitlab_client
            .post(&format!("https://gitlab.com/api/v4/projects/{project_id}/issues?title={title}&labels=bug&description={desc}&created_at={created}",
                project_id=self.config.gitlab.project_id,
                created=&timestamp,
                title=squadov_common::url_encode(&format!("[USER REPORTED BUG] {title}", title=title)),
                desc=squadov_common::url_encode(&format!(r#"
USER ID: {user_id}

LOGS: {log}

DESCRIPTION: {description}
                "#, user_id=user_id, log=&file_upload_result.markdown, description=description),
            )))
            .send()
            .await?;

        let status = issue_result.status().as_u16();
        if status != 201 {
            return Err(SquadOvError::InternalError(format!("Failed to create Gitlab issue [{}]: {}", status, issue_result.text().await?)));
        }

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
    let mut logs = web::BytesMut::new();

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