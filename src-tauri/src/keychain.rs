use anyhow::Result;

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
