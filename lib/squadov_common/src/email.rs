use serde::{Serialize, Deserialize};
use crate::SquadOvError;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug)]
pub struct EmailConfig {
    pub postmark_api_key: String,
    pub invite_template: String,
    pub welcome_template: String,
}

pub struct EmailClient {
    config: EmailConfig,
}

#[derive(Serialize)]
pub struct EmailUser {
    pub email: String,
    pub name: Option<String>,
}

impl ToString for EmailUser {
    fn to_string(&self) -> String {
        if self.name.is_some() {
            format!("{} <{}>", self.name.as_ref().unwrap().as_str(), &self.email)
        } else {
            self.email.clone()
        }
    }
}

pub struct EmailTemplate{
    pub params: HashMap<String, String>,
    pub to: EmailUser
}

#[derive(Serialize)]
struct PostmarkSendEmailBody {
    #[serde(rename="From")]
    from: String,
    #[serde(rename="To")]
    to: String,
    #[serde(rename="ReplyTo")]
    reply_to: String,
    #[serde(rename="TemplateModel")]
    template_model: HashMap<String, String>,
    #[serde(rename="TemplateAlias")]
    template_alias: String,
}

#[derive(Serialize)]
struct PostmarkBulkBody {
    #[serde(rename="Messages")]
    messages: Vec<PostmarkSendEmailBody>
}

impl EmailClient {
    pub fn new(config: &EmailConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub async fn send_bulk_templated_email(&self, template_id: &str, templates: Vec<EmailTemplate>) -> Result<(), SquadOvError> {
        let client = reqwest::ClientBuilder::new().build()?;
        let endpoint = String::from("https://api.postmarkapp.com/email/batchWithTemplates");
        let resp = client.post(&endpoint)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("X-Postmark-Server-Token", &self.config.postmark_api_key)
            .json(&PostmarkBulkBody{
                messages: templates.into_iter().map(|x| {
                    PostmarkSendEmailBody{
                        from: EmailUser{
                            email: String::from("no-reply@squadov.gg"),
                            name: Some(String::from("SquadOV")),
                        }.to_string(),
                        reply_to: EmailUser{
                            email: String::from("no-reply@squadov.gg"),
                            name: Some(String::from("SquadOV")),
                        }.to_string(),
                        to: x.to.to_string(),
                        template_model: x.params,
                        template_alias: String::from(template_id),
                    }
                }).collect::<Vec<PostmarkSendEmailBody>>()
            })
            .send()
            .await?;

        if resp.status().as_u16() != 200 {
            Err(SquadOvError::InternalError(format!("Postmark Send Email Error: {} - {}", resp.status().as_u16(), resp.text().await?)))
        } else {
            Ok(())
        }
    }
}