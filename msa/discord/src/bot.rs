use squadov_common::{
    rabbitmq::RabbitMqInterface,
    discord::rabbitmq::DiscordTaskConsumer,
};
use crate::BotClient;
use std::sync::Arc;

impl BotClient {
    pub async fn start_external_workers(&self) {
        let itf = Arc::new(DiscordTaskConsumer::new(self.discord.cache_and_http.clone(), self.db.clone(), self.config.discord.clone()));
        for _i in 0..self.config.external_workers {
            RabbitMqInterface::add_listener(
                self.rabbitmq.clone(),
                self.config.rabbitmq.discord_queue.clone(),
                itf.clone(),
                self.config.rabbitmq.prefetch_count
            ).await.unwrap();
        }
    }
}