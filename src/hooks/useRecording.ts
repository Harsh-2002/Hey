import { useState, useCallback, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { register, unregister, isRegistered } from '@tauri-apps/plugin-global-shortcut';
import type { RecordingState, Config, TranscriptionResult } from '../types';

interface UseRecordingProps {
    config: Config | null;
    onTranscriptionComplete: (result: TranscriptionResult) => void;
    onError: (message: string) => void;
}

export function useRecording({ config, onTranscriptionComplete, onError }: UseRecordingProps) {
    const [recordingState, setRecordingState] = useState<RecordingState>('idle');
    const [duration, setDuration] = useState(0);
    const [audioLevels, setAudioLevels] = useState<number[]>([]);
    const timerRef = useRef<number | null>(null);
    const levelIntervalRef = useRef<number | null>(null);
    const shortcutRegistered = useRef<string | null>(null);

    const startRecording = useCallback(async () => {
        if (recordingState !== 'idle') return;

        try {
            console.log('[Recording] Starting...');

            // Set audio device if configured
            if (config?.selected_device) {
                await invoke('set_audio_device', { deviceName: config.selected_device });
            }

            // Optimistic UI update
            setRecordingState('recording');
            setDuration(0);
            setAudioLevels([]);
            console.log('[Recording] Starting...');

            await invoke('start_recording');
            console.log('[Recording] Started successfully');

            // Duration timer
            timerRef.current = window.setInterval(() => {
                setDuration(d => d + 1);
            }, 1000);

            // Audio levels polling for waveform
            levelIntervalRef.current = window.setInterval(async () => {
                try {
                    const levels = await invoke<number[]>('get_audio_levels');
                    setAudioLevels(levels);
                } catch {
                    // Ignore level polling errors
                }
            }, 50);
        } catch (error) {
            setRecordingState('idle');
            setDuration(0);
            console.error('[Recording] Start failed:', error);
            onError(`Failed to start recording: ${error}`);
        }
    }, [recordingState, config, onError]);

    const stopRecording = useCallback(async () => {
        if (recordingState !== 'recording') return;

        console.log('[Recording] Stopping...');

        if (timerRef.current) {
            clearInterval(timerRef.current);
            timerRef.current = null;
        }

        if (levelIntervalRef.current) {
            clearInterval(levelIntervalRef.current);
            levelIntervalRef.current = null;
        }

        setRecordingState('processing');
        setAudioLevels([]);

        try {
            console.log('[Recording] Stopping and saving...');

            if (!config) {
                throw new Error('Configuration not loaded');
            }

            // 1. Stop recording and save to session directly in Rust
            // This prevents the "Application not responding" freeze by avoiding 
            // passing huge audio arrays to JS
            const session = await invoke<{ id: string }>('stop_recording_and_save', {
                provider: config.active_provider,
                durationSecs: duration,
            });
            console.log('[Recording] Saved session:', session.id);

            // 2. Transcribe from the saved session
            try {
                // Get API key
                console.log('[API Key] Getting key for provider:', config.active_provider);
                const apiKey = await invoke<string | null>('get_api_key', {
                    provider: config.active_provider
                });

                if (!apiKey) {
                    throw new Error(`No API key configured for ${config.active_provider}. Please add it in Settings.`);
                }

                console.log('[Transcription] Transcribing session:', session.id);
                let transcription = await invoke<string>('transcribe_session', {
                    sessionId: session.id,
                    apiKey,
                });
                console.log('[Transcription] Got result:', transcription.substring(0, 100));

                let rawText = transcription;
                let formatted = false;

                // Format the transcript if enabled
                // Format the transcript if enabled (and not AssemblyAI)
                if (config.format_with_ai && config.active_provider !== 'assemblyai') {
                    // Use the same provider for formatting
                    const formatProvider = config.active_provider;
                    const formatKey = await invoke<string | null>('get_api_key', { provider: formatProvider });

                    if (formatKey) {
                        try {
                            // Select model based on provider
                            const model = formatProvider === 'openai'
                                ? config.openai_formatting_model
                                : config.groq_formatting_model;

                            transcription = await invoke<string>('format_transcript', {
                                text: transcription,
                                provider: formatProvider,
                                apiKey: formatKey,
                                model: model,
                                systemPrompt: config.system_prompt,
                            });
                            formatted = true;
                        } catch (formatError) {
                            console.warn('Formatting failed, using raw transcript:', formatError);
                        }
                    }
                }

                // Update session with successful transcription
                try {
                    await invoke('update_session_transcript', {
                        sessionId: session.id,
                        text: transcription,
                        rawText: formatted ? rawText : null,
                        formatted,
                        success: true,
                        errorMessage: null,
                    });
                    console.log('[Session] Updated with transcription');
                } catch (updateError) {
                    console.warn('[Session] Failed to update:', updateError);
                }

                const result: TranscriptionResult = {
                    text: transcription,
                    raw_text: formatted ? rawText : undefined,
                    provider: config.active_provider,
                    timestamp: Date.now(),
                    formatted,
                    duration: duration,
                };

                onTranscriptionComplete(result);

            } catch (transcriptionError) {
                const errorMsg = String(transcriptionError);
                console.error('[Transcription] Failed:', errorMsg);

                // Mark session as failed
                try {
                    await invoke('update_session_transcript', {
                        sessionId: session.id,
                        text: `[Transcription failed: ${errorMsg}]`,
                        rawText: null,
                        formatted: false,
                        success: false,
                        errorMessage: errorMsg,
                    });
                } catch (updateError) {
                    console.warn('[Session] Failed to mark as failed:', updateError);
                }

                onError(`Transcription failed: ${transcriptionError}`);
            }
        } catch (error) {
            console.error('[Recording] Failed to stop/save:', error);
            onError(`Recording failed: ${error}`);
        } finally {
            setRecordingState('idle');
        }
    }, [recordingState, config, duration, onTranscriptionComplete, onError]);

    const toggleRecording = useCallback(() => {
        console.log('[Toggle] Current state:', recordingState);
        if (recordingState === 'idle') {
            startRecording();
        } else if (recordingState === 'recording') {
            stopRecording();
        }
    }, [recordingState, startRecording, stopRecording]);

    // Register global shortcut
    useEffect(() => {
        if (!config?.shortcut) return;

        const registerShortcut = async () => {
            try {
                // Validate shortcut format - check for NBSP and other whitespace issues
                const shortcut = config.shortcut.replace(/\u00A0/g, 'Space'); // Convert NBSP to Space
                const isInvalid = !shortcut ||
                    shortcut.trim() === '' ||
                    shortcut.endsWith('+') ||
                    shortcut.includes('+ ') ||
                    /\+\s+$/.test(shortcut) || // ends with + followed by whitespace
                    !/\+[A-Za-z0-9]+$/.test(shortcut); // must end with +Key

                if (isInvalid) {
                    console.warn('[Shortcut] Invalid shortcut format, skipping:', JSON.stringify(config.shortcut));
                    return;
                }

                console.log('[Shortcut] Attempting to register:', config.shortcut, 'mode:', config.recording_mode);

                // Unregister previous shortcut if it changed
                if (shortcutRegistered.current && shortcutRegistered.current !== config.shortcut) {
                    try {
                        await unregister(shortcutRegistered.current);
                        console.log('[Shortcut] Unregistered previous:', shortcutRegistered.current);
                    } catch {
                        // Ignore unregister errors
                    }
                    shortcutRegistered.current = null;
                }

                const alreadyRegistered = await isRegistered(config.shortcut);
                console.log('[Shortcut] Already registered:', alreadyRegistered);

                if (!alreadyRegistered) {
                    await register(config.shortcut, (event) => {
                        console.log('[Shortcut] Event:', event.state, 'Mode:', config.recording_mode);

                        const isHoldMode = config.recording_mode === 'hold';

                        if (event.state === 'Pressed') {
                            if (isHoldMode) {
                                // Hold mode: always start on press
                                startRecording();
                            } else {
                                // Toggle mode: toggle on press
                                toggleRecording();
                            }
                        } else if (event.state === 'Released') {
                            if (isHoldMode) {
                                // Hold mode: stop on release
                                stopRecording();
                            }
                            // Toggle mode: ignore release
                        }
                    });
                    shortcutRegistered.current = config.shortcut;
                    console.log('[Shortcut] Successfully registered:', config.shortcut);
                }
            } catch (error) {
                console.error('[Shortcut] Failed to register:', error);
            }
        };

        registerShortcut();

        return () => {
            if (shortcutRegistered.current) {
                unregister(shortcutRegistered.current).catch(() => { });
                shortcutRegistered.current = null;
            }
        };
    }, [config?.shortcut, config?.recording_mode, startRecording, stopRecording, toggleRecording]);

    // Cleanup timers on unmount
    useEffect(() => {
        return () => {
            if (timerRef.current) {
                clearInterval(timerRef.current);
            }
            if (levelIntervalRef.current) {
                clearInterval(levelIntervalRef.current);
            }
        };
    }, []);

    return {
        recordingState,
        duration,
        audioLevels,
        toggleRecording,
        startRecording,
        stopRecording,
    };
}
