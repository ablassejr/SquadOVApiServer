use crate::SquadOvError;
use super::demo::{
    CsgoDemo,
    CsgoDemoCmdHeader,
    CsgoDemoCmdMessage,
    parse_csgo_demo_header,
    parse_csgo_demo_cmd_header,
    parse_csgo_demo_cmd_info,
};
use super::entity::{
    CsgoEntity,
    CsgoEntityCallback,
    CsgoDemoEntityParseState,
};
use super::data_table::{
    CsgoDemoDataTable,
    CsgoServerClass,
};
use super::math::{
    CsgoVector,
    CsgoBoundingBox,
};
use std::path::Path;
use std::io::Read;
use nom::number::complete::{
    le_i32,
};
use crate::proto::csgo::{
    NetMessages,
    SvcMessages,
    CsvcMsgGameEventList,
    CsvcMsgGameEvent,
    CsvcMsgCreateStringTable,
    CsvcMsgUpdateStringTable,
    CsvcMsgSendTable,
    CsvcMsgPacketEntities,
    csvc_msg_game_event_list,
};
use prost::Message;
use crate::bit_reader::BitReader;
use std::sync::{RwLock, Arc};

const DEMO_HEADER_ID: &str = "HL2DEMO";
const DEMO_PROTOCOL: i32 = 4;
const NET_MAX_PAYLOAD: i32 = 262144 - 4;
const DEMO_RECORD_BUFFER_SIZE: usize = 2 * 1024 * 1024;
const CSGO_DEMO_HEADER_SIZE: usize = 1072;
const CSGO_DEMO_CMD_HEADER_SIZE: usize = 6;
const CSGO_DEMO_CMD_INFO_SIZE: usize = 152;

struct CsgoDemoRawFile<'a> {
    // Raw byte data
    reader: BitReader<'a>,
    // Intermediary state tracking
    event_list: Option<CsvcMsgGameEventList>,
    data_table: Option<Arc<RwLock<CsgoDemoDataTable>>>,
}

impl<'a> CsgoDemoRawFile<'a> {
    fn new(buffer: &'a [u8]) -> Self {
        Self {
            reader: BitReader::new(&buffer),
            event_list: None,
            data_table: None,
        }
    }

    fn get_raw_payload_data(&mut self, max_size: usize, context: &str) -> Result<&[u8], SquadOvError> {
        let payload_len = le_i32(self.reader.read_aligned_bytes(4)?)?.1;
        if max_size > 0 && payload_len as usize > max_size {
            return Err(SquadOvError::InternalError(format!("{} size {} > {}", context, payload_len, max_size)));
        }

        Ok(self.reader.read_aligned_bytes(payload_len as usize)?)
    }

    // This function forces us to read the header.
    // So it'll reset the ptr to 0 even if we already technically
    // read past it already.
    fn read_header(&mut self, demo: &mut CsgoDemo) -> Result<(), SquadOvError> {
        demo.header = parse_csgo_demo_header(self.reader.read_aligned_bytes(CSGO_DEMO_HEADER_SIZE)?)?.1;
        Ok(())
    }

    fn read_command_header(&mut self) -> Result<CsgoDemoCmdHeader, SquadOvError> {
        let hdr = parse_csgo_demo_cmd_header(self.reader.read_aligned_bytes(CSGO_DEMO_CMD_HEADER_SIZE)?)?.1;
        Ok(hdr)
    }

    fn find_event_descriptor(&self, event_id: i32) -> Option<&csvc_msg_game_event_list::DescriptorT> {
        match &self.event_list {
            Some(el) => {
                for d in &el.descriptors {
                    if let Some(ref_event_id) = d.eventid {
                        if event_id == ref_event_id {
                            return Some(d);
                        }
                    }
                }
                None
            },
            None => None
        }
    }

