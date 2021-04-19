use async_trait::async_trait;
use crate::SquadOvError;
use serde::{Serialize, Deserialize};
use lapin::{
    BasicProperties,
    Connection,
    ConnectionProperties,
    Channel,
    options::{QueueDeclareOptions, BasicConsumeOptions, BasicPublishOptions, BasicAckOptions, BasicQosOptions},
    types::{FieldTable, AMQPValue, ShortString},
    Consumer,
};
use futures_util::stream::StreamExt;
use async_std::sync::{Arc, RwLock};
use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Utc, NaiveDateTime};
use rand::Rng;
use sqlx::PgPool;

pub const RABBITMQ_DEFAULT_PRIORITY: u8 = 0;
pub const RABBITMQ_HIGH_PRIORITY: u8 = 5;
const RABBITMQ_MAX_DELAY_MS: i64 = 3600000; // 1 hour
const SQUADOV_RETRY_COUNT_HEADER: &'static str = "x-squadov-retry-count";
const SQUADOV_MESSAGE_MAX_AGE_HEADER: &'static str = "x-squadov-max-age";
const DEFAULT_MAX_AGE_SECONDS: i64 = 3600; // 1 hour
const INFITE_MAX_AGE: i64 = -1;

#[derive(Deserialize,Debug,Clone)]
pub struct RabbitMqConfig {
    pub amqp_url: String,
    pub prefetch_count: u16,
    pub enable_rso: bool,
    pub rso_queue: String,
    pub enable_valorant: bool,
    pub valorant_queue: String,
    pub enable_lol: bool,
    pub lol_queue: String,
    pub enable_tft: bool,
    pub tft_queue: String,
    pub enable_vod: bool,
    pub vod_queue: String,
}

#[async_trait]
pub trait RabbitMqListener: Send + Sync {
    async fn handle(&self, data: &[u8]) -> Result<(), SquadOvError>;
}

pub struct RabbitMqConnectionBundle {
    channels: Vec<Channel>,
    listeners: Arc<RwLock<HashMap<String, Vec<Arc<dyn RabbitMqListener>>>>>,
    db: Arc<PgPool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RabbitMqPacket {
    queue: String,
    data: Vec<u8>,
    priority: u8,
    timestamp: DateTime<Utc>,
    retry_count: u32,
    pub base_delay_ms: Option<i64>,
    max_age_seconds: i64,
}

pub struct RabbitMqInterface {
    pub config: RabbitMqConfig,
    publish_queue: Arc<RwLock<VecDeque<RabbitMqPacket>>>,
    // Each queue gets its own vector of consumers so that we allocate 1 thread
    // per connection bundle and we allow having multiple threads all receiving
    // work from a single queue.
    consumers: RwLock<HashMap<String, Vec<Arc<RabbitMqConnectionBundle>>>>,
    db: Arc<PgPool>,
}

type RequeueCallbackFn = fn(&RabbitMqInterface, RabbitMqPacket);

impl RabbitMqConnectionBundle {
    fn num_channels(&self) -> usize {
        self.channels.len()
    }

