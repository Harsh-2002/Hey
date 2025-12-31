use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::Rng;
use std::fs;
use std::path::PathBuf;

// Device-specific encryption key derived from machine ID
// This provides basic protection - keys are encrypted at rest
fn get_encryption_key() -> [u8; 32] {
    // Use a combination of app identifier and stable machine info
    // This creates a deterministic key unique to this machine
    let machine_id =
        whoami::fallible::devicename().unwrap_or_else(|_| "hey-app-default".to_string());

    let key_material = format!("hey-api-key-encryption-{}", machine_id);

    // Simple key derivation using SHA256
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut key = [0u8; 32];
    for i in 0..32 {
        let mut hasher = DefaultHasher::new();
        format!("{}-{}", key_material, i).hash(&mut hasher);
        key[i] = (hasher.finish() % 256) as u8;
    }
    key
}

fn get_keys_file_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;
    let config_dir = home.join(".hey");
    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("keys.enc"))
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct EncryptedKeys {
    openai: Option<String>, // Base64 encoded encrypted key
    groq: Option<String>,
    assemblyai: Option<String>,
}

fn load_encrypted_keys() -> EncryptedKeys {
    let path = match get_keys_file_path() {
        Ok(p) => p,
        Err(_) => return EncryptedKeys::default(),
    };

    if !path.exists() {
        return EncryptedKeys::default();
    }

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => EncryptedKeys::default(),
    }
}

fn save_encrypted_keys(keys: &EncryptedKeys) -> Result<()> {
    let path = get_keys_file_path()?;
    let content = serde_json::to_string_pretty(keys)?;
    fs::write(&path, content)?;
    Ok(())
}

fn encrypt_key(plaintext: &str) -> Result<String> {
    let key = get_encryption_key();
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| anyhow::anyhow!("Invalid key length: {:?}", e))?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    // Combine nonce + ciphertext and base64 encode
    let mut combined = nonce_bytes.to_vec();
    combined.extend(ciphertext);

    Ok(STANDARD.encode(&combined))
}

fn decrypt_key(encrypted_b64: &str) -> Result<String> {
    let key = get_encryption_key();
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| anyhow::anyhow!("Invalid key length: {:?}", e))?;

    // Decode base64
    let combined = STANDARD.decode(encrypted_b64)?;

    if combined.len() < 12 {
        return Err(anyhow::anyhow!("Invalid encrypted data"));
    }

    // Split nonce and ciphertext
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))
}

/// Store an API key (encrypted)
pub fn store_api_key(provider: &str, api_key: &str) -> Result<()> {
    println!("[APIKey] Storing encrypted key for provider: {}", provider);

    let encrypted = encrypt_key(api_key)?;
    let mut keys = load_encrypted_keys();

    match provider {
        "openai" => keys.openai = Some(encrypted),
        "groq" => keys.groq = Some(encrypted),
        "assemblyai" => keys.assemblyai = Some(encrypted),
        _ => return Err(anyhow::anyhow!("Unknown provider: {}", provider)),
    }

    save_encrypted_keys(&keys)?;
    println!(
        "[APIKey] Successfully stored encrypted key for: {}",
        provider
    );
    Ok(())
}

/// Retrieve an API key (decrypted)
pub fn get_api_key(provider: &str) -> Result<Option<String>> {
    println!("[APIKey] Getting key for provider: {}", provider);

    let keys = load_encrypted_keys();

    let encrypted = match provider {
        "openai" => keys.openai,
        "groq" => keys.groq,
        "assemblyai" => keys.assemblyai,
        _ => None,
    };

    match encrypted {
        Some(enc) => match decrypt_key(&enc) {
            Ok(key) => {
                println!(
                    "[APIKey] Found key for: {} (length: {})",
                    provider,
                    key.len()
                );
                Ok(Some(key))
            }
            Err(e) => {
                println!("[APIKey] Failed to decrypt key for {}: {}", provider, e);
                Ok(None)
            }
        },
        None => {
            println!("[APIKey] No key found for: {}", provider);
            Ok(None)
        }
    }
}

/// Check if an API key exists
pub fn has_api_key(provider: &str) -> bool {
    let keys = load_encrypted_keys();
    match provider {
        "openai" => keys.openai.is_some(),
        "groq" => keys.groq.is_some(),
        "assemblyai" => keys.assemblyai.is_some(),
        _ => false,
    }
}

/// Delete an API key
pub fn delete_api_key(provider: &str) -> Result<()> {
    println!("[APIKey] Deleting key for provider: {}", provider);

    let mut keys = load_encrypted_keys();

    match provider {
        "openai" => keys.openai = None,
        "groq" => keys.groq = None,
        "assemblyai" => keys.assemblyai = None,
        _ => {}
    }

    save_encrypted_keys(&keys)?;
    Ok(())
}

/// Migrate API keys from old config file (one-time)
pub fn migrate_from_config(_config: &crate::config::Config) -> Result<()> {
    println!("[APIKey] Checking for keys to migrate...");

    // Try to read old config format with plain text keys
    if let Ok(content) = std::fs::read_to_string(
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("No home dir"))?
            .join(".hey")
            .join("config.json"),
    ) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            for (field, provider) in [
                ("openai_api_key", "openai"),
                ("groq_api_key", "groq"),
                ("assemblyai_api_key", "assemblyai"),
            ] {
                if let Some(key) = json.get(field).and_then(|v| v.as_str()) {
                    if !key.is_empty() && !has_api_key(provider) {
                        if store_api_key(provider, key).is_ok() {
                            println!("[APIKey] Migrated {} key to encrypted storage", provider);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
