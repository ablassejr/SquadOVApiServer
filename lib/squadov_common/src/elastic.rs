pub mod vod;
pub mod rabbitmq;

use serde::{Deserialize, de::DeserializeOwned};
use crate::{
    SquadOvError,
};
use reqwest::{header};

#[derive(Deserialize, Debug, Clone)]
pub struct ElasticSearchConfig {
    host: String,
    username: String,
    password: String,
    pub vod_index_read: String,
    pub vod_index_write: String,
}

pub struct ElasticSearchClient {
    config: ElasticSearchConfig
}

#[derive(Deserialize)]
pub struct ESSearchResponseSingleHit<T> {
    #[serde(rename="_source")]
    source: T
}

#[derive(Deserialize)]
pub struct ESSearchResponseHits<T> {
    hits: Vec<ESSearchResponseSingleHit<T>>
}

#[derive(Deserialize)]
pub struct ESSearchResponse<T> {
    hits: ESSearchResponseHits<T>
}

impl ElasticSearchClient {
    pub fn new(config: ElasticSearchConfig) -> Self {
        Self {
            config,
        }
    }

    fn build_endpoint(&self, path: &str) -> String {
        format!("{}/{}", &self.config.host, path)
    }

    fn create_http_client(&self) -> Result<reqwest::Client, SquadOvError> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(
                &format!(
                    "Basic {}",
                    base64::encode(
                        format!(
                            "{}:{}",
                            &self.config.username,
                            &self.config.password,
                        )
                    )
                )
            )?
        );

        Ok(reqwest::ClientBuilder::new()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(60))
            .danger_accept_invalid_certs(if cfg!(debug_assertions) {
                true
            } else {
                false
            })
            .build()?)
    }

    pub async fn delete_document(&self, index: &str, id: &str) -> Result<(), SquadOvError> {
        let client = self.create_http_client()?;
        let endpoint = self.build_endpoint(&format!("{}/_doc/{}", index, id));

        let resp = client.delete(&endpoint)
            .send()
            .await?;

        if resp.status().as_u16() >= 300 {
            return Err(SquadOvError::InternalError(format!("Failed to delete ES document {} - {}", resp.status().as_u16(), resp.text().await?)));
        }
        Ok(())
    }

    pub async fn add_or_update_document(&self, index: &str, id: &str, value: serde_json::Value) -> Result<(), SquadOvError> {
        let client = self.create_http_client()?;
        let endpoint = self.build_endpoint(&format!("{}/_doc/{}", index, id));

        let resp = client.put(&endpoint)
            .json(&value)
            .send()
            .await?;

        if resp.status().as_u16() >= 300 {
            return Err(SquadOvError::InternalError(format!("Failed to add/update ES document {} - {}", resp.status().as_u16(), resp.text().await?)));
        }
        Ok(())
    }

    pub async fn search_documents<T: DeserializeOwned>(&self, index: &str, query: serde_json::Value) -> Result<Vec<T>, SquadOvError> {
        let client = self.create_http_client()?;
        let endpoint = self.build_endpoint(&format!("{}/_search", index));

        let resp = client.post(&endpoint)
            .json(&query)
            .send()
            .await?;

        if resp.status().as_u16() >= 300 {
            return Err(SquadOvError::InternalError(format!("Failed to search ES document {} - {}", resp.status().as_u16(), resp.text().await?)));
        }

        let data = resp.json::<ESSearchResponse<T>>().await?;
        Ok(data.hits.hits.into_iter().map(|x| { x.source }).collect())
    }
}