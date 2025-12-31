import { motion } from 'framer-motion';
import { Mic, Square } from 'lucide-react';
import type { RecordingState } from '../types';

interface RecordButtonProps {
    state: RecordingState;
    duration: number;
    onToggle: () => void;
}

export function RecordButton({ state, duration, onToggle }: RecordButtonProps) {
    const isRecording = state === 'recording';
    const isProcessing = state === 'processing';

    const formatDuration = (seconds: number): string => {
        const mins = Math.floor(seconds / 60);
        const secs = seconds % 60;
        return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    };

    return (
        <div className="recording-section">
            <div className="record-button-container">
                {isRecording && (
                    <>
                        <motion.div
                            className="record-ring"
                            initial={{ scale: 1, opacity: 0.6 }}
                            animate={{ scale: 1.8, opacity: 0 }}
                            transition={{ duration: 1.5, repeat: Infinity }}
                        />
                        <motion.div
                            className="record-ring"
                            initial={{ scale: 1, opacity: 0.6 }}
                            animate={{ scale: 1.8, opacity: 0 }}
                            transition={{ duration: 1.5, repeat: Infinity, delay: 0.5 }}
                        />
                    </>
                )}

                <motion.button
                    className={`record-button ${isRecording ? 'recording' : ''}`}
                    onClick={onToggle}
                    disabled={isProcessing}
                    whileTap={{ scale: 0.95 }}
                    initial={false}
                    animate={{
                        scale: isProcessing ? 0.95 : 1,
                    }}
                >
                    {isRecording ? (
                        <Square className="icon" fill="white" />
                    ) : (
                        <Mic className="icon" />
                    )}
                </motion.button>
            </div>

            <div className="recording-status">
                {state === 'idle' && (
                    <motion.div
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                    >
                        <h2>Ready to Record</h2>
                        <p>Click the button or use the shortcut</p>
                    </motion.div>
                )}

                {state === 'recording' && (
                    <motion.div
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                    >
                        <div className="recording-duration">{formatDuration(duration)}</div>
                        <p>Recording... Click to stop</p>
                    </motion.div>
                )}

                {state === 'processing' && (
                    <motion.div
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                    >
                        <h2>Processing...</h2>
                        <p>Transcribing your audio</p>
                    </motion.div>
                )}
            </div>

            {state === 'idle' && (
                <motion.div
                    className="shortcut-hint"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    transition={{ delay: 0.2 }}
                >
                    <span>Hold</span>
                    <kbd>⌘</kbd>
                    <kbd>⇧</kbd>
                    <kbd>Space</kbd>
                    <span>to record</span>
                </motion.div>
            )}
        </div>
    );
}
