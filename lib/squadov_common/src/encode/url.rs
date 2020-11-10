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