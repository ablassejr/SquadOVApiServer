use crate::bit_reader::BitReader;
use crate::SquadOvError;
use super::data_table::{CsgoServerClassFlatPropEntry, CsgoServerClass};

pub struct CsgoProp {

}

impl CsgoProp {
    pub fn parse(reader: &mut BitReader, prop_entry: &CsgoServerClassFlatPropEntry, class: &CsgoServerClass, field_index: i32) -> Result<Self, SquadOvError> {
        Ok(Self {

        })
    }

}