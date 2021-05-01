use bitvec::prelude::*;
use crate::SquadOvError;
use nom::number::complete::le_f32;

const CSGO_COORD_INTEGER_BITS_MP: usize = 11;
const CSGO_COORD_INTEGER_BITS: usize = 14;
const CSGO_COORD_FRACTIONAL_BITS: usize = 5;
const CSGO_COORD_FRACTIONAL_BITS_LOWPRECISION: usize = 3;
const CSGO_COORD_DENOM: usize = 1 << CSGO_COORD_FRACTIONAL_BITS;
const CSGO_COORD_DENOM_LOWPRECISION: usize = 1 << CSGO_COORD_FRACTIONAL_BITS_LOWPRECISION;
const CSGO_COORD_RESOLUTION: f32 = 1.0 / CSGO_COORD_DENOM as f32;
const CSGO_COORD_RESOLUTION_LOWPRECISION: f32 = 1.0 / CSGO_COORD_DENOM_LOWPRECISION as f32;

const NORMAL_FRACTIONAL_BITS: usize = 11;
const NORMAL_DENOMINATOR: usize = (1 << NORMAL_FRACTIONAL_BITS) - 1;
const NORMAL_RESOLUTION: f32 = 1.0 / NORMAL_DENOMINATOR as f32;

#[derive(PartialEq)]
#[repr(i32)]
pub enum CsgoCoordType {
    None,
    Integral,
    LowPrecision
}

pub struct BitReader<'a> {
    view: &'a BitSlice<Lsb0, u8>,
    ptr: usize,
}

fn zig_zag_decode32(n : u32) -> i32 {
    let n = n as i32;
    let bit: i32 = n & 1;
    (n >> 1) ^ -bit
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

    pub fn read_signed_multibit(&mut self, bits: usize) -> Result<i32, SquadOvError> {
        let og = self.read_multibit::<u32>(bits)? as i32;
        let shift = 32 - bits;
        Ok((og << shift) >> shift)
    }

    pub fn read_f32(&mut self) -> Result<f32, SquadOvError> {
        let bytes = self.read_raw(32)?;
        Ok(le_f32(bytes.as_slice())?.1)
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

    // EVERYTHING AFTER THIS ARE PROBABLY CSGO SPECIFIC?
    pub fn read_csgo_bit_coord(&mut self) -> Result<f32, SquadOvError> {
        let has_int = self.read_bit()?;
        let has_fract = self.read_bit()?;

        Ok(if has_int || has_fract {
            let sign = self.read_bit()?;

            let int_val = if has_int {
                self.read_multibit::<u32>(CSGO_COORD_INTEGER_BITS)? + 1
            } else {
                0
            } as f32;

            let fract_val = if has_fract {
                self.read_multibit::<u32>(CSGO_COORD_FRACTIONAL_BITS)?
            } else {
                0
            } as f32;

            let val = int_val + fract_val * CSGO_COORD_RESOLUTION;
            if sign {
                -val
            } else {
                val
            }
        } else {
            0.0
        })
    }

    pub fn read_csgo_bit_coord_mp(&mut self, typ: CsgoCoordType) -> Result<f32, SquadOvError> {
        let integral = typ == CsgoCoordType::Integral;
        let low_precision = typ == CsgoCoordType::LowPrecision;
        let in_bounds = self.read_bit()?;
        let mut is_signed: bool = false;
        
        let val = if integral {
            if self.read_bit()? {
                is_signed = self.read_bit()?;
                (if in_bounds {
                    self.read_multibit::<u32>(CSGO_COORD_INTEGER_BITS_MP)?
                } else {
                    self.read_multibit::<u32>(CSGO_COORD_INTEGER_BITS)? 
                } + 1) as f32
            } else {
                0.0
            }
        } else {
            let has_int = self.read_bit()?;
            is_signed = self.read_bit()?;

            let int_val = if has_int {
                (if in_bounds {
                    self.read_multibit::<u32>(CSGO_COORD_INTEGER_BITS_MP)?
                } else {
                    self.read_multibit::<u32>(CSGO_COORD_INTEGER_BITS)?
                }) + 1
            } else {
                0
            } as f32;

            let fract_val = self.read_multibit::<u32>(if low_precision { CSGO_COORD_FRACTIONAL_BITS_LOWPRECISION } else { CSGO_COORD_FRACTIONAL_BITS })? as f32;
            int_val + fract_val * if low_precision { CSGO_COORD_RESOLUTION_LOWPRECISION } else { CSGO_COORD_RESOLUTION }
        };

        Ok(if is_signed {
            -val
        } else {
            val
        })
    }

    pub fn read_csgo_bit_normal(&mut self) -> Result<f32, SquadOvError> {
        let sign = self.read_bit()?;
        let value = self.read_multibit::<u32>(NORMAL_FRACTIONAL_BITS)? as f32 * NORMAL_RESOLUTION;
        Ok(if sign {
            -value
        } else {
            value
        })
    }

    pub fn read_csgo_bit_cell_coord(&mut self, bits: usize, typ: CsgoCoordType) -> Result<f32, SquadOvError> {
        let integral = typ == CsgoCoordType::Integral;
        let low_precision = typ == CsgoCoordType::LowPrecision;

        Ok(if integral {
            self.read_multibit::<u32>(bits)? as f32
        } else {
            let int_val = self.read_multibit::<u32>(bits)? as f32;
            let fract_val = self.read_multibit::<u32>(if low_precision { CSGO_COORD_FRACTIONAL_BITS_LOWPRECISION } else { CSGO_COORD_FRACTIONAL_BITS })? as f32;
            int_val + fract_val * if low_precision { CSGO_COORD_RESOLUTION_LOWPRECISION } else { CSGO_COORD_RESOLUTION }
        })
    }

    pub fn read_var_u32(&mut self) -> Result<u32, SquadOvError> {
        // The format here is that we use 5 bytes to store up to a potentially 32 bit integer.
        // Each byte has 7 bits of numerical data (4x7 + 5 presumably) and the 8th bit is used to
        // indicate whether or not the next byte should be used.
        let mut res: u32 = 0;
        let mut count = 0;

        let mut use_next = true;
        while use_next && count < 5 {
            let byte = self.read_multibit::<u8>(8)? as u32;
            res |= (byte & 0x7F) << (7 * count);
            count += 1;
            use_next = byte & 0x80 != 0;
        }

        Ok(res)
    }

    pub fn read_var_i32(&mut self) -> Result<i32, SquadOvError> {
        Ok(zig_zag_decode32(self.read_var_u32()?))
    }

    pub fn read_var_u64(&mut self) -> Result<u64, SquadOvError> {
        Ok(0)
    }

    pub fn read_var_i64(&mut self) -> Result<i64, SquadOvError> {
        Ok(0)
    }

    pub fn read_var_ubits(&mut self) -> Result<u32, SquadOvError> {
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
