use sqlx::{Executor, Postgres};
use crate::{
    SquadOvError,
    hearthstone::{
        GameType,
        FormatType,
        HearthstoneDeckSlot,
        HearthstoneCardCount,
        HearthstoneDeck,
        game_packet::{
            HearthstoneGamePacket,
            HearthstoneMatchMetadata,
        },
        HearthstonePlayer,
        HearthstonePlayerMedalInfo,
        game_state::{
            HearthstoneGameSnapshot,
            HearthstoneGameSnapshotAuxData,
            HearthstoneEntity,
            game_step::GameStep,
        },
    },
};
use uuid::Uuid;
use std::collections::HashMap;
use std::convert::TryFrom;

pub async fn get_hearthstone_players_for_match<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<HashMap<i32, HearthstonePlayer>, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    let raw_match_players = sqlx::query!(
        "
        SELECT
            hmp.user_id,
            hmp.player_match_id,
            hmp.player_name,
            hmp.card_back_id,
            hmp.arena_wins,
            hmp.arena_loss,
            hmp.tavern_brawl_wins,
            hmp.tavern_brawl_loss,
            hmp.battlegrounds_rating,
            hmp.duels_casual_rating,
            hmp.duels_heroic_rating
        FROM squadov.hearthstone_match_players AS hmp
        INNER JOIN squadov.hearthstone_match_view AS hmv
            ON hmv.view_uuid = hmp.view_uuid
        WHERE hmv.match_uuid = $1 AND hmv.user_id = $2
        ",
        match_uuid,
        user_id
    )
        .fetch_all(ex)
        .await?;

    let raw_match_player_medals = sqlx::query!(
        "
        SELECT
            hmpm.player_match_id,
            hmpm.league_id,
            hmpm.earned_stars,
            hmpm.star_level,
            hmpm.best_star_level,
            hmpm.win_streak,
            hmpm.legend_index,
            hmpm.is_standard
        FROM squadov.hearthstone_match_player_medals AS hmpm
        INNER JOIN squadov.hearthstone_match_view AS hmv
            ON hmv.view_uuid = hmpm.view_uuid
        WHERE hmv.match_uuid = $1 AND hmv.user_id = $2
        ",
        match_uuid,
        user_id
    )
        .fetch_all(ex)
        .await?;

    let mut ret_map: HashMap<i32, HearthstonePlayer> = HashMap::new();
    for rmp in raw_match_players {
        let new_player = HearthstonePlayer{
            name: rmp.player_name,
            local: rmp.user_id.unwrap_or(-1) == user_id,
            side: 0,
            card_back_id: rmp.card_back_id,
            arena_wins: rmp.arena_wins as u32,
            arena_loss: rmp.arena_loss as u32,
            tavern_brawl_wins: rmp.tavern_brawl_wins as u32,
            tavern_brawl_loss: rmp.tavern_brawl_loss as u32,
            medal_info: HearthstonePlayerMedalInfo::new(),
            battlegrounds_rating: rmp.battlegrounds_rating,
            duels_casual_rating: rmp.duels_casual_rating,
            duels_heroic_rating: rmp.duels_heroic_rating,
        };

        ret_map.insert(rmp.player_match_id, new_player);
    }

    for medal in raw_match_player_medals {
        let player = ret_map.get_mut(&medal.player_match_id);
        if player.is_none() {
            continue;
        }
        let player = player.unwrap();
        let medal_info = if medal.is_standard {
            &mut player.medal_info.standard
        } else {
            &mut player.medal_info.wild
        };

        medal_info.league_id = medal.league_id;
        medal_info.earned_stars = medal.earned_stars;
        medal_info.star_level = medal.star_level;
        medal_info.best_star_level = medal.best_star_level;
        medal_info.win_streak = medal.win_streak;
        medal_info.legend_index = medal.legend_index;
    }

    Ok(ret_map)
}

pub async fn get_hearthstone_deck_slots_for_version<'a, T>(ex: T, version_id: i64) -> Result<Vec<HearthstoneDeckSlot>, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
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
        .fetch_all(ex)
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

pub async fn get_versioned_hearthstone_deck<'a, T>(ex: T, deck_id: i64, version_id: i64, user_id: i64) -> Result<Option<HearthstoneDeck>, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
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
        .fetch_optional(ex)
        .await?;

    if raw_deck.is_none() {
        return Ok(None);
    }

    let raw_deck = raw_deck.unwrap();

    Ok(Some(HearthstoneDeck{
        slots: get_hearthstone_deck_slots_for_version(ex, version_id).await?,
        name: raw_deck.name,
        deck_id: raw_deck.deck_id,
        hero_card: raw_deck.hero_card,
        hero_premium: raw_deck.hero_premium,
        deck_type: raw_deck.deck_type,
        create_date: raw_deck.create_date,
        is_wild: raw_deck.is_wild
    }))
}

pub async fn get_hearthstone_deck_for_match_user<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<Option<HearthstoneDeck>, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
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
        .fetch_optional(ex)
        .await?;
    
    Ok(match data {
        Some(x) => get_versioned_hearthstone_deck(ex, x.deck_id, x.deck_version_id, user_id).await?,
        None => None
    })
}

