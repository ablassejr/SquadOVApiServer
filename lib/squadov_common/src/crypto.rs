use crate::SquadOvError;
use sha2::Sha256;
use hkdf::Hkdf;

pub fn hash_str_to_i64(input: &str) -> Result<i64, SquadOvError> {
    let hk = Hkdf::<Sha256>::new(None, input.as_bytes());
    let mut okm = [0u8; 8];
    hk.expand("SquadOV Poggers".as_bytes(), &mut okm).map_err(|x| { SquadOvError::InternalError(format!("Failed to expand OKM: {:?}", x)) })?;
    Ok(i64::from_le_bytes(okm))
}