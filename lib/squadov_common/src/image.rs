use crate::SquadOvError;
use image::{
    io::Reader,
    ImageOutputFormat,
};
use std::io::Cursor;

pub fn process_raw_image_buffer_into_standard_jpeg(data: &[u8]) -> Result<Vec<u8>, SquadOvError> {
    let img = Reader::new(Cursor::new(data)).with_guessed_format()?.decode()?;
    let mut bytes: Vec<u8> = Vec::new();
    img.write_to(&mut bytes, ImageOutputFormat::Jpeg(80))?;
    Ok(bytes)
}