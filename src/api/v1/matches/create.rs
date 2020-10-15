use crate::common;
use uuid::Uuid;
use sqlx;
use sqlx::{Executor, Transaction, Postgres};

impl crate::api::ApiApplication {

    // This is used to create a new "match." This shouldn't be called directly
    // but rather indirectly via a game-specific endpoint this is to prevent
    // multiple UUIDs representing a single logical "match." For example, take a
    // VALORANT match, there's a unique match ID for each VALORANT match. We can
    // have up to 10-14 players reporting that they're part of the same match. Thus
    // if they all call the 'create_new_match' endpoint, there'll be 10-14 UUIDs that
    // represent the same logical match. Thus, the match must be protected via a VALORANT
    // specific endpoint that only creates a new match for a new VALORANT match ID that
    // we haven't seen.
    pub async fn create_new_match(&self, tx : &mut Transaction<'_, Postgres>) -> Result<super::Match, common::SquadOvError> {
        let new_match = super::Match {
            uuid: Uuid::new_v4()
        };

        tx.execute(
            sqlx::query!(
                "
                INSERT INTO squadov.matches (uuid)
                VALUES ($1)
                ",
                new_match.uuid
            )
        ).await?;

        return Ok(new_match);
    }
}