    pub async fn connect(config: &RabbitMqConfig, db: Arc<PgPool>, num_channels: i32) -> Result<Self, SquadOvError> {
        let connection = Connection::connect(
            &config.amqp_url,
            ConnectionProperties::default()
        ).await?;

        let mut default_table = FieldTable::default();
        default_table.insert(ShortString::from("x-max-priority"), AMQPValue::LongUInt(10));

        let queue_opts = QueueDeclareOptions{
            passive: false,
            durable: true,
            exclusive: false,
            auto_delete: false,
            nowait: false,
        };

        let mut channels: Vec<Channel> = Vec::new();
        for _i in 0..num_channels {
            let ch = connection.create_channel().await?;

            ch.queue_declare(
                &config.rso_queue,
                queue_opts.clone(),
                default_table.clone(),
            ).await?;

            ch.queue_declare(
                &config.valorant_queue,
                queue_opts.clone(),
                default_table.clone(),
            ).await?;

            ch.queue_declare(
                &config.lol_queue,
                queue_opts.clone(),
                default_table.clone(),
            ).await?;

            ch.queue_declare(
                &config.tft_queue,
                queue_opts.clone(),
                default_table.clone(),
            ).await?;

            ch.queue_declare(
                &config.vod_queue,
                queue_opts.clone(),
                default_table.clone(),
            ).await?;

            channels.push(ch);
        }

        Ok(Self {
            channels,
            db,
            listeners: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn publish(&self, msg: RabbitMqPacket, ch_idx: usize) -> Result<(), SquadOvError> {
        let ch = self.channels.get(ch_idx);
        if ch.is_none() {
            return Err(SquadOvError::BadRequest)
        }

        let mut headers = FieldTable::default();
        headers.insert(ShortString::from(SQUADOV_RETRY_COUNT_HEADER), AMQPValue::LongUInt(msg.retry_count));
        headers.insert(ShortString::from(SQUADOV_MESSAGE_MAX_AGE_HEADER), AMQPValue::LongLongInt(msg.max_age_seconds));

        let ch = ch.unwrap();
        if msg.base_delay_ms.is_none() {
            ch.basic_publish(
                "",
                &msg.queue,
                BasicPublishOptions::default(),
                msg.data,
                BasicProperties::default()
                    .with_priority(msg.priority)
                    .with_timestamp(msg.timestamp.timestamp() as u64)
                    .with_headers(headers),
            ).await?.await?;
        } else {
            let total_delay_ms = {
                let mut rng = rand::thread_rng();
                std::cmp::min(
                    2i64.pow(msg.retry_count) +  rng.gen_range(0..1000) + msg.base_delay_ms.unwrap(),
                    RABBITMQ_MAX_DELAY_MS)
            };
            log::info!("Delaying RabbitMQ message for {}ms.", total_delay_ms);
            self.add_delayed_rabbitmq_message(msg, total_delay_ms).await?;
        }

        Ok(())
    }

    async fn add_delayed_rabbitmq_message(&self, msg: RabbitMqPacket, total_delay_ms: i64) -> Result<(), SquadOvError> {
        let execute_time = Utc::now() + chrono::Duration::milliseconds(total_delay_ms);
        sqlx::query!(
            "
            INSERT INTO squadov.deferred_rabbitmq_messages (
                execute_time,
                message
            )
            VALUES (
                $1,
                $2
            )
            ",
            execute_time,
            serde_json::to_vec(&msg)?,
        )
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    fn start_consumer(&self, queue: &str, mut consumer: Consumer, itf: Arc<RabbitMqInterface>, requeue_callback: RequeueCallbackFn) {
        let queue = String::from(queue);
        let listeners = self.listeners.clone();
        tokio::task::spawn(async move {
            while let Some(msg) = consumer.next().await {
                if msg.is_err() {
                    log::warn!("Failed to consume from RabbitMQ: {:?}", msg.err().unwrap());
                    continue;
                }

                let msg = msg.unwrap();

                // Check the application defined max age. If we're past the max age of this particular message then
                // we'd want to discard the message.
                let current_timestamp = Utc::now().timestamp() as u64;
                let og_timestamp = msg.properties.timestamp().unwrap_or(current_timestamp);
                let max_age_seconds = match msg.properties.headers() {
                    Some(h) => h.inner().get(&ShortString::from(SQUADOV_MESSAGE_MAX_AGE_HEADER)),
                    None => None,
                }.map(|x| {
                    match x {
                        AMQPValue::LongLongInt(y) => *y,
                        _ => {
                            log::warn!("Max age header in an unexpected format: {:?}", x);
                            DEFAULT_MAX_AGE_SECONDS
                        },
                    }
                }).unwrap_or(DEFAULT_MAX_AGE_SECONDS);

                let expired = (current_timestamp - og_timestamp) > (max_age_seconds as u64) && max_age_seconds != INFITE_MAX_AGE;
                let mut requeue_ms: Option<i64> = None;

                if !expired {
                    let current_listeners = listeners.read().await.clone();
                    let topic_listeners = current_listeners.get(&queue);
                    if topic_listeners.is_some() {
                        let topic_listeners = topic_listeners.unwrap();
                        for l in topic_listeners {
                            match l.handle(&msg.data).await {
                                Ok(_) => (),
                                Err(err) => {
                                    log::warn!("Failure in processing RabbitMQ message: {:?}", err);
                                    match err {
                                        SquadOvError::Defer(ms) => { requeue_ms = Some(ms); },
                                        SquadOvError::RateLimit => { requeue_ms = Some(100); },
                                        _ => {},
                                    }
                                },
                            };
                        }    
                    }
                } else {
                    log::warn!("Ignoring message because it expired: {:?}", &msg);
                }

                match msg.acker.ack(BasicAckOptions::default()).await {
                    Ok(_) => (),
                    Err(err) => log::warn!("Failed to ack RabbitMQ message: {:?}", err)
                };

                if requeue_ms.is_some() {
                    let retry_count = match msg.properties.headers() {
                        Some(h) => h.inner().get(&ShortString::from(SQUADOV_RETRY_COUNT_HEADER)),
                        None => None,
                    }.map(|x| {
                        match x {
                            AMQPValue::LongUInt(y) => *y,
                            _ => {
                                log::warn!("Retry header in an unexpected format: {:?}", x);
                                0
                            },
                        }
                    }).unwrap_or(0);

                    requeue_callback(&*itf, RabbitMqPacket{
                        queue: queue.clone(),
                        data: msg.data.clone(),
                        priority: msg.properties.priority().unwrap_or(RABBITMQ_DEFAULT_PRIORITY),
                        timestamp: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(og_timestamp as i64, 0), Utc),
                        retry_count: retry_count + 1,
                        base_delay_ms: requeue_ms,
                        max_age_seconds,
                    });
                }
            }
        });
    }

    pub async fn begin_consuming(&self, itf: Arc<RabbitMqInterface>, queue: &str, requeue_callback: RequeueCallbackFn, prefetch_count: u16) -> Result<(), SquadOvError> {
        // Each channel gets its own thread to start consuming from every channel.
        // I think we should probably only limit ourselves to having 1 consumer channel anyway.
        for ch in &self.channels {
            ch.basic_qos(prefetch_count, BasicQosOptions::default()).await?;

            let consumer = ch.basic_consume(
                queue,
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),

            ).await?;
            self.start_consumer(queue, consumer, itf.clone(), requeue_callback);
        }

        Ok(())
    }
}

impl RabbitMqInterface {
    pub async fn new(config: &RabbitMqConfig, db: Arc<PgPool>, enabled: bool) -> Result<Arc<Self>, SquadOvError> {
        log::info!("Connecting to RabbitMQ...");
        let publisher = Arc::new(RabbitMqConnectionBundle::connect(&config, db.clone(), if enabled { 4 } else { 0 }).await?);
        let publish_queue = Arc::new(RwLock::new(VecDeque::new()));

        if enabled {
            log::info!("\tStart Publishing (RabbitMQ)...");
            {
                let publisher = publisher.clone();
                let publish_queue = publish_queue.clone();
                tokio::task::spawn(async move {
                    let mut publish_idx: usize = 0;
                    loop {
                        let mut queue_clone = {
                            let mut queue_lock = publish_queue.write().await;
                            let ret = queue_lock.clone();
                            queue_lock.clear();
                            ret
                        };

                        if !queue_clone.is_empty() {
                            log::info!("RabbitMQ Publishing {} messages", queue_clone.len());
                        }

                        while !queue_clone.is_empty() {
                            let next_msg = queue_clone.pop_front();
                            if next_msg.is_some() {
                                let next_msg = next_msg.unwrap();
                                match publisher.publish(next_msg, publish_idx).await {
                                    Ok(_) => (),
                                    Err(err) => log::warn!("Failed to publish RabbitMQ message: {:?}", err)
                                }
                                publish_idx = (publish_idx + 1) % publisher.num_channels();
                            }
                        }
                        async_std::task::sleep(std::time::Duration::from_millis(1)).await;
                    }
                });
            }
        }

        let itf = Arc::new(Self {
            config: config.clone(),
            publish_queue,
            db,
            consumers: RwLock::new(HashMap::new()),
        });
        log::info!("RabbitMQ Successfully Connected");
        Ok(itf)
    }

    pub fn publish_direct(&self, packet: RabbitMqPacket) {
        let queue = self.publish_queue.clone();
        tokio::task::spawn(async move {
            queue.write().await.push_back(packet);
        });
    }

    pub async fn add_listener(itf: Arc<RabbitMqInterface>, queue: String, listener: Arc<dyn RabbitMqListener>, prefetch_count: u16) -> Result<(), SquadOvError> {
        let mut consumers = itf.consumers.write().await;
        if !consumers.contains_key(&queue) {
            consumers.insert(queue.clone(), Vec::new());
        }

        let consumer_array = consumers.get_mut(&queue).unwrap();
        log::info!("Start Consuming (RabbitMQ) on Queue {} [{}]...", &queue, consumer_array.len());

        let consumer = Arc::new(RabbitMqConnectionBundle::connect(&itf.config, itf.db.clone(), 1).await?);
        
        {
            let mut all_listeners = consumer.listeners.write().await;
            if !all_listeners.contains_key(&queue) {
                all_listeners.insert(queue.clone(), Vec::new());
            }

            let arr = all_listeners.get_mut(&queue).unwrap();
            arr.push(listener);
        }

        consumer_array.push(consumer.clone());
        consumer.begin_consuming(itf.clone(), &queue, RabbitMqInterface::publish_direct, prefetch_count).await?;
        log::info!("\t...Success.");
        Ok(())
    }

    pub async fn publish(&self, queue: &str, data: Vec<u8>, priority: u8, max_age_seconds: i64) {
        self.publish_queue.write().await.push_back(RabbitMqPacket{
            queue: String::from(queue),
            data,
            priority,
            timestamp: Utc::now(),
            retry_count: 0,
            base_delay_ms: None,
            max_age_seconds,
        });
    }
}