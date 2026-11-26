import ctypes, ctypes.wintypes, io, json, os, queue, sys, threading, time, winreg
from datetime import datetime
from pathlib import Path
import tkinter as tk
from tkinter import ttk, filedialog, messagebox

import keyboard
import pystray
from google import genai
from google.genai import types
from PIL import Image, ImageDraw, ImageGrab

try:
    ctypes.windll.shcore.SetProcessDpiAwareness(2)  # full-res screenshots
except Exception:
    pass

CONFIG_DIR = Path(os.environ.get("APPDATA", Path.home())) / "ScreenAsk"
CONFIG_FILE = CONFIG_DIR / "config.json"
RUN_KEY = r"Software\Microsoft\Windows\CurrentVersion\Run"
DEFAULTS = {
    "api_key": "",
    "model": "gemini-flash-latest",
    "hotkey": "f10",
    "folder": str(Path.home() / "Pictures" / "ScreenAsk"),
    "prompt": "Look at this screenshot and give me short, helpful feedback. Max 100 words.",
    "overlay_seconds": 20,   # 0 = stays until clicked
    "opacity": 0.88,
    "font_size": 10,
    "width": 480,
}


def load_config():
    cfg = dict(DEFAULTS)
    try:
        cfg.update(json.loads(CONFIG_FILE.read_text(encoding="utf-8")))
    except Exception:
        pass
    # leftovers from the old Anthropic version
    if str(cfg.get("model", "")).startswith("claude"):
        cfg["model"] = DEFAULTS["model"]
    if str(cfg.get("api_key", "")).startswith("sk-ant"):
        cfg["api_key"] = ""
    return cfg


def save_config(cfg):
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    CONFIG_FILE.write_text(json.dumps(cfg, indent=2), encoding="utf-8")


def work_area():
    rect = ctypes.wintypes.RECT()
    ctypes.windll.user32.SystemParametersInfoW(48, 0, ctypes.byref(rect), 0)  # SPI_GETWORKAREA
    return rect


def get_autostart():
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, RUN_KEY) as k:
            winreg.QueryValueEx(k, "ScreenAsk")
            return True
    except OSError:
        return False


def set_autostart(on):
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, RUN_KEY, 0, winreg.KEY_SET_VALUE) as k:
        if on:
            exe = Path(sys.executable)
            if exe.with_name("pythonw.exe").exists():
                exe = exe.with_name("pythonw.exe")  # no console window
            winreg.SetValueEx(k, "ScreenAsk", 0, winreg.REG_SZ,
                              f'"{exe}" "{Path(__file__).resolve()}"')
        else:
            try:
                winreg.DeleteValue(k, "ScreenAsk")
            except FileNotFoundError:
                pass


