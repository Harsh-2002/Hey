import { useState, useCallback } from 'react';
import { FileVideo, FileAudio, X, Loader2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

interface AudioUploadProps {
    provider: string;
    formatWithAi: boolean;
    onTranscription: (text: string, duration: number, rawText?: string) => void;
    onError: (message: string) => void;
    onClose: () => void;
}

// Supported file extensions
const AUDIO_EXTENSIONS = ['wav', 'mp3', 'm4a', 'aac', 'ogg', 'flac', 'webm'];
const VIDEO_EXTENSIONS = ['mp4', 'mov', 'm4v', 'avi', 'mkv', 'webm'];
const ALL_EXTENSIONS = [...AUDIO_EXTENSIONS, ...VIDEO_EXTENSIONS];

export function AudioUpload({
    provider,
    formatWithAi,
    onTranscription,
    onError,
    onClose,
}: AudioUploadProps) {
    const [processing, setProcessing] = useState(false);
    const [progress, setProgress] = useState(0);
    const [fileName, setFileName] = useState<string | null>(null);
    const [status, setStatus] = useState('');

    const processFile = async (filePath: string, name: string) => {
        console.log('[Upload] Processing file:', filePath);
        setFileName(name);
        setProcessing(true);
        setProgress(10);
        setStatus('Extracting audio...');

        try {
            // Get API key
            const apiKey = await invoke<string | null>('get_api_key', { provider });
            if (!apiKey) {
                throw new Error(`No API key configured for ${provider}. Please add it in Settings.`);
            }

            setProgress(20);

            // Check if it's a video that needs extraction, or audio we can read directly
            const ext = name.split('.').pop()?.toLowerCase() || '';
            let audioData: number[];

            if (VIDEO_EXTENSIONS.includes(ext) || !['wav'].includes(ext)) {
                // Extract/convert audio using native macOS afconvert
                setStatus('Converting audio...');
                console.log('[Upload] Extracting audio from:', ext);

                const extractedBytes = await invoke<number[]>('extract_audio_from_file', {
                    filePath
                });
                audioData = extractedBytes;
                console.log('[Upload] Extracted audio bytes:', audioData.length);
            } else {
                // It's already a WAV, read it directly
                setStatus('Reading file...');
                // For WAV files, we need to read via the file system
                // Since Tauri's file system API, we use the extraction which also works for audio
                const extractedBytes = await invoke<number[]>('extract_audio_from_file', {
                    filePath
                });
                audioData = extractedBytes;
            }

            setProgress(50);
            setStatus('Transcribing...');

            // Transcribe
            const text = await invoke<string>('transcribe_audio', {
                audioData,
                provider,
                apiKey,
            });

            console.log('[Upload] Transcription result:', text.substring(0, 100));
            setProgress(80);

            let rawText = text;

            // Optionally format
            let finalText = text;
            if (formatWithAi) {
                setStatus('Formatting...');
                try {
                    const openaiKey = await invoke<string | null>('get_api_key', { provider: 'openai' });
                    if (openaiKey) {
                        finalText = await invoke<string>('format_transcript', {
                            text,
                            apiKey: openaiKey,
                        });
                    }
                } catch {
                    // Use unformatted if formatting fails
                }
            }

            setProgress(100);

            // Estimate duration from audio data size (16kHz, 16-bit mono)
            const durationEstimate = audioData.length / (16000 * 2);

            onTranscription(finalText, durationEstimate, formatWithAi ? rawText : undefined);
        } catch (error) {
            console.error('[Upload] Error:', error);
            onError(`${error}`);
        } finally {
            setProcessing(false);
            setProgress(0);
            setFileName(null);
            setStatus('');
        }
    };

    const handleChooseFile = async () => {
        try {
            // Use Tauri's native file dialog
            const selected = await open({
                multiple: false,
                directory: false,
                filters: [{
                    name: 'Media Files',
                    extensions: ALL_EXTENSIONS
                }]
            });

            if (selected && typeof selected === 'string') {
                const name = selected.split('/').pop() || 'file';
                await processFile(selected, name);
            }
        } catch (error) {
            console.error('[Upload] File dialog error:', error);
            onError(`Failed to select file: ${error}`);
        }
    };

    const handleDrop = useCallback(async (e: React.DragEvent) => {
        e.preventDefault();

        // Get the dropped file path from Tauri
        // Note: Native drag-drop provides file paths in Tauri
        const files = e.dataTransfer.files;
        if (files.length > 0) {
            const file = files[0];
            const ext = file.name.split('.').pop()?.toLowerCase() || '';

            if (!ALL_EXTENSIONS.includes(ext)) {
                onError('Unsupported file type. Supported: ' + ALL_EXTENSIONS.join(', '));
                return;
            }

            // For web File objects, we need to use readAsArrayBuffer
            // But for proper video support, we should use the native dialog
            onError('Please use "Choose File" button for video files');
        }
    }, [onError]);

    return (
        <div className="panel">
            {/* Header */}
            <div className="panel-header">
                <h2>Upload Media</h2>
                <button className="close-btn" onClick={onClose}>
                    <X size={18} />
                </button>
            </div>

            {/* Content */}
            <div className="panel-content centered">
                {processing ? (
                    <div className="upload-processing">
                        <Loader2 className="spinner-icon" size={32} />
                        <p className="file-name">{fileName}</p>
                        <div className="progress-bar">
                            <div className="progress-fill" style={{ width: `${progress}%` }} />
                        </div>
                        <p className="status-text">{status}</p>
                    </div>
                ) : (
                    <div
                        className="upload-dropzone"
                        onDragOver={(e) => e.preventDefault()}
                        onDrop={handleDrop}
                    >
                        <div style={{ display: 'flex', gap: '8px' }}>
                            <FileAudio size={28} />
                            <FileVideo size={28} />
                        </div>
                        <p>Upload audio or video</p>
                        <span>Extract audio and transcribe</span>
                        <button className="primary-btn" onClick={handleChooseFile}>
                            Choose File
                        </button>
                        <p className="hint">MP4, MOV, MP3, WAV, M4A, OGG, and more</p>
                    </div>
                )}
            </div>
        </div>
    );
}
