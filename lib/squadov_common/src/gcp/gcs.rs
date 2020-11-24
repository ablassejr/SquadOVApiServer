mod bucket;
mod object;
mod signed_url;

const STORAGE_BASE_URL : &'static str = "https://storage.googleapis.com/storage/v1";
const STORAGE_UPLOAD_URL : &'static str = "https://storage.googleapis.com/upload/storage/v1";

use std::sync::Arc;

pub struct GCSClient {
    http: Arc<super::GCPHttpAuthClient>
}

impl GCSClient {
    pub fn new(http: Arc<super::GCPHttpAuthClient>) -> GCSClient {
        GCSClient{
            http: http,
        }
    }
}

pub use bucket::*;
pub use object::*;
pub use signed_url::*;