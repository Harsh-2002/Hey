import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import './RecordingOverlay.css';

type OverlayStatus = 'idle' | 'recording' | 'processing';

interface OverlayPayload {
    status: OverlayStatus;
    duration?: number;
}

export function RecordingOverlay() {
    const [status, setStatus] = useState<OverlayStatus>('idle');
    const [duration, setDuration] = useState(0);

    useEffect(() => {
        const unlisten = listen<OverlayPayload>('overlay-update', (event) => {
            setStatus(event.payload.status);
            if (event.payload.duration !== undefined) {
                setDuration(event.payload.duration);
            }
        });

        return () => {
            unlisten.then(fn => fn());
        };
    }, []);

    const formatDuration = (secs: number) => {
        const m = Math.floor(secs / 60);
        const s = secs % 60;
        return `${m}:${s.toString().padStart(2, '0')}`;
    };

    if (status === 'idle') {
        return null;
    }

    return (
        <div className={`overlay-container ${status}`}>
            <div className="overlay-content">
                <div className="pulse-indicator" />
                <span className="overlay-text">
                    {status === 'recording' ? formatDuration(duration) : 'Processing...'}
                </span>
            </div>
        </div>
    );
}
