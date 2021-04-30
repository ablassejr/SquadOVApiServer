use nom::number::streaming::{
    le_i32,
    le_f32,
    le_u8,
    le_u32,
    le_u64,
    be_u32,
    be_u64,
    be_i32,
};
use crate::proto::csgo::{
    CsvcMsgGameEvent,
    CsvcMsgCreateStringTable,
    CsvcMsgUpdateStringTable,
    csvc_msg_game_event_list,
};
use crate::SquadOvError;
use crate::parse::bit_reader::BitReader;
use std::collections::{HashMap, VecDeque};

const SUBSTRING_BITS: usize = 5;
const MAX_USERDATA_BITS: usize = 14;

#[derive(Debug)]
pub struct CsgoDemoHeader {
    pub demo_filestamp: String,
    pub demo_protocol: i32,
    pub network_protocol: i32,
    pub server_name: String,
    pub client_name: String,
    pub map_name: String,
    pub game_directory: String,
    pub playback_time: f32,
    pub playback_ticks: i32,
    pub playback_frames: i32,
    pub signon_length: i32
}

impl Default for CsgoDemoHeader {
    fn default() -> Self {
        Self {
            demo_filestamp: String::new(),
            demo_protocol: 0,
            network_protocol: 0,
            server_name: String::new(),
            client_name: String::new(),
            map_name: String::new(),
            game_directory: String::new(),
            playback_time: 0.0,
            playback_ticks: 0,
            playback_frames: 0,
            signon_length: 0,
        }
    }
}

named!(pub parse_csgo_demo_header<CsgoDemoHeader>,
    complete!(do_parse!(
        demo_filestamp: take_str!(8) >>
        demo_protocol: le_i32 >>
        network_protocol: le_i32 >>
        server_name: take_str!(260) >>
        client_name: take_str!(260) >>
        map_name: take_str!(260) >>
        game_directory: take_str!(260) >>
        playback_time: le_f32 >>
        playback_ticks: le_i32 >>
        playback_frames: le_i32 >>
        signon_length: le_i32 >>
        (CsgoDemoHeader{
            demo_filestamp: String::from(demo_filestamp).trim_end_matches(char::from(0)).to_string(),
            demo_protocol: demo_protocol,
            network_protocol: network_protocol,
            server_name: String::from(server_name).trim_end_matches(char::from(0)).to_string(),
            client_name: String::from(client_name).trim_end_matches(char::from(0)).to_string(),
            map_name: String::from(map_name).trim_end_matches(char::from(0)).to_string(),
            game_directory: String::from(game_directory).trim_end_matches(char::from(0)).to_string(),
            playback_time: playback_time,
            playback_ticks: playback_ticks,
            playback_frames: playback_frames,
            signon_length: signon_length,
        })
    ))
);

#[repr(u8)]
#[derive(Debug)]
pub enum CsgoDemoCmdMessage {
    // Startup message
    SignOn = 1,
    // Normal network packet
    Packet,
    // Sync client clock to demo tick
    SyncTick,
    // Console command
    ConsoleCmd,
    // User Cmd
    UserCmd,
    // Network data tables
    DataTables,
    // End of time
    Stop,
    // Blob of binary data understood by a callback function
    CustomData,
    // String tables?
    StringTables
}

#[derive(Debug)]
pub struct CsgoDemoCmdHeader {
    pub cmd: CsgoDemoCmdMessage,
    pub tick: i32,
    pub player_slot: u8,
}

named!(pub parse_csgo_demo_cmd_header<CsgoDemoCmdHeader>,
    complete!(do_parse!(
        cmd: switch!(le_u8,
            1 => value!(CsgoDemoCmdMessage::SignOn) |
            2 => value!(CsgoDemoCmdMessage::Packet) |
            3 => value!(CsgoDemoCmdMessage::SyncTick) |
            4 => value!(CsgoDemoCmdMessage::ConsoleCmd) |
            5 => value!(CsgoDemoCmdMessage::UserCmd) |
            6 => value!(CsgoDemoCmdMessage::DataTables) |
            7 => value!(CsgoDemoCmdMessage::Stop) |
            8 => value!(CsgoDemoCmdMessage::CustomData) |
            9 => value!(CsgoDemoCmdMessage::StringTables) |
            _ => value!(CsgoDemoCmdMessage::Stop)
        ) >>
        tick: le_i32 >>
        player_slot: le_u8 >>
        (CsgoDemoCmdHeader{
            cmd: cmd,
            tick: tick,
            player_slot: player_slot,
        })
    ))
);