    fn read_demo_packet(&mut self, tick: i32, demo: &mut CsgoDemo) -> Result<(), SquadOvError> {
        let _info = parse_csgo_demo_cmd_info(self.reader.read_aligned_bytes(CSGO_DEMO_CMD_INFO_SIZE)?)?.1;
        // Skip over 2 int32_t worths of data (dummy info).
        self.reader.advance_bytes(8);

        let payload_len = le_i32(self.reader.read_aligned_bytes(4)?)?.1;
        if payload_len > NET_MAX_PAYLOAD {
            return Err(SquadOvError::InternalError(format!("CS:GO Demo Payload greater than NET_MAX_PAYLOAD: {}", payload_len)));
        }

        let target_ptr = self.reader.loc_bytes() + payload_len as usize;
        while self.reader.loc_bytes() < target_ptr {
            let cmd = self.reader.read_var_u32()?;
            let size = self.reader.read_var_u32()? as usize;
            
            if self.reader.loc_bytes() + size > target_ptr {
                return Err(SquadOvError::InternalError(format!("CS:GO Demo Payload failed to parse for cmd [{}] with size [{}]", cmd, size)));
            }

            // This needs to happen here to increment the pointer.
            let raw_buffer = self.reader.read_aligned_bytes(size as usize)?;

            if let Some(_ncmd) = NetMessages::from_i32(cmd as i32) {
                // Not much of importance happens within NetMessages I think.
            } else if let Some(scmd) = SvcMessages::from_i32(cmd as i32) {
                match scmd {
                    SvcMessages::SvcGameEventList => {
                        self.event_list = Some(CsvcMsgGameEventList::decode(raw_buffer)?);
                    },
                    SvcMessages::SvcGameEvent => {
                        let game_event = CsvcMsgGameEvent::decode(raw_buffer)?;
                        if let Some(event_id) = game_event.eventid {
                            if let Some(descriptor) = self.find_event_descriptor(event_id) {
                                demo.handle_game_event(tick, game_event, descriptor)?;
                            }
                        }
                    },
                    SvcMessages::SvcCreateStringTable => {
                        let string_table_msg = CsvcMsgCreateStringTable::decode(raw_buffer)?;
                        demo.handle_string_table_create(string_table_msg)?;
                    },
                    SvcMessages::SvcUpdateStringTable => {
                        let string_table_msg = CsvcMsgUpdateStringTable::decode(raw_buffer)?;
                        demo.handle_string_table_update(string_table_msg)?;
                    },
                    // Handling the svc_PacketEntities message is crucial for tracking the state of players and other entities (weapons, nades, etc.).
                    SvcMessages::SvcPacketEntities => {
                        let msg = CsvcMsgPacketEntities::decode(raw_buffer)?;
                        demo.handle_entity_update(msg)?;
                    },
                    SvcMessages::SvcSendTable => {
                        let msg = CsvcMsgSendTable::decode(raw_buffer)?;
                        if let Some(dt) = &self.data_table {
                            dt.write()?.receive_table(msg)?;
                        }
                    },
                    _ => (),
                }
            }
        }

        Ok(())
    }

    fn read_demo_data_table(&mut self) -> Result<(), SquadOvError> {
        // The data table is the table that contains all the classes known by the server.
        // Entities will be created using classes stored in the data table!!!
        let raw_buffer = self.get_raw_payload_data(DEMO_RECORD_BUFFER_SIZE, "CS:GO Demo Data Table")?;
        self.data_table = Some(Arc::new(RwLock::new(CsgoDemoDataTable::parse(raw_buffer)?)));
        Ok(())
    }

    fn read_demo_string_table(&mut self) -> Result<(), SquadOvError> {
        let _ = self.get_raw_payload_data(DEMO_RECORD_BUFFER_SIZE, "CS:GO Demo String Table")?;
        Ok(())
    }

    fn read_demo_console_cmd(&mut self) -> Result<(), SquadOvError> {
        let _ = self.get_raw_payload_data(0, "CS:GO Console Cmd")?;
        Ok(())
    }

    fn read_demo_user_cmd(&mut self) -> Result<(), SquadOvError> {
        let _outgoing_sequence = le_i32(self.reader.read_aligned_bytes(4)?)?.1;
        let _ = self.get_raw_payload_data(0, "CS:GO User Cmd")?;  
        Ok(())
    }

    fn read_body(&mut self, demo: &mut CsgoDemo) -> Result<(), SquadOvError> {
        loop {
            let cmd_header = self.read_command_header()?;
            match cmd_header.cmd {
                CsgoDemoCmdMessage::Stop => {
                    log::debug!("CSGO Demo Stop");
                    break;
                },
                CsgoDemoCmdMessage::SignOn | CsgoDemoCmdMessage::Packet => self.read_demo_packet(cmd_header.tick, demo)?,
                CsgoDemoCmdMessage::ConsoleCmd => self.read_demo_console_cmd()?,
                CsgoDemoCmdMessage::DataTables => {
                    self.read_demo_data_table()?;
                    if let Some(dt) = self.data_table.as_ref() {
                        demo.on_data_table(dt.clone())?;
                    }
                },
                CsgoDemoCmdMessage::StringTables => self.read_demo_string_table()?,
                CsgoDemoCmdMessage::UserCmd => self.read_demo_user_cmd()?,
                _ => {
                    log::debug!("CSGO Demo OTHER CMD - Not yet supported");
                },
            };
        }
        Ok(())
    }
}

struct CsgoCCSTeamHandler {
}

