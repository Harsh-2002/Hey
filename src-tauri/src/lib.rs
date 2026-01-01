mod audio;
mod chunking;
mod config;
mod keychain;
mod paste;
mod sessions;
mod transcription;

use audio::{AudioDevice, AudioRecorder};
use config::Config;
use sessions::Session;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, Submenu, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    image::Image,
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;
use transcription::TranscriptionProvider;

struct AppState {
    recorder: Mutex<AudioRecorder>,
    config: Mutex<Config>,
    last_transcript: Mutex<Option<String>>,
}

// ============== Recording Commands ==============

#[tauri::command]
fn start_recording(state: tauri::State<AppState>) -> Result<(), String> {
    let recorder = state.recorder.lock().map_err(|e| e.to_string())?;
    recorder.start_recording().map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_recording(state: tauri::State<AppState>) -> Result<Vec<u8>, String> {
    let recorder = state.recorder.lock().map_err(|e| e.to_string())?;
    recorder.stop_recording().map_err(|e| e.to_string())
}

#[tauri::command]
fn is_recording(state: tauri::State<AppState>) -> Result<bool, String> {
    let recorder = state.recorder.lock().map_err(|e| e.to_string())?;
    Ok(recorder.is_recording())
}

#[tauri::command]
fn get_audio_levels(state: tauri::State<AppState>) -> Result<Vec<f32>, String> {
    let recorder = state.recorder.lock().map_err(|e| e.to_string())?;
    let levels = recorder.get_audio_levels();
    // Return just the RMS values for waveform
    Ok(levels.iter().map(|l| l.rms).collect())
}

#[tauri::command]
fn get_recording_duration(state: tauri::State<AppState>) -> Result<f32, String> {
    let recorder = state.recorder.lock().map_err(|e| e.to_string())?;
    Ok(recorder.get_duration())
}

// ============== Audio Device Commands ==============

#[tauri::command]
fn list_audio_devices(state: tauri::State<AppState>) -> Result<Vec<AudioDevice>, String> {
    let recorder = state.recorder.lock().map_err(|e| e.to_string())?;
    recorder.list_devices().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_audio_device(
    state: tauri::State<AppState>,
    device_name: Option<String>,
) -> Result<(), String> {
    let recorder = state.recorder.lock().map_err(|e| e.to_string())?;
    recorder.set_device(device_name);
    Ok(())
}

// ============== Transcription Commands ==============

#[tauri::command]
async fn transcribe_audio(
    audio_data: Vec<u8>,
    provider: String,
    api_key: String,
) -> Result<String, String> {
    let provider_enum = match provider.as_str() {
        "openai" => TranscriptionProvider::OpenAI,
        "groq" => TranscriptionProvider::Groq,
        "assemblyai" => TranscriptionProvider::AssemblyAI,
        _ => return Err("Invalid provider".to_string()),
    };

    // Get chunk config based on provider
    let chunk_config = match provider_enum {
        TranscriptionProvider::OpenAI => chunking::ChunkConfig::for_openai(),
        TranscriptionProvider::Groq => chunking::ChunkConfig::for_groq(),
        TranscriptionProvider::AssemblyAI => chunking::ChunkConfig::for_assemblyai(),
    };

    // Split audio if too large
    let chunks = chunking::split_audio(&audio_data, &chunk_config).map_err(|e| e.to_string())?;

    if chunks.len() == 1 {
        // No chunking needed
        transcription::transcribe(audio_data, provider_enum, &api_key)
            .await
            .map_err(|e| e.to_string())
    } else {
        // Process chunks and merge
        let mut transcripts = Vec::new();
        for chunk in chunks {
            let transcript = transcription::transcribe(chunk.data, provider_enum.clone(), &api_key)
                .await
                .map_err(|e| e.to_string())?;
            transcripts.push(transcript);
        }
        Ok(chunking::merge_transcripts(&transcripts))
    }
}

/// Stop recording and save immediately to session (avoids passing large data to frontend)
#[tauri::command]
fn stop_recording_and_save(
    state: tauri::State<AppState>,
    provider: String,
    duration_secs: f32,
) -> Result<sessions::Session, String> {
    let recorder = state.recorder.lock().map_err(|e| e.to_string())?;

    // 1. Stop recording and get WAV data (in memory)
    // encoding happens here effectively
    let wav_data = recorder.stop_recording().map_err(|e| e.to_string())?;

    // 2. Save directly to session (disk)
    // This avoids serialization overhead of sending Vec<u8> to frontend
    sessions::save_pending_session(&wav_data, &provider, duration_secs).map_err(|e| e.to_string())
}

/// Transcribe an existing session (audio already on disk)
#[tauri::command]
async fn transcribe_session(session_id: String, api_key: String) -> Result<String, String> {
    // 1. Read audio from session
    let audio_data = sessions::get_session_audio(&session_id).map_err(|e| e.to_string())?;

    // 2. Identify provider from session metadata (optional, but passed in for now?)
    // Actually we need to know WHICH provider to use.
    // Ideally we read metadata, but for now let's assume the passed api_key matches the provider in metadata.
    // Or we just look at the API key... wait. The transcribe function needs provider enum.
    // Let's get provider from metadata.

    let base_dir = sessions::sessions_dir().map_err(|e| e.to_string())?;
    let session_dir = base_dir.join(&session_id);
    let session = sessions::load_session(&session_dir).map_err(|e| e.to_string())?;

    let provider_enum = match session.provider.as_str() {
        "openai" => transcription::TranscriptionProvider::OpenAI,
        "groq" => transcription::TranscriptionProvider::Groq,
        "assemblyai" => transcription::TranscriptionProvider::AssemblyAI,
        _ => return Err("Unknown provider".to_string()),
    };

    // 3. Transcribe
    // Check if chunking needed based on provider config
    let config = match provider_enum {
        transcription::TranscriptionProvider::OpenAI => chunking::ChunkConfig::for_openai(),
        transcription::TranscriptionProvider::Groq => chunking::ChunkConfig::for_groq(),
        transcription::TranscriptionProvider::AssemblyAI => chunking::ChunkConfig::for_assemblyai(),
    };

    let chunking_needed = audio_data.len() > config.max_size_bytes;

    if !chunking_needed {
        transcription::transcribe(audio_data, provider_enum, &api_key)
            .await
            .map_err(|e| e.to_string())
    } else {
        // Use chunking
        let chunks = chunking::split_audio(&audio_data, &config).map_err(|e| e.to_string())?;
        let mut transcripts = Vec::new();
        for chunk in chunks {
            let transcript = transcription::transcribe(chunk.data, provider_enum.clone(), &api_key)
                .await
                .map_err(|e| e.to_string())?;
            transcripts.push(transcript);
        }
        Ok(chunking::merge_transcripts(&transcripts))
    }
}

#[tauri::command]
async fn format_transcript(
    text: String,
    provider: String,
    api_key: String,
    model: String,
    system_prompt: String,
) -> Result<String, String> {
    let provider_enum = match provider.as_str() {
        "openai" => transcription::TranscriptionProvider::OpenAI,
        "groq" => transcription::TranscriptionProvider::Groq,
        // AssemblyAI doesn't support formatting, shouldn't reach here but handle gracefully
        _ => return Err("Provider does not support formatting".to_string()),
    };

    let result = transcription::format_transcript(&text, provider_enum, &api_key, &model, &system_prompt)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
async fn validate_provider_api_key(provider: String, api_key: String) -> Result<bool, String> {
    let provider_enum = match provider.as_str() {
        "openai" => transcription::TranscriptionProvider::OpenAI,
        "groq" => transcription::TranscriptionProvider::Groq,
        "assemblyai" => transcription::TranscriptionProvider::AssemblyAI,
        _ => return Err("Unknown provider".to_string()),
    };

    transcription::validate_api_key(provider_enum, &api_key)
        .await
        .map_err(|e| e.to_string())
}

// ============== Audio Extraction Commands ==============

/// Extract audio from a video/audio file using macOS native afconvert
/// Returns the raw WAV audio bytes ready for transcription
#[tauri::command]
async fn extract_audio_from_file(file_path: String) -> Result<Vec<u8>, String> {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    println!("[AudioExtract] Processing file: {}", file_path);

    let input_path = Path::new(&file_path);
    if !input_path.exists() {
        return Err("File not found".to_string());
    }

    // Create temp output path - use m4a which afconvert handles well
    let temp_dir = std::env::temp_dir();
    let output_filename = format!("hey_audio_{}.m4a", uuid::Uuid::new_v4());
    let output_path = temp_dir.join(&output_filename);

    // Use afconvert to extract and convert audio to AAC (m4a) which is smaller
    // and well-supported by transcription APIs
    let result = Command::new("afconvert")
        .args([
            "-f",
            "m4af", // Output format: M4A (AAC)
            "-d",
            "aac", // Data format: AAC
            "-b",
            "64000", // Bitrate: 64kbps (good for speech)
            "-c",
            "1", // Channels: mono
            &file_path,
            output_path.to_str().unwrap(),
        ])
        .output();

    match result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("[AudioExtract] afconvert failed: {}", stderr);

                // Try fallback with ffmpeg if available
                let ffmpeg_result = Command::new("ffmpeg")
                    .args([
                        "-i",
                        &file_path,
                        "-vn", // No video
                        "-acodec",
                        "pcm_s16le", // 16-bit PCM
                        "-ar",
                        "16000", // 16kHz sample rate
                        "-ac",
                        "1",  // Mono
                        "-y", // Overwrite
                        output_path.to_str().unwrap(),
                    ])
                    .output();

                if let Ok(ff_out) = ffmpeg_result {
                    if !ff_out.status.success() {
                        return Err(format!(
                            "Audio extraction failed. afconvert error: {}",
                            stderr
                        ));
                    }
                } else {
                    return Err(format!("Audio extraction failed: {}", stderr));
                }
            }

            // Read the output file
            match fs::read(&output_path) {
                Ok(data) => {
                    println!("[AudioExtract] Successfully extracted {} bytes", data.len());
                    // Clean up temp file
                    let _ = fs::remove_file(&output_path);
                    Ok(data)
                }
                Err(e) => {
                    let _ = fs::remove_file(&output_path);
                    Err(format!("Failed to read converted audio: {}", e))
                }
            }
        }
        Err(e) => Err(format!("Failed to run afconvert: {}", e)),
    }
}

