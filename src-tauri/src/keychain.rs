use anyhow::Result;
use std::fs;

const SERVICE: &str = "io.firstfinger.hey";

/// Store an API key in the native OS keychain
pub fn store_api_key(provider: &str, api_key: &str) -> Result<()> {
    println!("[Keyring] Storing key for provider: {}", provider);

    let entry = keyring::Entry::new(SERVICE, provider)?;
    entry.set_password(api_key)?;

    println!("[Keyring] Successfully stored key for: {}", provider);
    Ok(())
}

/// Retrieve an API key from the native OS keychain
pub fn get_api_key(provider: &str) -> Result<Option<String>> {
    println!("[Keyring] Getting key for provider: {}", provider);

    let entry = keyring::Entry::new(SERVICE, provider)?;
    match entry.get_password() {
        Ok(password) => {
            println!(
                "[Keyring] Found key for: {} (length: {})",
                provider,
                password.len()
            );
            Ok(Some(password))
        }
        Err(keyring::Error::NoEntry) => {
            println!("[Keyring] No key found for: {}", provider);
            Ok(None)
        }
        Err(e) => {
            println!("[Keyring] Error getting key for {}: {}", provider, e);
            Err(anyhow::anyhow!("Keyring error: {}", e))
        }
    }
}

/// Check if an API key exists in the native OS keychain
pub fn has_api_key(provider: &str) -> bool {
    if let Ok(entry) = keyring::Entry::new(SERVICE, provider) {
        entry.get_password().is_ok()
    } else {
        false
    }
}

/// Delete an API key from the native OS keychain
pub fn delete_api_key(provider: &str) -> Result<()> {
    println!("[Keyring] Deleting key for provider: {}", provider);

    let entry = keyring::Entry::new(SERVICE, provider)?;
    match entry.delete_credential() {
        Ok(_) => {
            println!("[Keyring] Deleted key for: {}", provider);
            Ok(())
        }
        Err(keyring::Error::NoEntry) => {
            // Already doesn't exist, that's fine
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Failed to delete key: {}", e)),
    }
}

/// Migrate API keys from old encrypted file storage to native keychain
pub fn migrate_from_config(_config: &crate::config::Config) -> Result<()> {
    println!("[Keyring] Checking for keys to migrate from old storage...");

    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;
    let old_keys_file = home.join(".hey").join("keys.enc");

    if !old_keys_file.exists() {
        println!("[Keyring] No old keys file found, nothing to migrate");
        return Ok(());
    }

    // Try to read and decrypt old keys
    if let Ok(content) = fs::read_to_string(&old_keys_file) {
        if let Ok(old_keys) = serde_json::from_str::<OldEncryptedKeys>(&content) {
            for (encrypted, provider) in [
                (old_keys.openai, "openai"),
                (old_keys.groq, "groq"),
                (old_keys.assemblyai, "assemblyai"),
            ] {
                if let Some(enc) = encrypted {
                    // Skip if already migrated
                    if has_api_key(provider) {
                        println!("[Keyring] {} already migrated, skipping", provider);
                        continue;
                    }

                    // Try to decrypt old key
                    if let Ok(decrypted) = decrypt_old_key(&enc) {
                        if store_api_key(provider, &decrypted).is_ok() {
                            println!("[Keyring] Migrated {} key to native keychain", provider);
                        }
                    }
                }
            }

            // Rename old file to indicate migration complete
            let backup_path = home.join(".hey").join("keys.enc.migrated");
            let _ = fs::rename(&old_keys_file, &backup_path);
            println!("[Keyring] Migration complete, old file renamed to keys.enc.migrated");
        }
    }

    Ok(())
}

// Old encrypted keys format for migration
#[derive(serde::Deserialize, Default)]
struct OldEncryptedKeys {
    openai: Option<String>,
    groq: Option<String>,
    assemblyai: Option<String>,
}

// Decrypt old AES-GCM encrypted keys (for migration only)
fn decrypt_old_key(encrypted_b64: &str) -> Result<String> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Recreate old encryption key
    let machine_id =
        whoami::fallible::devicename().unwrap_or_else(|_| "hey-app-default".to_string());
    let key_material = format!("hey-api-key-encryption-{}", machine_id);

    let mut key = [0u8; 32];
    for i in 0..32 {
        let mut hasher = DefaultHasher::new();
        format!("{}-{}", key_material, i).hash(&mut hasher);
        key[i] = (hasher.finish() % 256) as u8;
    }

    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| anyhow::anyhow!("Invalid key: {:?}", e))?;

    let combined = STANDARD.decode(encrypted_b64)?;

    if combined.len() < 12 {
        return Err(anyhow::anyhow!("Invalid encrypted data"));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))
}