#[derive(Debug)]
pub struct CsgoVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

named!(parse_csgo_vector<CsgoVector>,
    complete!(do_parse!(
        x: le_f32 >>
        y: le_f32 >>
        z: le_f32 >>
        (CsgoVector{
            x: x,
            y: y,
            z: z,
        })
    ))
);

#[derive(Debug)]
pub struct CsgoQAngle {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

named!(parse_csgo_qangle<CsgoQAngle>,
    complete!(do_parse!(
        x: le_f32 >>
        y: le_f32 >>
        z: le_f32 >>
        (CsgoQAngle{
            x: x,
            y: y,
            z: z,
        })
    ))
);

#[derive(Debug)]
pub struct CsgoDemoCmdSplitInfo {
    pub flags: i32,
    pub view_origin: CsgoVector,
    pub view_angles: CsgoQAngle,
    pub local_view_angles: CsgoQAngle,
    pub view_origin_2: CsgoVector,
    pub view_angles_2: CsgoQAngle,
    pub local_view_angles_2: CsgoQAngle,
}

named!(parse_csgo_demo_cmd_split_info<CsgoDemoCmdSplitInfo>,
    complete!(do_parse!(
        flags: le_i32 >>
        view_origin: parse_csgo_vector >>
        view_angles: parse_csgo_qangle >>
        local_view_angles: parse_csgo_qangle >>
        view_origin_2: parse_csgo_vector >>
        view_angles_2: parse_csgo_qangle >>
        local_view_angles_2: parse_csgo_qangle >>
        (CsgoDemoCmdSplitInfo{
            flags: flags,
            view_origin: view_origin,
            view_angles: view_angles,
            local_view_angles: local_view_angles,
            view_origin_2: view_origin_2,
            view_angles_2: view_angles_2,
            local_view_angles_2: local_view_angles_2,
        })
    ))
);

#[derive(Debug)]
pub struct CsgoDemoCmdInfo {
    pub splits: Vec<CsgoDemoCmdSplitInfo>
}

named!(pub parse_csgo_demo_cmd_info<CsgoDemoCmdInfo>,
    complete!(do_parse!(
        splits: many_m_n!(2, 2, parse_csgo_demo_cmd_split_info) >>
        (CsgoDemoCmdInfo{
            splits: splits,
        })
    ))
);

#[derive(Debug)]
pub struct CsgoDemoPacketMessage {
    pub cmd: i32,
    pub payload: Vec<u8>,
}

named!(parse_csgo_demo_packet_message<CsgoDemoPacketMessage>,
    complete!(do_parse!(
        cmd: le_i32 >>
        sz: le_i32 >>
        payload: take!(sz) >>
        (CsgoDemoPacketMessage{
            cmd: cmd,
            payload: payload.to_vec(),
        })
    ))
);

#[derive(Debug)]
pub struct CsgoDemoRound {

}

#[derive(Debug,Clone)]
pub struct CsgoDemoStringTable {
    name: String,
    max_entries: i32,
    user_data_size: i32,
    user_data_size_bits: i32,
    user_data_fixed_size: bool,
}

#[derive(Debug,Clone)]
pub struct CsgoDemoPlayerInfo {
    version: u64,
    // This is the same SteamID that we can find via GSI.
    xuid: u64,
    name: String,
    user_id: i32,
    guid: String,
    friends_id: u32,
    friends_name: String,
    fake_player: bool,
    is_hltv: bool,
    custom_files: Vec<u32>,
    files_downloaded: u8,
    entity_id: i32,
}

named!(parse_csgo_demo_player_info<CsgoDemoPlayerInfo>,
    complete!(do_parse!(
        version: le_u64 >>
        xuid: be_u64 >>
        name: take_str!(128) >>
        user_id: be_i32 >>
        guid: take_str!(33) >>
        p1: take!(3) >>
        friends_id: be_u32 >>
        friends_name: take_str!(128) >>
        fake_player: le_u8 >>
        is_hltv: le_u8 >>
        p2: take!(2) >>
        custom_files: many_m_n!(4, 4, le_u32) >>
        files_downloaded: le_u8 >>
        p3: take!(3) >>
        (CsgoDemoPlayerInfo{
            version: version,
            xuid: xuid,
            name: String::from(name).trim_end_matches(char::from(0)).to_string(),
            user_id: user_id,
            guid: String::from(guid).trim_end_matches(char::from(0)).to_string(),
            friends_id: friends_id,
            friends_name: String::from(friends_name).trim_end_matches(char::from(0)).to_string(),
            fake_player: fake_player > 0,
            is_hltv: is_hltv > 0,
            custom_files: custom_files,
            files_downloaded: files_downloaded,
            // Entity ID is **NOT** serialized by Valve so we don't need to read it here.
            entity_id: -1,
        })
    ))
);

// Note that this is not a byte-by-byte representation of the CS:GO demo
// (aside from the header). It's meant to be a slimmed down representation
// that extracts useful information out and presents it into a more useful
// manner.
#[derive(Debug)]
pub struct CsgoDemo {
    pub header: CsgoDemoHeader,
    pub game_start_tick: Option<i32>,
    pub rounds: Vec<CsgoDemoRound>,
    // These variables are only needed when parsing the demo file.
    string_tables: Vec<CsgoDemoStringTable>,
    player_info: HashMap<i32, CsgoDemoPlayerInfo>,
}

impl Default for CsgoDemo {
    fn default() -> Self {
        Self {
            header: CsgoDemoHeader::default(),
            game_start_tick: None,
            rounds: vec![],
            string_tables: vec![],
            player_info: HashMap::new(),
        }
    }
}

impl CsgoDemo {
    pub fn handle_game_event(&mut self, tick: i32, event: CsvcMsgGameEvent, desc: &csvc_msg_game_event_list::DescriptorT) -> Result<(), SquadOvError> {
        let event_name = desc.name();
        match event_name {
            "round_announce_match_start" => {
                log::debug!("csgo game start at: {} [{}]", tick, event_name);
                self.game_start_tick = Some(tick);
            },
            "player_death" => {
                if self.game_start_tick.is_some() {
                    // Need to do a game_start_tick check to ensure that
                    // we only record kills/deaths that occur post warm-up.
                    log::debug!("csgo player death at: {}", tick);
                }
            },
            "player_hurt" => {
                log::debug!("csgo player hurt at: {}", tick);
            },
            "player_spawn" => {
                log::debug!("csgo player spawn: {}", tick);
            },
            "round_start" => {
                log::debug!("csgo round start at: {}", tick);
            },
            "round_end" => {
                log::debug!("csgo round end at: {}", tick);
            },
            "bomb_planted" => {
                log::debug!("csgo bomb planted: {}", tick);
            },
            "bomb_defused" => {
                log::debug!("csgo bomb defused: {}", tick);
            },
            "bomb_exploded" => {
                log::debug!("csgo bomb exploded: {}", tick);
            },
            "round_freeze_end" => {
                log::debug!("csgo round freeze end: {}", tick);
            },
            "round_mvp" => {
                log::debug!("csgo round mvp: {}", tick);
            },
            _ => log::debug!("unhandled: {} at {}", event_name, tick),
        };
        Ok(())
    }

