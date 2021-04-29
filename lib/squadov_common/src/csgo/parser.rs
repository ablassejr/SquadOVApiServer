use crate::SquadOvError;
use super::demo::{
    CsgoDemo,
    CsgoDemoHeader,
    CsgoDemoCmdHeader,
    CsgoDemoCmdMessage,
    parse_csgo_demo_header,
    parse_csgo_demo_cmd_header,
    parse_csgo_demo_cmd_info,
};
use std::path::Path;
use std::io::Read;
use nom::number::complete::{
    le_i32,
    le_i8
};
use crate::proto::csgo::{
    NetMessages,
    SvcMessages,
};

const DEMO_HEADER_ID: &str = "HL2DEMO";
const DEMO_PROTOCOL: i32 = 4;
const NET_MAX_PAYLOAD: i32 = 262144 - 4;
const CSGO_DEMO_HEADER_SIZE: usize = 1072;
const CSGO_DEMO_CMD_HEADER_SIZE: usize = 6;
const CSGO_DEMO_CMD_INFO_SIZE: usize = 152;

struct CsgoDemoRawFile {
    buffer: Vec<u8>,
    ptr: usize,
    demo: CsgoDemo,
}

impl CsgoDemoRawFile {
    fn new(buffer: Vec<u8>) -> Self {
        Self {
            buffer,
            ptr: 0,
            demo: CsgoDemo::default(),
        }
    }

    fn get_current_sized_slice(&mut self, sz: usize) -> &[u8] {
        self.ptr += sz;
        &self.buffer[self.ptr-sz..self.ptr]
    }

    fn read_var_i32(&mut self) -> Result<i32, SquadOvError> {
        // The format here is that we use 5 bytes to store up to a potentially 32 bit integer.
        // Each byte has 7 bits of numerical data (4x7 + 5 presumably) and the 8th bit is used to
        // indicate whether or not the next byte should be used.
        let mut res: i32 = 0;
        let mut count = 0;

        let mut use_next = true;
        while use_next && count < 5 {
            let byte = le_i8(self.get_current_sized_slice(1))?.1 as i32;
            res |= (byte & 0x7F) << (7 * count);
            count += 1;
            use_next = byte & 0x80 != 0;
        }

        Ok(res)
    }

    // This function forces us to read the header.
    // So it'll reset the ptr to 0 even if we already technically
    // read past it already.
    fn read_header(&mut self) -> Result<&'_ CsgoDemoHeader, SquadOvError> {
        self.ptr = 0;
        let header = parse_csgo_demo_header(self.get_current_sized_slice(CSGO_DEMO_HEADER_SIZE))?.1;
        self.demo.header = header;
        Ok(&self.demo.header)
    }

    fn read_command_header(&mut self) -> Result<CsgoDemoCmdHeader, SquadOvError> {
        let hdr = parse_csgo_demo_cmd_header(self.get_current_sized_slice(CSGO_DEMO_CMD_HEADER_SIZE))?.1;
        Ok(hdr)
    }

    fn read_demo_packet(&mut self) -> Result<(), SquadOvError> {
        let _info = parse_csgo_demo_cmd_info(self.get_current_sized_slice(CSGO_DEMO_CMD_INFO_SIZE))?.1;
        self.ptr += 8; // Skip over 2 int32_t worths of data (dummy info).

        let payload_len = le_i32(self.get_current_sized_slice(4))?.1;
        if payload_len > NET_MAX_PAYLOAD {
            return Err(SquadOvError::InternalError(format!("CS:GO Demo Payload greater than NET_MAX_PAYLOAD: {}", payload_len)));
        }

        let target_ptr = self.ptr + payload_len as usize;
        while self.ptr < target_ptr {
            let cmd = self.read_var_i32()?;
            let size = self.read_var_i32()? as usize;
            
            log::debug!("cmd {} - {}", cmd, size);
            if self.ptr + size > target_ptr {
                return Err(SquadOvError::InternalError(format!("CS:GO Demo Payload failed to parse for cmd [{}] with size [{}]", cmd, size)));
            }

            // Whether or not we do anything with the buffer is separate from the fact
            // that we absolutely must increment the pointer for proper reading...
            // In fact i don't think we particularly care about any of these so...leaving
            // these here as a placeholder for the future.
            let _raw_buffer = self.get_current_sized_slice(size as usize);
            if let Some(_ncmd) = NetMessages::from_i32(cmd) {
            } else if let Some(_scmd) = SvcMessages::from_i32(cmd) {
            }
        }

        Ok(())
    }

    fn read_body(&mut self) -> Result<(), SquadOvError> {
        loop {
            let cmd_header = self.read_command_header()?;
            match cmd_header.cmd {
                CsgoDemoCmdMessage::Stop => {
                    log::debug!("CSGO Demo Stop");
                    break;
                },
                CsgoDemoCmdMessage::SignOn | CsgoDemoCmdMessage::Packet => {
                    log::debug!("CSGO Demo Sign On/Packet");
                    self.read_demo_packet()?;
                },
                CsgoDemoCmdMessage::ConsoleCmd => {
                    log::debug!("CSGO Demo Console Command");
                },
                CsgoDemoCmdMessage::DataTables => {
                    log::debug!("CSGO Demo Data Tables");
                },
                CsgoDemoCmdMessage::StringTables => {
                    log::debug!("CSGO Demo String Tables");
                },
                CsgoDemoCmdMessage::UserCmd => {
                    log::debug!("CSGO Demo User Cmd");
                },
                _ => {
                    log::debug!("CSGO Demo OTHER CMD");
                },
            }
        }
        Ok(())
    }

    fn finish(self) -> CsgoDemo {
        self.demo
    }
}


pub struct CsgoDemoParser {}

// Largely based off Valve's CSGO demo parser:
// https://github.com/ValveSoftware/csgo-demoinfo
impl CsgoDemoParser {
    fn from_file(file: &mut std::fs::File) -> Result<CsgoDemo, SquadOvError> {
        let mut raw_buffer: Vec<u8> = vec![];
        file.read_to_end(&mut raw_buffer)?;

        let mut demo_file = CsgoDemoRawFile::new(raw_buffer);
        let header = demo_file.read_header()?;
        // Verify that we actually read a valid CS:GO demo.
        if header.demo_filestamp != DEMO_HEADER_ID {
            return Err(SquadOvError::InternalError("CSGO Demo filestamp mismatch.".to_string()));
        }

        if header.demo_protocol != DEMO_PROTOCOL {
            return Err(SquadOvError::InternalError("CSGO Demo protocol mismatch.".to_string()));
        }

        demo_file.read_body()?;
        Ok(demo_file.finish())
    }

    pub fn from_path(path: &Path) -> Result<CsgoDemo, SquadOvError> {
        let mut file = std::fs::File::open(path)?;
        Ok(CsgoDemoParser::from_file(&mut file)?)
    }
}