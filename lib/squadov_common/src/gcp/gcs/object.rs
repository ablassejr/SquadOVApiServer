use crate::SquadOvError;
use reqwest::StatusCode;

impl super::GCSClient {
    pub async fn get_object(&self, bucket_id: &str, path: &str) -> Result<(), SquadOvError> {
        let client = self.http.create_http_client()?;

        // TODO: Parse this response if we ever need it.
        let resp = client.get(
            &format!(
                "{}/b/{}/o/{}",
                super::STORAGE_BASE_URL,
                bucket_id,
                crate::url_encode(path),
            ))
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::NotFound);
        }
        Ok(())
    }

    pub async fn delete_object(&self, bucket_id: &str, path: &str) -> Result<(), SquadOvError> {
        let client = self.http.create_http_client()?;

        let resp = client.delete(
            &format!(
                "{}/b/{}/o/{}",
                super::STORAGE_BASE_URL,
                bucket_id,
                crate::url_encode(path),
            ))
            .send()
            .await?;

        if resp.status() != StatusCode::NO_CONTENT {
            return Err(SquadOvError::InternalError(format!("GCS Error: {} - {}", resp.status(), resp.text().await?)));
        }
        Ok(())
    }
}