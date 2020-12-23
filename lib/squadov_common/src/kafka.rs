use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KafkaCredentialKeyPair {
    key: String,
    secret: String
}