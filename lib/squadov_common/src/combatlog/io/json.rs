use crate::{
    SquadOvError,
    combatlog::{
        io::CombatLogDiskIO,
    },
};
use serde::Serialize;

pub struct CombatLogJsonFileIO {
    file: std::fs::File,
}

impl CombatLogDiskIO for CombatLogJsonFileIO {
    fn handle<T>(&mut self, data: T) -> Result<(), SquadOvError>
    where
        T: Serialize
    {
        serde_json::to_writer(&self.file, &data)?;
        Ok(())
    }

    fn get_underlying_file(self) -> Result<tokio::fs::File, SquadOvError> {
        Ok(tokio::fs::File::from_std(self.file))
    }
}

impl CombatLogJsonFileIO {
    pub fn new(dir: &str) -> Result<Self, SquadOvError> {
        let file = tempfile::tempfile_in(dir)?;
        Ok(Self{
            file,
        })
    }
}