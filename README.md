# ScreenAsk

Press one key or mouse button. Get instant Gemini AI feedback right on your screen.

Rebuilt in **Rust + egui** as a fast, lightweight native Windows app with a modern Blur-inspired UI and a live HUD layout editor.

## Features

- **One-Click Capture**: Trigger with any keyboard shortcut or mouse button (`F10`, `Mouse4`/`Mouse5`, combos, etc.) — captures only the currently focused monitor.
- **Automatic Reference PDFs**: Drop `.pdf` files into your screenshots folder and their text is automatically included as context with every screenshot (with caching).
- **Optional PNG Saving**: Toggle screenshot saving on/off anytime while still loading reference PDFs from the folder.
- **Live HUD Layout Editor**: Drag to reposition over your real screen, resize freely from all 4 sides & 4 corners, customize font size (down to `2pt`), opacity, box/text colors, plain-text-only mode, rounding, padding, border, and alignment.
- **Custom Dismiss Behavior**: Dismiss on any input, mouse click only, same hotkey toggle (hides without re-capturing), or a custom key/mouse button. **Right-click** copies the answer to clipboard and closes.
- **Modern Customizable UI**: Dark & light themes (animated Moon/Sun toggle), custom HSV accent color picker, bundled `JetBrains Mono NL` + `Libron H` + searchable system fonts, Windows autostart, and discreet **Hidden Mode** for the system tray.
- **Automatic Fallback**: Retries automatically with your fallback model on `429` / `503` / `500` errors.

## Quick Setup

1. **Install**: Download and run **`ScreenAsk-Setup.exe`** from [Releases](https://github.com/GreenThumb-org/ScreenAsk/releases) (installs per-user with no admin prompt).
2. **Get a Free Gemini API Key**:
   - Go to [aistudio.google.com](https://aistudio.google.com), sign in, click **Get API key** → **Create API key** (starts with `AIza`).
   - Paste it into the **Gemini API** tab (key icon) in ScreenAsk.
3. **Recommended Models**:
   - **Primary Model**: `gemini-3.5-flash-lite`
   - **Fallback Model**: `gemini-3.8-flash`
4. Click **Save**, then **Test Capture** to try it out.

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
