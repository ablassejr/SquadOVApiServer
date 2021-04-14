use crate::api;
use squadov_common::{SquadOvError};
use uuid::Uuid;

impl api::ApiApplication {
    pub async fn list_squadov_accounts_can_access_lol_match(&self, match_uuid: &Uuid) -> Result<Vec<i64>, SquadOvError> {
        // We need to go from the puuids of the users in the match to the user IDs stored by us
        // and then go from that to the users in the squads that can access the match.
        Ok(
            sqlx::query!(
                "
                SELECT ral.user_id
                FROM squadov.lol_match_participant_identities AS lmpi
                INNER JOIN squadov.riot_accounts AS ra
                    ON ra.account_id = lmpi.account_id
                INNER JOIN squadov.riot_account_links AS ral
                    ON ral.puuid = ra.puuid
                WHERE lmpi.match_uuid = $1
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