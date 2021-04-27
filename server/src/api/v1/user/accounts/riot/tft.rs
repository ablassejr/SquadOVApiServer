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
                SELECT DISTINCT ou.id
                FROM squadov.tft_match_participants AS tmp
                INNER JOIN squadov.riot_accounts AS ra
                    ON ra.puuid = tmp.puuid
                INNER JOIN squadov.riot_account_links AS ral
                    ON ral.puuid = ra.puuid
                LEFT JOIN squadov.squad_role_assignments AS sra
                    ON sra.user_id = ral.user_id
                LEFT JOIN squadov.squad_role_assignments AS ora
                    ON ora.squad_id = sra.squad_id
                INNER JOIN squadov.users AS ou
                    ON ou.id = ora.user_id
                        OR ou.id = ral.user_id
                WHERE tmp.match_uuid = $1
                ",
                match_uuid,
            )
                .fetch_all(&*self.pool)
                .await?
                .into_iter()
                .map(|x| {
                    x.id
                })
                .collect()
        )
    }
}