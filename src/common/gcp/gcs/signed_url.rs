use crate::common;
use std::vec::Vec;
use chrono::{Utc, DateTime};
use sha2::{Sha256, Digest};
use openssl::sign::Signer;
use openssl::pkey::PKey;
use openssl::hash::MessageDigest;

impl super::GCSClient {
    fn create_credential_scope(&self, dt: &DateTime<Utc>) -> String {
        format!(
            "{date}/{location}/{service}/{request_type}",
            date=dt.format("%Y%m%d"),
            location="auto",
            service="storage",
            request_type="goog4_request"
        )
    }
    
    fn create_canonical_query_string(&self, dt: &DateTime<Utc>) -> String {
        let encoded_email = common::url_encode(&self.http.credentials.client_email);
        let encoded_scope = common::url_encode(&self.create_credential_scope(dt));
        
        // required query string parameters: 
        // - X-Goog-Algorithm
        // - X-Goog-Credential
        // - X-Goog-Date
        // - X-Goog-Expires
        // - X-Goog-SignedHeaders
        format!(
            "X-Goog-Algorithm={alg}&X-Goog-Credential={authorizer}%2F{scope}&X-Goog-Date={date}&X-Goog-Expires={expires}&X-Goog-SignedHeaders={headers}",
            alg="GOOG4-RSA-SHA256",
            authorizer=&encoded_email,
            scope=&encoded_scope,
            date=dt.format("%Y%m%dT%H%M%SZ"),
            expires="43200", // ~12 hours. Should be good enough for pretty much all situations.
            headers="host"
        )
    }

    fn create_canonical_request(&self, dt: &DateTime<Utc>, bucket_id: &str, path: &str) -> String {
        let mut contents: Vec<String> = Vec::new();

        // HTTP verb
        contents.push(String::from("GET"));

        // path to resource
        contents.push(format!("/{}/{}", bucket_id, path));

        // canonical query string
        contents.push(self.create_canonical_query_string(dt));

        // canonical headers
        contents.push(String::from("host:storage.googleapis.com"));
        
        // mandatory new line
        contents.push(String::from(""));

        // Signed headers
        contents.push(String::from("host"));

        // payload
        contents.push(String::from("UNSIGNED-PAYLOAD"));

        return contents.join("\n")
    }

    fn create_string_to_sign(&self, dt: &DateTime<Utc>, canonical_request: &str) -> String {
        let mut contents: Vec<String> = Vec::new();
        let hashed_data = hex::encode(Sha256::digest(canonical_request.as_bytes()).as_slice());

        contents.push(String::from("GOOG4-RSA-SHA256"));
        contents.push(dt.format("%Y%m%dT%H%M%SZ").to_string());
        contents.push(self.create_credential_scope(dt));
        contents.push(hashed_data);
        return contents.join("\n")
    }

    fn create_request_signature(&self, dt: &DateTime<Utc>, canonical_request: &str) -> Result<String, common::SquadOvError> {
        let pkey = PKey::private_key_from_pem(self.http.credentials.private_key.as_bytes())?;
        let mut signer = Signer::new(MessageDigest::sha256(), &pkey)?;
        let string_to_sign = self.create_string_to_sign(dt, canonical_request);
        signer.update(string_to_sign.as_bytes())?;
        Ok(hex::encode(signer.sign_to_vec()?))
    }

    pub fn create_signed_url(&self, bucket_id: &str, path: &str) -> Result<String, common::SquadOvError> {
        let now_time = Utc::now();
        let canonical_request = self.create_canonical_request(&now_time, bucket_id, path);
        let signature = self.create_request_signature(&now_time, &canonical_request)?;

        Ok(
            format!(
                "{host}/{bucket}/{path}?{query}&X-Goog-Signature={sig}",
                host="https://storage.googleapis.com",
                bucket=bucket_id,
                path=path,
                query=self.create_canonical_query_string(&now_time),
                sig=&signature,
            )
        )
    }
}