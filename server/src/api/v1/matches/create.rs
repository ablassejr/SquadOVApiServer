use squadov_common;
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
    pub async fn create_new_match(&self, tx : &mut Transaction<'_, Postgres>) -> Result<super::Match, squadov_common::SquadOvError> {
        let new_match = super::Match {
            uuid: Uuid::new_v4(),
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

    // Similarly to the create_new_match function, this creates a new "match collection" which is just
    // a logical grouping of matches. It should be accessed via game-specific endpoints.
    pub async fn create_new_match_collection(&self, tx : &mut Transaction<'_, Postgres>) -> Result<super::MatchCollection, squadov_common::SquadOvError> {
        let new_collection = super::MatchCollection {
            uuid: Uuid::new_v4()
        };

        tx.execute(
            sqlx::query!(
                "
                INSERT INTO squadov.match_collections (uuid)
                VALUES ($1)
                ",
                new_collection.uuid
            )
        ).await?;

        return Ok(new_collection);
    }

    pub async fn bullk_create_matches(&self, tx : &mut Transaction<'_, Postgres>, count : usize) -> Result<Vec<super::Match>, squadov_common::SquadOvError> {
        let matches = sqlx::query_as!(
            super::Match,
            "
            INSERT INTO squadov.matches (uuid)
            SELECT gen_random_uuid()
            FROM generate_series(1, $1)
            RETURNING *
            ",
            count as i32
        )
            .fetch_all(tx)
            .await?;
        return Ok(matches);
    }
}