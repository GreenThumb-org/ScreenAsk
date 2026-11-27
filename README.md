# ScreenAsk

Press one key or mouse button. Get AI feedback on your screen.

## How it works

1. You press your trigger key or mouse button (default: **F10**).
2. ScreenAsk saves a screenshot in your folder silently.
3. If there are any `.pdf` files in your screenshot folder, their text is automatically sent along as reference context.
4. The image and prompt go to your **Primary Model** (with automatic switch to your **Fallback Model** if the primary model hits a rate limit or overload error).
5. The answer appears in a see-through box at the bottom left:
   - **Any key or left click** = close the box.
   - **Right click** = copy the text to clipboard and close the box.
   - Auto-hides after 20 seconds (configurable).
- ScreenAsk runs in the background via the system tray icon (bottom right, possibly under the `^` arrow).

## Setup

**1. Install dependencies:**

```bash
pip install -r requirements.txt
```

**2. Get a free API key:**
Go to `aistudio.google.com`, sign in, click **Get API key**, then **Create API key** (starts with `AIza`).

**3. Start ScreenAsk:**

```bash
python screenask.py
```

**4. Configure the window:**

| Field | Recommended Value |
|---|---|
| API key | Your `AIza...` key |
| Primary Model | `gemini-4-hydrogen` |
| Fallback Model | `gemini-3.8-flash` (used automatically on `429`/`503` errors) |
| Screenshot Key / Mouse Button | Click **Taste / Maus wählen** and press any key combo or mouse button |
| Screenshot folder | Folder for saved screenshots (and optional reference PDFs) |
| Standard prompt | Instruction sent with every screenshot |

Click **Save**, then **Test capture** to try it.

**5. Run without a console window:**

```bash
pythonw screenask.py
```

Tick **Start with Windows** in the settings window to launch automatically at login.

## Tray Icon

- **Left click** = open settings window.
- **Right click** = open screenshot folder, or **Quit**.
- Closing the settings window (`X`) only hides it to the tray; use **Quit** to exit completely.

## Where files are saved

- **Screenshots & reference PDFs:** `Pictures\ScreenAsk` (changeable in settings)
- **Settings:** `%APPDATA%\ScreenAsk\config.json`

## Troubleshooting

| Message | What to do |
|---|---|
| `404 NOT_FOUND` | Check the spelling of your Primary/Fallback Model name. |
| `429` or `503` | Rate limit or temporary overload. Set a Fallback Model or wait a moment. |
| `401` or `403` | Invalid or expired API key. Generate a new key in Google AI Studio. |
| Hotkey does nothing | Another instance is already running. Right-click tray icon -> **Quit**, then restart. |
| Black screenshot in a game | Switch the game to borderless windowed mode. |

## Prompt Reference

```
Examine the screenshot and locate the cosmology quiz near the center of the screen (covering topics such as Kepler's laws, cosmic history, and the solar system).

Use the reference PDFs to verify accurate answers before responding.

Follow the output rule that matches the question type:

1. Multiple-choice question (answer options are visible):
   - Map options from top to bottom starting with A (for example, A, B, C, D, or further down the alphabet if more options are shown).
   - Output ONLY the single letter of the correct answer (no punctuation, explanation, or extra words).

2. Open / non-multiple-choice question (no answer options are visible):
   - Write out the answer directly.
   - Keep the response strictly between 1 and 10 words (never more than 10 words).
```

## Privacy note

On the free API tier, Google may use submitted prompts and screenshots to improve its products. Avoid capturing sensitive data such as passwords or banking pages.
