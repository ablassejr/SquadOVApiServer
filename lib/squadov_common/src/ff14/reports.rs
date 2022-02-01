mod death;

use crate::{
    SquadOvError,
    combatlog::{
        CombatLogReportHandler,
        CombatLogReportIO,
        RawStaticCombatLogReport,
    },
    ff14::combatlog::Ff14CombatLogPacket,
};
use uuid::Uuid;
use num_enum::TryFromPrimitive;

#[derive(Default)]
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

impl<'a> CombatLogReportHandler for Ff14ReportsGenerator<'a> {
    type Data = Ff14CombatLogPacket;
    fn handle(&mut self, data: &Self::Data) -> Result<(), SquadOvError> {
        if let Some(dr) = self.death_report.as_mut() {
            dr.handle(data)?;
        }
        Ok(())
    }
}

impl<'a> CombatLogReportIO for Ff14ReportsGenerator<'a> {
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
            let mut dr = death::Ff14DeathReportGenerator::default();
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
            ..Ff14ReportsGenerator::default()
        })
    }
}