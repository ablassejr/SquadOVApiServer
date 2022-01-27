#[macro_use]
extern crate byte_unit;

use lambda_runtime::{handler_fn, Context};
use serde::Deserialize;
use serde_json::{Value};
use std::{
    io::Read,
    sync::Arc,
    str::FromStr,
};
use squadov_common::{
    SquadOvError,
    ff14::combatlog::{
        self,
        Ff14CombatLogPacket,
        Ff14PacketData,
    },
    rabbitmq::{
        RabbitMqConfig,
        RabbitMqInterface,
        RABBITMQ_DEFAULT_PRIORITY,
    },
    combatlog::CombatLogTasks,
};
use chrono::Utc;
use rusoto_core::Region;
use rusoto_firehose::{
    KinesisFirehose,
    KinesisFirehoseClient,
    PutRecordBatchInput,
    Record as KRecord,
};
use bytes::Bytes;

#[derive(Deserialize)]
struct Payload {
    #[serde(rename="Records")]
    records: Vec<Record>,
}

#[derive(Deserialize)]
struct Record {
    kinesis: KinesisData,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
struct KinesisData {
    partition_key: String,
    data: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
struct CombatLogData {
    logs: Vec<String>,
    #[serde(default)]
    generate_reports: bool,
}

const COMBATLOG_MAX_AGE_SECONDS: i64 = 86400;

struct SharedClient {
    firehose: Arc<KinesisFirehoseClient>,
    delivery_stream: String,
    rmq: Arc<RabbitMqInterface>,
    combatlog_report_queue: String,
}

impl SharedClient {
    async fn upload_to_firehose(&self, data: Vec<Ff14CombatLogPacket>) -> Result<(), SquadOvError> {
        // PutRecordBatch has a couple of limits:
        //  1) 4MB for the entire request.
        //  2) 500 Records
        //  3) Each record can be up to 1,000KB.
        // We should be clear of all these limits but just to be safe we do checks and split if necessary.
        let mut batch: Vec<KRecord> = vec![];
        
        let mut total_batch_size_bytes: usize = 0;
        for d in data {
            let byte_data = {
                let raw_string = match serde_json::to_string(&d) {
                    Ok(x) => x,
                    Err(err) => {
                        log::warn!("Failed to serialize packet to JSON: {:?}", err);
                        continue;
                    }
                } + "\n";

                Bytes::copy_from_slice(raw_string.as_bytes())
            };

            if byte_data.len() as u128 > n_kb_bytes!(1000) {
                log::warn!("Packet too large: {:?}", &d);
                continue;
            }
           
            if (total_batch_size_bytes + byte_data.len()) as u128 > n_mb_bytes!(4) || batch.len() == 500usize {
                self.firehose.put_record_batch(PutRecordBatchInput{
                    delivery_stream_name: self.delivery_stream.clone(),
                    records: batch.drain(0..).collect(),
                }).await?;
            } else {
                total_batch_size_bytes += byte_data.len();
                batch.push(KRecord{
                    data: byte_data,
                });
            }
        }

        if !batch.is_empty() {
            self.firehose.put_record_batch(PutRecordBatchInput{
                delivery_stream_name: self.delivery_stream.clone(),
                records: batch.drain(0..).collect(),
            }).await?;
        }
        Ok(())
    }

    fn parse_logs(partition_key: &str, decoded: CombatLogData) -> Vec<(String, Ff14CombatLogPacket)> {
        decoded.logs.into_iter()
            .map(|x| {
                let result = std::panic::catch_unwind(|| {
                    combatlog::parse_ff14_combat_log_line(partition_key.to_string(), x.clone())
                });

                // Note that result is of type Result<(String, Result<Ff14CombatLogPacket, SquadOverror>), Err>
                // We want to boil this down to just the inner type.
                match result {
                    Ok(y) => y,
                    Err(e) => (
                        x,
                        Err(
                            SquadOvError::InternalError(
                                format!("FF14 Parse Panic: {:?}", e)
                            ),
                        )
                    ),
                }
            })
            .filter(|x| {
                if let Err(err) = &x.1 {
                    log::warn!("Failed to parse FF14 Combat Log Line: {:?} - {}", err, &x.0);
                }
                x.1.is_ok()
            })
            .map(|x| {
                (x.0, x.1.unwrap())
            })
            .collect()
    }

    fn split_raw_parsed(data: Vec<(String, Ff14CombatLogPacket)>) -> (Vec<Ff14CombatLogPacket>, Vec<Ff14CombatLogPacket>) {
        let raw_logs = data.iter().map(|x| {
            Ff14CombatLogPacket{
                data: Ff14PacketData::Raw(x.0.clone()),
                ..x.1.clone()
            }
        }).collect::<Vec<Ff14CombatLogPacket>>();

        let parsed_logs = data.into_iter().map(|x| { x.1 }).collect::<Vec<Ff14CombatLogPacket>>();
        (raw_logs, parsed_logs)
    }

