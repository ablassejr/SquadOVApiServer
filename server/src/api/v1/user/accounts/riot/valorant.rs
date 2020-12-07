use crate::api;
use squadov_common::{SquadOvError};

impl api::ApiApplication {
    pub async fn list_squadov_accounts_can_access_match(&self, match_id: &str) -> Result<Vec<i64>, SquadOvError> {
        // We need to go from the puuids of the users in the match to the user IDs stored by us
        // and then go from that to the users in the squads that can access the match.
        Ok(sqlx::query_scalar(
            "
            SELECT COALESCE(ora.user_id, ral.user_id)
            FROM squadov.valorant_match_players AS vmp
            INNER JOIN squadov.riot_account_links AS ral
                ON ral.puuid = vmp.puuid
            LEFT JOIN squadov.squad_role_assignments AS sra
                ON sra.user_id = ral.user_id
            LEFT JOIN squadov.squad_role_assignments AS ora
                ON ora.squad_id = sra.squad_id
            WHERE vmp.match_id = $1
            ",
        )
            .bind(match_id)
            .fetch_all(&*self.pool)
            .await?)
    }
}