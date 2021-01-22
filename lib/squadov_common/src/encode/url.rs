use crate::SquadOvError;
use percent_encoding::{CONTROLS, AsciiSet};

const URL_ENCODE_ASCII_SET : &AsciiSet = &CONTROLS
    .add(b'!')
    .add(b'*')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b';')
    .add(b':')
    .add(b'@')
    .add(b'&')
    .add(b'=')
    .add(b'+')
    .add(b'$')
    .add(b',')
    .add(b'/')
    .add(b'?')
    .add(b'#')
    .add(b'[')
    .add(b']');

pub fn url_encode(input: &str) -> String {
    percent_encoding::percent_encode(input.as_bytes(), &URL_ENCODE_ASCII_SET).to_string()
}

pub fn url_decode(input: &str) -> Result<String, SquadOvError> {
    Ok(percent_encoding::percent_decode(input.as_bytes()).decode_utf8()?.into_owned())
}