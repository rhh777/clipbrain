# ClipBrain

> A local-first AI clipboard assistant for macOS, built with Tauri 2, SolidJS, and Rust.

[中文说明](./README_ZH.md)

ClipBrain monitors your clipboard in the background, detects content types automatically (URL, code, JSON, tables, images, and more), and provides context-aware actions such as translation, summarization, formatting, explanation, and sensitive data masking. When connected to a vision-capable OpenAI-compatible model, it can also run OCR, generate image descriptions, and answer custom questions about clipboard images. All history is stored in local SQLite, and API keys are managed through the system Keychain.

<p align="center">
  <img src="docs/clip_history.png" alt="Clipboard history panel" width="720" />
</p>
<p align="center">
  <img src="docs/clip_history_image.png" alt="Image history and detail panel" width="720" />
</p>

## Features

- **Clipboard history**: Automatically captures text, images, and files with tags, search, and time-range filtering
- **Smart classification**: Identifies content types with rules plus optional LLM support
- **AI actions**: Built-in URL preview, JSON/YAML formatting, math evaluation, Markdown table conversion, sensitive field masking, and LLM-powered text processing; image entries support OCR, image description, and image Q&A after configuring a vision model
- **Plugin-based actions**: Define custom AI actions with `plugin.toml` and prompts
- **Model connectivity**: Connects to remote or local model services through OpenAI-compatible APIs, including gateways and Ollama-style endpoints
- **Native macOS NSPanel**: Overlay-style floating panel without focus switching or Dock icon noise
- **Global shortcut**: Launch instantly with a customizable shortcut
- **Privacy first**: Local processing by default, with configurable redaction before sending data to remote models

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop shell | [Tauri 2](https://tauri.app/) |
| Frontend | [SolidJS](https://www.solidjs.com/) + TypeScript + [TailwindCSS 4](https://tailwindcss.com/) |
| Backend | Rust + Tokio + rusqlite + reqwest |
| Platform | macOS (NSPanel support is implemented; other platforms need extra adaptation) |

## Quick Start

### Requirements

- macOS 12+
- Node.js 18+
- Rust 1.75+ (install with [rustup](https://rustup.rs/))
- Xcode Command Line Tools

### Install and Run

```bash
# Install frontend dependencies
npm install

# Development mode (starts frontend and Rust backend together)
npm run tauri dev

# Production build (outputs .app / .dmg to src-tauri/target/release/bundle/)
npm run tauri build
```

On first launch, the app opens the onboarding flow. After configuring a model API key, ClipBrain is ready to use. API keys are stored in the system Keychain and are never written to project files.

## Project Structure

```text
clipbrain/
├── src/                    # SolidJS frontend
│   ├── components/         # UI components (MainLayout / ClipboardPanel / DetailPanel / SettingsPage)
│   ├── lib/                # IPC / i18n / theme
│   ├── locales/            # Localization resources
│   └── App.tsx             # Route entry
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── actions/        # AI actions (built-in + plugin-based)
│   │   ├── classifier/     # Content type detection
│   │   ├── clipboard/      # Clipboard monitoring
│   │   ├── commands/       # Tauri IPC commands
│   │   ├── config/         # Configuration + privacy rules
│   │   ├── model/          # LLM backend abstraction (local + remote)
│   │   ├── storage/        # SQLite persistence
│   │   ├── macos_panel.rs  # macOS NSPanel integration
│   │   └── lib.rs          # Entry point
│   ├── Cargo.toml
│   └── tauri.conf.json
├── index.html
├── package.json
└── vite.config.ts
```

## Contributing

Issues and pull requests are welcome. Before submitting:

- Ensure `npm run tauri build` succeeds
- Ensure `cargo clippy` introduces no new warnings
- Follow [Conventional Commits](https://www.conventionalcommits.org/)

## License

[MIT](./LICENSE) © 2026 rhh777
