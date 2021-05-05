use crate::SquadOvError;
use crate::proto::csgo::{
    CsvcMsgSendTable,
    csvc_msg_send_table,
};
use crate::parse::bit_reader::BitReader;
use std::collections::{HashSet, HashMap, BTreeSet};
use super::prop_types::{
    CsgoPropType,
    SPROP_EXCLUDE,
    SPROP_INSIDEARRAY,
    SPROP_COLLAPSIBLE,
    SPROP_CHANGES_OFTEN,
};
use prost::Message;
use std::sync::Arc;

#[derive(Clone)]
pub struct CsgoDtPropHandle {
    table: Arc<CsvcMsgSendTable>,
    idx: usize,
}

impl CsgoDtPropHandle {
    pub fn get(&self) -> Option<&csvc_msg_send_table::SendpropT> {
        self.table.props.get(self.idx)
    }
}

impl std::fmt::Debug for CsgoDtPropHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

#[derive(Debug, Clone)]
pub struct CsgoServerClassFlatPropEntry {
    pub prop: CsgoDtPropHandle,
    pub array_element_prop: Option<CsgoDtPropHandle>,
    pub prefix: String,
}

impl CsgoServerClassFlatPropEntry {
    pub fn full_name(&self) -> String {
        let base_name = if let Some(p) = self.prop.get() {
            String::from(p.var_name())
        } else {
            String::new()
        };

        if self.prefix.is_empty() {
            base_name
        } else {
            format!("{}.{}", &self.prefix, base_name)
        }
    }    
}

#[derive(Debug)]
pub struct CsgoServerClass {
    pub class_id: i32,
    pub name: String,
    pub dt_name: String,
    pub props: Vec<CsgoServerClassFlatPropEntry>,
    pub baseclasses: CsgoBaseClasses,
}

impl CsgoServerClass {
    pub fn get_prop(&self, idx: usize) -> Option<&CsgoServerClassFlatPropEntry> {
        self.props.get(idx)
    }
}

#[derive(Debug)]
pub struct CsgoDemoDataTable {
    // Indexed by table name
    tables: HashMap<String, Arc<CsvcMsgSendTable>>,
    // Indexed by class ID.
    pub classes: HashMap<i32, CsgoServerClass>,
    // Used later when parsing entities
    server_class_bits: usize, 
}

// Indexed by: (DT Name, Variable name)
type CsgoExcludedProps = HashSet<(String, String)>;
type CsgoBaseClasses = HashSet<String>;

impl CsgoDemoDataTable {
    pub fn receive_table(&mut self, table: CsvcMsgSendTable) -> Result<(), SquadOvError> {
        let table_name = table.net_table_name.clone().ok_or(SquadOvError::BadRequest)?;
        if !self.tables.contains_key(&table_name) {
            self.tables.insert(table_name.clone(), Arc::new(table.clone()));
        }
        Ok(())
    }

    pub fn get_server_class_from_id(&self, class_id: i32) -> Option<&CsgoServerClass> {
        self.classes.get(&class_id)
    }

    pub fn get_server_class_bits(&self) -> usize {
        return self.server_class_bits
    }

