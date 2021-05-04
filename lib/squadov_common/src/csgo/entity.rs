use crate::proto::csgo::{
    CsvcMsgPacketEntities,
};
use crate::SquadOvError;
use crate::parse::bit_reader::BitReader;
use super::data_table::{
    CsgoDemoDataTable,
    CsgoServerClass,
};
use super::math::{
    CsgoVector,
    CsgoBoundingBox,
};
use super::prop::CsgoProp;
use super::weapon::{CsgoWeapon, csgo_string_to_weapon};
use std::sync::{RwLock, Arc};
use std::collections::HashMap;

enum CsgoEntityUpdateType {
    EnterPvs = 0,
    LeavePvs,
    DeltaEnt,
}

const FHDR_ZERO: i32 = 0x0000;
const FHDR_LEAVEPVS: i32 = 0x0001;
const FHDR_DELETE: i32 = 0x0002;
const FHDR_ENTERPVS: i32 = 0x0004;
const ENTITY_SENTINEL: i32 = 9999;
const NUM_NETWORKED_EHANDLE_SERIAL_NUMBER_BITS: usize = 10;

#[derive(Debug)]
pub struct CsgoEntity {
    pub id: i32,
    pub class: u32,
    serial: u32,
    props: HashMap<String, CsgoProp>,
}

impl CsgoEntity {
    fn add_update_prop(&mut self, prop: CsgoProp) {
        self.props.insert(prop.entry().full_name(), prop);
    }

    pub fn get_prop(&self, name: &str) -> Option<&CsgoProp> {
        self.props.get(name)
    }

    pub fn to_weapon(&self, class_name: &str) -> CsgoWeapon {
        csgo_string_to_weapon(class_name)
    }
}

// TODO: Add event emission support for entities for when
// we'd want to extract more in depth information out of the demo.
pub trait CsgoEntityCallback {
    fn notify(&self, _entity: &CsgoEntity, _class: &CsgoServerClass, _state: &mut CsgoDemoEntityParseState) -> Result<(), SquadOvError>;
}

// Generic parsing state so we know what certain numbers refer to.
// I.e. we need to know which ID is what team/what bombsites/etc.
#[derive(Debug)]
pub struct CsgoDemoEntityParseState {
    pub ct_id: i32,
    pub terrorist_id: i32,
    pub site_a_center: CsgoVector,
    pub site_b_center: CsgoVector,
    pub triggers: HashMap<i32, CsgoBoundingBox>,
    pub site_a_trigger: Option<i32>,
    pub site_b_trigger: Option<i32>,
}

impl Default for CsgoDemoEntityParseState {
    fn default() -> Self {
        Self {
            ct_id: -1,
            terrorist_id: -1,
            site_a_center: CsgoVector::default(),
            site_b_center: CsgoVector::default(),
            triggers: HashMap::new(),
            site_a_trigger: None,
            site_b_trigger: None,
        }
    }
}

impl CsgoDemoEntityParseState {
    pub fn update_bombsites(&mut self) {
        // So the CSGO demo only seems to send the bounding box of the
        // trigger for a single bombsite? It also doesn't seem to be reliable
        // on which one it sends either. So we need to assume that the demo
        // sends AT LEAST one. Thus, when we go to check which bomb site
        // the bomb has been planted at, we just need to check the sent bombsite
        // trigger bounding box. And if it doesn't match, then it's most likely
        // the other bomb site (obviously assumes that CSGO won't ever go crazy
        // and add a 3rd bomb site).
        for (eid, trigger) in &self.triggers {
            if trigger.contains(&self.site_a_center) {
                self.site_a_trigger = Some(*eid);
            } else if trigger.contains(&self.site_b_center) {
                self.site_b_trigger = Some(*eid);
            }
        }
    }
}

pub struct CsgoEntityScene {
    data_table: Option<Arc<RwLock<CsgoDemoDataTable>>>,
    entities: HashMap<i32, CsgoEntity>,
    // Entity update handler(s).
    callbacks: HashMap<String, Vec<Box<dyn CsgoEntityCallback>>>,
    pub parse_state: CsgoDemoEntityParseState,
}

impl std::fmt::Debug for CsgoEntityScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CsgoEntity")
            .field("data_table", &self.data_table)
            .field("entities", &self.entities)
            .finish()
    }
}

impl Default for CsgoEntityScene {
    fn default() -> Self {
        Self {
            data_table: None,
            entities: HashMap::new(),
            callbacks: HashMap::new(),
            parse_state: CsgoDemoEntityParseState::default(),
        }
    }
}

fn read_csgo_demo_field_index(reader: &mut BitReader, last_index: i32, new_way: bool) -> Result<i32, SquadOvError> {
    if new_way {
        if reader.read_bit()? {
            return Ok(last_index + 1);
        }
    }
    
    let ret: i32 = if new_way && reader.read_bit()? {
        reader.read_multibit::<u32>(3)? as i32
    } else {
        let tmp = reader.read_multibit::<u32>(7)? as i32;
        (tmp & !96) | ((match tmp & (32 | 64) {
            32 => reader.read_multibit::<u32>(2)?,
            64 => reader.read_multibit::<u32>(4)?,
            96 => reader.read_multibit::<u32>(7)?,
            _ => 0,
        } as i32) << 5)
    };

    Ok(if ret == 0xFFF {
        -1
    } else {
        last_index + 1 + ret
    })
}

impl CsgoEntityScene {
    pub fn connect_data_table(&mut self, table: Arc<RwLock<CsgoDemoDataTable>>){
        self.data_table = Some(table);
    }

