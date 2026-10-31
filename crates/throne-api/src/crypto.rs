use chacha20poly1305::{
    aead::{AeadInPlace, KeyInit},
    ChaCha20Poly1305, Nonce, Tag,
};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

pub const KEY_SIZE: usize = 32;
pub const RANDOM_SIZE: usize = 16;
pub const TAG_SIZE: usize = 16;
pub const FRAME_HEADER_SIZE: usize = 4;
pub const PROTOCOL_VERSION: u8 = 0x01;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid key size: expected {0}, got {1}")]
    InvalidKeySize(usize, usize),

    #[error("Decryption / authentication failed")]
    AeadFailure,

    #[error("Frame length header mismatch: header={0}, received={1}")]
    FrameMismatch(u32, usize),
}

pub struct SessionKeys {
    pub c2s: [u8; 32],
    pub s2c: [u8; 32],
}

pub fn generate_random_bytes(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

pub fn derive_keys(key: &[u8], server_random: &[u8], client_random: &[u8]) -> Result<SessionKeys, CryptoError> {
    if key.len() != KEY_SIZE {
        return Err(CryptoError::InvalidKeySize(KEY_SIZE, key.len()));
    }

    let mut randoms = Vec::with_capacity(32);
    randoms.extend_from_slice(server_random);
    randoms.extend_from_slice(client_random);

    // c2s
    let mut mac_c2s = <HmacSha256 as Mac>::new_from_slice(key).map_err(|_| CryptoError::AeadFailure)?;
    mac_c2s.update(b"throne-api/1 c2s");
    mac_c2s.update(&randoms);
    let c2s_res = mac_c2s.finalize().into_bytes();

    // s2c
    let mut mac_s2c = <HmacSha256 as Mac>::new_from_slice(key).map_err(|_| CryptoError::AeadFailure)?;
    mac_s2c.update(b"throne-api/1 s2c");
    mac_s2c.update(&randoms);
    let s2c_res = mac_s2c.finalize().into_bytes();

    let mut c2s = [0u8; 32];
    let mut s2c = [0u8; 32];
    c2s.copy_from_slice(&c2s_res[..32]);
    s2c.copy_from_slice(&s2c_res[..32]);

    Ok(SessionKeys { c2s, s2c })
}

pub struct FrameCodec {
    cipher: ChaCha20Poly1305,
    counter: u64,
}

impl FrameCodec {
    pub fn new(key: &[u8; 32]) -> Self {
        Self {
            cipher: ChaCha20Poly1305::new(key.into()),
            counter: 0,
        }
    }

    pub fn seal(&mut self, plaintext: &[u8]) -> Vec<u8> {
        let sealed_len = (plaintext.len() + TAG_SIZE) as u32;
        let mut header = [0u8; 4];
        header.copy_from_slice(&sealed_len.to_be_bytes());

        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..12].copy_from_slice(&self.counter.to_be_bytes());
        self.counter = self.counter.wrapping_add(1);

        let nonce = Nonce::from_slice(&nonce_bytes);
        let mut buffer = plaintext.to_vec();
        let tag = self
            .cipher
            .encrypt_in_place_detached(nonce, &header, &mut buffer)
            .expect("ChaCha20Poly1305 in place detached encryption");

        let mut frame = Vec::with_capacity(4 + buffer.len() + TAG_SIZE);
        frame.extend_from_slice(&header);
        frame.extend_from_slice(&buffer);
        frame.extend_from_slice(&tag);
        frame
    }

    pub fn open(&mut self, header: &[u8; 4], sealed: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let expected_len = u32::from_be_bytes(*header) as usize;
        if expected_len != sealed.len() || sealed.len() < TAG_SIZE {
            return Err(CryptoError::FrameMismatch(expected_len as u32, sealed.len()));
        }

        let cipher_len = sealed.len() - TAG_SIZE;
        let (ciphertext, tag_slice) = sealed.split_at(cipher_len);

        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..12].copy_from_slice(&self.counter.to_be_bytes());
        self.counter = self.counter.wrapping_add(1);

        let nonce = Nonce::from_slice(&nonce_bytes);
        let tag = Tag::from_slice(tag_slice);

        let mut buffer = ciphertext.to_vec();
        self.cipher
            .decrypt_in_place_detached(nonce, header, &mut buffer, tag)
            .map_err(|_| CryptoError::AeadFailure)?;

        Ok(buffer)
    }
}
