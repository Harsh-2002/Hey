import { useState, useCallback, useEffect } from 'react';
import { Mic, Copy, Check, AlertCircle } from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import './App.css';

import { Settings } from './components/Settings';
import { Onboarding } from './components/Onboarding';
import { History } from './components/History';
import { AudioUpload } from './components/AudioUpload';
import { Waveform } from './components/Waveform';
import { useRecording } from './hooks/useRecording';
import { useSettings } from './hooks/useSettings';
import type { TranscriptionResult } from './types';

type View = 'main' | 'settings' | 'history' | 'upload';

function App() {
  const [currentView, setCurrentView] = useState<View>('main');
  const [transcription, setTranscription] = useState<TranscriptionResult | null>(null);
  const [copied, setCopied] = useState(false);
  const [statusMessage, setStatusMessage] = useState<{ text: string; type: 'success' | 'error' } | null>(null);

  const {
    config,
    saveConfig,
    loading: configLoading,
    hasCompletedOnboarding,
    completeOnboarding,
    resetOnboarding,
  } = useSettings();

  const handleTranscriptionComplete = useCallback((result: TranscriptionResult) => {
    setTranscription(result);
  }, []);

  const handleError = useCallback((message: string) => {
    setStatusMessage({ text: message, type: 'error' });
    setTimeout(() => setStatusMessage(null), 4000);
  }, []);

  const { recordingState, duration, audioLevels, toggleRecording } = useRecording({
    config,
    onTranscriptionComplete: handleTranscriptionComplete,
    onError: handleError,
  });

  // Listen for tray menu events
  useEffect(() => {
    const unlisten = listen('open-settings', () => setCurrentView('settings'));
    return () => { unlisten.then(fn => fn()); };
  }, []);

  // Handle upload transcription
  const handleUploadTranscription = useCallback(async (text: string, dur: number, rawText?: string) => {
    const result: TranscriptionResult = {
      text,
      raw_text: rawText,
      provider: config?.active_provider || 'openai',
      timestamp: Date.now(),
      formatted: config?.format_with_ai || false,
      duration: dur,
    };

    if (config?.save_history !== false) {
      try {
        await invoke('save_session', {
          text,
          rawText: rawText || null,
          provider: config?.active_provider || 'openai',
          durationSecs: dur,
          audioData: null,
          formatted: config?.format_with_ai || false,
        });
      } catch (e) {
        console.warn('Failed to save session:', e);
      }
    }

    setTranscription(result);
    setCurrentView('main');
  }, [config]);

  const handleCopy = async () => {
    if (!transcription) return;
    try {
      await writeText(transcription.text);
      setCopied(true);
      // Reset to idle state after 2 seconds
      setTimeout(() => {
        setCopied(false);
        setTranscription(null);
      }, 2000);
    } catch {
      setStatusMessage({ text: 'Copy failed', type: 'error' });
    }
  };

  const formatDuration = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}:${s.toString().padStart(2, '0')}`;
  };

  // Loading
  if (configLoading || hasCompletedOnboarding === null) {
    return (
      <div className="app">
        <div className="app-center">
          <div className="spinner" />
        </div>
      </div>
    );
  }

  // Onboarding
  if (!hasCompletedOnboarding) {
    return <Onboarding onComplete={completeOnboarding} />;
  }

  return (
    <div className="app">
      {currentView === 'main' && (
        <div className="main-view">
          {/* Recording Area */}
          <div className={`record-area ${transcription ? 'has-result' : ''}`}>
            <button
              className={`mic-btn ${recordingState}`}
              onClick={toggleRecording}
              disabled={recordingState === 'processing'}
            >
              <Mic size={28} />
            </button>

            {recordingState === 'idle' && !transcription && !statusMessage && (
              <div className="status-container">
                <p className="status-text">Tap to record</p>
                <p className="status-shortcut">or hold {config?.shortcut.split('+').map(k => k === 'CommandOrControl' ? '⌘' : k === 'Option' ? '⌥' : k).join(' + ')}</p>
              </div>
            )}

            {recordingState === 'recording' && (
              <>
                <p className="status-text recording">{formatDuration(duration)}</p>
                <Waveform levels={audioLevels} isRecording={true} />
              </>
            )}

            {recordingState === 'processing' && (
              <p className="status-text">Transcribing...</p>
            )}

            {/* Inline Status Message */}
            {statusMessage && recordingState === 'idle' && (
              <div className={`inline-status ${statusMessage.type}`}>
                {statusMessage.type === 'error' && <AlertCircle size={14} />}
                <span>{statusMessage.text}</span>
              </div>
            )}
          </div>

          {/* Transcription Result */}
          {transcription && recordingState === 'idle' && (
            <div className="result-card">
              <div className="result-text">{transcription.text}</div>
              <button className="copy-btn" onClick={handleCopy}>
                {copied ? <Check size={16} /> : <Copy size={16} />}
                {copied ? 'Copied' : 'Copy'}
              </button>
            </div>
          )}

          {/* Bottom Nav */}
          <div className="bottom-nav">
            <button className="nav-btn" onClick={() => setCurrentView('history')}>
              History
            </button>
            <button className="nav-btn" onClick={() => setCurrentView('upload')}>
              Upload
            </button>
            <button className="nav-btn" onClick={() => setCurrentView('settings')}>
              Settings
            </button>
          </div>
        </div>
      )}

      {currentView === 'settings' && config && (
        <Settings
          config={config}
          onSave={saveConfig}
          onClose={() => setCurrentView('main')}
          onReset={() => {
            resetOnboarding();
            window.location.reload();
          }}
        />
      )}

      {currentView === 'history' && (
        <History onClose={() => setCurrentView('main')} />
      )}

      {currentView === 'upload' && config && (
        <AudioUpload
          provider={config.active_provider}
          formatWithAi={config.format_with_ai}
          onTranscription={handleUploadTranscription}
          onError={handleError}
          onClose={() => setCurrentView('main')}
        />
      )}
    </div>
  );
}

export default App;
