pub mod keys;

use rand::{rngs::OsRng, RngCore};
use ring::aead::{self, Nonce, UnboundKey, AES_256_GCM};
use sha2::{Sha256, Digest};
use thiserror::Error;

pub const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
pub const IDENTITY_KEY_LEN: usize = 32;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Noise protocol error: {0}")]
    Noise(String),
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Invalid key length")]
    InvalidKeyLength,
}

pub type Result<T> = std::result::Result<T, CryptoError>;

/// Generate cryptographically secure random bytes
pub fn secure_random(bytes: &mut [u8]) {
    OsRng.fill_bytes(bytes);
}

/// Derive a session ID from public key material
pub fn derive_session_id(public_key: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(public_key);
    let result = hasher.finalize();
    base64::encode(&result[..16]) // First 128 bits
}

/// Encrypt with AES-256-GCM (for data at rest)
pub fn encrypt_data(key: &[u8; 32], plaintext: &[u8], associated_data: &[u8]) -> Result<Vec<u8>> {
    let nonce_bytes = {
        let mut buf = [0u8; 12];
        secure_random(&mut buf);
        buf
    };
    
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let unbound_key = UnboundKey::new(&AES_256_GCM, key)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    
    let mut in_out = plaintext.to_vec();
    let tag = aead::seal_in_place_separate_tag(
        &unbound_key,
        nonce,
        aead::Aad::from(associated_data),
        &mut in_out,
        16, // tag length
    ).map_err(|_| CryptoError::EncryptionFailed)?;
    
    let mut result = Vec::with_capacity(12 + in_out.len() + 16);
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&in_out);
    result.extend_from_slice(tag.as_ref());
    
    Ok(result)
}

/// Decrypt with AES-256-GCM
pub fn decrypt_data(key: &[u8; 32], ciphertext: &[u8], associated_data: &[u8]) -> Result<Vec<u8>> {
    if ciphertext.len() < 28 { // nonce(12) + min data + tag(16)
        return Err(CryptoError::DecryptionFailed);
    }
    
    let (nonce_bytes, rest) = ciphertext.split_at(12);
    let (encrypted, tag_bytes) = rest.split_at(rest.len() - 16);
    
    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
        .map_err(|_| CryptoError::DecryptionFailed)?;
    
    let unbound_key = UnboundKey::new(&AES_256_GCM, key)
        .map_err(|_| CryptoError::DecryptionFailed)?;
    
    let mut in_out = encrypted.to_vec();
    in_out.extend_from_slice(tag_bytes);
    
    let plaintext = aead::open_in_place(
        &unbound_key,
        nonce,
        aead::Aad::from(associated_data),
        0,
        &mut in_out,
    ).map_err(|_| CryptoError::DecryptionFailed)?;
    
    Ok(plaintext.to_vec())
}