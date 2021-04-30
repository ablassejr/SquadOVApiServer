use crate::SquadOvError;
use crate::proto::csgo::{
    CsvcMsgSendTable,
    csvc_msg_send_table,
};
use crate::parse::bit_reader::BitReader;
use std::collections::{HashSet, HashMap};
use prost::Message;
use std::sync::Arc;

const SPROP_EXCLUDE: i32 = 1 << 6;
const SPROP_INSIDEARRAY: i32 = 1 << 8;
const SPROP_COLLAPSIBLE: i32 = 1 << 11;

#[repr(i32)]
enum CsgoPropType {
    //DptInt = 0,
    //DptFloat = 1,
    //DptVector = 2,
    //DptVectorXy = 3,
    //DptString = 4,
    DptArray = 5,
    DptDataTable = 6,
    //DptIn64 = 7,
}

struct CsgoDtPropHandle {
    table: Arc<CsvcMsgSendTable>,
    idx: usize,
}

impl CsgoDtPropHandle {
    fn get(&self) -> Option<&csvc_msg_send_table::SendpropT> {
        self.table.props.get(self.idx)
    }
}

impl std::fmt::Debug for CsgoDtPropHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

#[derive(Debug)]
pub struct CsgoServerClassFlatPropEntry {
    prop: CsgoDtPropHandle,
    array_element_prop: Option<CsgoDtPropHandle>,
}

#[derive(Debug)]
pub struct CsgoServerClass {
    class_id: i32,
    name: String,
    dt_name: String,
    props: Vec<CsgoServerClassFlatPropEntry>,
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
    classes: HashMap<i32, CsgoServerClass>,
    // Used later when parsing entities
    server_class_bits: usize, 
}

// Indexed by: (DT Name, Variable name)
type CsgoExcludedProps = HashSet<(String, String)>;

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

    fn gather_exclude_entries(&self, table: Arc<CsvcMsgSendTable>, excluded: &mut CsgoExcludedProps) {
        for prop in &table.props {
            if prop.flags() & SPROP_EXCLUDE > 0 {
                let sub_dt_name = prop.dt_name().to_string();
                excluded.insert((
                    sub_dt_name.clone(),
                    prop.var_name().to_string(),
                ));

                if prop.r#type() == CsgoPropType::DptDataTable as i32 {
                    let sub_table = self.tables.get(&sub_dt_name);
                    if let Some(st) = sub_table {
                        self.gather_exclude_entries(st.clone(), excluded);
                    }
                }
            }
        }
    }

    fn gather_props_iterate_props(&self, table: Arc<CsvcMsgSendTable>, class: &mut CsgoServerClass, excluded: &CsgoExcludedProps, output_props: &mut Vec<CsgoServerClassFlatPropEntry>) {
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
                        self.gather_props_iterate_props(st.clone(), class, excluded, output_props);
                    } else {
                        self.gather_props(st.clone(), class, excluded);
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
                    }
                );
            }
        }
    }

    fn gather_props(&self, table: Arc<CsvcMsgSendTable>, class: &mut CsgoServerClass, excluded: &CsgoExcludedProps) {
        let mut tmp_props: Vec<CsgoServerClassFlatPropEntry> = vec![];
        self.gather_props_iterate_props(table.clone(), class, excluded, &mut tmp_props);
        class.props.append(&mut tmp_props);
    }

    fn flatten_data_table(&self, entry: &mut CsgoServerClass) {
        // Modifies the class to store a handle to all the DT props.
        // While Valve's C++ can store a pointer, Rust prevents us from doing
        // that so store a handle object that can effectively function as a pointer.
        let data_table = self.tables.get(&entry.dt_name);
        if let Some(dt) = data_table {
            let mut excluded: CsgoExcludedProps = HashSet::new();
            self.gather_exclude_entries(dt.clone(), &mut excluded);
            self.gather_props(dt.clone(), entry, &excluded);

            // Sort props by priority.
            entry.props.sort_by(|a, b| {
                let a_prio = if let Some(aprop) = a.prop.get() {
                    aprop.priority()
                } else {
                    0
                };
                let b_prio = if let Some(bprop) = b.prop.get() {
                    bprop.priority()
                } else {
                    0
                };
                a_prio.cmp(&b_prio)
            });
        }
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
            dt.classes.insert(entry.class_id, entry);
        }

        dt.server_class_bits = crate::math::integer_log2(num_server_classes) as usize + 1;
        Ok(dt)
    }
}