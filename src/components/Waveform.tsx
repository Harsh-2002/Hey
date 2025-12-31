import { useRef, useEffect } from 'react';

interface WaveformProps {
    levels: number[];
    isRecording: boolean;
}

export function Waveform({ levels, isRecording }: WaveformProps) {
    const canvasRef = useRef<HTMLCanvasElement>(null);

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        // Get actual canvas size
        const rect = canvas.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;

        canvas.width = rect.width * dpr;
        canvas.height = rect.height * dpr;
        ctx.scale(dpr, dpr);

        const width = rect.width;
        const height = rect.height;
        const centerY = height / 2;

        // Clear canvas
        ctx.clearRect(0, 0, width, height);

        if (levels.length === 0) {
            // Draw static line when not recording
            ctx.beginPath();
            ctx.moveTo(0, centerY);
            ctx.lineTo(width, centerY);
            ctx.strokeStyle = getComputedStyle(document.documentElement)
                .getPropertyValue('--text-tertiary').trim() || '#636366';
            ctx.lineWidth = 1;
            ctx.stroke();
            return;
        }

        // Calculate bar width
        const numBars = Math.min(levels.length, 50);
        const barWidth = width / numBars;
        const gap = 2;

        // Get color from CSS variable
        const color = isRecording
            ? '#ff453a' // Red when recording
            : getComputedStyle(document.documentElement)
                .getPropertyValue('--accent-primary').trim() || '#007aff';

        // Draw bars
        for (let i = 0; i < numBars; i++) {
            const levelIndex = Math.floor((i / numBars) * levels.length);
            const level = levels[levelIndex] || 0;

            // Normalize and scale the level (RMS is typically 0-0.3)
            const normalizedLevel = Math.min(level * 4, 1);
            const barHeight = Math.max(normalizedLevel * (height - 4), 2);

            const x = i * barWidth + gap / 2;
            const y = centerY - barHeight / 2;

            // Draw rounded bar
            ctx.fillStyle = color;
            ctx.beginPath();
            ctx.roundRect(x, y, barWidth - gap, barHeight, 2);
            ctx.fill();
        }
    }, [levels, isRecording]);

    return (
        <div className="waveform-container">
            <canvas ref={canvasRef} className="waveform-canvas" />
        </div>
    );
}
