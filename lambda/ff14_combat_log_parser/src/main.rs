use lambda_runtime::{handler_fn, Context};
use serde::Deserialize;
use serde_json::{Value};
use std::{
    io::Read,
};
use squadov_common::{
    SquadOvError,
    ff14::combatlog::{
        self,
        Ff14CombatLogPacket,
        Ff14PacketData,
    },
};

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

struct SharedClient {
}

impl SharedClient {
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

        // Stream the raw and parsed data into AWS Firehose to dump that data out into S3.
        let (raw_logs, parsed_logs) = Self::split_raw_parsed(parsed_logs);
        

        // Generate reports. Note that this is a flag sent by the client if this is the last batch of combat
        // log lines for the current partition key. We don't generate the reports here but we stick a message
        // on a queue and let someone else take care of it. Note that we need to delay this a bit since it takes
        // time for Firehose to flush out the data to S3.
        if generate_reports {

        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), SquadOvError> {
    std::env::set_var("RUST_LOG", "info,ff14_combat_log_parser=info");
    env_logger::init();

    // Pull environment variables.
    log::info!("Creating Shared Client...");
    let shared = SharedClient{
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