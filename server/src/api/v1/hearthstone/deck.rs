use squadov_common::SquadOvError;
use crate::api;
use sqlx::{Transaction, Executor, Postgres};
use squadov_common::hearthstone::{HearthstoneDeck, HearthstoneDeckSlot, HearthstoneCardCount, are_deck_slots_equivalent};
use uuid::Uuid;
use chrono::Utc;

impl api::ApiApplication {
    pub async fn get_hearthstone_deck_for_match_user(&self, match_uuid: &Uuid, user_id: i64) -> Result<Option<HearthstoneDeck>, SquadOvError> {
        let data = sqlx::query!(
            "
            SELECT hmud.deck_version_id, hdv.deck_id
            FROM squadov.hearthstone_match_user_deck AS hmud
            INNER JOIN squadov.hearthstone_deck_versions AS hdv
                ON hdv.version_id = hmud.deck_version_id
            WHERE hmud.match_uuid = $1 AND hmud.user_id = $2
            ",
            match_uuid,
            user_id
        )
            .fetch_optional(&*self.pool)
            .await?;
        
        Ok(match data {
            Some(x) => self.get_versioned_hearthstone_deck(x.deck_id, x.deck_version_id, user_id).await?,
            None => None
        })
    }

    pub async fn get_hearthstone_deck_for_arena_run(&self, arena_uuid: &Uuid, user_id: i64) -> Result<Option<HearthstoneDeck>, SquadOvError> {
        let deck_id : Option<i64> = sqlx::query_scalar(
            "
            SELECT draft_deck_id
            FROM squadov.hearthstone_arena_drafts
            WHERE collection_uuid = $1 AND user_id = $2
            "
        )
            .bind(arena_uuid)
            .bind(user_id)
            .fetch_optional(&*self.pool)
            .await?;
        
        Ok(match deck_id {
            Some(x) => self.get_latest_hearthstone_deck(x, user_id).await?,
            None => None
        })
    }

    pub async fn get_versioned_hearthstone_deck(&self, deck_id: i64, version_id: i64, user_id: i64) -> Result<Option<HearthstoneDeck>, SquadOvError> {
        let raw_deck = sqlx::query!(
            "
            SELECT
                deck_id,
                hero_card,
                hero_premium,
                deck_type,
                create_date,
                is_wild,
                deck_name AS \"name\"
            FROM squadov.hearthstone_decks
            WHERE deck_id = $1
                AND user_id = $2
            ",
            deck_id,
            user_id
        )
            .fetch_optional(&*self.pool)
            .await?;

        if raw_deck.is_none() {
            return Ok(None);
        }

        let raw_deck = raw_deck.unwrap();

        Ok(Some(HearthstoneDeck{
            slots: self.get_hearthstone_deck_slots_for_version(version_id).await?,
            name: raw_deck.name,
            deck_id: raw_deck.deck_id,
            hero_card: raw_deck.hero_card,
            hero_premium: raw_deck.hero_premium,
            deck_type: raw_deck.deck_type,
            create_date: raw_deck.create_date,
            is_wild: raw_deck.is_wild
        }))
    }

    pub async fn get_latest_hearthstone_deck(&self, deck_id: i64, user_id: i64) -> Result<Option<HearthstoneDeck>, SquadOvError> {
        let version = self.get_latest_deck_version_id(deck_id).await?;
        if version.is_none() {
            Ok(None)
        } else {
            Ok(self.get_versioned_hearthstone_deck(deck_id, version.unwrap(), user_id).await?)
        }
    }

    pub async fn get_latest_deck_version_id(&self, deck_id: i64) -> Result<Option<i64>, SquadOvError> {
        Ok(sqlx::query_scalar(
            "
            SELECT version_id
            FROM squadov.hearthstone_deck_versions
            WHERE deck_id = $1
            ORDER BY version_id DESC
            LIMIT 1
            "
        )
            .bind(deck_id)
            .fetch_optional(&*self.pool)
            .await?)
    }

    pub async fn create_new_deck_version(&self, tx : &mut Transaction<'_, Postgres>, deck_id: i64) -> Result<i64, SquadOvError> {
        Ok(sqlx::query_scalar(
            "
            INSERT INTO squadov.hearthstone_deck_versions(deck_id)
            VALUES ($1)
            RETURNING version_id
            "
        )
            .bind(deck_id)
            .fetch_one(tx)
            .await?)
    }

    pub async fn get_latest_hearthstone_deck_slots_for_deck(&self, deck_id: i64) -> Result<Vec<HearthstoneDeckSlot>, SquadOvError> {
        let version_id = self.get_latest_deck_version_id(deck_id).await?;
        Ok(match version_id {
            Some(id) => self.get_hearthstone_deck_slots_for_version(id).await?,
            None => vec![],
        })
    }
    