    fn gather_baseclass(&self, table: Arc<CsvcMsgSendTable>, baseclasses: &mut CsgoBaseClasses) {
        for prop in &table.props {
            let sub_dt_name = prop.dt_name().to_string();
            if prop.var_name() == "baseclass" {
                for (_id, class) in &self.classes {
                    if class.dt_name == sub_dt_name {
                        baseclasses.insert(class.name.clone());
                        break;
                    }   
                }
            }

            if prop.r#type() == CsgoPropType::DptDataTable as i32 {
                if let Some(st) = self.tables.get(&sub_dt_name) {
                    self.gather_baseclass(st.clone(), baseclasses);
                }
            }
        }
    }

    fn gather_exclude_entries(&self, table: Arc<CsvcMsgSendTable>, excluded: &mut CsgoExcludedProps) {
        for prop in &table.props {
            let sub_dt_name = prop.dt_name().to_string();
            if prop.flags() & SPROP_EXCLUDE > 0 {
                excluded.insert((
                    sub_dt_name.clone(),
                    prop.var_name().to_string(),
                ));
            }

            if prop.r#type() == CsgoPropType::DptDataTable as i32 {
                if let Some(st) = self.tables.get(&sub_dt_name) {
                    self.gather_exclude_entries(st.clone(), excluded);
                }
            }
        }
    }

    fn gather_props_iterate_props(&self, table: Arc<CsvcMsgSendTable>, class: &mut CsgoServerClass, excluded: &CsgoExcludedProps, output_props: &mut Vec<CsgoServerClassFlatPropEntry>, prefix: &str) {
        for (idx, prop) in table.props.iter().enumerate() {
            if prop.flags() & SPROP_INSIDEARRAY > 0 ||
                prop.flags() & SPROP_EXCLUDE > 0 ||
                excluded.contains(&(table.net_table_name().to_string(), prop.var_name().to_string())) {
                continue;
            }

            let prop_type = prop.r#type();
            if prop_type == CsgoPropType::DptDataTable as i32 {
                let sub_dt_name = prop.dt_name().to_string();
                let sub_table = self.tables.get(&sub_dt_name);
                if let Some(st) = sub_table {
                    if prop.flags() & SPROP_COLLAPSIBLE > 0 {
                        self.gather_props_iterate_props(st.clone(), class, excluded, output_props, prefix);
                    } else {
                        let new_prefix = if prefix.is_empty() {
                            prop.var_name().to_string()
                        } else {
                            format!("{}.{}", prefix, prop.var_name())
                        };
                        self.gather_props(st.clone(), class, excluded, &new_prefix);
                    }
                }
            } else if prop_type == CsgoPropType::DptArray as i32 {
                output_props.push(
                    CsgoServerClassFlatPropEntry{
                        prop: CsgoDtPropHandle{
                            table: table.clone(),
                            idx,
                        },
                        array_element_prop: Some(CsgoDtPropHandle{
                            table: table.clone(),
                            idx: idx - 1,
                        }),
                        prefix: prefix.to_string(),
                    }
                );
            } else {
                output_props.push(
                    CsgoServerClassFlatPropEntry{
                        prop: CsgoDtPropHandle{
                            table: table.clone(),
                            idx,
                        },
                        array_element_prop: None,
                        prefix: prefix.to_string(),
                    }
                );
            }
        }
    }

    fn gather_props(&self, table: Arc<CsvcMsgSendTable>, class: &mut CsgoServerClass, excluded: &CsgoExcludedProps, prefix: &str) {
        let mut tmp_props: Vec<CsgoServerClassFlatPropEntry> = vec![];
        self.gather_props_iterate_props(table.clone(), class, excluded, &mut tmp_props, prefix);
        class.props.append(&mut tmp_props);
    }

    fn flatten_data_table(&self, entry: &mut CsgoServerClass) -> Result<(), SquadOvError> {
        // Modifies the class to store a handle to all the DT props.
        // While Valve's C++ can store a pointer, Rust prevents us from doing
        // that so store a handle object that can effectively function as a pointer.
        let data_table = self.tables.get(&entry.dt_name);
        if let Some(dt) = data_table {
            let mut excluded: CsgoExcludedProps = HashSet::new();

            self.gather_exclude_entries(dt.clone(), &mut excluded);
            self.gather_props(dt.clone(), entry, &excluded, "");

            // Sort props by priority. There's an additional requirement that
            // props with the SPROP_CHANGES_OFTEN flag are functionally equivalent
            // to priority = 64. Note that Valve does a really weird sorting algorithm here.
            // What they do is 1) collect all the unique priority values (N) and then
            // 2) perform N passes through the props. In each pass, Valve keeps track of
            // the next un-sorted index and will swap the next valid prop into that slot.
            // THIS IS NOT A STABLE SORT. Thus we have to match Valve's algorithm here instead
            // of just condensing it into a regular sort algorithm.
            let mut unique_priorities: BTreeSet<i32> = BTreeSet::new();
            unique_priorities.insert(64);

            for p in &entry.props {
                if let Some(prop) = p.prop.get() {
                    unique_priorities.insert(prop.priority());
                }
            }

            // 'start' is the current index of the sorted props vector.
            let mut start: usize = 0;
            for priority in unique_priorities {
                // 'current_prop' is the current index in the props vector that we're testing to see
                // if it should be put into 'start' instead.
                loop {
                    let mut current_prop = start;
                    while current_prop < entry.props.len() {
                        // The first get indexes the array and since we check the index in the while loop
                        // the unwrap should be safe here.
                        let prop_priority: i32;
                        let prop_flags: i32;

                        if let Some(prop) = entry.props.get(current_prop).unwrap().prop.get() {
                            prop_priority = prop.priority();
                            prop_flags = prop.flags();
                        } else {
                            log::warn!("Detected invalid prop pointer?");
                            return Err(SquadOvError::BadRequest);
                        }

                        if prop_priority == priority || (priority == 64 && (prop_flags & SPROP_CHANGES_OFTEN > 0)) {
                            if start != current_prop {
                                let old_start = entry.props[start].clone();
                                entry.props[start] = entry.props[current_prop].clone();
                                entry.props[current_prop] = old_start;
                            }
                            start += 1;
                            break;
                        }

                        current_prop += 1;
                    }

                    if current_prop == entry.props.len() {
                        break
                    }
                }
            }
        }

        Ok(())
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
            let _ = reader.read_var_u32()?;
            let buffer_size = reader.read_var_u32()? as usize;
            let buffer = reader.read_aligned_bytes(buffer_size)?;
            let msg = CsvcMsgSendTable::decode(buffer)?;

            if msg.is_end.unwrap_or(false) {
                break;
            }

            dt.receive_table(msg)?;
        }

        let num_server_classes = reader.read_signed_multibit(16)? as i32;
        for _i in 0..num_server_classes {
            let mut entry = CsgoServerClass{
                class_id: reader.read_signed_multibit(16)? as i32,
                name: reader.read_null_terminated_str()?,
                dt_name: reader.read_null_terminated_str()?,
                props: vec![],
                baseclasses: HashSet::new(),
            };

            if entry.class_id >= num_server_classes {
                log::warn!("Entry Class ID {} is greater than max {}", entry.class_id, num_server_classes);
                return Err(SquadOvError::BadRequest);
            }

            // Nothing really jumps out to me as to why we can't flatten the data table stuff early here
            // since all the data tables have been added already.
            dt.flatten_data_table(&mut entry)?;

            if entry.name == "CCSPlayer" {
                log::info!("CSGO CLASS: {}", entry.name);
                log::info!("--- PROPS ---");
                for p in &entry.props {
                    log::info!("\t{} [type: {}]", p.full_name(), p.prop.get().unwrap().r#type());
                }
            }
            dt.classes.insert(entry.class_id, entry);
        }

        // Second pass to determine baseclasses.
        let all_class_keys: Vec<i32> = dt.classes.keys().map(|x| { *x }).collect();
        for key in all_class_keys {
            let dt_name = if let Some(class) = dt.classes.get(&key) {
                class.dt_name.clone()
            } else {
                continue;
            };

            let table = dt.tables.get(&dt_name);
            if table.is_none() {
                continue;
            }

            let mut baseclasses: CsgoBaseClasses = HashSet::new();
            dt.gather_baseclass(table.unwrap().clone(), &mut baseclasses);

            if let Some(class) = dt.classes.get_mut(&key) {
                class.baseclasses = baseclasses;
            }
        }

        dt.server_class_bits = crate::math::integer_log2(num_server_classes) as usize + 1;
        Ok(dt)
    }
}