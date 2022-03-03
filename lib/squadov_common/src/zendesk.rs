use crate::SquadOvError;
use serde::{Serialize, Deserialize};
use actix_web::web::Bytes;
use reqwest::{header};

#[derive(Deserialize, Clone, Debug)]
pub struct ZendeskConfig {
    email: String, 
    api_key: String,
}

pub struct ZendeskClient {
    pub config: ZendeskConfig,
}

#[derive(Serialize)]
pub struct ZendeskTicketVia {
    pub channel: String,
}

#[derive(Serialize)]
pub struct ZendeskTicketRequester {
    pub locale_id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Serialize)]
pub struct ZendeskTicketComment {
    uploads: Option<Vec<String>>,
    body: Option<String>,
    via: Option<ZendeskTicketVia>,
}

impl ZendeskTicketComment {
    pub fn new(desc: String) -> Self {
        Self {
            uploads: None,
            body: Some(desc),
            via: Some(ZendeskTicketVia{
                channel: "web_service".to_string(),
            }),
        }
    }

    pub fn add_upload(mut self, upload_id: String) -> Self {
        if let Some(uploads) = self.uploads.as_mut() {
            uploads.push(upload_id);
        } else {
            self.uploads = Some(vec![upload_id]);
        }
        self
    }
}

#[derive(Serialize)]
pub struct ZendeskCustomField {
    id: i64,
    value: String,
}

#[derive(Serialize)]
pub struct ZendeskTicket {
    email_ccs: Option<Vec<String>>,
    subject: Option<String>,
    tags: Option<Vec<String>>,
    comment: ZendeskTicketComment,
    requester: Option<ZendeskTicketRequester>,
    custom_fields: Option<Vec<ZendeskCustomField>>
}

impl ZendeskTicket {
    pub fn new(subject: String, comment: ZendeskTicketComment, username: String, email: String) -> Self {
        Self {
            email_ccs: Some(vec![email.clone()]),
            subject: Some(subject),
            tags: None,
            comment,
            requester: Some(ZendeskTicketRequester{
                locale_id: 1,
                name: username.clone(),
                email: email.clone(),
            }),
            custom_fields: Some(vec![
                // SquadOV username custom field.
                ZendeskCustomField{
                    id: 4497278910235,
                    value: username.clone(),
                }
            ]),
        }
    }
} 
impl ZendeskClient {
    pub fn new(config: ZendeskConfig) -> Self {
        Self {
            config,
        }
    }

    fn create_http_client(&self) -> Result<reqwest::Client, SquadOvError> {
        let mut headers = header::HeaderMap::new();
        let access_token = format!("Basic {}", base64::encode(&format!("{}/token:{}", &self.config.email, &self.config.api_key)));
        headers.insert(header::AUTHORIZATION, header::HeaderValue::from_str(&access_token)?);
        Ok(reqwest::ClientBuilder::new()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(60))
            .build()?)
    }

    pub async fn create_ticket(&self, ticket: ZendeskTicket) -> Result<(), SquadOvError> {
        self.create_http_client()?
            .post("/api/v2/tickets")
            .json(&ticket)
            .send()
            .await?;
        Ok(())
    }

    pub async fn upload_attachment(&self, filename: String, data: Bytes) -> Result<String, SquadOvError> {
        #[derive(Deserialize)]
        pub struct UploadField {
            token: String,
        }

        #[derive(Deserialize)]
        pub struct Response {
            upload: UploadField
        }

        let resp = self.create_http_client()?
            .post(&format!("/api/v2/uploads?filename={}", &filename))
            .header("content-type", "application/binary")
            .body(data)
            .send()
            .await?
            .json::<Response>().await?;
        Ok(resp.upload.token)
    }
}