    pub fn add_entity_callback(&mut self, class: &str, cb: Box<dyn CsgoEntityCallback>) {
        if !self.callbacks.contains_key(class) {
            self.callbacks.insert(class.to_string(), vec![]);
        }

        if let Some(callbacks) = self.callbacks.get_mut(class) {
            callbacks.push(cb);
        }
    }

    pub fn get_entity(&self, id: i32) -> Option<&CsgoEntity> {
        self.entities.get(&id)
    }

    pub fn get_class_name(&self, id: i32) -> Result<Option<String>, SquadOvError> {
        Ok(if let Some(dt) = &self.data_table {
            if let Some(class) = dt.read()?.get_server_class_from_id(id) {
                Some(class.name.clone())
            } else {
                None
            }
        } else {
            None
        })
    }

    fn create_entity(&mut self, entity_id: i32, class_id: u32, serial_num: u32) -> Result<(), SquadOvError> {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            entity.class = class_id;
            entity.serial = serial_num;
        } else {
            let entity = CsgoEntity{
                id: entity_id,
                class: class_id,
                serial: serial_num,
                props: HashMap::new(),
            };

            log::debug!("Creating entity: {}", entity_id);
            self.entities.insert(entity_id, entity);
        }
        Ok(())
    }

    fn update_entity_from_data(&mut self, entity_id: i32, reader: &mut BitReader) -> Result<(), SquadOvError> {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            let use_new_way = reader.read_bit()?;

            let mut field_indices: Vec<i32> = vec![];
            let mut index: i32 = -1;
            loop {
                index = read_csgo_demo_field_index(reader, index, use_new_way)?;
                if index == -1 {
                    break;
                }
                field_indices.push(index);
            }

            let data_table = self.data_table.as_ref().ok_or(SquadOvError::BadRequest)?.read()?;
            let server_class = data_table.get_server_class_from_id(entity.class as i32).ok_or(SquadOvError::NotFound)?;
            for idx in field_indices {
                let prop_entry = server_class.get_prop(idx as usize).ok_or(SquadOvError::NotFound)?;
                entity.add_update_prop(CsgoProp::parse(reader, prop_entry)?);
            }

            if let Some(callbacks) = self.callbacks.get(&server_class.name) {
                for cb in callbacks {
                    cb.notify(entity, server_class, &mut self.parse_state)?;
                }
            }
            Ok(())
        } else {
            log::warn!("Failed to find entity: {}", entity_id);
            Err(SquadOvError::NotFound)
        }
    }

    fn delete_entity(&mut self, _entity_id: i32) -> Result<(), SquadOvError> {
        // Don't delete entities?? The game seems to delete our player entities for some reason.
        Ok(())
    }

    pub fn handle_entity_update(&mut self, msg: CsvcMsgPacketEntities) -> Result<(), SquadOvError> {
        let server_class_bits = self.data_table.as_ref().ok_or(SquadOvError::BadRequest)?.read()?.get_server_class_bits();
        let is_delta = msg.is_delta();
        let mut reader = BitReader::new(msg.entity_data());
        let mut total_headers = msg.updated_entries();
        let mut header_base = -1;
        let mut new_entity;
        let mut update_flags: i32;

        while total_headers > 0 {
            total_headers -= 1;

            let is_entity = total_headers >= 0;
            if is_entity {
                update_flags = FHDR_ZERO;
                new_entity = header_base + 1 + reader.read_var_ubits()? as i32;
                header_base = new_entity;

                // Check whether or not we left the PVS.
                if reader.read_bit()? {
                    update_flags |= FHDR_LEAVEPVS;

                    // Check if we also want to delete the entity.
                    if reader.read_bit()? {
                        update_flags |= FHDR_DELETE;
                    }
                } else {
                    // If we didn't leave PVS, check if we entered it.
                    if reader.read_bit()? {
                        update_flags |= FHDR_ENTERPVS;
                    }
                }
            } else {
                break;
            }

            if new_entity > ENTITY_SENTINEL {
                break;
            }

            let update_type: CsgoEntityUpdateType = if update_flags & FHDR_ENTERPVS > 0 {
                CsgoEntityUpdateType::EnterPvs
            } else if update_flags & FHDR_LEAVEPVS > 0 {
                CsgoEntityUpdateType::LeavePvs
            } else {
                CsgoEntityUpdateType::DeltaEnt
            };

            match update_type {
                CsgoEntityUpdateType::EnterPvs => {
                    // Create a new entity of the appropriate class.
                    let class_id = reader.read_multibit::<u32>(server_class_bits)?;
                    let serial_num = reader.read_multibit::<u32>(NUM_NETWORKED_EHANDLE_SERIAL_NUMBER_BITS)?;
                    self.create_entity(new_entity, class_id, serial_num)?;
                    self.update_entity_from_data(new_entity, &mut reader)?;
                },
                CsgoEntityUpdateType::LeavePvs => {
                    // Once the entity leaves PVS it probably is no longer relevant.
                    // Only delete the entity if the delete flag is set.
                    if !is_delta {
                        log::warn!("Received LeavePvs during full update.");
                        return Err(SquadOvError::BadRequest);
                    }

                    if update_flags & FHDR_DELETE > 0 {
                        self.delete_entity(new_entity)?;
                    }
                },
                CsgoEntityUpdateType::DeltaEnt => {
                    self.update_entity_from_data(new_entity, &mut reader)?
                },
            }
        }

        Ok(())
    }   
}