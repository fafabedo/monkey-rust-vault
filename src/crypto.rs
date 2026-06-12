use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};

use crate::error::{VaultError, VaultResult};

/// Encrypt plaintext with AES-256-GCM. Returns base64(nonce || ciphertext).
pub fn encrypt(plaintext: &str, key_hex: &str) -> VaultResult<String> {
    let key_bytes = hex::decode(key_hex)
        .map_err(|e| VaultError::Crypto(format!("Invalid key hex: {e}")))?;
    if key_bytes.len() != 32 {
        return Err(VaultError::Crypto("Encryption key must be exactly 32 bytes".into()));
    }

    let key    = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce  = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| VaultError::Crypto(e.to_string()))?;

    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(STANDARD.encode(combined))
}

/// Decrypt a value produced by `encrypt`. Expects base64(nonce || ciphertext).
pub fn decrypt(encoded: &str, key_hex: &str) -> VaultResult<String> {
    let key_bytes = hex::decode(key_hex)
        .map_err(|e| VaultError::Crypto(format!("Invalid key hex: {e}")))?;
    if key_bytes.len() != 32 {
        return Err(VaultError::Crypto("Encryption key must be exactly 32 bytes".into()));
    }

    let combined = STANDARD
        .decode(encoded)
        .map_err(|e| VaultError::Crypto(format!("Base64 decode failed: {e}")))?;
    if combined.len() < 12 {
        return Err(VaultError::Crypto("Encrypted payload too short".into()));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let key    = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce  = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| VaultError::Crypto(format!("Decryption failed: {e}")))?;

    String::from_utf8(plaintext)
        .map_err(|e| VaultError::Crypto(format!("UTF-8 decode failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn roundtrip() {
        let plaintext = "super-secret-token";
        let enc = encrypt(plaintext, KEY).unwrap();
        let dec = decrypt(&enc, KEY).unwrap();
        assert_eq!(dec, plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let enc = encrypt("hello", KEY).unwrap();
        let bad_key = "1111111111111111111111111111111111111111111111111111111111111111";
        assert!(decrypt(&enc, bad_key).is_err());
    }
}
