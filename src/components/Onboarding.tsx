import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ChevronRight, Mic, Keyboard, Shield, CheckCircle2 } from 'lucide-react';
import './Onboarding.css';

interface OnboardingProps {
    onComplete: () => void;
}

interface OnboardingStep {
    id: string;
    title: string;
    description: string;
    icon?: React.ReactNode;
    content: React.ReactNode;
}

export function Onboarding({ onComplete }: OnboardingProps) {
    const [currentStep, setCurrentStep] = useState(0);
    const [tosAccepted, setTosAccepted] = useState(false);

    const steps: OnboardingStep[] = [
        {
            id: 'welcome',
            title: 'Welcome to Hey',
            description: 'Your personal voice-to-text assistant',
            // icon removed to avoid duplication with app-icon-large
            content: (
                <div className="onboarding-welcome">
                    <div className="app-icon-large">
                        <img src="/128-mac.png" alt="Hey" />
                    </div>

                    <div className="onboarding-card">
                        <p className="welcome-text">
                            Hey transforms your voice into perfectly formatted text.
                            Just speak, and let AI do the rest.
                        </p>
                        <div className="feature-list">
                            <div className="feature-item">
                                <CheckCircle2 size={16} />
                                <span>Record with a single click or shortcut</span>
                            </div>
                            <div className="feature-item">
                                <CheckCircle2 size={16} />
                                <span>AI-powered transcription & formatting</span>
                            </div>
                            <div className="feature-item">
                                <CheckCircle2 size={16} />
                                <span>Paste directly to any app</span>
                            </div>
                        </div>
                    </div>
                </div>
            ),
        },
        {
            id: 'tos',
            title: 'Terms of Service',
            description: 'Please review and accept to continue',
            icon: <Shield size={32} />,
            content: (
                <div className="onboarding-tos">
                    <div className="onboarding-card">
                        <div className="feature-list">
                            <div className="feature-item">
                                <CheckCircle2 size={16} />
                                <span><strong>Privacy:</strong> Audio sent to your chosen provider (OpenAI/Groq/AssemblyAI). We don't store your data.</span>
                            </div>
                            <div className="feature-item">
                                <CheckCircle2 size={16} />
                                <span><strong>API Keys:</strong> Stored securely in your macOS Keychain.</span>
                            </div>
                            <div className="feature-item">
                                <CheckCircle2 size={16} />
                                <span><strong>Permissions:</strong> Microphone required. Accessibility optional for auto-paste.</span>
                            </div>
                        </div>
                    </div>

                    <label className="tos-checkbox">
                        <input
                            type="checkbox"
                            checked={tosAccepted}
                            onChange={(e) => setTosAccepted(e.target.checked)}
                        />
                        <span className="checkmark" />
                        <span>I have read and agree to the Terms of Service</span>
                    </label>
                </div>
            ),
        },
        {
            id: 'shortcuts',
            title: 'Quick Recording',
            description: 'Learn how to use keyboard shortcuts',
            icon: <Keyboard size={32} />,
            content: (
                <div className="onboarding-shortcuts">
                    <div className="onboarding-card">
                        <div className="shortcut-demo">
                            <div className="shortcut-visual">
                                <div className="key-combo">
                                    <kbd>⌥</kbd>
                                    <span>+</span>
                                    <kbd>Space</kbd>
                                </div>
                                <p className="shortcut-action">Hold to record, release to stop</p>
                            </div>
                        </div>

                        <div className="shortcut-tips">
                            <div className="tip">
                                <div className="tip-icon">1</div>
                                <div className="tip-text">
                                    <strong>Hold shortcut</strong>
                                    <span>Speak your message</span>
                                </div>
                            </div>
                            <div className="tip">
                                <div className="tip-icon">2</div>
                                <div className="tip-text">
                                    <strong>Release</strong>
                                    <span>AI processes audio</span>
                                </div>
                            </div>
                        </div>

                        <p className="shortcut-note">
                            💡 Customize in Settings later
                        </p>
                    </div>
                </div>
            ),
        },
        {
            id: 'permissions',
            title: 'Almost Ready',
            description: 'Grant permissions to get started',
            icon: <Shield size={32} />,
            content: (
                <div className="onboarding-permissions">
                    <div className="onboarding-card">
                        <div className="permission-item">
                            <div className="permission-icon">
                                <Mic size={24} />
                            </div>
                            <div className="permission-info">
                                <strong>Microphone Access</strong>
                                <span>Required to record</span>
                            </div>
                            <div className="permission-status granted">
                                <CheckCircle2 size={16} />
                            </div>
                        </div>

                        <div className="permission-item">
                            <div className="permission-icon">
                                <Keyboard size={24} />
                            </div>
                            <div className="permission-info">
                                <strong>Accessibility</strong>
                                <span>Optional (Auto-paste)</span>
                            </div>
                            <div className="permission-status optional">
                                Optional
                            </div>
                        </div>
                    </div>

                    <p className="permission-note">
                        Change anytime in System Settings.
                    </p>
                </div>
            ),
        },
    ];

    const currentStepData = steps[currentStep];
    const isLastStep = currentStep === steps.length - 1;
    const canProceed = currentStep !== 1 || tosAccepted;

    const handleNext = () => {
        if (isLastStep) {
            onComplete();
        } else {
            setCurrentStep(prev => prev + 1);
        }
    };

    const handleBack = () => {
        if (currentStep > 0) {
            setCurrentStep(prev => prev - 1);
        }
    };

    return (
        <div className="onboarding-overlay">
            <motion.div
                className="onboarding-container"
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ duration: 0.3 }}
            >
                {/* Progress dots */}
                <div className="onboarding-progress">
                    {steps.map((step, index) => (
                        <div
                            key={step.id}
                            className={`progress-dot ${index === currentStep ? 'active' : ''} ${index < currentStep ? 'completed' : ''}`}
                        />
                    ))}
                </div>

                {/* Step content */}
                <AnimatePresence mode="wait">
                    <motion.div
                        key={currentStep}
                        className="onboarding-content"
                        initial={{ opacity: 0, x: 20 }}
                        animate={{ opacity: 1, x: 0 }}
                        exit={{ opacity: 0, x: -20 }}
                        transition={{ duration: 0.2 }}
                    >
                        <div className="step-header">
                            {currentStepData.icon && <div className="step-icon">{currentStepData.icon}</div>}
                            <h2>{currentStepData.title}</h2>
                            <p>{currentStepData.description}</p>
                        </div>

                        <div className="step-body">
                            {currentStepData.content}
                        </div>
                    </motion.div>
                </AnimatePresence>

                {/* Navigation */}
                <div className="onboarding-nav">
                    {currentStep > 0 ? (
                        <button className="nav-btn secondary" onClick={handleBack}>
                            Back
                        </button>
                    ) : (
                        <div />
                    )}

                    <button
                        className="nav-btn primary"
                        onClick={handleNext}
                        disabled={!canProceed}
                    >
                        {isLastStep ? 'Get Started' : 'Continue'}
                        <ChevronRight size={16} />
                    </button>
                </div>
            </motion.div>
        </div>
    );
}
