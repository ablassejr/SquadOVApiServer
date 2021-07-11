use sqlx::postgres::{PgPool};
use std::sync::Arc;

pub struct AWSBlobManager {
    bucket: String,
    db: Arc<PgPool>,
}