    fn generic_handle_string_table_change(&mut self, table: &CsgoDemoStringTable, num_entries: i32, data: &[u8]) -> Result<(), SquadOvError> {
        let is_user_info = table.name == "userinfo";
        // There may be more relevant string tables but for now we only care about userinfo ones.
        if !is_user_info {
            return Ok(());
        }

        let mut reader = BitReader::new(data);
        let encode_using_dictionaries = reader.read_bit()?;
        if encode_using_dictionaries {
            log::warn!("Skip dictionary-encoded string table.");
            return Ok(());
        }

        let mut last_entry = -1;

        // Forces a log2 on max_entries (on the table) to get
        // the number of entry bits.
        let num_entry_bits = crate::math::integer_log2(table.max_entries) as usize;

        let mut history : VecDeque<String> = VecDeque::new();
        for _i in 0..num_entries {
            last_entry = if reader.read_bit()? {
                last_entry + 1
            } else {
                reader.read_multibit::<u32>(num_entry_bits)? as i32
            };

            if last_entry < 0 || last_entry >= table.max_entries {
                log::warn!("Bad string table index: {} [{}]", last_entry, table.max_entries);
                return Ok(());
            }

            // Obtain entry/substring keys.
            let mut entry: String = String::new();

            if reader.read_bit()? {
                if reader.read_bit()? {
                    // We have a substring in addition to the entry.
                    let index = reader.read_multibit::<usize>(5)?;
                    if index >= history.len() {
                        log::warn!("Invalid history index: {} vs {}", index, history.len());
                        break;
                    }

                    let bytes_to_copy = reader.read_multibit::<usize>(SUBSTRING_BITS)?;
                    entry = String::from(&history.get(index).unwrap()[0..bytes_to_copy]);
                    let substr = reader.read_null_terminated_str()?; 
                    entry.push_str(&substr);
                } else {
                    // Just a single string entry.
                    entry = reader.read_null_terminated_str()?;
                }
            }

            // Now obtain user data.
            let mut user_data: Option<Vec<u8>> = None;
            if reader.read_bit()? {
                let bytes_to_read = if table.user_data_fixed_size {
                    table.user_data_size as usize
                } else {
                    reader.read_multibit::<usize>(MAX_USERDATA_BITS)?
                };

                user_data = Some(reader.read_raw(bytes_to_read * 8)?);
            }

            if is_user_info {
                if let Some(raw_user_data) = user_data {
                    let mut player_info = parse_csgo_demo_player_info(&raw_user_data)?.1;
                    player_info.entity_id = last_entry;
                    log::debug!("Player Info: {:?}", player_info);
                    self.player_info.insert(player_info.entity_id, player_info);
                }
            }

            if history.len() > 31 {
                history.pop_front();
            }

            history.push_back(entry);
        }

        Ok(())
    }

