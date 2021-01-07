use uuid::Uuid;
use sqlx::{Executor, Postgres};
use crate::{
    SquadOvError,
};

pub async fn create_new_match<'a, T>(ex: T) -> Result<Uuid, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let uuid = Uuid::new_v4();
    sqlx::query!(
        "
        INSERT INTO squadov.matches (uuid)
        VALUES ($1)
        ",
        &uuid,
    )
        .execute(ex)
        .await?;

    Ok(uuid)
}