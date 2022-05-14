pub mod sliding_window;

use crate::SquadOvError;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fmt::Debug;

#[derive(Debug)]
pub struct InputAggregatorPacket<T: Debug> {
    pub tm: DateTime<Utc>,
    pub data: T,
}

#[derive(Serialize)]
pub struct OutputAggregatorPacket<T> {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub value: T,
}

pub trait CombatLogAggregator<T: Debug> {
    fn handle(&mut self, packet: InputAggregatorPacket<T>) -> Result<Option<OutputAggregatorPacket<T>>, SquadOvError>;
    fn flush(&mut self) -> Result<OutputAggregatorPacket<T>, SquadOvError>;
}