#[tauri::command]
fn paste_to_window() -> Result<(), String> {
    paste::paste_to_active_window().map_err(|e| e.to_string())
}

// ============== Secure Storage Commands ==============

#[tauri::command]
fn store_api_key(provider: String, api_key: String) -> Result<(), String> {
    keychain::store_api_key(&provider, &api_key).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_api_key(provider: String) -> Result<Option<String>, String> {
    keychain::get_api_key(&provider).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_api_key(provider: String) -> Result<(), String> {
    keychain::delete_api_key(&provider).map_err(|e| e.to_string())
}

#[tauri::command]
fn has_api_key(provider: String) -> bool {
    keychain::has_api_key(&provider)
}

// ============== Session Commands ==============

#[tauri::command]
fn get_sessions(limit: usize, offset: usize) -> Result<Vec<Session>, String> {
    sessions::list_sessions(limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_session(
    text: String,
    raw_text: Option<String>,
    provider: String,
    duration_secs: f32,
    audio_data: Option<Vec<u8>>,
    formatted: bool,
) -> Result<Session, String> {
    let audio_bytes = audio_data.unwrap_or_default();
    sessions::create_session(
        &audio_bytes,
        &text,
        raw_text.as_deref(),
        &provider,
        duration_secs,
        formatted,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_session(id: String) -> Result<(), String> {
    sessions::delete_session(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_sessions() -> Result<(), String> {
    sessions::clear_all_sessions().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_session_count() -> Result<usize, String> {
    sessions::count_sessions().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_total_storage_size() -> Result<u64, String> {
    sessions::get_storage_usage().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_pending_session(
    audio_data: Vec<u8>,
    provider: String,
    duration_secs: f32,
) -> Result<sessions::Session, String> {
    sessions::save_pending_session(&audio_data, &provider, duration_secs).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_session_transcript(
    state: tauri::State<AppState>,
    session_id: String,
    text: String,
    raw_text: Option<String>,
    formatted: bool,
    success: bool,
    error_message: Option<String>,
) -> Result<sessions::Session, String> {
    // Store in last_transcript if successful
    if success {
        if let Ok(mut last) = state.last_transcript.lock() {
            *last = Some(text.clone());
        }
    }

    sessions::update_session_transcript(
        &session_id,
        &text,
        raw_text.as_deref(),
        formatted,
        success,
        error_message.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_session_audio(session_id: String) -> Result<Vec<u8>, String> {
    sessions::get_session_audio(&session_id).map_err(|e| e.to_string())
}

// ============== Config Commands ==============

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Result<Config, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[tauri::command]
fn save_config(state: tauri::State<AppState>, config: Config) -> Result<(), String> {
    let mut current_config = state.config.lock().map_err(|e| e.to_string())?;
    *current_config = config;
    current_config.save().map_err(|e| e.to_string())
}

// ============== App Entry Point ==============

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let recorder = AudioRecorder::new().expect("Failed to initialize audio recorder");
    let config = Config::load().unwrap_or_default();

    // Migrate API keys from config to keychain on first run
    let _ = keychain::migrate_from_config(&config);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_keyring::init())
        .manage(AppState {
            recorder: Mutex::new(recorder),
            config: Mutex::new(config),
            last_transcript: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            // Recording
            start_recording,
            stop_recording,
            is_recording,
            get_audio_levels,
            get_recording_duration,
            // Audio devices
            list_audio_devices,
            set_audio_device,
            // Transcription
            transcribe_audio,
            format_transcript,
            paste_to_window,
            extract_audio_from_file,
            validate_provider_api_key,
            // Secure storage
            store_api_key,
            get_api_key,
            delete_api_key,
            has_api_key,
            // Sessions
            get_sessions,
            save_session,
            delete_session,
            clear_sessions,
            get_session_count,
            save_pending_session,
            stop_recording_and_save,
            transcribe_session,
            update_session_transcript,
            get_session_audio,
            get_total_storage_size,
            // Config
            get_config,
            save_config,
        ])
        .setup(|app| {
            // Get initial audio devices
            let state = app.state::<AppState>();
            let mut devices = Vec::new();
            if let Ok(recorder) = state.recorder.lock() {
                if let Ok(devs) = recorder.list_devices() {
                    devices = devs;
                }
            }

            // Create tray menu items
            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let copy = MenuItem::with_id(app, "copy", "Copy Last Transcript", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            
            // Microphones submenu
            let mic_menu = Submenu::new(app, "Microphones", true)?;
            for device in devices {
                let id = format!("mic_{}", device.name);
                let item = MenuItem::with_id(app, &id, &device.name, true, None::<&str>)?;
                mic_menu.append(&item)?;
            }
            
            let updates = MenuItem::with_id(app, "update", "Check for Updates", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[
                &show,
                &copy,
                &separator,
                &mic_menu,
                &separator, 
                &updates,
                &settings, 
                &quit
            ])?;

            // Create tray icon
            let tray_icon_bytes = include_bytes!("../icons/TrayIcon.png");
            let icon_img = image::load_from_memory(tray_icon_bytes)
                .expect("Failed to load tray icon image")
                .to_rgba8();
            let (width, height) = icon_img.dimensions();
            let rgba = icon_img.into_raw();
            let tray_icon = Image::new_owned(rgba, width, height);

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&menu)
                .icon_as_template(true)
                .on_menu_event(|app, event| {
                    let id = event.id.as_ref();
                    if id == "show" {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    } else if id == "copy" {
                        let state = app.state::<AppState>();
                        if let Ok(last) = state.last_transcript.lock() {
                            if let Some(text) = last.as_ref() {
                                let _ = app.clipboard().write_text(text);
                            }
                        };
                    } else if id == "settings" {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("open-settings", ());
                        }
                    } else if id == "update" {
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            match handle.updater() {
                                Ok(updater) => {
                                    match updater.check().await {
                                        Ok(Some(update)) => {
                                            handle.dialog()
                                                .message(format!("Version {} is available.", update.version))
                                                .title("Update Available")
                                                .kind(MessageDialogKind::Info)
                                                .show(|_| {});
                                        }
                                        Ok(None) => {
                                            handle.dialog()
                                                .message("You are on the latest version.")
                                                .title("No Updates")
                                                .kind(MessageDialogKind::Info)
                                                .show(|_| {});
                                        }
                                        Err(e) => {
                                            handle.dialog()
                                                .message(format!("Error: {}", e))
                                                .title("Update Check Failed")
                                                .kind(MessageDialogKind::Error)
                                                .show(|_| {});
                                        }
                                    }
                                }
                                Err(e) => {
                                    handle.dialog()
                                        .message(format!("Failed to initialize updater: {}", e))
                                        .title("Updater Error")
                                        .kind(MessageDialogKind::Error)
                                        .show(|_| {});
                                }
                            }
                        });
                    } else if id == "quit" {
                        app.exit(0);
                    } else if id.starts_with("mic_") {
                        let device_name = &id[4..];
                        let state = app.state::<AppState>();
                        if let Ok(recorder) = state.recorder.lock() {
                            recorder.set_device(Some(device_name.to_string()));
                        };
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // Handle window close to hide instead of quit
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });
            }

            // Hide from dock - menu bar only mode
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
