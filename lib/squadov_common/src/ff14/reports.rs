use crate::{
    SquadOvError,
    combatlog::{
        CombatLogReportGenerator,
        RawCombatLogReport,
    },
    ff14::combatlog::Ff14CombatLogPacket,
};
use uuid::Uuid;

pub struct Ff14ReportsGenerator {
    view_id: Uuid,
}

impl CombatLogReportGenerator for Ff14ReportsGenerator {
    fn handle(&mut self, data: &str) -> Result<(), SquadOvError> {
        let data: Ff14CombatLogPacket = serde_json::from_str(data)?;
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), SquadOvError> {
        Ok(())
    }

    fn initialize_work_dir(&mut self, dir: &str) -> Result<(), SquadOvError> {
        Ok(())
    }

    fn get_reports(&mut self) -> Vec<RawCombatLogReport> {
        vec![]
    }
}

impl Ff14ReportsGenerator {
    pub fn new(view_id: &str) -> Result<Self, SquadOvError> {
        Ok(Self{
            view_id: Uuid::parse_str(view_id)?
        })
    }
}