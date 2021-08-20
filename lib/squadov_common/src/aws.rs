use rusoto_core::{Region, HttpClient};
use rusoto_s3::S3Client;
use rusoto_credential::ProfileProvider;
use serde::{Deserialize};

#[derive(Deserialize,Debug,Clone)]
pub struct AWSCDNConfig {
    pub public_cdn_domain: String,
    pub private_cdn_domain: String,
    pub public_key_id: String,
    pub private_key_fname: String,
}

#[derive(Deserialize,Debug,Clone)]
pub struct AWSConfig {
    pub enabled: bool,
    pub credential_path: String,
    pub profile: String,
    pub cdn: AWSCDNConfig,
}

pub struct AWSClient {
    pub region: Region,
    pub provider: ProfileProvider,
    pub s3: S3Client,
}

impl AWSClient {
    pub fn new(config: &AWSConfig) -> Self {
        let provider = ProfileProvider::with_configuration(&config.credential_path, &config.profile);
        // TODO: Don't hard-code region.
        let region = Region::UsEast2;
        Self {
            region: region.clone(),
            provider: provider.clone(),
            s3: S3Client::new_with(HttpClient::new().unwrap(), provider.clone(), region)
        }
    }
}