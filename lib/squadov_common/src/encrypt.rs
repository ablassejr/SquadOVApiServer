use crate::SquadOvError;
use std::string::ToString;
use sha2::{Sha256, Digest};
use openssl::{
    symm::{encrypt_aead, decrypt_aead, Cipher},
    rand::rand_bytes,
};

pub struct AESEncryptRequest {
    pub data: Vec<u8>,
    pub aad: Vec<u8>,
}

pub struct AESEncryptToken {
    pub data: Vec<u8>,
    pub iv: Vec<u8>,
    pub aad: Vec<u8>,
    pub tag: Vec<u8>,
}

pub fn squadov_encrypt(req: AESEncryptRequest, key: &str) -> Result<AESEncryptToken, SquadOvError> {
    let mut iv = [0; 256];
    rand_bytes(&mut iv)?;

    let mut tag: Vec<u8> = vec![0; 16];
    let cipher = Cipher::aes_256_gcm();
    let data = encrypt_aead(
        cipher,
        &Sha256::digest(key.as_bytes()).as_slice(),
        Some(&iv),
        &req.aad,
        &req.data,
        &mut tag
    )?;

    Ok(AESEncryptToken{
        data,
        iv: iv.to_vec(),
        aad: req.aad,
        tag: tag.to_vec(),
    })
}

pub fn squadov_decrypt(token: AESEncryptToken, key: &str) -> Result<AESEncryptRequest, SquadOvError> {
    let cipher = Cipher::aes_256_gcm();
    let unencrypted = decrypt_aead(
        cipher,
        &Sha256::digest(key.as_bytes()).as_slice(),
        Some(&token.iv),
        &token.aad,
        &token.data,
        &token.tag,
    )?;

    Ok(AESEncryptRequest{
        data: unencrypted,
        aad: token.aad,
    })
}

impl ToString for AESEncryptToken {
    fn to_string(&self) -> String {
        format!(
            "{}.{}.{}.{}",
            base64::encode(&self.data),
            base64::encode(&self.iv),
            base64::encode(&self.aad),
            base64::encode(&self.tag),
        )
    }
}

impl AESEncryptToken {
    pub fn from_string(data: &str) -> Result<Self, SquadOvError> {
        let parts: Vec<&str> = data.split('.').collect();
        if parts.len() != 4 {
            return Err(SquadOvError::BadRequest);
        }

        Ok(AESEncryptToken{
            data: base64::decode(parts[0])?,
            iv: base64::decode(parts[1])?,
            aad: base64::decode(parts[2])?,
            tag: base64::decode(parts[3])?,
        })
    }

}