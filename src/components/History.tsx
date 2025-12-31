import { useState, useEffect } from 'react';
import { X, Clock, Trash2, Copy, Check, AlertCircle, ChevronDown, ChevronUp, RefreshCw, Loader2, Search } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import type { TranscriptionRecord } from '../types';

interface HistoryProps {
    onClose: () => void;
}

export function History({ onClose }: HistoryProps) {
    const [records, setRecords] = useState<TranscriptionRecord[]>([]);
    const [loading, setLoading] = useState(true);
    const [copiedId, setCopiedId] = useState<string | null>(null);
    const [status, setStatus] = useState<{ text: string; type: 'success' | 'error' } | null>(null);
    const [showRaw, setShowRaw] = useState<Record<string, boolean>>({});
    const [expandedId, setExpandedId] = useState<string | null>(null);
    const [retryingId, setRetryingId] = useState<string | null>(null);
    const [searchQuery, setSearchQuery] = useState('');

    useEffect(() => {
        loadHistory();
    }, []);

    const loadHistory = async () => {
        try {
            const history = await invoke<TranscriptionRecord[]>('get_sessions', {
                limit: 50,
                offset: 0,
            });
            console.log('[History] Loaded records:', history);
            setRecords(history);
        } catch (err) {
            console.error('Failed to load history:', err);
            setStatus({ text: 'Failed to load history', type: 'error' });
        } finally {
            setLoading(false);
        }
    };

    const handleCopy = async (e: React.MouseEvent, text: string, id: string) => {
        e.stopPropagation();
        try {
            await writeText(text);
            setCopiedId(id);
            setTimeout(() => setCopiedId(null), 2000);
        } catch {
            setStatus({ text: 'Copy failed', type: 'error' });
            setTimeout(() => setStatus(null), 2000);
        }
    };

    const handleDelete = async (e: React.MouseEvent, id: string) => {
        e.stopPropagation();
        try {
            await invoke('delete_session', { id });
            setRecords(prev => prev.filter(r => r.id !== id));
            if (expandedId === id) setExpandedId(null);
        } catch {
            setStatus({ text: 'Failed to delete', type: 'error' });
            setTimeout(() => setStatus(null), 2000);
        }
    };

    const handleClearAll = async () => {
        try {
            await invoke('clear_sessions');
            setRecords([]);
            setStatus({ text: 'History cleared', type: 'success' });
            setTimeout(() => setStatus(null), 2000);
        } catch (err) {
            setStatus({ text: 'Failed to clear history', type: 'error' });
            setTimeout(() => setStatus(null), 2000);
        }
    };

    const toggleExpand = (id: string) => {
        setExpandedId(expandedId === id ? null : id);
    };

    const handleRetry = async (e: React.MouseEvent, record: TranscriptionRecord) => {
        e.stopPropagation();
        setRetryingId(record.id);
        setStatus({ text: 'Retrying transcription...', type: 'success' });

        try {
            // Get API key
            const apiKey = await invoke<string | null>('get_api_key', {
                provider: record.provider
            });

            if (!apiKey) {
                throw new Error(`No API key for ${record.provider}`);
            }

            // Transcribe using backend session processing (keeps audio off JS thread)
            let transcription = await invoke<string>('transcribe_session', {
                sessionId: record.id,
                apiKey,
            });

            // Update session with result
            await invoke('update_session_transcript', {
                sessionId: record.id,
                text: transcription,
                rawText: null,
                formatted: false,
                success: true,
                errorMessage: null,
            });

            // Reload to show updated record
            await loadHistory();
            setStatus({ text: 'Transcription successful!', type: 'success' });
        } catch (err) {
            console.error('[Retry] Failed:', err);
            setStatus({ text: `Retry failed: ${err}`, type: 'error' });
        } finally {
            setRetryingId(null);
            setTimeout(() => setStatus(null), 3000);
        }
    };

    const formatDate = (timestamp: string) => {
        const date = new Date(timestamp);
        const now = new Date();
        const diff = now.getTime() - date.getTime();

        if (diff < 60000) return 'Just now';
        if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
        if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
        if (diff < 604800000) return `${Math.floor(diff / 86400000)}d ago`;

        return date.toLocaleDateString();
    };

    const formatDuration = (secs: number) => {
        const m = Math.floor(secs / 60);
        const s = Math.round(secs % 60);
        if (m > 0) return `${m}m ${s}s`;
        return `${s}s`;
    };

    const formatFileSize = (bytes: number) => {
        if (bytes < 1024) return `${bytes} B`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
        return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    };

    return (
        <div className="panel">
            {/* Header */}
            <div className="panel-header">
                <h2>History</h2>
                <div className="panel-header-actions">
                    {records.length > 0 && (
                        <button className="text-btn danger" onClick={handleClearAll}>
                            Clear All
                        </button>
                    )}
                    <button className="close-btn" onClick={onClose}>
                        <X size={18} />
                    </button>
                </div>
            </div>

            {/* Content */}
            <div className="panel-content">
                {/* Inline Status */}
                {status && (
                    <div className={`inline-status ${status.type}`}>
                        {status.type === 'error' && <AlertCircle size={14} />}
                        {status.type === 'success' && <Check size={14} />}
                        <span>{status.text}</span>
                    </div>
                )}

                {loading ? (
                    <div className="panel-empty">
                        <div className="spinner" />
                    </div>
                ) : records.length === 0 ? (
                    <div className="panel-empty">
                        <Clock size={32} />
                        <p>No transcriptions yet</p>
                        <span>Your recordings will appear here</span>
                    </div>
                ) : (
                    <>
                        {/* Search Input */}
                        <div className="search-container">
                            <Search size={16} className="search-icon" />
                            <input
                                type="text"
                                className="search-input"
                                placeholder="Search transcripts..."
                                value={searchQuery}
                                onChange={e => setSearchQuery(e.target.value)}
                            />
                        </div>
                        <div className="history-list">
                            {records
                                .filter(r =>
                                    searchQuery === '' ||
                                    r.text.toLowerCase().includes(searchQuery.toLowerCase()) ||
                                    (r.raw_text && r.raw_text.toLowerCase().includes(searchQuery.toLowerCase()))
                                )
                                .map(record => {
                                    const isExpanded = expandedId === record.id;
                                    const displayText = showRaw[record.id] ? (record.raw_text || record.text) : record.text;

                                    return (
                                        <div
                                            key={record.id}
                                            className={`history-item ${isExpanded ? 'expanded' : ''}`}
                                            onClick={() => toggleExpand(record.id)}
                                        >
                                            <div className="history-item-header">
                                                <div className="history-meta">
                                                    <span className="meta-time">{formatDate(record.timestamp)}</span>
                                                    <span className="meta-provider">{record.provider}</span>
                                                    <span className="meta-duration">{formatDuration(record.duration_secs)}</span>
                                                    {record.file_size_bytes && (
                                                        <span className="meta-size">{formatFileSize(record.file_size_bytes)}</span>
                                                    )}
                                                    {record.status === 'Pending' && (
                                                        <span className="status-badge pending">⏳ Pending</span>
                                                    )}
                                                    {record.status === 'Failed' && (
                                                        <span className="status-badge failed">❌ Failed</span>
                                                    )}
                                                </div>
                                                <div className="history-item-actions">
                                                    {record.status === 'Failed' && (
                                                        <button
                                                            className="icon-btn retry"
                                                            onClick={(e) => handleRetry(e, record)}
                                                            disabled={retryingId === record.id}
                                                            title="Retry transcription"
                                                        >
                                                            {retryingId === record.id ? (
                                                                <Loader2 size={14} className="spinner-icon" />
                                                            ) : (
                                                                <RefreshCw size={14} />
                                                            )}
                                                        </button>
                                                    )}
                                                    <button
                                                        className="icon-btn"
                                                        onClick={(e) => handleCopy(e, displayText, record.id)}
                                                        title="Copy text"
                                                    >
                                                        {copiedId === record.id ? <Check size={14} /> : <Copy size={14} />}
                                                    </button>
                                                    <button
                                                        className="icon-btn danger"
                                                        onClick={(e) => handleDelete(e, record.id)}
                                                        title="Delete"
                                                    >
                                                        <Trash2 size={14} />
                                                    </button>
                                                    <div className="expand-icon">
                                                        {isExpanded ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
                                                    </div>
                                                </div>
                                            </div>

                                            {isExpanded && (
                                                <div className="history-expanded-content" onClick={e => e.stopPropagation()}>
                                                    {record.audio_path && (
                                                        <div className="audio-player-container">
                                                            <audio
                                                                controls
                                                                className="native-audio"
                                                                src={convertFileSrc(record.audio_path)}
                                                            />
                                                        </div>
                                                    )}


                                                </div>
                                            )}

                                            <div className={`history-content ${isExpanded ? 'expanded' : ''}`}>
                                                {record.formatted && record.raw_text ? (
                                                    <div className="stacked-view">
                                                        <div className="transcript-section">
                                                            <div className="section-label">Formatted</div>
                                                            <p className="history-text scrollable-text">
                                                                {record.text}
                                                            </p>
                                                        </div>
                                                        <div className="transcript-section raw">
                                                            <div className="section-header" onClick={() => setShowRaw(prev => ({ ...prev, [record.id]: !prev[record.id] }))}>
                                                                <span>Raw Transcript</span>
                                                                {showRaw[record.id] ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                                                            </div>
                                                            {showRaw[record.id] && (
                                                                <p className="history-text raw-text scrollable-text">
                                                                    {record.raw_text}
                                                                </p>
                                                            )}
                                                        </div>
                                                    </div>
                                                ) : (
                                                    <p className="history-text scrollable-text">
                                                        {displayText}
                                                    </p>
                                                )}
                                            </div>
                                        </div>
                                    );
                                })}
                        </div>
                    </>
                )}
            </div>
        </div>
    );
}
