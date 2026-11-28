# ScreenAsk

Press one key or mouse button. Get instant AI feedback right on your screen.

Rebuilt in **Rust + egui** as a fast, lightweight native Windows app with a modern Blur-inspired UI, multi-provider AI support, and a live HUD layout editor.

## Features

- **Multi-Provider AI**: Supports **Google Gemini**, **Anthropic Claude**, and any **OpenAI-compatible** API (OpenAI, OpenRouter, Groq, local Ollama / LM Studio) with automatic fallback retries.
- **One-Click Capture**: Trigger with any keyboard shortcut or mouse button (`F10`, `Mouse4`/`Mouse5`, combos, etc.) — captures only the currently focused monitor.
- **Automatic Reference PDFs**: Drop `.pdf` files into your screenshots folder and their text is automatically included as context with every screenshot (with caching).
- **Optional PNG Saving**: Toggle screenshot saving on/off anytime while still loading reference PDFs from the folder.
- **Live HUD Layout Editor**: Drag to reposition over your real screen, resize freely from all 4 sides & 4 corners, customize font size (down to `2pt`), opacity, box/text colors, plain-text-only mode, rounding, padding, border, and alignment.
- **Custom Dismiss Behavior**: Dismiss on any input, mouse click only, same hotkey toggle (hides without re-capturing), or a custom key/mouse button. **Right-click** copies the answer to clipboard and closes.
- **Modern Customizable UI**: Dark & light themes (animated Moon/Sun toggle), custom HSV accent color picker, bundled `JetBrains Mono NL` + `Libron H` + searchable system fonts, Windows autostart, and discreet **Hidden Mode** for the system tray.

## Quick Setup

1. **Install**: Download and run **`ScreenAsk-Setup.exe`** from [Releases](https://github.com/GreenThumb-org/ScreenAsk/releases) (installs per-user with no admin prompt).
2. **Choose a Provider & API Key** (key icon tab):
   - **Gemini (Recommended Free Tier)**: Get a free key at [aistudio.google.com](https://aistudio.google.com) → **Get API key** → **Create API key** (`AIza...`).
     - Primary: `gemini-3.5-flash-lite` · Fallback: `gemini-3.8-flash`
   - **Claude**: Paste your Anthropic key (`sk-ant-...`), e.g. `claude-sonnet-4-6` / `claude-haiku-4-5`.
   - **OpenAI / Compatible**: Set Base URL (e.g. `https://api.openai.com/v1`, OpenRouter, Groq, or `http://localhost:11434/v1` for Ollama) + model name.
3. Click **Save**, then **Test Capture** to try it out.

## Controls & Tray

- **Capture**: Press your configured key or mouse button (default `F10`).
- **Copy Answer**: Right-click while the overlay is visible.
- **System Tray**: Left-click opens Settings; right-click opens HUD Layout, folder, or **Quit**. Closing the window (`×`) hides it to the tray.
- **Config File**: `%APPDATA%\ScreenAsk\config.json`

## Build From Source

Requires [Rust](https://rustup.rs/) on Windows:

```powershell
cargo build --release
```

The standalone binary is output to `target\release\screenask.exe`.

## License

MIT — see [LICENSE](LICENSE). Free to use and modify; a shoutout is appreciated!
