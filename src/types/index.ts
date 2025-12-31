export type TranscriptionProvider = 'openai' | 'groq' | 'assemblyai';
export type RecordingMode = 'toggle' | 'hold';

export interface Config {
    // API keys are stored in macOS Keychain, not here
    active_provider: TranscriptionProvider;
    shortcut: string;
    auto_paste: boolean;
    format_with_ai: boolean;
    openai_formatting_model: string;
    groq_formatting_model: string;
    system_prompt: string;
    selected_device: string | null;
    save_history: boolean;
    launch_at_login: boolean;
    recording_mode: RecordingMode;
}

export interface TranscriptionResult {
    text: string;
    raw_text?: string;
    provider: TranscriptionProvider;
    timestamp: number;
    formatted: boolean;
    duration?: number;
}

export interface Session {
    id: string;
    timestamp: string;
    text: string;
    raw_text?: string;
    provider: string;
    duration_secs: number;
    audio_path: string | null;
    file_size_bytes: number | null;
    formatted: boolean;
    status: 'Pending' | 'Completed' | 'Failed';
    error_message?: string | null;
}

// Alias for backwards compatibility
export type TranscriptionRecord = Session;

export type RecordingState = 'idle' | 'recording' | 'processing';

export interface Toast {
    id: string;
    message: string;
    type: 'success' | 'error' | 'info';
}

export interface AudioDevice {
    name: string;
    is_default: boolean;
}
