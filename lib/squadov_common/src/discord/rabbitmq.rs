use serde::{Serialize, Deserialize};
use crate::{
    SquadOvError,
    rabbitmq::{
        RabbitMqInterface,
        RabbitMqListener,
        RABBITMQ_DEFAULT_PRIORITY,
        RabbitMqConfig,
    },
    discord::{
        bot::DiscordBotConfig,
        db,
    },
    subscriptions::{
        self,
        SquadOvSubTiers,
    },
};
use std::sync::Arc;
use async_trait::async_trait;
use sqlx::PgPool;
use serenity::{
    CacheAndHttp,
    http::CacheHttp,
};

const DISCORD_MAX_AGE_SECONDS: i64 = 172800; // 2 day

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DiscordTask {
    SyncUser{
        user_id: i64,
    }
}

pub struct DiscordTaskProducer {
    rmq: Arc<RabbitMqInterface>,
    config: RabbitMqConfig,
}

impl DiscordTaskProducer {
    pub fn new(rmq: Arc<RabbitMqInterface>, config: RabbitMqConfig) -> Self {
        Self {
            rmq,
            config,
        }
    }

    pub async fn request_sync_user(&self, user_id: i64) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.config.discord_queue, serde_json::to_vec(&DiscordTask::SyncUser{
            user_id,
        })?, RABBITMQ_DEFAULT_PRIORITY, DISCORD_MAX_AGE_SECONDS).await;
        Ok(())
    }
}

pub struct DiscordTaskConsumer {
    http: Arc<CacheAndHttp>,
    db: Arc<PgPool>,
    config: DiscordBotConfig,
}

#[async_trait]
impl RabbitMqListener for DiscordTaskConsumer {
    async fn handle(&self, data: &[u8], _queue: &str, _priority: u8) -> Result<(), SquadOvError> {
        log::info!("Handle Discord Task: {}", std::str::from_utf8(data).unwrap_or("failure"));
        let task: DiscordTask = serde_json::from_slice(data)?;
        match task {
            DiscordTask::SyncUser{user_id} => self.sync_user(user_id).await?,
        };
        Ok(())
    }
}

impl DiscordTaskConsumer {
    pub fn new(http: Arc<CacheAndHttp>, db: Arc<PgPool>, config: DiscordBotConfig) -> DiscordTaskConsumer {
        DiscordTaskConsumer{
            http,
            db,
            config,
        }
    }

    async fn sync_user(&self, user_id: i64) -> Result<(), SquadOvError> {
        // Check what role the user has based on their subscription tier.
        let sub_tier = subscriptions::get_user_sub_tier(&*self.db, user_id).await?;
        let discord_accounts: Vec<_> = db::find_discord_accounts_for_user(&*self.db, user_id).await?;
        for acc in discord_accounts {
            let discord_user_id = acc.id.parse::<u64>()?;
            match sub_tier {
                SquadOvSubTiers::Basic => {
                    self.http.http().remove_member_role(self.config.server_id, discord_user_id, self.config.roles.silver, None).await?;
                    self.http.http().remove_member_role(self.config.server_id, discord_user_id, self.config.roles.gold, None).await?;
                    self.http.http().remove_member_role(self.config.server_id, discord_user_id, self.config.roles.diamond, None).await?;
                },
                SquadOvSubTiers::Silver => {
                    self.http.http().add_member_role(self.config.server_id, discord_user_id, self.config.roles.silver, None).await?;
                    self.http.http().remove_member_role(self.config.server_id, discord_user_id, self.config.roles.gold, None).await?;
                    self.http.http().remove_member_role(self.config.server_id, discord_user_id, self.config.roles.diamond, None).await?;
                },
                SquadOvSubTiers::Gold => {
                    self.http.http().remove_member_role(self.config.server_id, discord_user_id, self.config.roles.silver, None).await?;
                    self.http.http().add_member_role(self.config.server_id, discord_user_id, self.config.roles.gold, None).await?;
                    self.http.http().remove_member_role(self.config.server_id, discord_user_id, self.config.roles.diamond, None).await?;
                },
                SquadOvSubTiers::Diamond => {
                    self.http.http().remove_member_role(self.config.server_id, discord_user_id, self.config.roles.silver, None).await?;
                    self.http.http().remove_member_role(self.config.server_id, discord_user_id, self.config.roles.gold, None).await?;
                    self.http.http().add_member_role(self.config.server_id, discord_user_id, self.config.roles.diamond, None).await?;
                },
            }

            // Check whether or not the user is early access based on their feature flag.
            let early_access = sqlx::query!(
                "
                SELECT early_access
                FROM squadov.user_feature_flags
                WHERE user_id = $1
                ",
                user_id,
            )
                .fetch_one(&*self.db)
                .await?
                .early_access;

            if early_access {
                self.http.http().add_member_role(self.config.server_id, discord_user_id, self.config.roles.early_access, None).await?;
            } else {
                self.http.http().remove_member_role(self.config.server_id, discord_user_id, self.config.roles.early_access, None).await?;
            }
        }

        Ok(())
    }    
}