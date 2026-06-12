/// Quick helper — prints the AES-256-GCM encrypted form of a secret.
/// Usage:
///   VAULT_ENCRYPTION_KEY=<hex-key> cargo run --bin encrypt-secret -- "your-secret-value"
fn main() {
    // Inline the crypto logic so this binary has zero extra deps.
    use aes_gcm::{
        aead::{Aead, AeadCore, KeyInit, OsRng},
        Aes256Gcm, Key, Nonce,
    };
    use base64::{engine::general_purpose::STANDARD, Engine};

    let key_hex = std::env::var("VAULT_ENCRYPTION_KEY")
        .expect("VAULT_ENCRYPTION_KEY env var must be set");

    let plaintext = std::env::args()
        .nth(1)
        .expect("Usage: encrypt-secret <value-to-encrypt>");

    let key_bytes = hex::decode(&key_hex).expect("VAULT_ENCRYPTION_KEY must be valid hex");
    assert_eq!(key_bytes.len(), 32, "VAULT_ENCRYPTION_KEY must be 32 bytes (64 hex chars)");

    let key    = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce  = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes()).expect("encryption failed");

    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);

    println!("{}", STANDARD.encode(combined));
}
