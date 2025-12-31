# Hey

A lightweight macOS menu bar app for voice transcription. Record audio with a global shortcut, get text back in seconds.

## What it does

- Records audio from your mic or system audio
- Transcribes using OpenAI Whisper, Groq, or AssemblyAI
- Optionally cleans up the transcript with AI
- Copies result to clipboard
- Keeps a searchable history

## Install

Download the latest `.dmg` from [Releases](https://github.com/Harsh-2002/Hey/releases), or build from source:

```bash
git clone https://github.com/Harsh-2002/Hey.git
cd Hey
npm install
npm run tauri build
```

The app bundle will be at `src-tauri/target/release/bundle/macos/Hey.app`.

## Requirements

- macOS 12+
- An API key from OpenAI, Groq, or AssemblyAI

## Usage

1. Open Hey from your menu bar
2. Add your API key in Settings
3. Press `Cmd+Shift+Space` to record (configurable)
4. Press again to stop, or hold if using push-to-talk mode
5. Text appears and is copied to clipboard

## Configuration

All settings are in the app. Key options:

| Setting | Description |
|---------|-------------|
| Provider | OpenAI, Groq, or AssemblyAI |
| Shortcut | Global hotkey for recording |
| Push-to-Talk | Hold to record vs toggle |
| Format with AI | Clean up punctuation and filler words |
| Launch at Login | Start Hey when your Mac boots |

API keys are stored in macOS Keychain. Audio and transcripts are saved in `~/.hey/`.

## Development

```bash
npm install
npm run tauri dev
```

Requires Node.js 18+ and Rust (latest stable).

## Stack

- Frontend: React, TypeScript, Vite
- Backend: Rust, Tauri 2.0
- Audio: CoreAudio (native macOS)

## License

MIT

---

Built by [Anurag Vishwakarma](https://firstfinger.io)
