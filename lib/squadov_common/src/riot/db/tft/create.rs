use crate::{
    SquadOvError,
    matches,
    riot::games::{
        TftMatchDto,
        TftParticipantDto,
        TftTraitDto,
        TftUnitDto,
    },
};
use sqlx::{Transaction, Postgres};
use uuid::Uuid;

async fn link_match_uuid_to_tft_match(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, platform: &str, region: &str, game_id: i64) -> Result<(), SquadOvError> {
    sqlx::query!(
        "
        INSERT INTO squadov.tft_matches (
            match_uuid,
            platform,
            region,
            match_id
        )
        VALUES (
            $1,
            $2,
            $3,
            $4
        )
        ",
        match_uuid,
        platform,
        region,
        game_id
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn create_or_get_match_uuid_for_tft_match(ex: &mut Transaction<'_, Postgres>, platform: &str, region: &str, game_id: i64) -> Result<Uuid, SquadOvError> {
    Ok(match super::get_tft_match_uuid_if_exists(&mut *ex, platform, game_id).await? {
        Some(x) => x,
        None => {
            let match_uuid = matches::create_new_match(&mut *ex).await?;
            link_match_uuid_to_tft_match(&mut *ex, &match_uuid, platform, region, game_id).await?;
            match_uuid
        }
    })
}

struct WrappedTftTraitDto<'a> {
    puuid: String,
    base: &'a TftTraitDto
}

struct WrappedTftUnitDto<'a> {
    puuid: String,
    base: &'a TftUnitDto
}

async fn store_tft_match_participant_traits<'a>(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, traits: &[WrappedTftTraitDto<'a>]) -> Result<(), SquadOvError> {
    if traits.is_empty() {
        return Ok(());
    }

    let mut sql = vec![
        "
            INSERT INTO squadov.tft_match_participant_traits (
                match_uuid,
                puuid,
                name,
                num_units,
                style,
                tier_current,
                tier_total
            )
            VALUES
        ".to_string()
    ];

    for t in traits {
        sql.push(
            format!("
                (
                    '{match_uuid}',
                    '{puuid}',
                    '{name}',
                    {num_units},
                    {style},
                    {tier_current},
                    {tier_total}
                )
                ",
                match_uuid=match_uuid,
                puuid=&t.puuid,
                name=&t.base.name,
                num_units=t.base.num_units,
                style=t.base.style,
                tier_current=t.base.tier_current,
                tier_total=t.base.tier_total,
            )
        );
        sql.push(",".to_string());
    }

    sql.truncate(sql.len() - 1);
    sqlx::query(&sql.join("")).execute(&mut *ex).await?;
    Ok(())
}

async fn store_tft_match_participant_units<'a>(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, units: &[WrappedTftUnitDto<'a>]) -> Result<(), SquadOvError> {
    if units.is_empty() {
        return Ok(());
    }

    let mut sql = vec![
        "
            INSERT INTO squadov.tft_match_participant_units (
                match_uuid,
                puuid,
                character_id,
                chosen,
                name,
                rarity,
                tier,
                items
            )
            VALUES
        ".to_string()
    ];

    for u in units {
        sql.push(
            format!("
                (
                    '{match_uuid}',
                    '{puuid}',
                    {character_id},
                    {chosen},
                    '{name}',
                    {rarity},
                    {tier},
                    {items}
                )
                ",
                match_uuid=match_uuid,
                puuid=&u.puuid,
                character_id=crate::sql_format_option_string(&u.base.character_id),
                chosen=crate::sql_format_option_string(&u.base.chosen),
                name=u.base.name,
                rarity=u.base.rarity,
                tier=u.base.tier,
                items=crate::sql_format_integer_array(&u.base.items),
            )
        );
        sql.push(",".to_string());
    }

    sql.truncate(sql.len() - 1);
    sqlx::query(&sql.join("")).execute(&mut *ex).await?;
    Ok(())
}

pub async fn store_tft_match_participants(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, participants: &[TftParticipantDto]) -> Result<(), SquadOvError> {
    if participants.is_empty() {
        return Ok(());
    }

    // We need to do this to do an effective upsert across multiple tables since we need to
    // gracefully handle the case where we pull in TFT match info multiple times. This can happen
    // because we'll grab match history info as people start to die in TFT and leave the game.
    // This information isn't 100% accurate until the game is 100% completed. So as players die
    // and trigger requests to update match history details, we'll need to keep refreshing our
    // store of participant info as they keep coming in.
    sqlx::query!("DELETE FROM squadov.tft_match_participants").execute(&mut *ex).await?;

    let mut sql = vec![
        "
            INSERT INTO squadov.tft_match_participants (
                match_uuid,
                puuid,
                gold_left,
                last_round,
                level,
                placement,
                players_eliminated,
                time_eliminated,
                total_damage_to_players,
                companion_content_id,
                companion_skin_id,
                companion_species
            )
            VALUES
        ".to_string()
    ];

    let mut traits: Vec<WrappedTftTraitDto> = Vec::new();
    let mut units: Vec<WrappedTftUnitDto> = Vec::new();

    for p in participants {
        sql.push(
            format!("
                (
                    '{match_uuid}',
                    '{puuid}',
                    {gold_left},
                    {last_round},
                    {level},
                    {placement},
                    {players_eliminated},
                    {time_eliminated},
                    {total_damage_to_players},
                    '{companion_content_id}',
                    '{companion_skin_id}',
                    '{companion_species}'
                )
                ",
                match_uuid=match_uuid,
                puuid=&p.puuid,
                gold_left=p.gold_left,
                last_round=p.last_round,
                level=p.level,
                placement=p.placement,
                players_eliminated=p.players_eliminated,
                time_eliminated=p.time_eliminated,
                total_damage_to_players=p.total_damage_to_players,
                companion_content_id=&p.companion.content_id,
                companion_skin_id=&p.companion.skin_id,
                companion_species=&p.companion.species,
            )
        );
        sql.push(",".to_string());

        traits.extend(p.traits.iter().map(|x| {
            WrappedTftTraitDto{
                puuid: p.puuid.clone(),
                base: x
            }
        }));

        units.extend(p.units.iter().map(|x| {
            WrappedTftUnitDto {
                puuid: p.puuid.clone(),
                base: x
            }
        }));
    }

    sql.truncate(sql.len() - 1);
    sqlx::query(&sql.join("")).execute(&mut *ex).await?;

    store_tft_match_participant_traits(&mut *ex, match_uuid, &traits).await?;
    store_tft_match_participant_units(&mut *ex, match_uuid, &units).await?;
    Ok(())
}

pub async fn store_tft_match_info(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, tft_match: &TftMatchDto) -> Result<(), SquadOvError> {
    sqlx::query!(
        "
        INSERT INTO squadov.tft_match_info (
            match_uuid,
            game_datetime,
            game_length,
            game_variation,
            game_version,
            queue_id,
            tft_set_number
        )
        VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7
        )
        ",
        match_uuid,
        &tft_match.info.game_datetime.ok_or(SquadOvError::BadRequest)?,
        tft_match.info.game_length,
        tft_match.info.game_variation,
        &tft_match.info.game_version,
        tft_match.info.queue_id,
        tft_match.info.tft_set_number,
    )
        .execute(&mut *ex)
        .await?;
    store_tft_match_participants(&mut *ex, match_uuid, &tft_match.info.participants).await?;
    Ok(())
}