class App:
    def __init__(self):
        self.cfg = load_config()
        self.root = tk.Tk()
        self.root.withdraw()
        self.q = queue.Queue()
        self.overlay = None
        self.hide_job = None
        self.win = None
        self.hotkey_handle = None
        self.register_hotkey()
        self.start_tray()
        if not (self.cfg["api_key"] or os.environ.get("GEMINI_API_KEY")):
            self.q.put(("settings", None))
        self.root.after(100, self.poll)

    # ---------- hotkey / tray ----------
    def register_hotkey(self):
        try:
            if self.hotkey_handle:
                keyboard.remove_hotkey(self.hotkey_handle)
            self.hotkey_handle = keyboard.add_hotkey(
                self.cfg["hotkey"], lambda: self.q.put(("capture", None))
            )
        except Exception as e:
            self.q.put(("show", f"Hotkey error: {e}"))

    def start_tray(self):
        img = Image.new("RGB", (64, 64), "#1e1e1e")
        ImageDraw.Draw(img).rectangle((12, 16, 52, 48), outline="white", width=4)
        menu = pystray.Menu(
            pystray.MenuItem("Open ScreenAsk", lambda i, m: self.q.put(("settings", None)), default=True),
            pystray.MenuItem("Open screenshot folder", lambda i, m: self.open_folder()),
            pystray.MenuItem("Quit", lambda i, m: self.q.put(("quit", None))),
        )
        self.icon = pystray.Icon("ScreenAsk", img, "ScreenAsk", menu)
        self.icon.run_detached()

    def open_folder(self):
        Path(self.cfg["folder"]).mkdir(parents=True, exist_ok=True)
        os.startfile(self.cfg["folder"])

    # ---------- main loop ----------
    def poll(self):
        try:
            while True:
                kind, data = self.q.get_nowait()
                if kind == "capture":
                    self.capture()
                elif kind == "show":
                    self.show_overlay(data)
                elif kind == "settings":
                    self.open_settings()
                elif kind == "quit":
                    self.icon.stop()
                    keyboard.unhook_all()
                    self.root.destroy()
                    return
        except queue.Empty:
            pass
        self.root.after(100, self.poll)

    # ---------- capture + AI ----------
    def capture(self):
        self.close_overlay()
        self.root.update()
        time.sleep(0.05)
        img = ImageGrab.grab()
        folder = Path(self.cfg["folder"])
        folder.mkdir(parents=True, exist_ok=True)
        img.save(folder / f"screenshot_{datetime.now():%Y%m%d_%H%M%S}.png")
        threading.Thread(target=self.ask, args=(img,), daemon=True).start()

    def ask(self, img):
        try:
            key = self.cfg["api_key"] or os.environ.get("GEMINI_API_KEY")
            if not key:
                raise RuntimeError("No API key set (tray icon > Open ScreenAsk).")
            small = img.convert("RGB")
            small.thumbnail((1568, 1568))
            buf = io.BytesIO()
            small.save(buf, "JPEG", quality=85)
            client = genai.Client(api_key=key)
            resp = client.models.generate_content(
                model=self.cfg["model"],
                contents=[
                    types.Part.from_bytes(data=buf.getvalue(), mime_type="image/jpeg"),
                    self.cfg["prompt"],
                ],
            )
            self.q.put(("show", (resp.text or "").strip() or "(empty answer)"))
        except Exception as e:
            self.q.put(("show", f"Error: {e}"))

    # ---------- overlay ----------
    def close_overlay(self):
        if self.hide_job:
            self.root.after_cancel(self.hide_job)
            self.hide_job = None
        if self.overlay is not None:
            self.overlay.destroy()
            self.overlay = None

    def show_overlay(self, text):
        self.close_overlay()
        w = self.overlay = tk.Toplevel(self.root)
        w.overrideredirect(True)
        w.attributes("-topmost", True)
        w.attributes("-alpha", float(self.cfg["opacity"]))
        lbl = tk.Label(
            w, text=text, bg="#1e1e1e", fg="#f0f0f0", justify="left",
            wraplength=int(self.cfg["width"]), padx=14, pady=10,
            font=("Segoe UI", int(self.cfg["font_size"])),
        )
        lbl.pack()

        def copy(_=None):
            self.root.clipboard_clear()
            self.root.clipboard_append(text)
            self.close_overlay()

        for widget in (w, lbl):
            widget.bind("<Button-1>", lambda e: self.close_overlay())  # left click: close
            widget.bind("<Button-3>", copy)                            # right click: copy + close

        w.update_idletasks()
        r = work_area()
        w.geometry(f"+{r.left + 16}+{r.bottom - w.winfo_height() - 16}")
        secs = int(self.cfg["overlay_seconds"])
        if secs > 0:
            self.hide_job = self.root.after(secs * 1000, self.close_overlay)

    # ---------- control panel ----------
    def open_settings(self):
        if self.win and self.win.winfo_exists():
            self.win.deiconify()
            self.win.lift()
            self.win.focus_force()
            return
        w = self.win = tk.Toplevel(self.root)
        w.title("ScreenAsk")
        w.resizable(False, False)
        w.protocol("WM_DELETE_WINDOW", w.withdraw)  # X hides to tray, app keeps running
        f = ttk.Frame(w, padding=12)
        f.pack()
        v = {}

        def row(r, label, key, show=None):
            ttk.Label(f, text=label).grid(row=r, column=0, sticky="w", pady=3)
            v[key] = tk.StringVar(value=str(self.cfg[key]))
            ttk.Entry(f, textvariable=v[key], width=46, show=show).grid(row=r, column=1, pady=3)

        row(0, "API key (Gemini, starts with AIza)", "api_key", "*")
        row(1, "Model", "model")
        row(2, "Hotkey (e.g. f10, ctrl+shift+s)", "hotkey")
        row(3, "Screenshot folder", "folder")
        ttk.Button(f, text="Browse", command=lambda: v["folder"].set(
            filedialog.askdirectory() or v["folder"].get())).grid(row=3, column=2, padx=4)
        ttk.Label(f, text="Standard prompt").grid(row=4, column=0, sticky="nw", pady=3)
        prompt = tk.Text(f, width=46, height=6, wrap="word")
        prompt.insert("1.0", self.cfg["prompt"])
        prompt.grid(row=4, column=1, pady=3)
        row(5, "Box visible (seconds, 0 = until click)", "overlay_seconds")
        row(6, "Box opacity (0.2 - 1.0)", "opacity")
        row(7, "Font size", "font_size")
        row(8, "Box width (px)", "width")

        auto = tk.BooleanVar(value=get_autostart())
        ttk.Checkbutton(f, text="Start with Windows", variable=auto).grid(
            row=9, column=1, sticky="w", pady=3)

        status = tk.StringVar(value="Running in background. You can close this window.")
        ttk.Label(f, textvariable=status, foreground="gray").grid(
            row=10, column=0, columnspan=3, sticky="w", pady=(6, 0))

        def save():
            try:
                self.cfg.update(
                    api_key=v["api_key"].get().strip(),
                    model=v["model"].get().strip(),
                    hotkey=v["hotkey"].get().strip().lower(),
                    folder=v["folder"].get().strip(),
                    prompt=prompt.get("1.0", "end").strip(),
                    overlay_seconds=int(v["overlay_seconds"].get()),
                    opacity=min(1.0, max(0.2, float(v["opacity"].get()))),
                    font_size=int(v["font_size"].get()),
                    width=int(v["width"].get()),
                )
            except ValueError:
                messagebox.showerror("ScreenAsk", "A number field has an invalid value.")
                return False
            save_config(self.cfg)
            self.register_hotkey()
            try:
                set_autostart(auto.get())
            except OSError as e:
                status.set(f"Autostart failed: {e}")
                return True
            status.set("Saved. Running in background.")
            return True

        def test():
            if save():
                w.withdraw()
                w.after(300, lambda: self.q.put(("capture", None)))

        btns = ttk.Frame(f)
        btns.grid(row=11, column=0, columnspan=3, sticky="e", pady=(8, 0))
        ttk.Button(btns, text="Test capture", command=test).pack(side="left", padx=4)
        ttk.Button(btns, text="Save", command=save).pack(side="left", padx=4)
        ttk.Button(btns, text="Save & hide", command=lambda: save() and w.withdraw()).pack(side="left", padx=4)

    def run(self):
        self.root.mainloop()


if __name__ == "__main__":
    App().run()
