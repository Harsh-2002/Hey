import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import './RecordingOverlay.css';

type OverlayStatus = 'idle' | 'recording' | 'processing';

interface OverlayPayload {
    status: OverlayStatus;
    levels?: number[];
}

export function RecordingOverlay() {
    const [status, setStatus] = useState<OverlayStatus>('idle');
    const [levels, setLevels] = useState<number[]>([0, 0, 0, 0, 0]);

    useEffect(() => {
        const unlisten = listen<OverlayPayload>('overlay-update', (event) => {
            setStatus(event.payload.status);
            if (event.payload.levels) {
                setLevels(event.payload.levels);
            }
        });

        return () => {
            unlisten.then(fn => fn());
        };
    }, []);

    if (status === 'idle') {
        return null;
    }

    return (
        <div className={`overlay-container ${status}`}>
            <div className="overlay-content">
                {status === 'recording' ? (
                    <div className="waveform">
                        {levels.map((level, i) => (
                            <div
                                key={i}
                                className="waveform-bar"
                                style={{ height: `${Math.max(4, Math.min(24, level * 24))}px` }}
                            />
                        ))}
                    </div>
                ) : (
                    <>
                        <div className="pulse-indicator" />
                        <span className="overlay-text">Processing</span>
                    </>
                )}
            </div>
        </div>
    );
}