impl CsgoEntityCallback for CsgoCCSTeamHandler {
    fn notify(&self, entity: &CsgoEntity, _class: &CsgoServerClass, parse_state: &mut CsgoDemoEntityParseState) -> Result<(), SquadOvError> {
        let team_name = if let Some(prop_team_name) = entity.get_prop("m_szTeamname") {
            if let Some(team_name) = prop_team_name.value.v_str.as_ref() {
                team_name.as_str()
            } else {
                ""
            }
        } else {
            ""
        };

        let team_num = if let Some(prop_team_num) = entity.get_prop("") {
            prop_team_num.value.v_i32.unwrap_or(-1)
        } else {
            -1
        };

        if team_num != -1 {
            if team_name == "TERRORIST" {
                parse_state.terrorist_id = team_num;
            } else if team_name == "CT" {
                parse_state.ct_id = team_num;
            }
        }

        Ok(())
    }
}

struct CsgoCCSPlayerResourceHandler {
}

impl CsgoEntityCallback for CsgoCCSPlayerResourceHandler {
    fn notify(&self, entity: &CsgoEntity, _class: &CsgoServerClass, parse_state: &mut CsgoDemoEntityParseState) -> Result<(), SquadOvError> {
        if let Some(prop_bombsite_a) = entity.get_prop("m_bombsiteCenterA") {
            if let Some(bombsite_a) = prop_bombsite_a.value.v_vec.as_ref() {
                parse_state.site_a_center = bombsite_a.clone();
            }
        }
        
        if let Some(prop_bombsite_b) = entity.get_prop("m_bombsiteCenterB") {
            if let Some(bombsite_b) = prop_bombsite_b.value.v_vec.as_ref() {
                parse_state.site_b_center = bombsite_b.clone();
            }
        }

        parse_state.update_bombsites();
        Ok(())
    }
}

struct CsgoCBaseTriggerHandler {
}

impl CsgoEntityCallback for CsgoCBaseTriggerHandler {
    fn notify(&self, entity: &CsgoEntity, _class: &CsgoServerClass, parse_state: &mut CsgoDemoEntityParseState) -> Result<(), SquadOvError> {
        let min_vec = if let Some(prop) = entity.get_prop("m_vecMins") {
            if let Some(value) = prop.value.v_vec.as_ref() {
                value.clone()
            } else {
                CsgoVector::default()
            }
        } else {
            CsgoVector::default()
        };

        let max_vec = if let Some(prop) = entity.get_prop("m_vecMaxs") {
            if let Some(value) = prop.value.v_vec.as_ref() {
                value.clone()
            } else {
                CsgoVector::default()
            }
        } else {
            CsgoVector::default()
        };

        parse_state.triggers.insert(entity.id, CsgoBoundingBox{
            min: min_vec,
            max: max_vec,
        });

        parse_state.update_bombsites();
        Ok(())
    }
}

pub struct CsgoDemoParser {}

// Largely based off Valve's CSGO demo parser:
// https://github.com/ValveSoftware/csgo-demoinfo
impl CsgoDemoParser {
    fn from_file(file: &mut std::fs::File) -> Result<CsgoDemo, SquadOvError> {
        let mut raw_buffer: Vec<u8> = vec![];
        file.read_to_end(&mut raw_buffer)?;

        let mut demo = CsgoDemo::default();

        // Link multiple callbacks for when we receive entities from the demo files.
        // First thing, detect CCSTeam entities so that we know the appropriate team IDs.
        {
            demo.entities.add_entity_callback("CCSTeam", Box::new(CsgoCCSTeamHandler{}));
            demo.entities.add_entity_callback("CCSPlayerResource", Box::new(CsgoCCSPlayerResourceHandler{}));
            demo.entities.add_entity_callback("CBaseTrigger", Box::new(CsgoCBaseTriggerHandler{}));
        }

        let mut demo_file = CsgoDemoRawFile::new(&raw_buffer);
        demo_file.read_header(&mut demo)?;
        // Verify that we actually read a valid CS:GO demo.
        if demo.header.demo_filestamp != DEMO_HEADER_ID {
            return Err(SquadOvError::InternalError("CSGO Demo filestamp mismatch.".to_string()));
        }

        if demo.header.demo_protocol != DEMO_PROTOCOL {
            return Err(SquadOvError::InternalError("CSGO Demo protocol mismatch.".to_string()));
        }

        demo_file.read_body(&mut demo)?;

        Ok(demo)
    }

    pub fn from_path(path: &Path) -> Result<CsgoDemo, SquadOvError> {
        let mut file = std::fs::File::open(path)?;
        Ok(CsgoDemoParser::from_file(&mut file)?)
    }
}