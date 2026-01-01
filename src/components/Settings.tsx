import { useState, useEffect } from 'react';
import { X, Keyboard, Mic, Shield, Check, Loader2, Database, Sparkles } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import type { Config, TranscriptionProvider } from '../types';

interface SettingsProps {
    config: Config;
    onSave: (config: Config) => Promise<boolean>;
    onClose: () => void;
}

interface AudioDevice {
    name: string;
    is_default: boolean;
}

interface ProviderInfo {
    id: TranscriptionProvider;
    name: string;
    description: string;
}

const providers: ProviderInfo[] = [
    { id: 'openai', name: 'OpenAI Whisper', description: 'High accuracy' },
    { id: 'groq', name: 'Groq', description: 'Ultra-fast' },
    { id: 'assemblyai', name: 'AssemblyAI', description: 'Feature-rich' },
];

export function Settings({ config, onSave, onClose }: SettingsProps) {
    const [localConfig, setLocalConfig] = useState<Config>(config);
    const [apiKeys, setApiKeys] = useState<Record<string, string>>({
        openai: '',
        groq: '',
        assemblyai: '',
    });
    const [saving, setSaving] = useState(false);
    const [audioDevices, setAudioDevices] = useState<AudioDevice[]>([]);
    const [keysInKeychain, setKeysInKeychain] = useState<Record<string, boolean>>({});
    const [validatingKey, setValidatingKey] = useState<string | null>(null);
    const [keyValidation, setKeyValidation] = useState<Record<string, 'valid' | 'invalid' | null>>({});
    const [recordingShortcut, setRecordingShortcut] = useState(false);
    const [tempShortcut, setTempShortcut] = useState('');
    const [storageSize, setStorageSize] = useState<number | null>(null);

    useEffect(() => {
        setLocalConfig(config);
        loadAudioDevices();
        checkKeychainStatus();
        loadStorageSize();
    }, [config]);

    const loadStorageSize = async () => {
        try {
            const size = await invoke<number>('get_total_storage_size');
            setStorageSize(size);
        } catch (error) {
            console.error('Failed to load storage size:', error);
        }
    };

    const formatBytes = (bytes: number) => {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    };

    const loadAudioDevices = async () => {
        try {
            const devices = await invoke<AudioDevice[]>('list_audio_devices');
            setAudioDevices(devices);
        } catch (error) {
            console.error('Failed to load audio devices:', error);
        }
    };

    const checkKeychainStatus = async () => {
        const status: Record<string, boolean> = {};
        for (const p of ['openai', 'groq', 'assemblyai']) {
            status[p] = await invoke<boolean>('has_api_key', { provider: p });
        }
        setKeysInKeychain(status);
    };

    const validateApiKey = async (provider: string) => {
        const key = apiKeys[provider];
        if (!key) return;

        setValidatingKey(provider);
        setKeyValidation(prev => ({ ...prev, [provider]: null }));

        try {
            // Real validation via backend
            const isValid = await invoke<boolean>('validate_provider_api_key', {
                provider,
                apiKey: key
            });
            setKeyValidation(prev => ({ ...prev, [provider]: isValid ? 'valid' : 'invalid' }));
        } catch (err) {
            console.error('Validation failed:', err);
            setKeyValidation(prev => ({ ...prev, [provider]: 'invalid' }));
        } finally {
            setValidatingKey(null);
        }
    };

    const handleSave = async () => {
        setSaving(true);

        // Save API keys to keychain
        for (const [provider, key] of Object.entries(apiKeys)) {
            if (key) {
                console.log('[Settings] Storing API key for:', provider, 'length:', key.length);
                try {
                    await invoke('store_api_key', { provider, apiKey: key });
                    console.log('[Settings] Stored successfully for:', provider);
                } catch (err) {
                    console.error('[Settings] Failed to store key for:', provider, err);
                }
            }
        }

        const success = await onSave(localConfig);
        setSaving(false);
        if (success) {
            onClose();
        }
    };



    const updateField = <K extends keyof Config>(key: K, value: Config[K]) => {
        setLocalConfig(prev => ({ ...prev, [key]: value }));
    };

    // Shortcut recording
    const startRecordingShortcut = () => {
        setRecordingShortcut(true);
        setTempShortcut('');
    };

    const handleShortcutKeyDown = (e: React.KeyboardEvent) => {
        if (!recordingShortcut) return;
        e.preventDefault();

        const parts: string[] = [];
        if (e.metaKey || e.ctrlKey) parts.push('CommandOrControl');
        if (e.shiftKey) parts.push('Shift');
        if (e.altKey) parts.push('Alt');

        let key = e.key;
        if (key === ' ' || key === '\u00A0') key = 'Space';
        else if (key.length === 1) key = key.toUpperCase();
        else if (['Control', 'Shift', 'Alt', 'Meta'].includes(key)) {
            setTempShortcut(parts.join('+'));
            return;
        }

        if (!parts.includes(key)) parts.push(key);

        const shortcut = parts.join('+');
        setTempShortcut(shortcut);

        if (parts.length >= 2) {
            updateField('shortcut', shortcut);
            setRecordingShortcut(false);
        }
    };

    const cancelShortcutRecording = () => {
        setRecordingShortcut(false);
        setTempShortcut('');
    };

    const formatShortcut = (s: string) => {
        return s.replace('CommandOrControl', '⌘').replace('Shift', '⇧').replace('Alt', '⌥').split('+').join(' ');
    };

    return (
        <div className="panel">
            {/* Header */}
            <div className="panel-header">
                <h2>Settings</h2>
                <button className="close-btn" onClick={onClose}>
                    <X size={18} />
                </button>
            </div>

            {/* Content */}
            <div className="panel-content">
                {/* Provider */}
                <div className="section">
                    <h3>Provider</h3>
                    <div className="provider-list">
                        {providers.map(p => (
                            <div
                                key={p.id}
                                className={`provider-option ${localConfig.active_provider === p.id ? 'active' : ''}`}
                                onClick={() => updateField('active_provider', p.id)}
                            >
                                <div className="radio" />
                                <div className="provider-text">
                                    <span className="provider-name">{p.name}</span>
                                    <span className="provider-desc">{p.description}</span>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>

                {/* API Keys */}
                <div className="section">
                    <h3><Shield size={12} /> API Keys</h3>
                    {['openai', 'groq', 'assemblyai'].map((provider) => (
                        <div className="field" key={provider}>
                            <label>
                                {provider === 'openai' ? 'OpenAI' : provider === 'groq' ? 'Groq' : 'AssemblyAI'}
                                {keysInKeychain[provider] && <Check size={12} className="check-icon" />}
                                {keyValidation[provider] === 'valid' && <span className="valid-badge">Valid</span>}
                                {keyValidation[provider] === 'invalid' && <span className="invalid-badge">Invalid</span>}
                            </label>
                            <div className="input-row">
                                <input
                                    type="password"
                                    value={apiKeys[provider] || ''}
                                    onChange={e => setApiKeys(prev => ({ ...prev, [provider]: e.target.value }))}
                                    placeholder={keysInKeychain[provider] ? '••••••••••••••••' : 'Enter API Key'}
                                />
                                <button
                                    className="icon-btn"
                                    onClick={() => validateApiKey(provider)}
                                    disabled={validatingKey === provider}
                                    title="Validate Key"
                                >
                                    {validatingKey === provider ? <Loader2 size={16} className="spinner-icon" /> : <Check size={16} />}
                                </button>
                            </div>
                        </div>
                    ))}
                </div>

                {/* Shortcut */}
                <div className="section">
                    <h3><Keyboard size={12} /> Shortcut</h3>
                    <div
                        className={`shortcut-input ${recordingShortcut ? 'recording' : ''}`}
                        onClick={!recordingShortcut ? startRecordingShortcut : undefined}
                        onKeyDown={handleShortcutKeyDown}
                        tabIndex={0}
                    >
                        <span className="shortcut-keys">
                            {recordingShortcut ? (tempShortcut ? formatShortcut(tempShortcut) : 'Press keys...') : formatShortcut(localConfig.shortcut)}
                        </span>
                        {recordingShortcut ? (
                            <button className="text-btn" onClick={cancelShortcutRecording}>Cancel</button>
                        ) : (
                            <span className="hint">Click to change</span>
                        )}
                    </div>
                    <div className="toggle-row" style={{ marginTop: '12px' }}>
                        <div className="toggle-info">
                            <span>Push-to-Talk</span>
                            <small>Hold shortcut to record, release to stop</small>
                        </div>
                        <div
                            className={`toggle ${localConfig.recording_mode === 'hold' ? 'active' : ''}`}
                            onClick={() => updateField('recording_mode', localConfig.recording_mode === 'hold' ? 'toggle' : 'hold')}
                        >
                            <div className="toggle-knob" />
                        </div>
                    </div>
                </div>

                {/* Audio Input */}
                <div className="section">
                    <h3><Mic size={12} /> Audio Input</h3>
                    <select
                        className="select"
                        value={localConfig.selected_device || ''}
                        onChange={e => updateField('selected_device', e.target.value || null)}
                    >
                        <option value="">System Default</option>
                        {audioDevices.map(d => (
                            <option key={d.name} value={d.name}>
                                {d.name} {d.is_default ? '(Default)' : ''}
                            </option>
                        ))}
                    </select>
                </div>

                {/* Toggles */}
                <div className="section">
                    <h3>
                        <Sparkles size={14} /> Formatting
                    </h3>
                    <div className="toggle-row">
                        <div className="toggle-info">
                            <span>Format with AI</span>
                            <small>Clean up transcripts (punctuation, filler words)</small>
                        </div>
                        <div
                            className={`toggle ${localConfig.format_with_ai ? 'active' : ''}`}
                            onClick={() => updateField('format_with_ai', !localConfig.format_with_ai)}
                        >
                            <div className="toggle-knob" />
                        </div>
                    </div>

                    {localConfig.format_with_ai && (
                        <>
                            <div className="field">
                                <label>System Prompt</label>
                                <textarea
                                    className="settings-textarea"
                                    value={localConfig.system_prompt}
                                    onChange={e => updateField('system_prompt', e.target.value)}
                                    placeholder="You are a transcript formatter..."
                                    rows={4}
                                />
                                <span className="hint">Instructions for how to clean up the text.</span>
                            </div>

                            <div className="field">
                                <label>OpenAI Model</label>
                                <select
                                    className="select"
                                    value={localConfig.openai_formatting_model}
                                    onChange={e => updateField('openai_formatting_model', e.target.value)}
                                >
                                    <option value="gpt-4o">GPT 4.1 Nano</option>
                                    <option value="gpt-4o-mini">GPT 4.0 Mini</option>
                                </select>
                            </div>

                            <div className="field">
                                <label>Groq Model</label>
                                <select
                                    className="select"
                                    value={localConfig.groq_formatting_model}
                                    onChange={e => updateField('groq_formatting_model', e.target.value)}
                                >
                                    <option value="llama-3.1-8b-instant">Llama 3.1 8B Instant</option>
                                    <option value="openai/gpt-oss-20b">GPT OSS 20B 128k</option>
                                </select>
                            </div>
                        </>
                    )}
                </div>

                <div className="section">
                    <h3>General</h3>
                    <div className="toggle-row">
                        <div className="toggle-info">
                            <span>Auto-paste</span>
                            <small>Show paste button</small>
                        </div>
                        <div
                            className={`toggle ${localConfig.auto_paste ? 'active' : ''}`}
                            onClick={() => updateField('auto_paste', !localConfig.auto_paste)}
                        >
                            <div className="toggle-knob" />
                        </div>
                    </div>
                    <div className="toggle-row">
                        <div className="toggle-info">
                            <span>Save History</span>
                            <small>Keep transcriptions</small>
                        </div>
                        <div
                            className={`toggle ${localConfig.save_history ? 'active' : ''}`}
                            onClick={() => updateField('save_history', !localConfig.save_history)}
                        >
                            <div className="toggle-knob" />
                        </div>
                    </div>
                    <div className="toggle-row">
                        <div className="toggle-info">
                            <span>Launch at Login</span>
                            <small>Start Hey when Mac boots</small>
                        </div>
                        <div
                            className={`toggle ${localConfig.launch_at_login ? 'active' : ''}`}
                            onClick={async () => {
                                const newValue = !localConfig.launch_at_login;
                                updateField('launch_at_login', newValue);
                                try {
                                    if (newValue) {
                                        await invoke('plugin:autostart|enable');
                                    } else {
                                        await invoke('plugin:autostart|disable');
                                    }
                                } catch (err) {
                                    console.error('Failed to update autostart:', err);
                                }
                            }}
                        >
                            <div className="toggle-knob" />
                        </div>
                    </div>
                </div>

                {/* Save */}
                <div className="section">
                    <h3><Database size={12} /> Data</h3>
                    <div className="data-row">
                        <div className="data-info">
                            <span>Local Storage</span>
                            <small>Audio sessions in ~/.hey</small>
                        </div>
                        <div className="data-value">
                            {storageSize !== null ? formatBytes(storageSize) : 'Calculating...'}
                        </div>
                    </div>
                </div>

                {/* Credits */}
                <div className="section credits-section">
                    <div className="credits-content">
                        <span className="credits-text">Built by </span>
                        <a href="https://firstfinger.io" target="_blank" rel="noopener noreferrer" className="credits-link">
                            Anurag Vishwakarma
                        </a>
                        <span className="credits-separator">•</span>
                        <a href="https://github.com/Harsh-2002" target="_blank" rel="noopener noreferrer" className="credits-link">
                            GitHub
                        </a>
                    </div>
                </div>

                {/* Save */}
                <button className="primary-btn full-width" onClick={handleSave} disabled={saving}>
                    {saving ? 'Saving...' : 'Save Settings'}
                </button>
            </div>
        </div>
    );
}
