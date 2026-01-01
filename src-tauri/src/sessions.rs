use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SessionStatus {
    #[default]
    Pending, // Audio saved, transcription in progress
    Completed, // Transcription successful
    Failed,    // Transcription failed
}

/// Session metadata stored in metadata.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub duration_secs: f32,
    pub provider: String,
    pub formatted: bool,
    pub file_size_bytes: Option<u64>,
    #[serde(default)]
    pub status: SessionStatus,
    #[serde(default)]
    pub error_message: Option<String>,
}

/// Full session data returned to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub text: String,
    pub raw_text: Option<String>,
    pub provider: String,
    pub duration_secs: f32,
    pub audio_path: Option<String>,
    pub file_size_bytes: Option<u64>,
    pub formatted: bool,
    #[serde(default)]
    pub status: SessionStatus,
    #[serde(default)]
    pub error_message: Option<String>,
}

/// Get the base sessions directory
pub fn sessions_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;
    let sessions_dir = home.join(".hey").join("sessions");
    fs::create_dir_all(&sessions_dir)?;
    Ok(sessions_dir)
}

/// Generate a session ID (date_time_shortid)
fn generate_session_id() -> String {
    let now = Utc::now();
    let short_id = &uuid::Uuid::new_v4().to_string()[..8];
    format!("{}_{}", now.format("%Y-%m-%d_%H%M%S"), short_id)
}

/// Create a new session with audio and transcription
pub fn create_session(
    audio_data: &[u8],
    text: &str,
    raw_text: Option<&str>,
    provider: &str,
    duration_secs: f32,
    formatted: bool,
) -> Result<Session> {
    let session_id = generate_session_id();
    let base_dir = sessions_dir()?;
    let session_dir = base_dir.join(&session_id);
    fs::create_dir_all(&session_dir)?;

    // Save audio as M4A (convert from WAV)
    let audio_path = session_dir.join("audio.m4a");
    let wav_temp = session_dir.join("temp.wav");

    // Write WAV temporarily
    fs::write(&wav_temp, audio_data)?;

    // Convert to M4A using afconvert (macOS)
    let conversion_result = std::process::Command::new("afconvert")
        .args([
            "-f",
            "m4af", // M4A container
            "-d",
            "aac", // AAC codec
            "-b",
            "128000", // 128kbps bitrate
            wav_temp.to_str().unwrap(),
            audio_path.to_str().unwrap(),
        ])
        .output();

    // Clean up temp WAV
    let _ = fs::remove_file(&wav_temp);

    let final_audio_path = if conversion_result.is_ok() && audio_path.exists() {
        audio_path.clone()
    } else {
        // Fallback: save as WAV if conversion fails
        let wav_path = session_dir.join("audio.wav");
        fs::write(&wav_path, audio_data)?;
        wav_path
    };

    // Get file size
    let file_size_bytes = fs::metadata(&final_audio_path).ok().map(|m| m.len());

    // Save transcription text
    let transcript_path = session_dir.join("transcript.txt");
    fs::write(&transcript_path, text)?;

    // Save raw text if different
    if let Some(raw) = raw_text {
        if raw != text {
            let raw_path = session_dir.join("transcript_raw.txt");
            fs::write(&raw_path, raw)?;
        }
    }

    // Save metadata
    let metadata = SessionMetadata {
        id: session_id.clone(),
        timestamp: Utc::now(),
        duration_secs,
        provider: provider.to_string(),
        formatted,
        file_size_bytes,
        status: SessionStatus::Completed,
        error_message: None,
    };
    let metadata_path = session_dir.join("metadata.json");
    let metadata_json = serde_json::to_string_pretty(&metadata)?;
    fs::write(&metadata_path, metadata_json)?;

    Ok(Session {
        id: session_id,
        timestamp: metadata.timestamp,
        text: text.to_string(),
        raw_text: raw_text.map(|s| s.to_string()),
        provider: provider.to_string(),
        duration_secs,
        audio_path: Some(final_audio_path.to_string_lossy().to_string()),
        file_size_bytes,
        formatted,
        status: SessionStatus::Completed,
        error_message: None,
    })
}

