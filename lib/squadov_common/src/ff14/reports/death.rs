use crate::{
    SquadOvError,
    combatlog::{
        CombatLogReportGenerator,
        RawStaticCombatLogReport,
    },
    ff14::{
        combatlog::Ff14CombatLogPacket,
        reports::Ff14ReportTypes,
    },
};
use avro_rs::{
    Writer,
    Codec,
    Schema,
};

pub struct Ff14DeathReportGenerator<'a> {
    writer: Option<Writer<'a, std::fs::File>>,
}

pub struct Ff14DeathReportEvent {

}

const DEATH_REPORT_SCHEMA_RAW: &'static str = r#"
    {
        "type": "record",
        "name": "ff14_death_event",
        "fields": [

        ]
    }
"#;

impl<'a> CombatLogReportGenerator for Ff14DeathReportGenerator<'a> {
    fn handle(&mut self, _data: &str) -> Result<(), SquadOvError> {
        Err(SquadOvError::BadRequest)
    }

    fn finalize(&mut self) -> Result<(), SquadOvError> {
        Ok(())
    }

    fn initialize_work_dir(&mut self, dir: &str) -> Result<(), SquadOvError> {
        lazy_static! {
            static ref SCHEMA: Schema = Schema::parse_str(DEATH_REPORT_SCHEMA_RAW).unwrap();
        }
        let file = tempfile::tempfile_in(dir)?;
        self.writer = Some(
            Writer::with_codec(&SCHEMA, file, Codec::Snappy)
        );
        Ok(())
    }

    fn get_reports(&mut self) -> Result<Vec<RawStaticCombatLogReport>, SquadOvError> {
        Ok(
            if let Some(w) = self.writer.take() {
                vec![RawStaticCombatLogReport{
                    key_name: String::from("deaths.avro"),
                    raw_file: tokio::fs::File::from_std(w.into_inner()?),
                    canonical_type: Ff14ReportTypes::Deaths as i32,
                }]
            } else {
                vec![]
            }
        )
    }
}

impl<'a> Ff14DeathReportGenerator<'a> {
    pub fn new() -> Self {
        Self{
            writer: None,
        }
    }
    
    pub fn handle_parsed(&mut self, data: &Ff14CombatLogPacket) -> Result<(), SquadOvError> {
        Ok(())
    }

}