use crate::common;
use std::vec::Vec;
use chrono::{Utc, DateTime};
use sha2::{Sha256, Digest};
use openssl::sign::Signer;
use openssl::pkey::PKey;
use openssl::hash::MessageDigest;
use url::Url;
use std::collections::{BTreeMap, btree_map::Entry};

const GOOGLE_STORAGE_API_URL: &str = "https://storage.googleapis.com";

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

    fn create_required_query_parameters(&self, dt: &DateTime<Utc>, headers: &BTreeMap<String, Vec<String>>) -> BTreeMap<String, String> {
        let encoded_email = common::url_encode(&self.http.credentials.client_email);
        let encoded_scope = common::url_encode(&self.create_credential_scope(dt));

        let mut ret: BTreeMap<String, String> = BTreeMap::new();
        // required query string parameters: 
        // - X-Goog-Algorithm
        // - X-Goog-Credential
        // - X-Goog-Date
        // - X-Goog-Expires
        // - X-Goog-SignedHeaders
        ret.insert("X-Goog-Algorithm".to_string(), "GOOG4-RSA-SHA256".to_string());
        ret.insert("X-Goog-Credential".to_string(), format!("{authorizer}%2F{scope}",
            authorizer=&encoded_email,
            scope=&encoded_scope,
        ));
        ret.insert("X-Goog-Date".to_string(), dt.format("%Y%m%dT%H%M%SZ").to_string());
        ret.insert("X-Goog-Expires".to_string(), "43200".to_string());
        ret.insert("X-Goog-SignedHeaders".to_string(), headers.keys().map(|x| x.clone()).collect::<Vec<String>>().join("%3B"));
        ret
    }

    fn create_canonical_base_url(&self, dt: &DateTime<Utc>, uri: Url, headers: &BTreeMap<String, Vec<String>>) -> Result<Url, common::SquadOvError> {
        let mut ret_uri = uri;

        // The main thing we want to change is the query string so that it *also* contains the
        // canonical query string. Note that the query string keys must be sorted in alphabetical order.
        let mut all_query_params: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (key, value) in ret_uri.query_pairs().into_owned() {
            match all_query_params.entry(key) {
                Entry::Vacant(e) => { e.insert(vec![value]); },
                Entry::Occupied(mut e) => {e.get_mut().push(value); }
            }
        }

        for (key, value) in self.create_required_query_parameters(dt, headers).into_iter() {
            all_query_params.insert(key, vec![value]);
        }

        let mut new_query_params: Vec<String> = Vec::new();
        for (key, values) in all_query_params.iter() {
            for v in values {
                new_query_params.push(format!("{}={}", key, v));
            }
        }
        ret_uri.set_query(Some(&new_query_params.join("&")));

        Ok(ret_uri)
    }

    fn create_canonical_request(&self, method: &str, uri: &Url, headers: &BTreeMap<String, Vec<String>>) -> String {
        let mut contents: Vec<String> = Vec::new();

        // HTTP verb
        contents.push(String::from(method));

        // path to resource
        contents.push(String::from(uri.path()));

        // canonical query string
        contents.push(String::from(uri.query().unwrap_or("")));

        // canonical headers
        for (key, values) in headers.iter() {
            contents.push(format!("{}:{}", key, values.join(",")));
        }
        
        // mandatory new line
        contents.push(String::from(""));

        // Signed headers
        contents.push(headers.keys().map(|x| x.clone()).collect::<Vec<String>>().join(";"));

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

    pub fn create_signed_url(&self, method: &str, uri: &str, addtl_headers: &BTreeMap<String, Vec<String>>) -> Result<String, common::SquadOvError> {
        let mut headers = addtl_headers.clone();
        headers.insert(String::from("host"), vec![String::from("storage.googleapis.com")]);

        let uri = format!("{}{}", GOOGLE_STORAGE_API_URL, uri);
        let uri = Url::parse(&uri)?;

        let now_time = Utc::now();
        let uri = self.create_canonical_base_url(&now_time, uri, &headers)?;
        

        let canonical_request = self.create_canonical_request(method, &uri, &headers);
        let signature = self.create_request_signature(&now_time, &canonical_request)?;

        Ok(
            format!(
                "{uri}&X-Goog-Signature={sig}",
                uri=uri.to_string(),
                sig=&signature,
            )
        )
    }
}