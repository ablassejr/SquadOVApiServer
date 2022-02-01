mod death;

use crate::{
    SquadOvError,
    combatlog::{
        CombatLogReportGenerator,
        RawStaticCombatLogReport,
    },
    ff14::combatlog::Ff14CombatLogPacket,
};
use uuid::Uuid;
use num_enum::TryFromPrimitive;

pub struct Ff14ReportsGenerator<'a> {
    view_id: Uuid,
    death_report: Option<death::Ff14DeathReportGenerator<'a>>,
    work_dir: Option<String>,
}

#[derive(Copy, Clone, Debug, TryFromPrimitive, PartialEq)]
#[repr(i32)]
pub enum Ff14ReportTypes {
    Deaths,
}

impl<'a> CombatLogReportGenerator for Ff14ReportsGenerator<'a> {
    fn handle(&mut self, data: &str) -> Result<(), SquadOvError> {
        let data: Ff14CombatLogPacket = serde_json::from_str(data)?;
        self.handle_parsed(&data)
    }

    fn finalize(&mut self) -> Result<(), SquadOvError> {
        if let Some(dr) = self.death_report.as_mut() {
            dr.finalize()?;
        }
        Ok(())
    }

    fn initialize_work_dir(&mut self, dir: &str) -> Result<(), SquadOvError> {
        self.work_dir = Some(String::from(dir));

        // Create game-wide reports here. Per player reports will be created when those players are introduced.
        {
            let mut dr = death::Ff14DeathReportGenerator::new();
            dr.initialize_work_dir(dir)?;
            self.death_report = Some(dr);
        }

        Ok(())
    }

    fn get_reports(&mut self) -> Result<Vec<RawStaticCombatLogReport>, SquadOvError> {
        let mut ret: Vec<RawStaticCombatLogReport> = vec![];

        if let Some(dr) = self.death_report.as_mut() {
            ret.extend(dr.get_reports()?);
        }

        Ok(ret)
    }
}

impl<'a> Ff14ReportsGenerator<'a> {
    pub fn new(view_id: &str) -> Result<Self, SquadOvError> {
        Ok(Self{
            view_id: Uuid::parse_str(view_id)?,
            death_report: None,
            work_dir: None,
        })
    }

    fn handle_parsed(&mut self, data: &Ff14CombatLogPacket) -> Result<(), SquadOvError> {
        if let Some(dr) = self.death_report.as_mut() {
            dr.handle_parsed(data)?;
        }

        Ok(())
    }
}