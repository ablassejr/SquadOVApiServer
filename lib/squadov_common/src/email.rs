use serde::{Serialize, Deserialize};
use crate::SquadOvError;
use std::collections::HashMap;

const SQUADOV_GROUP_ID: i64 = 14827;

#[derive(Deserialize, Clone, Debug)]
pub struct EmailConfig {
    pub sendgrid_api_key: String,
    pub invite_template: String,
}

pub struct EmailClient {
    config: EmailConfig,
}

#[derive(Serialize)]
pub struct EmailUser {
    pub email: String,
    pub name: Option<String>,
}

pub struct EmailTemplate{
    pub params: HashMap<String, String>,
    pub to: EmailUser
}

#[derive(Serialize)]
struct SendgridUnsubscribeOptions {
    group_id: i64
}

#[derive(Serialize)]
struct SendgridEmailPersonalization {
    to: Vec<EmailUser>,
    dynamic_template_data: HashMap<String, String>,
}

#[derive(Serialize)]
struct SendgridSendEmailBody {
    personalizations: Vec<SendgridEmailPersonalization>,
    from: EmailUser,
    reply_to: EmailUser,
    template_id: String,
    asm: SendgridUnsubscribeOptions,
}

impl EmailClient {
    pub fn new(config: &EmailConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub async fn send_bulk_templated_email(&self, template_id: &str, templates: Vec<EmailTemplate>) -> Result<(), SquadOvError> {
        let client = reqwest::ClientBuilder::new().build()?;
        let endpoint = String::from("https://api.sendgrid.com/v3/mail/send");
        let resp = client.post(&endpoint)
            .bearer_auth(&self.config.sendgrid_api_key)
            .json(&SendgridSendEmailBody{
                personalizations: templates.into_iter().map(|x| {
                    SendgridEmailPersonalization {
                        to: vec![x.to],
                        dynamic_template_data: x.params,
                    }
                }).collect(),
                from: EmailUser{
                    email: String::from("no-reply@squadov.gg"),
                    name: Some(String::from("SquadOV")),
                },
                reply_to: EmailUser{
                    email: String::from("no-reply@squadov.gg"),
                    name: Some(String::from("SquadOV")),
                },
                template_id: String::from(template_id),
                asm: SendgridUnsubscribeOptions{
                    group_id: SQUADOV_GROUP_ID,
                }
            })
            .send()
            .await?;

        if resp.status().as_u16() != 202 {
            Err(SquadOvError::InternalError(format!("SendGrid Send Email Error: {} - {}", resp.status().as_u16(), resp.text().await?)))
        } else {
            Ok(())
        }
    }
}