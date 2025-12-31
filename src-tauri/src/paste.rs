use anyhow::Result;
use enigo::{Enigo, Keyboard, Settings};
use std::thread;
use std::time::Duration;

/// Paste text to the active window by:
/// 1. Copying text to clipboard (done before calling this)
/// 2. Simulating Cmd+V keyboard shortcut
pub fn paste_to_active_window() -> Result<()> {
    // Small delay to ensure clipboard is ready
    thread::sleep(Duration::from_millis(100));

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| anyhow::anyhow!("Failed to create Enigo instance: {:?}", e))?;

    // Press Command+V on macOS
    #[cfg(target_os = "macos")]
    {
        use enigo::Key;

        enigo
            .key(Key::Meta, enigo::Direction::Press)
            .map_err(|e| anyhow::anyhow!("Failed to press Meta key: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        enigo
            .key(Key::Unicode('v'), enigo::Direction::Click)
            .map_err(|e| anyhow::anyhow!("Failed to press V key: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        enigo
            .key(Key::Meta, enigo::Direction::Release)
            .map_err(|e| anyhow::anyhow!("Failed to release Meta key: {:?}", e))?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        use enigo::Key;

        enigo
            .key(Key::Control, enigo::Direction::Press)
            .map_err(|e| anyhow::anyhow!("Failed to press Control key: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        enigo
            .key(Key::Unicode('v'), enigo::Direction::Click)
            .map_err(|e| anyhow::anyhow!("Failed to press V key: {:?}", e))?;

        thread::sleep(Duration::from_millis(50));

        enigo
            .key(Key::Control, enigo::Direction::Release)
            .map_err(|e| anyhow::anyhow!("Failed to release Control key: {:?}", e))?;
    }

    Ok(())
}
