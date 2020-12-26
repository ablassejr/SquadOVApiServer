use crate::SquadOvError;
use reqwest::{StatusCode, header::HeaderMap};
use serde::Serialize;
use byteorder::{ByteOrder, BigEndian};
use rand::Rng;
use actix_web::web::Bytes;

#[derive(Serialize)]
struct GCSObjectMetadata {
    crc32c: String,
    md5_hash: String,
    name: String
}

#[derive(PartialEq)]
pub enum GCSUploadStatus {
    Complete,
    Incomplete(usize)
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
            let status = resp.status().as_u16();
            return Err(match status {
                404 => SquadOvError::NotFound,
                _ => SquadOvError::InternalError(format!("GCS Get Object Error: {} - {}", status, resp.text().await?))
            });
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
            let status = resp.status().as_u16();
            return Err(match status {
                404 => SquadOvError::NotFound,
                _ => SquadOvError::InternalError(format!("GCS Download Object Error: {} - {}", status, resp.text().await?))
            });
        }

        Ok(resp.bytes().await?.into_iter().collect())
    }

    // Returns whether or not the upload is complete or is incomplete.
    pub async fn get_upload_status(&self, session_uri: &str) -> Result<GCSUploadStatus, SquadOvError> {
        let client = self.http.read()?.create_http_client()?;
        let mut addtl_headers = HeaderMap::new();
        addtl_headers.insert("Content-Length", "0".parse()?);
        addtl_headers.insert("Content-Range", "bytes */*".parse()?);
        let resp = client.put(session_uri).headers(addtl_headers).send().await?;

        match resp.status().as_u16() {
            200 | 201 => Ok(GCSUploadStatus::Complete),
            308 => {
                let ret_headers = resp.headers();
                let range = ret_headers.get("Range");
                if range.is_none() {
                    return Ok(GCSUploadStatus::Incomplete(0));
                }

                let tokens: Vec<&str> = range.unwrap().to_str()?.split('-').collect();
                if tokens.is_empty() {
                    return Err(SquadOvError::InternalError(String::from("No range [no separator found]")));
                }
                Ok(GCSUploadStatus::Incomplete(tokens.last().unwrap().parse()?))
            },
            _ => Err(SquadOvError::InternalError(format!("GCS Check Upload Status Error: {} - {}", resp.status().as_u16(), resp.text().await?)))
        }
    }

    pub async fn initiate_resumable_upload_session(&self, bucket_id: &str, path_parts: &Vec<String>) -> Result<String, SquadOvError> {
        let client = self.http.read()?.create_http_client()?;
        let mut addtl_headers = HeaderMap::new();
        addtl_headers.insert("Content-Length", "0".parse()?);
        addtl_headers.insert("Content-Type", "application/octet-stream".parse()?);
        addtl_headers.insert("x-goog-resumable", "start".parse()?);
        let path = path_parts
            .iter()
            .map(|x| {
                crate::url_encode(x)
            })
            .collect::<Vec<String>>()
            .join("/");
        let url = format!("{}/{}/{}", super::STORAGE_XML_BASE_URL, bucket_id, &path);
        let resp = client.post(&url).headers(addtl_headers).send().await?;

        match resp.status().as_u16() {
            200 | 201 => {
                let ret_headers = resp.headers();
                let loc = ret_headers.get("Location");
                if loc.is_none() {
                    return Err(SquadOvError::InternalError(String::from("No location")));
                }

                Ok(String::from(loc.unwrap().to_str()?))
            },
            _ => Err(SquadOvError::InternalError(format!("Failed to initiate GCS resumable upload {} - {}", resp.status().as_u16(), resp.text().await?)))
        }
    }

    pub async fn upload_resumable_object(&self, session: &str, last_byte: usize, data: Bytes, last: bool) -> Result<(), SquadOvError> {
        let client = self.http.read()?.create_http_client()?;

        let mut current_first_byte = 0;
        let mut current_chunk_last_byte = last_byte;
        let desired_chunk_end_byte = last_byte + data.len() - 1;
        while current_chunk_last_byte < desired_chunk_end_byte {
            let mut addtl_headers = HeaderMap::new();
            addtl_headers.insert("Content-Length", format!("{}", data.len()).parse()?);
            addtl_headers.insert("Content-Type", "application/octet-stream".parse()?);

            let content_range;
            if last {
                content_range = format!("bytes {}-{}/{}", current_chunk_last_byte, desired_chunk_end_byte, desired_chunk_end_byte + 1);
            } else {
                content_range = format!("bytes {}-{}/*", current_chunk_last_byte, desired_chunk_end_byte);
            }
            addtl_headers.insert("Content-Range", content_range.parse()?);
            
            let resp = client.put(session)
                .headers(addtl_headers)
                .body(data.slice(current_first_byte..data.len()))
                .send()
                .await?;

            match resp.status().as_u16() {
                200 | 201 => break,
                308 => {
                    let ret_headers = resp.headers();
                    let range = ret_headers.get("Range");
                    if range.is_none() {
                        return Err(SquadOvError::InternalError(String::from("No range")));
                    }
                    let range = range.unwrap().to_str()?;
                    let old_last_byte = current_chunk_last_byte;
                    current_chunk_last_byte = range.split('-').collect::<Vec<&str>>()[1].parse()?;
                    current_first_byte += current_chunk_last_byte - old_last_byte;
                },
                _ => return Err(SquadOvError::InternalError(format!("Failed to GCS resumable upload {} - {}", resp.status().as_u16(), resp.text().await?)))
            }
        }

        Ok(())
    }

    pub async fn upload_object(&self, bucket_id: &str, path_parts: &Vec<String>, data: &[u8]) -> Result<(), SquadOvError> {
        let mut backoff_tick = 0;
        let mut status = None;
        let mut body = None;

        let path = path_parts
            .iter()
            .map(|x| {
                crate::url_encode(x)
            })
            .collect::<Vec<String>>()
            .join("/");

        let mut i: i32 = 0;
        while i < 10 {
            log::info!("Trying to GCS Upload Object: {} - {} - {}", i, bucket_id, &path);
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
                name: path.clone(),
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