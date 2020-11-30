use crate::SquadOvError;
use reqwest::{StatusCode, header::HeaderMap};
use serde::Serialize;
use byteorder::{ByteOrder, BigEndian};
use rand::Rng;

#[derive(Serialize)]
struct GCSObjectMetadata {
    crc32c: String,
    md5_hash: String,
    name: String
}

impl super::GCSClient {
    pub async fn get_object(&self, bucket_id: &str, path: &str) -> Result<(), SquadOvError> {
        let client = self.http.read()?.create_http_client()?;

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
            return Err(SquadOvError::InternalError(format!("GCS Get Object Error: {} - {}", resp.status(), resp.text().await?)));
        }
        Ok(())
    }

    
    pub async fn download_object(&self, bucket_id: &str, path: &str) -> Result<Vec<u8>, SquadOvError> {
        let client = self.http.read()?.create_http_client()?;

        let resp = client.get(
            &format!(
                "{}/b/{}/o/{}?alt=media",
                super::STORAGE_BASE_URL,
                bucket_id,
                crate::url_encode(path),
            ))
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::InternalError(format!("GCS Download Object Error: {} - {}", resp.status(), resp.text().await?)));
        }

        Ok(resp.bytes().await?.into_iter().collect())
    }

    pub async fn upload_object(&self, bucket_id: &str, path: &str, data: &[u8]) -> Result<(), SquadOvError> {
        let mut backoff_tick = 0;
        let mut status = None;
        let mut body = None;

        let mut i: i32 = 0;
        while i < 10 {
            log::info!("Trying to GCS Upload Object: {}", i);
            let client = self.http.read()?.create_http_client()?;
            let boundary = "squadov_gcs";
            let mut send_data: Vec<String> = Vec::new();
            send_data.push(format!("--{}", boundary));
            send_data.push(String::from("Content-Type: application/json; charset=UTF-8"));
            send_data.push(String::new());

            let crc32c = crc32c::crc32c(data);
            let mut crc32c_bytes = [0; 4];
            BigEndian::write_u32(&mut crc32c_bytes, crc32c);

            let metadata = GCSObjectMetadata{
                crc32c: base64::encode(crc32c_bytes),
                md5_hash: format!("{:x}", md5::compute(data)),
                name: crate::url_encode(path),
            };
            send_data.push(serde_json::to_string_pretty(&metadata)?);

            send_data.push(format!("--{}", boundary));
            send_data.push(String::from("Content-Type: application/octet-stream"));
            send_data.push(String::from("Content-Transfer-Encoding: base64"));
            send_data.push(String::new());
            send_data.push(base64::encode(data));

            send_data.push(format!("--{}--", boundary));

            let final_send_data = send_data.join("\n");

            let mut addtl_headers = HeaderMap::new();
            addtl_headers.insert("Content-Type", format!("multipart/related; boundary={}", &boundary).parse()?);
            addtl_headers.insert("Content-Length", format!("{}", final_send_data.len()).parse()?);

            let resp = client.post(
                &format!(
                    "{}/b/{}/o?uploadType=multipart",
                    super::STORAGE_UPLOAD_URL,
                    bucket_id
                ))
                .headers(addtl_headers)
                .body(final_send_data)
                .send()
                .await?;
            i += 1;

            if resp.status() != StatusCode::OK {
                status = Some(resp.status());
                body = Some(resp.text().await?);
                let mut sleep_ms: u64 = 500;
                // 400 errors should result in a retry w/o exponential backoff. 
                // 500 errors should result in a retry with exponential backoff.
                if status.unwrap().as_u16() > 500 {
                    sleep_ms = 2u64.pow(backoff_tick) + rand::thread_rng().gen_range(0, 1000);
                    backoff_tick += 1;
                }
                
                log::info!("(Retry) Failed to upload GCS Upload Object: {:?} - {:?}", status, body);
                async_std::task::sleep(std::time::Duration::from_millis(sleep_ms)).await;
                continue;
            }

            return Ok(())
        }
        
        Err(SquadOvError::InternalError(format!("Failed to upload GCS: {:?} - {:?}", status, body)))
    }

    pub async fn delete_object(&self, bucket_id: &str, path: &str) -> Result<(), SquadOvError> {
        let client = self.http.read()?.create_http_client()?;

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
            return Err(SquadOvError::InternalError(format!("GCS Delete Object Error: {} - {}", resp.status(), resp.text().await?)));
        }
        Ok(())
    }
}