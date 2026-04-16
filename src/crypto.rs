use std::sync::OnceLock;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;

#[derive(Debug)]
pub struct CryptoError(pub String);

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "crypto error: {}", self.0)
    }
}

impl std::error::Error for CryptoError {}

static KEY: OnceLock<Result<[u8; 32], String>> = OnceLock::new();

fn get_key() -> Result<[u8; 32], CryptoError> {
    KEY.get_or_init(|| {
        let hex_key = std::env::var("CREDENTIAL_ENCRYPTION_KEY")
            .map_err(|_| "CREDENTIAL_ENCRYPTION_KEY not set".to_string())?;
        let bytes = hex::decode(&hex_key)
            .map_err(|_| "CREDENTIAL_ENCRYPTION_KEY is not valid hex".to_string())?;
        if bytes.len() != 32 {
            return Err("CREDENTIAL_ENCRYPTION_KEY must be 32 bytes (64 hex chars)".to_string());
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(key)
    })
    .clone()
    .map_err(CryptoError)
}

/// Encrypts `plaintext` with AES-256-GCM.
/// Returns base64(12-byte-nonce || ciphertext).
pub fn encrypt(plaintext: &str) -> Result<String, CryptoError> {
    let key = get_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| CryptoError(e.to_string()))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| CryptoError(e.to_string()))?;

    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(BASE64.encode(&combined))
}

/// Decrypts a value produced by `encrypt`.
pub fn decrypt(ciphertext_b64: &str) -> Result<String, CryptoError> {
    let key = get_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| CryptoError(e.to_string()))?;

    let combined = BASE64
        .decode(ciphertext_b64)
        .map_err(|_| CryptoError("invalid base64".into()))?;

    if combined.len() < 12 {
        return Err(CryptoError("ciphertext too short".into()));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext_bytes = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError(e.to_string()))?;

    String::from_utf8(plaintext_bytes)
        .map_err(|_| CryptoError("decrypted data is not valid UTF-8".into()))
}
