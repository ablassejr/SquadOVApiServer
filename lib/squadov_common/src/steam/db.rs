use crate::{
    SquadOvError,
    steam::api::SteamPlayerSummary,
};
use sqlx::{Executor, Transaction, Postgres};

pub async fn get_steam_accounts_that_need_sync<'a, T>(ex: T, steam_ids: &[i64]) -> Result<Vec<i64>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT steam_id
            FROM squadov.steam_users_cache
            WHERE steam_id = ANY($1)
                AND last_sync_time IS NULL
                    OR last_sync_time - NOW() > INTERVAL '1 day'
            ",
            steam_ids
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| { x.steam_id })
            .collect()
    )
}

pub async fn sync_steam_player_summaries(ex: &mut Transaction<'_, Postgres>, summaries: &[SteamPlayerSummary]) -> Result<(), SquadOvError> {
    if summaries.is_empty() {
        return Ok(());
    }

    let mut sql: Vec<String> = vec![];
    sql.push(String::from("
        INSERT INTO squadov.steam_users_cache (
            steam_id,
            steam_name,
            profile_image_url,
            last_sync_time
        ) VALUES
    "));

    for s in summaries {
        sql.push(format!("(
            {steam_id},
            {steam_name},
            {profile_image_url},
            NOW()
        )",
            steam_id=s.steamid.parse::<i64>()?,
            steam_name=crate::sql_format_string(&s.personaname),
            profile_image_url=crate::sql_format_string(&s.avatarfull),
        ));
        sql.push(String::from(","));
    }

    sql.truncate(sql.len() - 1);
    sql.push(String::from("
        ON CONFLICT (steam_id) DO UPDATE SET
            steam_name = EXCLUDED.steam_name,
            profile_image_url = EXCLUDED.profile_image_url,
            last_sync_time = EXCLUDED.last_sync_time
    "));
    sqlx::query(&sql.join("")).execute(&mut *ex).await?;

    Ok(())
}