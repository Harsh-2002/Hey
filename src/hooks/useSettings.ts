import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Config } from '../types';

const defaultConfig: Config = {
    // API keys are stored in macOS Keychain, not in config
    active_provider: 'openai',
    shortcut: 'CommandOrControl+Shift+Space',
    auto_paste: false,
    format_with_ai: true,
    openai_formatting_model: 'gpt-4o-mini',
    groq_formatting_model: 'llama-3.1-8b-instant',
    system_prompt: 'You are a transcript formatter. Clean up the following speech-to-text transcript by fixing punctuation, capitalization, and minor errors. Keep the original meaning and words as much as possible. Output only the cleaned text, nothing else.',
    selected_device: null,
    save_history: true,
    launch_at_login: false,
    recording_mode: 'toggle',
};

const ONBOARDING_KEY = 'hey_onboarding_complete';

export function useSettings() {
    const [config, setConfig] = useState<Config | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [hasCompletedOnboarding, setHasCompletedOnboarding] = useState<boolean | null>(null);

    // Check onboarding status
    useEffect(() => {
        const completed = localStorage.getItem(ONBOARDING_KEY) === 'true';
        setHasCompletedOnboarding(completed);
    }, []);

    // Load config on mount
    useEffect(() => {
        loadConfig();
    }, []);

    const loadConfig = async () => {
        try {
            setLoading(true);
            const loadedConfig = await invoke<Config>('get_config');

            // Auto-fix invalid shortcuts
            const isShortcutInvalid = !loadedConfig.shortcut ||
                loadedConfig.shortcut.includes('\u00A0') || // NBSP
                loadedConfig.shortcut.endsWith('+') ||
                !/\+[A-Za-z0-9]+$/.test(loadedConfig.shortcut);

            if (isShortcutInvalid) {
                console.warn('[Settings] Invalid shortcut detected, resetting to default:', loadedConfig.shortcut);
                loadedConfig.shortcut = defaultConfig.shortcut;
                // Save the fixed config
                await invoke('save_config', { config: loadedConfig });
            }

            setConfig(loadedConfig);
            setError(null);
        } catch (err) {
            console.error('Failed to load config:', err);
            setConfig(defaultConfig);
            setError('Failed to load settings');
        } finally {
            setLoading(false);
        }
    };

    const saveConfig = useCallback(async (newConfig: Config) => {
        try {
            await invoke('save_config', { config: newConfig });
            setConfig(newConfig);
            setError(null);
            return true;
        } catch (err) {
            console.error('Failed to save config:', err);
            setError('Failed to save settings');
            return false;
        }
    }, []);

    const updateConfig = useCallback((updates: Partial<Config>) => {
        if (!config) return;
        const newConfig = { ...config, ...updates };
        saveConfig(newConfig);
    }, [config, saveConfig]);

    const completeOnboarding = useCallback(() => {
        localStorage.setItem(ONBOARDING_KEY, 'true');
        setHasCompletedOnboarding(true);
    }, []);

    const resetOnboarding = useCallback(() => {
        localStorage.removeItem(ONBOARDING_KEY);
        setHasCompletedOnboarding(false);
    }, []);

    return {
        config,
        loading,
        error,
        saveConfig,
        updateConfig,
        reloadConfig: loadConfig,
        hasCompletedOnboarding,
        completeOnboarding,
        resetOnboarding,
    };
}