pub async fn get_hearthstone_snapshots_for_match<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<Vec<Uuid>, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    Ok(sqlx::query_scalar(
        "
        SELECT hs.snapshot_id
        FROM squadov.hearthstone_snapshots AS hs
        WHERE hs.match_uuid = $1 AND hs.user_id = $2
        ORDER BY hs.last_action_id ASC
        ",
    )
        .bind(match_uuid)
        .bind(user_id)
        .fetch_all(ex)
        .await?)
}

pub async fn get_hearthstone_snapshot<'a, T>(ex: T, snapshot_uuid: &Uuid) -> Result<HearthstoneGameSnapshot, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    let raw_metadata = sqlx::query!(
        "
        SELECT
            tm,
            game_entity_id,
            current_turn,
            step,
            current_player_id,
            last_action_id
        FROM squadov.hearthstone_snapshots
        WHERE snapshot_id = $1
        ",
        snapshot_uuid
    )
        .fetch_one(ex)
        .await?;

    let mut snapshot = HearthstoneGameSnapshot::new();
    snapshot.uuid = snapshot_uuid.clone();
    snapshot.tm = raw_metadata.tm;
    snapshot.game_entity_id = raw_metadata.game_entity_id;
    snapshot.aux_data = Some(HearthstoneGameSnapshotAuxData{
        current_turn: raw_metadata.current_turn,
        step: GameStep::try_from(raw_metadata.step)?,
        current_player_id: raw_metadata.current_player_id,
        last_action_index: raw_metadata.last_action_id as usize
    });

    let snapshot_players = sqlx::query!(
        "
        SELECT 
            player_name,
            player_id,
            entity_id
        FROM squadov.hearthstone_snapshots_player_map
        WHERE snapshot_id = $1
        ",
        snapshot_uuid
    )
        .fetch_all(ex)
        .await?;

    for sp in snapshot_players {
        snapshot.player_name_to_player_id.insert(sp.player_name, sp.player_id);
        snapshot.player_id_to_entity_id.insert(sp.player_id, sp.entity_id);
    }

    let snapshot_entities = sqlx::query!(
        "
        SELECT
            entity_id,
            tags,
            attributes
        FROM squadov.hearthstone_snapshots_entities
        WHERE snapshot_id = $1
        ",
        snapshot_uuid
    )
        .fetch_all(ex)
        .await?;

    for se in snapshot_entities {
        let entity = HearthstoneEntity{
            entity_id: se.entity_id,
            tags: serde_json::from_value(se.tags)?,
            attributes: serde_json::from_value(se.attributes)?
        };

        snapshot.entities.insert(se.entity_id, entity);
    }

    Ok(snapshot)
}

pub async fn get_hearthstone_game_packet<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<HearthstoneGamePacket, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    let raw_metadata = sqlx::query!(
        "
        SELECT
            hmm.game_type,
            hmm.format_type,
            hmm.scenario_id,
            hm.match_time,
            COALESCE(EXTRACT(EPOCH FROM (
                SELECT tm
                FROM squadov.hearthstone_snapshots
                WHERE match_uuid = $1
                ORDER BY last_action_id DESC
                LIMIT 1
            )) - EXTRACT(EPOCH FROM (
                SELECT tm
                FROM squadov.hearthstone_snapshots
                WHERE match_uuid = $1
                ORDER BY last_action_id ASC
                LIMIT 1
            )), 0) AS \"elapsed_seconds!\"
        FROM squadov.hearthstone_matches AS hm
        INNER JOIN squadov.hearthstone_match_view AS hmv
            ON hmv.match_uuid = hm.match_uuid
        INNER JOIN squadov.hearthstone_match_metadata AS hmm
            ON hmm.view_uuid = hmv.view_uuid
        WHERE hm.match_uuid = $1 AND hmv.user_id = $2
        ",
        match_uuid,
        user_id,
    )
        .fetch_one(ex)
        .await?;

    let metadata = HearthstoneMatchMetadata{
        game_type: GameType::try_from(raw_metadata.game_type)?,
        format_type: FormatType::try_from(raw_metadata.format_type)?,
        scenario_id: raw_metadata.scenario_id,
        match_time: raw_metadata.match_time,
        elapsed_seconds: raw_metadata.elapsed_seconds,
        deck: get_hearthstone_deck_for_match_user(ex, match_uuid, user_id).await?,
        players: get_hearthstone_players_for_match(ex, match_uuid, user_id).await?
    };
    
    let snapshot_ids = get_hearthstone_snapshots_for_match(ex, match_uuid, user_id).await?;
    let latest_snapshot = match snapshot_ids.last() {
        Some(x) => Some(get_hearthstone_snapshot(ex, x).await?),
        None => None
    };

    // Also retrieve the last snapshot of the game and just send it to the user "raw" and let them
    // parse it to retrieve some data about the final state of the game (e.g. this is what we're going
    // to use to display the final board state of the game).
    Ok(HearthstoneGamePacket{
        match_uuid: match_uuid.clone(),
        metadata: metadata,
        latest_snapshot: latest_snapshot
    })
}