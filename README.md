# ScreenAsk

Press one key. Get AI feedback on your screen.

## How it works

1. You press **F10**.
2. ScreenAsk saves a screenshot in a folder. You see nothing.
3. The image and your standard prompt go to Gemini.
4. The answer shows up in a see-through box at the bottom left.

- Left click on the box = close it.
- Right click on the box = copy the text and close it.
- The box hides itself after 20 seconds.
- ScreenAsk runs in the background. Look for its icon bottom right (maybe under the `^` arrow).

## Setup

**1. Install the parts:**

```
pip install google-genai keyboard pystray pillow
```

**2. Get a free key:**
Go to `aistudio.google.com`, sign in, click **Get API key**, then **Create API key**.
The key starts with `AIza`.

**3. Start it:**

```
python screenask.py
```

**4. Fill in the window:**

| Field | Value |
|---|---|
| API key | your `AIza...` key |
| Model | `gemini-3.5-flash-lite` |
| Hotkey | `f10` (or something like `ctrl+shift+s`) |
| Prompt | what the AI should do with every screenshot |

Click **Save**, then **Test capture** to try it.

**5. Run it without a console window:**

```
pythonw screenask.py
```

Tick **Start with Windows** if you want it to start by itself.

## The tray icon

- Click it = open the window.
- Right click = open the screenshot folder, or **Quit**.
- The X on the window only hides it. ScreenAsk keeps running.
- **Quit** is the only way to really stop it.

## Where things are saved

- Screenshots: `Pictures\ScreenAsk` (you can change this in the window)
- Settings: `%APPDATA%\ScreenAsk\config.json`

## If something goes wrong

| Message | What to do |
|---|---|
| `404 NOT_FOUND` | Wrong model name. Use `gemini-3.5-flash-lite`. |
| `429` | Free limit reached. Wait a bit, or try another model. |
| `401` or `403` | Wrong or old key. Make a new one in AI Studio. |
| Hotkey does nothing | An old copy is still running. Tray icon, then **Quit**, then start again. |
| Black screenshot in a game | Use borderless windowed mode. |

## Good to know

On the free plan, Google may use what you send to improve its products. Do not press the key while passwords or bank pages are on screen.