    async fn handle_kinesis_data(&self, data: KinesisData) -> Result<(), SquadOvError> {
        // Ensure that the partition key is for ff14.
        // Note that our partition keys will be of the form GAME_MATCHUUID.
        if !data.partition_key.starts_with("ff14_") {
            log::warn!("...Invalid Game Partition Key: {}", &data.partition_key);
            return Err(SquadOvError::BadRequest);
        }

        // The inner data is base64 encoded - note that we're expecting a JSON structure of FF14 combat logs.
        // The data that we get is BASE64(GZIP(JSON)) so we need to reverse those operations to
        // properly decode the packet.
        let decoded = serde_json::from_slice::<CombatLogData>(&{
            let mut uncompressed_data: Vec<u8> = Vec::new();
            {
                let raw_data = base64::decode(&data.data)?;
                let mut decoder = flate2::read::GzDecoder::new(&*raw_data);
                decoder.read_to_end(&mut uncompressed_data)?;
            }
            uncompressed_data
        })?;
        let generate_reports = decoded.generate_reports;

        // We do a best effort parsing of all the combat log lines. If any one line fails to parse,
        // that doesn't prevent the entire batch from being parsed. We ignore that line and move on.
        let parsed_logs = Self::parse_logs(&data.partition_key, decoded);

        let (raw_logs, mut parsed_logs) = Self::split_raw_parsed(parsed_logs);
        
        if generate_reports {
            // Write a "Flush" message to Firehose. This way we can wait for this End message to appear in S3 before trying to
            // actually generate the report. Since this message is written AFTER the parsed logs, there's a reasonable(?) guarantee
            // that once the End message gets flushed to disk, all the other messages have been flushed as well.
            parsed_logs.push(Ff14CombatLogPacket{
                partition_id: data.partition_key.clone(),
                time: Utc::now(),
                data: Ff14PacketData::Flush,
            });
        }

        // Stream the raw and parsed data into AWS Firehose to dump that data out into S3.
        self.upload_to_firehose(raw_logs).await?;
        self.upload_to_firehose(parsed_logs).await?;

        // Generate reports. Note that this is a flag sent by the client if this is the last batch of combat
        // log lines for the current partition key. We don't generate the reports here but we stick a message
        // on a queue and let someone else take care of it. We don't delay here but rather let the handler put
        // it onto a delay queue if it didn't detect the Flush message. This way in the best case scenario we
        // are still generating the reports immediately.
        if generate_reports {
            self.rmq.publish(
                &self.combatlog_report_queue,
                serde_json::to_vec(&CombatLogTasks::Ff14Reports(data.partition_key.clone()))?,
                RABBITMQ_DEFAULT_PRIORITY,
                COMBATLOG_MAX_AGE_SECONDS,
            ).await;
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), SquadOvError> {
    std::env::set_var("RUST_LOG", "info,ff14_combat_log_parser=info");
    env_logger::init();

    // Pull environment variables.
    let aws_region = std::env::var("SQUADOV_AWS_REGION").unwrap();
    let rabbitmq_url = std::env::var("SQUADOV_RABBITMQ_URL").unwrap();
    let delivery_stream = std::env::var("SQUADOV_FIREHOSE_DELIVERY_STREAM").unwrap();
    let combatlog_report_queue = std::env::var("SQUADOV_COMBATLOG_REPORT_QUEUE").unwrap();
    log::info!("AWS Region: {}", &aws_region);
    log::info!("AWS Firehose Delivery Stream: {}", &delivery_stream);
    log::info!("Report Queue: {}", &combatlog_report_queue);

    log::info!("Creating Shared Client...");
    let shared = SharedClient{
        firehose: Arc::new(KinesisFirehoseClient::new(
            Region::from_str(&aws_region)?
        )),
        delivery_stream,
        rmq: RabbitMqInterface::new(
            &RabbitMqConfig::default()
                .set_url(&rabbitmq_url)
                .add_queue(&combatlog_report_queue),
            None,
            true,
        ).await?,
        combatlog_report_queue,
    };

    let shared_ref = &shared;
    let closure = move |event: Value, _ctx: Context| async move {
        log::info!("Handling Kinesis Record: {:?}", event);
        
        let payload = serde_json::from_value::<Payload>(event)?;
        for record in payload.records {
            shared_ref.handle_kinesis_data(record.kinesis).await?;
        }

        Ok::<(), SquadOvError>(())
    };

    log::info!("Starting Runtime...");
    lambda_runtime::run(handler_fn(closure)).await?;
    Ok(())
}