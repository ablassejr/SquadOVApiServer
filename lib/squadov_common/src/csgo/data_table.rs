use crate::SquadOvError;
use crate::proto::csgo::{
    CsvcMsgSendTable,
    csvc_msg_send_table
};
use crate::parse::bit_reader::BitReader;
use std::collections::HashMap;
use prost::Message;
use std::sync::Arc;

#[derive(Debug)]
struct CsgoServerClassFlatPropEntry {
    prop: csvc_msg_send_table::SendpropT,
    array_element_prop: csvc_msg_send_table::SendpropT,
}

#[derive(Debug)]
struct CsgoServerClass {
    class_id: i32,
    name: String,
    dt_name: String,
    props: Vec<CsgoServerClassFlatPropEntry>,
}

pub struct CsgoDemoDataTable {
    // Indexed by table name
    tables: HashMap<String, Arc<CsvcMsgSendTable>>,
    // Indexed by class ID.
    classes: HashMap<i32, CsgoServerClass>,
    // Used later when parsing entities
    server_class_bits: usize, 
}

impl CsgoDemoDataTable {
    fn receive_table(&mut self, table: CsvcMsgSendTable) -> Result<(), SquadOvError> {
        let table_name = table.net_table_name.clone().ok_or(SquadOvError::BadRequest)?;
        if !self.tables.contains_key(&table_name) {
            self.tables.insert(table_name.clone(), Arc::new(table.clone()));
        }

        log::debug!("TABLE: {:?}", &table);
        Ok(())
    }

    fn flatten_data_table(&self, entry: &mut CsgoServerClass) {
        // Modifies the class to store a handle to all the DT props.
        // While Valve's C++ can store a pointer, Rust prevents us from doing
        // that so store a handle object that can effectively function as a pointer.
    }

    pub fn parse(data: &[u8]) -> Result<Self, SquadOvError> {
        let mut reader = BitReader::new(data);

        let mut dt = Self{
            tables: HashMap::new(),
            classes: HashMap::new(),
            server_class_bits: 0,
        };

        loop {
            // The 'type' doesn't seem to be necessary?
            let _ = reader.read_var_i32()?;
            let buffer_size = reader.read_var_i32()? as usize;
            let buffer = reader.read_aligned_bytes(buffer_size)?;
            let msg = CsvcMsgSendTable::decode(buffer)?;

            if msg.is_end.unwrap_or(false) {
                break;
            }

            dt.receive_table(msg)?;
        }

        let num_server_classes = reader.read_multibit::<u16>(16)? as i32;
        for _i in 0..num_server_classes {
            let mut entry = CsgoServerClass{
                class_id: reader.read_multibit::<u16>(16)? as i32,
                name: reader.read_null_terminated_str()?,
                dt_name: reader.read_null_terminated_str()?,
                props: vec![],
            };

            if entry.class_id >= num_server_classes {
                log::warn!("Entry Class ID {} is greater than max {}", entry.class_id, num_server_classes);
                return Err(SquadOvError::BadRequest);
            }

            // Nothing really jumps out to me as to why we can't flatten the data table stuff early here
            // since all the data tables have been added already.
            dt.flatten_data_table(&mut entry);

            log::debug!("server class: {:?}", entry);
            dt.classes.insert(entry.class_id, entry);
        }

        dt.server_class_bits = crate::math::integer_log2(num_server_classes) as usize + 1;
        Ok(dt)
    }
}