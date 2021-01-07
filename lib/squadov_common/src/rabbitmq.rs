use async_trait::async_trait;
use crate::SquadOvError;
use serde::Deserialize;
use lapin::{
    BasicProperties,
    Connection,
    ConnectionProperties,
    Channel,
    options::{QueueDeclareOptions, BasicConsumeOptions, BasicPublishOptions, BasicAckOptions, BasicQosOptions},
    types::FieldTable,
};
use futures_util::stream::StreamExt;
use async_std::sync::{Arc, RwLock};
use std::collections::{HashMap, VecDeque};

pub const RABBITMQ_DEFAULT_PRIORITY: u8 = 0;
pub const RABBITMQ_HIGH_PRIORITY: u8 = 10;
const RABBITMQ_PREFETCH_COUNT: u16 = 8;

#[derive(Deserialize,Debug,Clone)]
pub struct RabbitMqConfig {
    pub amqp_url: String,
    pub valorant_queue: String
}

#[async_trait]
pub trait RabbitMqListener: Send + Sync {
    async fn handle(&self, data: &[u8]) -> Result<(), SquadOvError>;
}

pub struct RabbitMqConnectionBundle {
    config: RabbitMqConfig,
    channels: Vec<Channel>,
    listeners: Arc<RwLock<HashMap<String, Vec<Arc<dyn RabbitMqListener>>>>>,
}

pub struct RabbitMqPacket {
    queue: String,
    data: Vec<u8>,
    priority: u8,
}

pub struct RabbitMqInterface {
    pub config: RabbitMqConfig,
    publish_queue: Arc<RwLock<VecDeque<RabbitMqPacket>>>,
    consumer: Arc<RabbitMqConnectionBundle>,
}

impl RabbitMqConnectionBundle {
    fn num_channels(&self) -> usize {
        self.channels.len()
    }

    pub async fn connect(config: &RabbitMqConfig, num_channels: i32) -> Result<Self, SquadOvError> {
        let connection = Connection::connect(
            &config.amqp_url,
            ConnectionProperties::default().with_default_executor(4)
        ).await?;

        let mut channels: Vec<Channel> = Vec::new();
        for _i in 0..num_channels {
            let ch = connection.create_channel().await?;
            ch.queue_declare(
                &config.valorant_queue,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            ).await?;

            channels.push(ch);
        }

        Ok(Self {
            config: config.clone(),
            channels,
            listeners: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn publish(&self, msg: RabbitMqPacket, ch_idx: usize) -> Result<(), SquadOvError> {
        let ch = self.channels.get(ch_idx);
        if ch.is_none() {
            return Err(SquadOvError::BadRequest)
        }

        let ch = ch.unwrap();
        ch.basic_publish(
            "",
            &msg.queue,
            BasicPublishOptions::default(),
            msg.data,
            BasicProperties::default()
                .with_priority(msg.priority),
        ).await?.await?;

        Ok(())
    }

    pub async fn begin_consuming(&self) -> Result<(), SquadOvError> {
        // Each channel gets its own thread to start consuming from every channel.
        // I think we should probably only limit ourselves to having 1 consumer channel anyway.
        for ch in &self.channels {
            ch.basic_qos(RABBITMQ_PREFETCH_COUNT, BasicQosOptions::default()).await?;
            let mut consumer = ch.basic_consume(
                &self.config.valorant_queue,
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            ).await?;

            let queue = self.config.valorant_queue.clone();
            let listeners = self.listeners.clone();
            tokio::task::spawn(async move {
                while let Some(msg) = consumer.next().await {
                    if msg.is_err() {
                        log::warn!("Failed to consume from RabbitMQ: {:?}", msg.err().unwrap());
                        continue;
                    }

                    let (_, msg) = msg.unwrap();
                    let current_listeners = listeners.read().await.clone();
                    let topic_listeners = current_listeners.get(&queue);
                    if topic_listeners.is_some() {
                        let topic_listeners = topic_listeners.unwrap();
                        for l in topic_listeners {
                            match l.handle(&msg.data).await {
                                Ok(_) => (),
                                Err(err) => {
                                    log::warn!("Error in handling message: {:?}", err);
                                }
                            };
                        }    
                    }

                    // TODO: Maybe nack some stuff too if we failed to parse something?
                    // We need to be able to detect that situation somehow though.
                    match msg.acker.ack(BasicAckOptions::default()).await {
                        Ok(_) => (),
                        Err(err) => log::warn!("Failed to ack RabbitMQ message: {:?}", err)
                    };
                }
            });
        }

        Ok(())
    }
}

impl RabbitMqInterface {
    pub async fn new(config: &RabbitMqConfig) -> Result<Self, SquadOvError> {
        log::info!("Connecting to RabbitMQ...");
        let publisher = Arc::new(RabbitMqConnectionBundle::connect(&config, 4).await?);
        let publish_queue = Arc::new(RwLock::new(VecDeque::new()));

        {
            let publisher = publisher.clone();
            let publish_queue = publish_queue.clone();
            tokio::task::spawn(async move {
                let mut publish_idx: usize = 0;
                loop {
                    let next_msg = publish_queue.write().await.pop_front();
                    if next_msg.is_some() {
                        let next_msg = next_msg.unwrap();
                        match publisher.publish(next_msg, publish_idx).await {
                            Ok(_) => (),
                            Err(err) => log::warn!("Failed to publish RabbitMQ message: {:?}", err)
                        }
                        publish_idx = (publish_idx + 1) % publisher.num_channels();
                    }
                    async_std::task::sleep(std::time::Duration::from_millis(1)).await;
                }
            });
        }

        let consumer = Arc::new(RabbitMqConnectionBundle::connect(&config, 1).await?);
        consumer.begin_consuming().await?;

        Ok(Self {
            config: config.clone(),
            publish_queue,
            consumer,
        })
    }

    pub async fn add_listener(&self, queue: String, listener: Arc<dyn RabbitMqListener>) {
        let mut all_listeners = self.consumer.listeners.write().await;
        if !all_listeners.contains_key(&queue) {
            all_listeners.insert(queue.clone(), Vec::new());
        }

        let arr = all_listeners.get_mut(&queue).unwrap();
        arr.push(listener);
    }

    pub async fn publish(&self, queue: &str, data: Vec<u8>, priority: u8) {
        self.publish_queue.write().await.push_back(RabbitMqPacket{
            queue: String::from(queue),
            data,
            priority
        });
    }
}