    pub async fn get_hearthstone_deck_slots_for_version(&self, version_id: i64) -> Result<Vec<HearthstoneDeckSlot>, SquadOvError> {
        let raw_slots = sqlx::query!(
            "
            SELECT
                index,
                card_id,
                owned,
                normal_count,
                golden_count
            FROM squadov.hearthstone_deck_slots
            WHERE deck_version_id = $1
            ",
            version_id,
        )
            .fetch_all(&*self.pool)
            .await?;

        Ok(raw_slots.into_iter().map(|x| {
            HearthstoneDeckSlot {
                index: x.index,
                card_id: x.card_id,
                owned: x.owned,
                count: HearthstoneCardCount{
                    normal: x.normal_count,
                    golden: x.golden_count
                },
            }
        }).collect())
    }

    pub async fn store_hearthstone_deck_slots(&self, tx : &mut Transaction<'_, Postgres>, deck_id: i64, slots: &[HearthstoneDeckSlot]) -> Result<(), SquadOvError> {
        // Deck slots need to be able to handle the case where we're using the same deck ID but it has different cards in it.
        // From what I gather, a new deck ID is only created when the user creates a new deck in their collection but is not
        // created when the user edits an existing deck with new cards. Thus, we need to create a concept of "deck version."
        // If the stored slots don't EXACTLY match the input slots, then we have to assume that the user edited their deck and
        // created a new version of the deck. We associate versions of a deck with a match so that the user is still able to see
        // exactly what deck they used in any given match.
        let latest_slots = self.get_latest_hearthstone_deck_slots_for_deck(deck_id).await?;
        let are_equivalent = are_deck_slots_equivalent(&latest_slots, slots);
        let version_id = if !latest_slots.is_empty() && are_equivalent {
            self.get_latest_deck_version_id(deck_id).await?.unwrap()
        } else {
            self.create_new_deck_version(tx, deck_id).await?
        };

        // This should be here AFTER creating a new version because we want
        // a version to be registered even if the deck is empty (e.g. when the
        // user starts an arena draft).
        if slots.len() == 0 {
            return Ok(())
        }

        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.hearthstone_deck_slots (
                deck_version_id,
                index,
                card_id,
                owned,
                normal_count,
                golden_count
            )
            VALUES
        "));

        for (idx, m) in slots.iter().enumerate() {
            sql.push(format!("(
                {deck_version_id},
                {index},
                '{card_id}',
                {owned},
                {normal_count},
                {golden_count}
            )",
                deck_version_id=version_id,
                index=m.index,
                card_id=&m.card_id,
                owned=m.owned,
                normal_count=m.count.normal,
                golden_count=m.count.golden,
            ));

            if idx != slots.len() - 1 {
                sql.push(String::from(","));
            }
        }

        sql.push(String::from(" ON CONFLICT DO NOTHING"));
        sqlx::query(&sql.join("")).execute(tx).await?;
        Ok(())
    }

    pub async fn create_empty_hearthstone_deck(&self, tx : &mut Transaction<'_, Postgres>, deck_id: i64, user_id: i64) -> Result<(), SquadOvError> {
        let new_deck = HearthstoneDeck{
            name: String::new(),
            deck_id: deck_id,
            hero_card: String::new(),
            hero_premium: 0,
            deck_type: 0,
            create_date: Utc::now(),
            is_wild: false,
            slots: Vec::new(),
        };

        self.store_hearthstone_deck(tx, &new_deck, user_id).await?;
        Ok(())
    }

    pub async fn store_hearthstone_deck(&self, tx : &mut Transaction<'_, Postgres>, deck: &HearthstoneDeck, user_id: i64) -> Result<(), SquadOvError> {
        tx.execute(
            sqlx::query!(
                "
                INSERT INTO squadov.hearthstone_decks (
                    user_id,
                    deck_id,
                    deck_name,
                    hero_card,
                    hero_premium,
                    deck_type,
                    create_date,
                    is_wild
                )
                VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    $8
                )
                ON CONFLICT (deck_id) DO UPDATE SET
                    user_id = EXCLUDED.user_id,
                    deck_name = EXCLUDED.deck_name,
                    hero_card = EXCLUDED.hero_card,
                    hero_premium = EXCLUDED.hero_premium,
                    deck_type = EXCLUDED.deck_type,
                    create_date = EXCLUDED.create_date,
                    is_wild = EXCLUDED.is_wild
                ",
                user_id,
                deck.deck_id,
                deck.name,
                deck.hero_card,
                deck.hero_premium,
                deck.deck_type,
                deck.create_date,
                deck.is_wild
            )
        ).await?;

        self.store_hearthstone_deck_slots(tx, deck.deck_id, &deck.slots).await?;
        Ok(())
    }

    pub async fn associate_deck_with_match_user(&self, tx : &mut Transaction<'_, Postgres>, deck_id: i64, match_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.hearthstone_match_user_deck (
                deck_version_id,
                user_id,
                match_uuid
            )
            SELECT version_id, $2, $3
            FROM squadov.hearthstone_deck_versions
            WHERE deck_id = $1
            ORDER BY version_id DESC
            LIMIT 1
            ",
            deck_id,
            user_id,
            match_uuid,
        )
            .execute(tx)
            .await?;
        Ok(())
    }

}