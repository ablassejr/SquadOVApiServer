use crate::api;
use squadov_common::{SquadOvError};
use uuid::Uuid;

impl api::ApiApplication {
    pub async fn list_squadov_accounts_can_access_tft_match(&self, match_uuid: &Uuid) -> Result<Vec<i64>, SquadOvError> {
        // We need to go from the puuids of the users in the match to the user IDs stored by us
        // and then go from that to the users in the squads that can access the match.
        Ok(
            sqlx::query!(
                "
                SELECT ral.user_id
                FROM squadov.tft_match_participants AS tmp
                INNER JOIN squadov.riot_accounts AS ra
                    ON ra.puuid = tmp.puuid
                INNER JOIN squadov.riot_account_links AS ral
                    ON ral.puuid = ra.puuid
                WHERE tmp.match_uuid = $1
                ",
                match_uuid,
            )
                .fetch_all(&*self.pool)
                .await?
                .into_iter()
                .map(|x| {
                    x.user_id
                })
                .collect()
        )
    }
}