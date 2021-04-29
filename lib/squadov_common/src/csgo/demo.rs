use nom::number::streaming::{
    le_i32,
    le_f32,
    le_u8,
};
use chrono::{DateTime, Utc, NaiveDateTime};

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
    pub tick: DateTime<Utc>,
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
            tick: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(tick as i64, 0), Utc),
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

// Note that this is not a byte-by-byte representation of the CS:GO demo
// (aside from the header). It's meant to be a slimmed down representation
// that extracts useful information out and presents it into a more useful
// manner.
#[derive(Debug)]
pub struct CsgoDemo {
    pub header: CsgoDemoHeader,
}

impl Default for CsgoDemo {
    fn default() -> Self {
        Self {
            header: CsgoDemoHeader::default(),
        }
    }
}