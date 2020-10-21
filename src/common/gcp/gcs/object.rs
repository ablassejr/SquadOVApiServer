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
                path,
            ))
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(common::SquadOvError::NotFound);
        }
        Ok(())
    }
}