    pub fn handle_string_table_create(&mut self, data: CsvcMsgCreateStringTable) -> Result<(), SquadOvError> {
        let table = CsgoDemoStringTable{
            name: data.name.ok_or(SquadOvError::NotFound)?,
            max_entries: data.max_entries.ok_or(SquadOvError::NotFound)?,
            user_data_size: data.user_data_size.ok_or(SquadOvError::NotFound)?,
            user_data_size_bits: data.user_data_size_bits.ok_or(SquadOvError::NotFound)?,
            user_data_fixed_size: data.user_data_fixed_size.ok_or(SquadOvError::NotFound)?,
        };

        self.generic_handle_string_table_change(&table, data.num_entries.ok_or(SquadOvError::NotFound)?, &data.string_data.ok_or(SquadOvError::NotFound)?)?;
        self.string_tables.push(table);
        Ok(())
    }

    pub fn handle_string_table_update(&mut self, data: CsvcMsgUpdateStringTable) -> Result<(), SquadOvError> {
        // Check that we're modifying an appropriate table:
        //  1) The table index (id) must exist.
        //  2) We shouldn't be changing more entries than can possibly exist.
        
        // Would really prefer not to clone here but can't figure out how to get it to play nicely with the borrow checker.
        let src_table = self.string_tables.get(data.table_id.ok_or(SquadOvError::NotFound)? as usize).cloned();
        if let Some(table) = src_table {
            let num_changed = data.num_changed_entries.ok_or(SquadOvError::NotFound)?;
            if num_changed < table.max_entries {
                self.generic_handle_string_table_change(&table, num_changed, &data.string_data.ok_or(SquadOvError::NotFound)?)?;
            } else {
                log::warn!("String table changing more entries than max.");
            }
        } else {
            log::warn!("Table ID does not exist...skipping.");
        }

        Ok(())
    }
}