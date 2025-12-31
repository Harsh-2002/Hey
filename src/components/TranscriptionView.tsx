import { motion } from 'framer-motion';
import { Copy, ClipboardPaste, Check } from 'lucide-react';
import { useState } from 'react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { invoke } from '@tauri-apps/api/core';
import type { TranscriptionResult, Config } from '../types';

interface TranscriptionViewProps {
    result: TranscriptionResult;
    config: Config | null;
    onSuccess: (message: string) => void;
    onError: (message: string) => void;
}

export function TranscriptionView({ result, config, onSuccess, onError }: TranscriptionViewProps) {
    const [copied, setCopied] = useState(false);
    const [pasted, setPasted] = useState(false);

    const handleCopy = async () => {
        try {
            await writeText(result.text);
            setCopied(true);
            onSuccess('Copied to clipboard!');
            setTimeout(() => setCopied(false), 2000);
        } catch (error) {
            onError('Failed to copy to clipboard');
        }
    };

    const handlePaste = async () => {
        try {
            // First copy to clipboard
            await writeText(result.text);
            // Then simulate paste
            await invoke('paste_to_window');
            setPasted(true);
            onSuccess('Pasted to active window!');
            setTimeout(() => setPasted(false), 2000);
        } catch (error) {
            onError('Failed to paste. Make sure Accessibility is enabled.');
        }
    };

    const providerLabel = {
        openai: 'OpenAI Whisper',
        groq: 'Groq',
        assemblyai: 'AssemblyAI',
    }[result.provider];

    return (
        <motion.div
            className="transcription-section"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.3 }}
        >
            <div className="transcription-card">
                <div className="transcription-header">
                    <h3>
                        Transcription
                        {result.formatted && <span style={{ opacity: 0.5, fontWeight: 400 }}> · Formatted</span>}
                    </h3>
                    <div className="transcription-actions">
                        <button className="action-btn" onClick={handleCopy}>
                            {copied ? <Check size={14} /> : <Copy size={14} />}
                            {copied ? 'Copied!' : 'Copy'}
                        </button>
                        {config?.auto_paste && (
                            <button className="action-btn primary" onClick={handlePaste}>
                                {pasted ? <Check size={14} /> : <ClipboardPaste size={14} />}
                                {pasted ? 'Pasted!' : 'Paste'}
                            </button>
                        )}
                    </div>
                </div>
                <div className="transcription-content">
                    <p className="transcription-text">{result.text}</p>
                </div>
                <div style={{
                    padding: '8px 16px',
                    borderTop: '1px solid var(--border-color)',
                    fontSize: '11px',
                    color: 'var(--text-tertiary)',
                    display: 'flex',
                    justifyContent: 'space-between'
                }}>
                    <span>via {providerLabel}</span>
                    <span>{new Date(result.timestamp).toLocaleTimeString()}</span>
                </div>
            </div>
        </motion.div>
    );
}
