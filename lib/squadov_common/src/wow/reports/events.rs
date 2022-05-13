mod deaths;
mod auras;
mod encounters;
mod resurrections;
mod aura_breaks;
mod spell_casts;

use crate::{
    SquadOvError,
    combatlog::{
        CombatLogReportHandler,
        CombatLogReportIO,
        CombatLogReport,
    },
    wow::combatlog::{
        WowCombatLogPacket
    },
};
use std::sync::Arc;

pub struct WowEventReportGenerator<'a> {
    work_dir: Option<String>,
    death_gen: Option<deaths::WowDeathEventsReportGenerator<'a>>,
    aura_gen: Option<auras::WowAuraReportGenerator<'a>>,
    encounter_gen: Option<encounters::WowEncounterReportGenerator<'a>>,
    resurrection_gen: Option<resurrections::WowResurrectionReportGenerator<'a>>,
    aura_break_gen: Option<aura_breaks::WowAuraBreakReportGenerator<'a>>,
    spell_cast_gen: Option<spell_casts::WowSpellCastReportGenerator<'a>>,
}

impl<'a> CombatLogReportHandler for WowEventReportGenerator<'a> {
    type Data = WowCombatLogPacket;
    fn handle(&mut self, data: &Self::Data) -> Result<(), SquadOvError> {
        if let Some(gen) = self.death_gen.as_mut() {
            gen.handle(data)?;
        }

        if let Some(gen) = self.aura_gen.as_mut() {
            gen.handle(data)?;
        }

        if let Some(gen) = self.encounter_gen.as_mut() {
            gen.handle(data)?;
        }

        if let Some(gen) = self.resurrection_gen.as_mut() {
            gen.handle(data)?;
        }

        if let Some(gen) = self.aura_break_gen.as_mut() {
            gen.handle(data)?;
        }

        if let Some(gen) = self.spell_cast_gen.as_mut() {
            gen.handle(data)?;
        }
        Ok(())
    }
}

impl<'a> WowEventReportGenerator<'a> {
    pub fn new() -> Self {
        Self {
            work_dir: None,
            death_gen: None,
            aura_gen: None,
            encounter_gen: None,
            resurrection_gen: None,
            aura_break_gen: None,
            spell_cast_gen: None,
        }
    }
}

impl<'a> CombatLogReportIO for WowEventReportGenerator<'a> {
    fn finalize(&mut self) -> Result<(), SquadOvError> {
        if let Some(gen) = self.death_gen.as_mut() {
            gen.finalize()?;
        }

        if let Some(gen) = self.aura_gen.as_mut() {
            gen.finalize()?;
        }

        if let Some(gen) = self.encounter_gen.as_mut() {
            gen.finalize()?;
        }

        if let Some(gen) = self.resurrection_gen.as_mut() {
            gen.finalize()?;
        }

        if let Some(gen) = self.aura_break_gen.as_mut() {
            gen.finalize()?;
        }

        if let Some(gen) = self.spell_cast_gen.as_mut() {
            gen.finalize()?;
        }

        Ok(())
    }

    fn initialize_work_dir(&mut self, dir: &str) -> Result<(), SquadOvError> {
        self.work_dir = Some(dir.to_string());

        {
            let mut gen = deaths::WowDeathEventsReportGenerator::new();
            gen.initialize_work_dir(dir)?;
            self.death_gen = Some(gen);
        }

        {
            let mut gen = auras::WowAuraReportGenerator::new();
            gen.initialize_work_dir(dir)?;
            self.aura_gen = Some(gen);
        }

        {
            let mut gen = encounters::WowEncounterReportGenerator::new();
            gen.initialize_work_dir(dir)?;
            self.encounter_gen = Some(gen);
        }

        {
            let mut gen = resurrections::WowResurrectionReportGenerator::new();
            gen.initialize_work_dir(dir)?;
            self.resurrection_gen = Some(gen);
        }

        {
            let mut gen = aura_breaks::WowAuraBreakReportGenerator::new();
            gen.initialize_work_dir(dir)?;
            self.aura_break_gen = Some(gen);
        }

        {
            let mut gen = spell_casts::WowSpellCastReportGenerator::new();
            gen.initialize_work_dir(dir)?;
            self.spell_cast_gen = Some(gen);
        }

        Ok(())
    }

    fn get_reports(&mut self) -> Result<Vec<Arc<dyn CombatLogReport + Send + Sync>>, SquadOvError> {
        let mut ret: Vec<Arc<dyn CombatLogReport + Send + Sync>> = vec![];

        if let Some(mut gen) = self.death_gen.take() {
            ret.extend(gen.get_reports()?);
        }

        if let Some(mut gen) = self.aura_gen.take() {
            ret.extend(gen.get_reports()?);
        }

        if let Some(mut gen) = self.encounter_gen.take() {
            ret.extend(gen.get_reports()?);
        }

        if let Some(mut gen) = self.resurrection_gen.take() {
            ret.extend(gen.get_reports()?);
        }

        if let Some(mut gen) = self.aura_break_gen.take() {
            ret.extend(gen.get_reports()?);
        }

        if let Some(mut gen) = self.spell_cast_gen.take() {
            ret.extend(gen.get_reports()?);
        }
        
        Ok(ret)
    }
}