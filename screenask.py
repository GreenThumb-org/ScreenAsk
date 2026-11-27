import ctypes, ctypes.wintypes, io, json, os, queue, sys, threading, time, winreg
from datetime import datetime
from pathlib import Path
import tkinter as tk
from tkinter import ttk, filedialog, messagebox

import keyboard
import mouse
import pystray
from pypdf import PdfReader
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
    "model": "gemini-4-hydrogen",
    "fallback_model": "gemini-3.8-flash",
    "hotkey": "f10",
    "folder": str(Path.home() / "Pictures" / "ScreenAsk"),
    "prompt": "Look at this screenshot and give me short, helpful feedback. Max 100 words.",
    "overlay_seconds": 20,   # 0 = stays until clicked
    "opacity": 0.88,
    "font_size": 10,
    "width": 480,
    "light_mode": False,
}

MOUSE_BUTTON_MAP = {
    "mouse4": "x",
    "mouse5": "x2",
    "x1": "x",
    "x2": "x2",
    "middle": "middle",
    "right": "right",
    "left": "left",
}


def load_config():
    cfg = dict(DEFAULTS)
    try:
        cfg.update(json.loads(CONFIG_FILE.read_text(encoding="utf-8")))
    except Exception:
        pass
    if str(cfg.get("model", "")).startswith("claude"):
        cfg["model"] = DEFAULTS["model"]
    if str(cfg.get("api_key", "")).startswith("sk-ant"):
        cfg["api_key"] = ""
    cfg["light_mode"] = bool(cfg.get("light_mode", False))
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
        self.is_recording = False

        keyboard.hook(self.global_event_handler)
        mouse.hook(self.global_mouse_handler)

        self.register_hotkey()
        self.start_tray()
        if not (self.cfg["api_key"] or os.environ.get("GEMINI_API_KEY")):
            self.q.put(("settings", None))
        self.root.after(100, self.poll)

    def global_event_handler(self, e):
        if self.overlay is not None and not self.is_recording:
            if e.event_type == "down":
                self.q.put(("close_overlay", None))

    def global_mouse_handler(self, e):
        if self.overlay is not None and not self.is_recording:
            if isinstance(e, mouse.ButtonEvent) and e.event_type == "down":
                if e.button == "right":
                    self.q.put(("copy_and_close", None))
                else:
                    self.q.put(("close_overlay", None))

    def unhook_hotkey(self):
        if self.hotkey_handle:
            h_type, h_obj = self.hotkey_handle
            if h_type == "keyboard":
                try:
                    keyboard.remove_hotkey(h_obj)
                except Exception:
                    pass
            elif h_type == "mouse":
                try:
                    mouse.unhook(h_obj)
                except Exception:
                    pass
            self.hotkey_handle = None

    def register_hotkey(self):
        self.unhook_hotkey()
        if self.is_recording:
            return
        try:
            hotkey_str = self.cfg["hotkey"].strip().lower()
            clean_hotkey = hotkey_str.replace("mouse_", "").replace("mouse+", "")

            if hotkey_str in MOUSE_BUTTON_MAP or clean_hotkey in MOUSE_BUTTON_MAP or hotkey_str.startswith("mouse"):
                btn_name = MOUSE_BUTTON_MAP.get(clean_hotkey, clean_hotkey)
                hook_obj = mouse.on_button(
                    lambda: self.q.put(("capture", None)),
                    buttons=(btn_name,),
                    types=("down",),
                )
                self.hotkey_handle = ("mouse", hook_obj)
            else:
                h_obj = keyboard.add_hotkey(
                    hotkey_str, lambda: self.q.put(("capture", None))
                )
                self.hotkey_handle = ("keyboard", h_obj)
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

    def poll(self):
        try:
            while True:
                kind, data = self.q.get_nowait()
                if kind == "capture":
                    if not self.is_recording:
                        self.capture()
                elif kind == "show":
                    self.show_overlay(data)
                elif kind == "close_overlay":
                    self.close_overlay()
                elif kind == "copy_and_close":
                    if self.overlay and hasattr(self, "current_text"):
                        self.root.clipboard_clear()
                        self.root.clipboard_append(self.current_text)
                    self.close_overlay()
                elif kind == "settings":
                    self.open_settings()
                elif kind == "quit":
                    self.icon.stop()
                    keyboard.unhook_all()
                    try:
                        mouse.unhook_all()
                    except Exception:
                        pass
                    self.root.destroy()
                    return
        except queue.Empty:
            pass
        self.root.after(100, self.poll)

    def load_pdf_sources(self):
        """Reads all PDF files from the screenshot folder and returns combined text."""
        folder = Path(self.cfg["folder"])
        if not folder.exists():
            return ""

        pdf_texts = []
        try:
            for pdf_path in folder.glob("*.pdf"):
                reader = PdfReader(str(pdf_path))
                doc_text = []
                for page in reader.pages:
                    t = page.extract_text()
                    if t:
                        doc_text.append(t)
                if doc_text:
                    pdf_texts.append(f"--- Source: {pdf_path.name} ---\n" + "\n".join(doc_text))
        except Exception as e:
            print(f"Error reading PDFs: {e}")

        return "\n\n".join(pdf_texts) if pdf_texts else ""

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

            pdf_context = self.load_pdf_sources()

            contents = []
            if pdf_context:
                contents.append(f"Additional context / reference documents from folder:\n{pdf_context}\n\n")

            small = img.convert("RGB")
            small.thumbnail((2560, 2560))
            buf = io.BytesIO()
            small.save(buf, "JPEG", quality=90)

            contents.append(types.Part.from_bytes(data=buf.getvalue(), mime_type="image/jpeg"))
            contents.append(self.cfg["prompt"])

            client = genai.Client(api_key=key)

            models_to_try = [self.cfg["model"].strip()]
            fb = self.cfg.get("fallback_model", "").strip()
            if fb and fb not in models_to_try:
                models_to_try.append(fb)

            last_err = None
            for idx, model_name in enumerate(models_to_try):
                try:
                    resp = client.models.generate_content(
                        model=model_name,
                        contents=contents,
                    )
                    answer = (resp.text or "").strip() or "(empty answer)"
                    if idx > 0:
                        answer = f"[Fallback]\n{answer}"
                    self.q.put(("show", answer))
                    return
                except Exception as e:
                    last_err = e
                    err_str = str(e)
                    is_retryable = any(code in err_str for code in ("429", "503", "500", "404", "RESOURCE_EXHAUSTED", "UNAVAILABLE", "overloaded"))
                    if idx < len(models_to_try) - 1 and is_retryable:
                        continue
                    raise

            if last_err:
                raise last_err
        except Exception as e:
            self.q.put(("show", f"Error: {e}"))

    def close_overlay(self):
        if self.hide_job:
            self.root.after_cancel(self.hide_job)
            self.hide_job = None
        if self.overlay is not None:
            self.overlay.destroy()
            self.overlay = None

    def show_overlay(self, text):
        self.close_overlay()
        self.current_text = text
        light = bool(self.cfg.get("light_mode", False))

        w = self.overlay = tk.Toplevel(self.root)
        w.overrideredirect(True)
        w.attributes("-topmost", True)

        if light:
            transparent_bg = "#010101"
            w.configure(bg=transparent_bg)
            w.attributes("-transparentcolor", transparent_bg)
            w.attributes("-alpha", 1.0)
            box_bg = transparent_bg
            text_fg = "#000000"
        else:
            w.attributes("-alpha", float(self.cfg["opacity"]))
            box_bg = "#1e1e1e"
            text_fg = "#f0f0f0"

        lbl = tk.Label(
            w, text=text, bg=box_bg, fg=text_fg, justify="left",
            wraplength=int(self.cfg["width"]), padx=14, pady=10,
            font=("Segoe UI", int(self.cfg["font_size"])),
        )
        lbl.pack()

        w.update_idletasks()
        r = work_area()
        w.geometry(f"+{r.left + 16}+{r.bottom - w.winfo_height() - 16}")
        secs = int(self.cfg["overlay_seconds"])
        if secs > 0:
            self.hide_job = self.root.after(secs * 1000, self.close_overlay)

    def open_settings(self):
        if self.win and self.win.winfo_exists():
            self.win.deiconify()
            self.win.lift()
            self.win.focus_force()
            return
        w = self.win = tk.Toplevel(self.root)
        w.title("ScreenAsk")
        w.resizable(False, False)
        w.protocol("WM_DELETE_WINDOW", w.withdraw)
        f = ttk.Frame(w, padding=12)
        f.pack()
        v = {}

        def row(r, label, key, show=None):
            ttk.Label(f, text=label).grid(row=r, column=0, sticky="w", pady=3)
            v[key] = tk.StringVar(value=str(self.cfg.get(key, "")))
            ttk.Entry(f, textvariable=v[key], width=46, show=show).grid(row=r, column=1, pady=3)

        row(0, "API key (Gemini, starts with AIza)", "api_key", "*")
        row(1, "Primary Model", "model")
        row(2, "Fallback Model (optional)", "fallback_model")

        ttk.Label(f, text="Screenshot Key / Mouse Button").grid(row=3, column=0, sticky="w", pady=3)
        hk_frame = ttk.Frame(f)
        hk_frame.grid(row=3, column=1, columnspan=2, sticky="w", pady=3)
        v["hotkey"] = tk.StringVar(value=str(self.cfg["hotkey"]))
        hk_entry = ttk.Entry(hk_frame, textvariable=v["hotkey"], width=28, state="readonly")
        hk_entry.pack(side="left")

        record_btn = ttk.Button(hk_frame, text="Taste / Maus wählen")
        record_btn.pack(side="left", padx=(6, 0))

        def start_recording():
            if self.is_recording:
                return
            self.is_recording = True
            self.unhook_hotkey()
            record_btn.config(text="Drücke Taste/Maus...", state="disabled")

            def finish(chosen_key):
                def _ui_update():
                    v["hotkey"].set(chosen_key)
                    record_btn.config(text="Taste / Maus wählen", state="normal")
                    self.is_recording = False
                    self.register_hotkey()
                self.root.after(0, _ui_update)

            def listen_worker():
                pressed_modifiers = set()
                chosen = None
                lock = threading.Lock()
                done_event = threading.Event()

                def on_key(e):
                    nonlocal chosen
                    if done_event.is_set():
                        return
                    name = (e.name or "").lower()
                    if not name:
                        return
                    if name in ("ctrl", "left ctrl", "right ctrl", "shift", "left shift", "right shift", "alt", "left alt", "right alt", "windows", "left windows", "right windows"):
                        norm = "ctrl" if "ctrl" in name else "shift" if "shift" in name else "alt" if "alt" in name else "windows"
                        if e.event_type == "down":
                            pressed_modifiers.add(norm)
                        elif e.event_type == "up":
                            pressed_modifiers.discard(norm)
                    elif e.event_type == "down":
                        with lock:
                            if not done_event.is_set():
                                combo = sorted(pressed_modifiers) + [name]
                                chosen = "+".join(combo)
                                done_event.set()

                def on_mouse(e):
                    nonlocal chosen
                    if done_event.is_set():
                        return
                    if isinstance(e, mouse.ButtonEvent) and e.event_type == "down":
                        btn = e.button
                        if btn == "x":
                            btn = "mouse4"
                        elif btn == "x2":
                            btn = "mouse5"
                        with lock:
                            if not done_event.is_set():
                                chosen = btn
                                done_event.set()

                time.sleep(0.25)
                kb_hook = keyboard.hook(on_key)
                ms_hook = mouse.hook(on_mouse)
                done_event.wait()
                keyboard.unhook(kb_hook)
                mouse.unhook(ms_hook)
                finish(chosen)

            threading.Thread(target=listen_worker, daemon=True).start()

        record_btn.config(command=start_recording)

        row(4, "Screenshot folder", "folder")
        ttk.Button(f, text="Browse", command=lambda: v["folder"].set(
            filedialog.askdirectory() or v["folder"].get())).grid(row=4, column=2, padx=4)
        ttk.Label(f, text="Standard prompt").grid(row=5, column=0, sticky="nw", pady=3)
        prompt = tk.Text(f, width=46, height=6, wrap="word")
        prompt.insert("1.0", self.cfg["prompt"])
        prompt.grid(row=5, column=1, pady=3)
        row(6, "Box visible (seconds, 0 = until click)", "overlay_seconds")
        row(7, "Box opacity (0.2 - 1.0)", "opacity")
        row(8, "Font size", "font_size")
        row(9, "Box width (px)", "width")

        opts_frame = ttk.Frame(f)
        opts_frame.grid(row=10, column=1, columnspan=2, sticky="w", pady=3)

        auto = tk.BooleanVar(value=get_autostart())
        ttk.Checkbutton(opts_frame, text="Start with Windows", variable=auto).pack(side="left", padx=(0, 14))

        light = tk.BooleanVar(value=bool(self.cfg.get("light_mode", False)))
        ttk.Checkbutton(opts_frame, text="Light mode", variable=light).pack(side="left")

        status = tk.StringVar(value="Running in background. You can close this window.")
        ttk.Label(f, textvariable=status, foreground="gray").grid(
            row=11, column=0, columnspan=3, sticky="w", pady=(6, 0))

        def save():
            try:
                self.cfg.update(
                    api_key=v["api_key"].get().strip(),
                    model=v["model"].get().strip(),
                    fallback_model=v["fallback_model"].get().strip(),
                    hotkey=v["hotkey"].get().strip().lower(),
                    folder=v["folder"].get().strip(),
                    prompt=prompt.get("1.0", "end").strip(),
                    overlay_seconds=int(v["overlay_seconds"].get()),
                    opacity=min(1.0, max(0.2, float(v["opacity"].get()))),
                    font_size=int(v["font_size"].get()),
                    width=int(v["width"].get()),
                    light_mode=bool(light.get()),
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
        btns.grid(row=12, column=0, columnspan=3, sticky="e", pady=(8, 0))
        ttk.Button(btns, text="Test capture", command=test).pack(side="left", padx=4)
        ttk.Button(btns, text="Save", command=save).pack(side="left", padx=4)
        ttk.Button(btns, text="Save & hide", command=lambda: save() and w.withdraw()).pack(side="left", padx=4)

    def run(self):
        self.root.mainloop()


if __name__ == "__main__":
    App().run()
