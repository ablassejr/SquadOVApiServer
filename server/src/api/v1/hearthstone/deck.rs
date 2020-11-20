use squadov_common::SquadOvError;
use crate::api;
use sqlx::{Transaction, Executor, Postgres};
use squadov_common::hearthstone::{HearthstoneDeck, HearthstoneDeckSlot, HearthstoneCardCount};
use uuid::Uuid;
use chrono::Utc;

impl api::ApiApplication {
    pub async fn get_hearthstone_deck_for_match_user(&self, match_uuid: &Uuid, user_id: i64) -> Result<Option<HearthstoneDeck>, SquadOvError> {
        let deck_id : Option<i64> = sqlx::query_scalar(
            "
            SELECT deck_id
            FROM squadov.hearthstone_match_user_deck
            WHERE match_uuid = $1 AND user_id = $2
            "
        )
            .bind(match_uuid)
            .bind(user_id)
            .fetch_optional(&*self.pool)
            .await?;
        
        Ok(match deck_id {
            Some(x) => self.get_hearthstone_deck(x, user_id).await?,
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
            Some(x) => self.get_hearthstone_deck(x, user_id).await?,
            None => None
        })
    }

    pub async fn get_hearthstone_deck(&self, deck_id: i64, user_id: i64) -> Result<Option<HearthstoneDeck>, SquadOvError> {
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
        let raw_slots = sqlx::query!(
            "
            SELECT
                index,
                card_id,
                owned,
                normal_count,
                golden_count
            FROM squadov.hearthstone_deck_slots
            WHERE deck_id = $1
            ",
            deck_id,
        )
            .fetch_all(&*self.pool)
            .await?;

        Ok(Some(HearthstoneDeck{
            slots: raw_slots.into_iter().map(|x| {
                HearthstoneDeckSlot {
                    index: x.index,
                    card_id: x.card_id,
                    owned: x.owned,
                    count: HearthstoneCardCount{
                        normal: x.normal_count,
                        golden: x.golden_count
                    },
                }
            }).collect(),
            name: raw_deck.name,
            deck_id: raw_deck.deck_id,
            hero_card: raw_deck.hero_card,
            hero_premium: raw_deck.hero_premium,
            deck_type: raw_deck.deck_type,
            create_date: raw_deck.create_date,
            is_wild: raw_deck.is_wild
        }))
    }

    pub async fn store_hearthstone_deck_slots(&self, tx : &mut Transaction<'_, Postgres>, deck_id: i64, slots: &[HearthstoneDeckSlot]) -> Result<(), SquadOvError> {
        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.hearthstone_deck_slots (
                deck_id,
                index,
                card_id,
                owned,
                normal_count,
                golden_count
            )
            VALUES
        "));

        let mut added = 0;
        for (idx, m) in slots.iter().enumerate() {
            sql.push(format!("(
                {deck_id},
                {index},
                '{card_id}',
                {owned},
                {normal_count},
                {golden_count}
            )",
                deck_id=deck_id,
                index=m.index,
                card_id=&m.card_id,
                owned=m.owned,
                normal_count=m.count.normal,
                golden_count=m.count.golden,
            ));

            if idx != slots.len() - 1 {
                sql.push(String::from(","));
            }

            added += 1;
        }

        sql.push(String::from(" ON CONFLICT DO NOTHING"));
        if added > 0 {
            sqlx::query(&sql.join("")).execute(tx).await?;
        }
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
                ON CONFLICT DO NOTHING
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
                deck_id,
                user_id,
                match_uuid
            )
            VALUES (
                $1,
                $2,
                $3
            )
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