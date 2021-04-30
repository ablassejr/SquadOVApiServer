use bitvec::prelude::*;
use crate::SquadOvError;

pub struct BitReader<'a> {
    view: &'a BitSlice<Lsb0, u8>,
    ptr: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a[u8]) -> Self {
        Self {
            view: data.view_bits::<Lsb0>(),
            ptr: 0,
        }
    }

    pub fn loc_bits(&self) -> usize {
        self.ptr
    }

    pub fn loc_bytes(&self) -> usize {
        return self.loc_bits() / 8
    }

    pub fn advance_bits(&mut self, bits: usize) {
        self.ptr += bits;
    }

    pub fn advance_bytes(&mut self, bytes: usize) {
        self.advance_bits(bytes * 8);
    }

    pub fn read_bit(&mut self) -> Result<bool, SquadOvError> {
        let res = self.view.get(self.ptr).ok_or(SquadOvError::BadRequest)?;
        self.advance_bits(1);
        Ok(*res)
    }

    // Is there a way to do this without a copy?
    fn get_byte_array(&mut self, start: usize, end: usize) -> Result<Vec<u8>, SquadOvError> {
        Ok(
            self.view.get(start..end)
                .ok_or(SquadOvError::BadRequest)?
                .chunks_exact(8).map(|x| {
                    x.load_le::<u8>()
                }).collect()
        )
    }

    pub fn read_raw(&mut self, bits: usize) -> Result<Vec<u8>, SquadOvError> {
        let res: Vec<u8> = self.get_byte_array(self.ptr, self.ptr+bits)?;
        self.advance_bits(bits);
        Ok(res)
    }

    pub fn read_multibit<T : BitMemory>(&mut self, bits: usize) -> Result<T, SquadOvError> {
        let res = self.view.get(self.ptr..self.ptr+bits).ok_or(SquadOvError::BadRequest)?.load_le::<T>();
        self.advance_bits(bits);
        Ok(res)
    }

    pub fn read_null_terminated_str(&mut self) -> Result<String, SquadOvError> {
        let start = self.ptr;
        let mut end = start;

        loop {
            let ch: char = self.read_multibit::<u8>(8)? as char;
            if ch == '\0' {
                break;
            }

            end = self.ptr;
        }
        
        let raw_data = self.get_byte_array(start, end)?;
        Ok(String::from_utf8(raw_data)?)
    }

    pub fn read_aligned_bytes(&mut self, bytes: usize) -> Result<&[u8], SquadOvError> {
        let bits = bytes * 8;
        let res = self.view.get(self.ptr..self.ptr+bits).ok_or(SquadOvError::BadRequest)?.as_raw_slice();
        self.advance_bits(bits);
        Ok(res)
    }

    pub fn read_var_i32(&mut self) -> Result<i32, SquadOvError> {
        // The format here is that we use 5 bytes to store up to a potentially 32 bit integer.
        // Each byte has 7 bits of numerical data (4x7 + 5 presumably) and the 8th bit is used to
        // indicate whether or not the next byte should be used.
        let mut res: i32 = 0;
        let mut count = 0;

        let mut use_next = true;
        while use_next && count < 5 {
            let byte = self.read_multibit::<u8>(8)? as i32;
            res |= (byte & 0x7F) << (7 * count);
            count += 1;
            use_next = byte & 0x80 != 0;
        }

        Ok(res)
    }

    pub fn read_var_u32(&mut self) -> Result<u32, SquadOvError> {
        // The initial section has 6 bits. In order of MSB -> LSB:
        // - 1 bit to indicator whether to read an extra 8 bits of data
        //   and use those as the MSBs.
        // - 1 bit to indicator whether to read an extra 4 bits of data
        //   and use those as the MSBs.
        // - 4 bits to always use as the 4 LSBs.
        // If BOTH indicators are set, then we read an extra 28 bits of data instead!
        // Why? Don't question it.
        let ret = self.read_multibit::<u32>(6)?;
        Ok((ret & 15) | (match ret & (16 | 32) {
            16 => self.read_multibit::<u32>(4)?,
            32 => self.read_multibit::<u32>(8)?,
            48 => self.read_multibit::<u32>(28)?,
            _ => 0
        } << 4))
    }
}
