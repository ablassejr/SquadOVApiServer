use crate::common;
use reqwest::StatusCode;

impl super::GCSClient {
    pub async fn get_object(&self, bucket_id: &str, path: &str) -> Result<(), common::SquadOvError> {
        let client = self.http.create_http_client()?;

        // TODO: Parse this response if we ever need it.
        let resp = client.get(
            &format!(
                "{}/b/{}/o/{}",
                super::STORAGE_BASE_URL,
                bucket_id,
                common::url_encode(path),
            ))
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(common::SquadOvError::NotFound);
        }
        Ok(())
    }

    pub async fn delete_object(&self, bucket_id: &str, path: &str) -> Result<(), common::SquadOvError> {
        let client = self.http.create_http_client()?;

        let resp = client.delete(
            &format!(
                "{}/b/{}/o/{}",
                super::STORAGE_BASE_URL,
                bucket_id,
                common::url_encode(path),
            ))
            .send()
            .await?;

        if resp.status() != StatusCode::NO_CONTENT {
            return Err(common::SquadOvError::InternalError(format!("GCS Error: {}", resp.status())));
        }
        Ok(())
    }
}