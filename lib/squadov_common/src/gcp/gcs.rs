mod bucket;
mod object;
mod signed_url;

const STORAGE_XML_BASE_URL : &'static str = "https://storage.googleapis.com";
const STORAGE_BASE_URL : &'static str = "https://storage.googleapis.com/storage/v1";
const STORAGE_UPLOAD_URL : &'static str = "https://storage.googleapis.com/upload/storage/v1";

use std::sync::{Arc, RwLock};

pub struct GCSClient {
    http: Arc<RwLock<super::GCPHttpAuthClient>>
}

impl GCSClient {
    pub fn new(http: Arc<RwLock<super::GCPHttpAuthClient>>) -> GCSClient {
        GCSClient{
            http: http,
        }
    }
}

pub use bucket::*;
pub use object::*;
pub use signed_url::*;