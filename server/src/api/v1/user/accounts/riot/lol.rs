use crate::api;
use squadov_common::{SquadOvError};
use uuid::Uuid;

impl api::ApiApplication {
    pub async fn list_squadov_accounts_in_lol_match(&self, match_uuid: &Uuid) -> Result<Vec<i64>, SquadOvError> {
        // We need to go from the puuids of the users in the match to the user IDs stored by us
        // and then go from that to the users in the squads that can access the match.
        Ok(
            sqlx::query!(
                "
                SELECT DISTINCT ora.user_id
                FROM squadov.lol_match_participant_identities AS lmpi
                INNER JOIN squadov.riot_accounts AS ra
                    ON ra.summoner_id = lmpi.summoner_id
                INNER JOIN squadov.riot_account_links AS ral
                    ON ral.puuid = ra.puuid
                LEFT JOIN squadov.squad_role_assignments AS sra
                    ON sra.user_id = ral.user_id
                LEFT JOIN squadov.squad_role_assignments AS ora
                    ON ora.squad_id = sra.squad_id
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