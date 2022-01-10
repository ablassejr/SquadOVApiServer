use serde::Deserialize;

#[derive(Deserialize,Debug,Clone)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: usize,
    pub timeout_ms: u64,
}
