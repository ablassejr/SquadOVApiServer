mod riot;

pub use riot::*;

use crate::api;
use squadov_common::SquadOvError;
use uuid::Uuid;
use std::collections::HashMap;

impl api::ApiApplication {

    pub async fn get_user_uuid_to_user_id_map(&self, uuids: &[Uuid]) -> Result<HashMap<Uuid, i64>, SquadOvError> {
        Ok(sqlx::query!(
            "
            SELECT u.uuid, u.id
            FROM squadov.users AS u
            WHERE u.uuid = any($1)
            ",
            uuids
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|x| { (x.uuid, x.id) } )
            .collect())
    }

}