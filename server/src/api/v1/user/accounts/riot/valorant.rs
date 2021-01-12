use crate::api;
use squadov_common::{SquadOvError};
use uuid::Uuid;

impl api::ApiApplication {
    pub async fn list_squadov_accounts_can_access_valorant_match(&self, match_uuid: &Uuid) -> Result<Vec<i64>, SquadOvError> {
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
            WHERE vmp.match_uuid = $1
            ",
        )
            .bind(match_uuid)
            .fetch_all(&*self.pool)
            .await?)
    }
}