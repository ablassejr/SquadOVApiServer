mod characters;
mod stats;

use crate::{
    SquadOvError,
    combatlog::{
        CombatLogReportHandler,
        CombatLogReportIO,
        CombatLogReport,
        CombatLog,
    },
    wow::combatlog::{
        WowCombatLogPacket,
        WoWCombatLogState,
    },
};
use num_enum::TryFromPrimitive;
use std::sync::Arc;
use sqlx::{
    postgres::{PgPool},
};

pub struct WowReportsGenerator {
    parent_cl: CombatLog,
    work_dir: Option<String>,
    character_gen: Option<characters::WowCharacterReportGenerator>,
    pool: Arc<PgPool>,
    cl_state: WoWCombatLogState,
}

#[derive(Copy, Clone, Debug, TryFromPrimitive, PartialEq)]
#[repr(i32)]
pub enum WowReportTypes {
    MatchCharacters,
    MatchCombatants,
    Events,
    StatSummary,
    StatDps,
    StatHps,
    StatDrps,
    CharacterLoadout,
    DeathRecap,
}

impl CombatLogReportHandler for WowReportsGenerator {
    type Data = WowCombatLogPacket;
    fn handle(&mut self, data: &Self::Data) -> Result<(), SquadOvError> {
        if let Some(gen) = self.character_gen.as_mut() {
            gen.handle(data)?;
        }
        Ok(())
    }
}

impl CombatLogReportIO for WowReportsGenerator {
    fn finalize(&mut self) -> Result<(), SquadOvError> {
        if let Some(g) = self.character_gen.as_mut() {
            g.finalize()?;
        }
        Ok(())
    }

    fn initialize_work_dir(&mut self, dir: &str) -> Result<(), SquadOvError> {
        self.work_dir = Some(String::from(dir));

        {
            let mut gen = characters::WowCharacterReportGenerator::new(self.pool.clone(), self.parent_cl.clone(), self.cl_state.build_version.clone());
            gen.initialize_work_dir(dir)?;
            self.character_gen = Some(gen);
        }

        Ok(())
    }

    fn get_reports(&mut self) -> Result<Vec<Arc<dyn CombatLogReport + Send + Sync>>, SquadOvError> {
        let mut ret: Vec<Arc<dyn CombatLogReport + Send + Sync>> = vec![];

        if let Some(gen) = self.character_gen.as_mut() {
            ret.extend(gen.get_reports()?);
        }

        Ok(ret)
    }
}

impl WowReportsGenerator {
    pub fn new(parent_cl: CombatLog, pool: Arc<PgPool>) -> Result<Self, SquadOvError> {
        let cl_state = serde_json::from_value(parent_cl.cl_state.clone())?;
        Ok(Self{
            parent_cl,
            character_gen: None,
            work_dir: None,
            pool,
            cl_state,
        })
    }
}