/// List all sessions (most recent first)
pub fn list_sessions(limit: usize, offset: usize) -> Result<Vec<Session>> {
    let base_dir = sessions_dir()?;

    let mut sessions: Vec<Session> = Vec::new();

    // Read all session directories
    let mut entries: Vec<_> = fs::read_dir(&base_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    // Sort by name (descending = newest first because of date prefix)
    entries.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

    // Apply pagination
    for entry in entries.into_iter().skip(offset).take(limit) {
        if let Ok(session) = load_session(&entry.path()) {
            sessions.push(session);
        }
    }

    Ok(sessions)
}

/// Calculate total storage usage of the sessions directory in bytes
pub fn get_storage_usage() -> Result<u64> {
    let dir = sessions_dir()?;
    let mut total_size = 0;

    if dir.exists() {
        for entry in walkdir::WalkDir::new(dir) {
            let entry = entry.map_err(|e| anyhow::anyhow!(e))?;
            if entry.metadata()?.is_file() {
                total_size += entry.metadata()?.len();
            }
        }
    }

    Ok(total_size)
}

/// Load a session from a directory
pub fn load_session(session_dir: &std::path::Path) -> Result<Session> {
    let metadata_path = session_dir.join("metadata.json");
    let metadata_content = fs::read_to_string(&metadata_path)?;
    let metadata: SessionMetadata = serde_json::from_str(&metadata_content)?;

    // Read transcript
    let transcript_path = session_dir.join("transcript.txt");
    let text = fs::read_to_string(&transcript_path).unwrap_or_default();

    // Read raw transcript if exists
    let raw_path = session_dir.join("transcript_raw.txt");
    let raw_text = fs::read_to_string(&raw_path).ok();

    // Find audio file (m4a or wav)
    let m4a_path = session_dir.join("audio.m4a");
    let wav_path = session_dir.join("audio.wav");
    let audio_path = if m4a_path.exists() {
        Some(m4a_path)
    } else if wav_path.exists() {
        Some(wav_path)
    } else {
        None
    };

    // Get current file size
    let file_size_bytes = audio_path
        .as_ref()
        .and_then(|p| fs::metadata(p).ok())
        .map(|m| m.len());

    Ok(Session {
        id: metadata.id,
        timestamp: metadata.timestamp,
        text,
        raw_text,
        provider: metadata.provider,
        duration_secs: metadata.duration_secs,
        audio_path: audio_path.map(|p| p.to_string_lossy().to_string()),
        file_size_bytes,
        formatted: metadata.formatted,
        status: metadata.status,
        error_message: metadata.error_message,
    })
}

/// Get a specific session by ID
#[allow(dead_code)]
pub fn get_session(session_id: &str) -> Result<Option<Session>> {
    let base_dir = sessions_dir()?;
    let session_dir = base_dir.join(session_id);

    if session_dir.exists() {
        Ok(Some(load_session(&session_dir)?))
    } else {
        Ok(None)
    }
}

/// Delete a session
pub fn delete_session(session_id: &str) -> Result<()> {
    let base_dir = sessions_dir()?;
    let session_dir = base_dir.join(session_id);

    if session_dir.exists() {
        fs::remove_dir_all(&session_dir)?;
    }

    Ok(())
}

/// Clear all sessions
pub fn clear_all_sessions() -> Result<()> {
    let base_dir = sessions_dir()?;

    for entry in fs::read_dir(&base_dir)?.flatten() {
        if entry.path().is_dir() {
            let _ = fs::remove_dir_all(entry.path());
        }
    }

    Ok(())
}

/// Count total sessions
pub fn count_sessions() -> Result<usize> {
    let base_dir = sessions_dir()?;

    let count = fs::read_dir(&base_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .count();

    Ok(count)
}

/// Save a pending session (audio only, before transcription)
/// This ensures audio is saved immediately even if transcription fails
pub fn save_pending_session(
    audio_data: &[u8],
    provider: &str,
    duration_secs: f32,
) -> Result<Session> {
    let session_id = generate_session_id();
    let base_dir = sessions_dir()?;
    let session_dir = base_dir.join(&session_id);
    fs::create_dir_all(&session_dir)?;

    // Save audio as M4A (convert from WAV)
    let audio_path = session_dir.join("audio.m4a");
    let wav_temp = session_dir.join("temp.wav");

    // Write WAV temporarily
    fs::write(&wav_temp, audio_data)?;

    // Convert to M4A using afconvert (macOS)
    let conversion_result = std::process::Command::new("afconvert")
        .args([
            "-f",
            "m4af",
            "-d",
            "aac",
            "-b",
            "128000",
            wav_temp.to_str().unwrap(),
            audio_path.to_str().unwrap(),
        ])
        .output();

    // Clean up temp WAV
    let _ = fs::remove_file(&wav_temp);

    let final_audio_path = if conversion_result.is_ok() && audio_path.exists() {
        audio_path.clone()
    } else {
        // Fallback: save as WAV if conversion fails
        let wav_path = session_dir.join("audio.wav");
        fs::write(&wav_path, audio_data)?;
        wav_path
    };

    // Get file size
    let file_size_bytes = fs::metadata(&final_audio_path).ok().map(|m| m.len());

    // Save empty transcript placeholder
    let transcript_path = session_dir.join("transcript.txt");
    fs::write(&transcript_path, "[Transcription pending...]")?;

    // Save metadata with pending status
    let metadata = SessionMetadata {
        id: session_id.clone(),
        timestamp: Utc::now(),
        duration_secs,
        provider: provider.to_string(),
        formatted: false,
        file_size_bytes,
        status: SessionStatus::Pending,
        error_message: None,
    };
    let metadata_path = session_dir.join("metadata.json");
    let metadata_json = serde_json::to_string_pretty(&metadata)?;
    fs::write(&metadata_path, metadata_json)?;

    Ok(Session {
        id: session_id,
        timestamp: metadata.timestamp,
        text: "[Transcription pending...]".to_string(),
        raw_text: None,
        provider: provider.to_string(),
        duration_secs,
        audio_path: Some(final_audio_path.to_string_lossy().to_string()),
        file_size_bytes,
        formatted: false,
        status: SessionStatus::Pending,
        error_message: None,
    })
}

/// Update a session with transcription result
pub fn update_session_transcript(
    session_id: &str,
    text: &str,
    raw_text: Option<&str>,
    formatted: bool,
    success: bool,
    error_message: Option<&str>,
) -> Result<Session> {
    let base_dir = sessions_dir()?;
    let session_dir = base_dir.join(session_id);

    if !session_dir.exists() {
        return Err(anyhow::anyhow!("Session not found: {}", session_id));
    }

    // Update transcript file
    let transcript_path = session_dir.join("transcript.txt");
    fs::write(&transcript_path, text)?;

    // Save raw text if different
    if let Some(raw) = raw_text {
        if raw != text {
            let raw_path = session_dir.join("transcript_raw.txt");
            fs::write(&raw_path, raw)?;
        }
    }

    // Load and update metadata
    let metadata_path = session_dir.join("metadata.json");
    let metadata_content = fs::read_to_string(&metadata_path)?;
    let mut metadata: SessionMetadata = serde_json::from_str(&metadata_content)?;

    metadata.formatted = formatted;
    metadata.status = if success {
        SessionStatus::Completed
    } else {
        SessionStatus::Failed
    };
    metadata.error_message = error_message.map(|s| s.to_string());

    let metadata_json = serde_json::to_string_pretty(&metadata)?;
    fs::write(&metadata_path, metadata_json)?;

    // Reload and return the updated session
    load_session(&session_dir)
}

/// Get session directory path
fn get_session_dir(session_id: &str) -> Result<PathBuf> {
    let base_dir = sessions_dir()?;
    Ok(base_dir.join(session_id))
}

/// Get audio data for a session (for retry transcription)
pub fn get_session_audio(session_id: &str) -> Result<Vec<u8>> {
    let session_dir = get_session_dir(session_id)?;

    // Try M4A first, then WAV
    let m4a_path = session_dir.join("audio.m4a");
    let wav_path = session_dir.join("audio.wav");

    if m4a_path.exists() {
        Ok(fs::read(&m4a_path)?)
    } else if wav_path.exists() {
        Ok(fs::read(&wav_path)?)
    } else {
        Err(anyhow::anyhow!(
            "No audio file found for session: {}",
            session_id
        ))
    }
}
