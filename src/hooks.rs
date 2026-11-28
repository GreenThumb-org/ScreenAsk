use crate::config::{Config, DismissMode};
use crate::events::{AppEvent, RecordingTarget};
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseBtn {
    Left,
    Right,
    Middle,
    Mouse4,
    Mouse5,
}

impl MouseBtn {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Middle => "middle",
            Self::Mouse4 => "mouse4",
            Self::Mouse5 => "mouse5",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    Key {
        vk: u32,
        ctrl: bool,
        shift: bool,
        alt: bool,
        win: bool,
    },
    Mouse(MouseBtn),
}

pub fn parse_binding(raw: &str) -> Option<Binding> {
    let s = raw.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }

    let cleaned = s.replace("mouse_", "").replace("mouse+", "");
    if let Some(btn) = parse_mouse_button(&cleaned).or_else(|| parse_mouse_button(&s)) {
        return Some(Binding::Mouse(btn));
    }

    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut win = false;
    let mut main_vk: Option<u32> = None;

    for part in s.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part {
            "ctrl" | "control" | "lctrl" | "rctrl" | "left ctrl" | "right ctrl" => ctrl = true,
            "shift" | "lshift" | "rshift" | "left shift" | "right shift" => shift = true,
            "alt" | "lalt" | "ralt" | "left alt" | "right alt" => alt = true,
            "win" | "windows" | "super" | "meta" | "left windows" | "right windows" => win = true,
            other => {
                main_vk = name_to_vk(other);
            }
        }
    }

    main_vk.map(|vk| Binding::Key {
        vk,
        ctrl,
        shift,
        alt,
        win,
    })
}

fn parse_mouse_button(name: &str) -> Option<MouseBtn> {
    match name {
        "left" | "mouse1" => Some(MouseBtn::Left),
        "right" | "mouse2" => Some(MouseBtn::Right),
        "middle" | "mouse3" => Some(MouseBtn::Middle),
        "mouse4" | "x1" | "x" => Some(MouseBtn::Mouse4),
        "mouse5" | "x2" => Some(MouseBtn::Mouse5),
        _ => None,
    }
}

fn name_to_vk(name: &str) -> Option<u32> {
    if let Some(hex) = name.strip_prefix("vk_") {
        if let Ok(vk) = u32::from_str_radix(hex, 16) {
            return Some(vk);
        }
    }

    if let Some(num) = name.strip_prefix('f') {
        if let Ok(n) = num.parse::<u32>() {
            if (1..=24).contains(&n) {
                return Some(0x70 + (n - 1));
            }
        }
    }

    if let Some(num) = name.strip_prefix("numpad") {
        if let Ok(n) = num.parse::<u32>() {
            if n <= 9 {
                return Some(0x60 + n);
            }
        }
    }

    let named = match name {
        "esc" | "escape" => Some(0x1B),
        "space" | "spacebar" => Some(0x20),
        "enter" | "return" => Some(0x0D),
        "tab" => Some(0x09),
        "backspace" => Some(0x08),
        "insert" | "ins" => Some(0x2D),
        "delete" | "del" => Some(0x2E),
        "home" => Some(0x24),
        "end" => Some(0x23),
        "pageup" | "pgup" | "page up" => Some(0x21),
        "pagedown" | "pgdn" | "page down" => Some(0x22),
        "left" | "arrow_left" => Some(0x25),
        "up" | "arrow_up" => Some(0x26),
        "right" | "arrow_right" => Some(0x27),
        "down" | "arrow_down" => Some(0x28),
        "printscreen" | "prtsc" | "print screen" | "snapshot" => Some(0x2C),
        "pause" => Some(0x13),
        "scrolllock" | "scroll lock" => Some(0x91),
        "capslock" | "caps lock" => Some(0x14),
        "numlock" | "num lock" => Some(0x90),
        "apps" | "menu" => Some(0x5D),
        "numpad_mul" | "numpad*" | "multiply" => Some(0x6A),
        "numpad_add" | "add" => Some(0x6B),
        "numpad_sub" | "numpad-" | "subtract" => Some(0x6D),
        "numpad_dec" | "numpad." | "decimal" => Some(0x6E),
        "numpad_div" | "numpad/" | "divide" => Some(0x6F),
        "volume_mute" => Some(0xAD),
        "volume_down" => Some(0xAE),
        "volume_up" => Some(0xAF),
        "media_next" => Some(0xB0),
        "media_prev" => Some(0xB1),
        "media_stop" => Some(0xB2),
        "media_play_pause" => Some(0xB3),
        "plus" | "equals" => Some(0xBB),
        "minus" => Some(0xBD),
        "comma" => Some(0xBC),
        "period" => Some(0xBE),
        _ => None,
    };
    if named.is_some() {
        return named;
    }

    // Single Unicode character (supports a-z, 0-9, German ä/ö/ü/ß/<, #, ^, ´, etc.)
    let mut chars = name.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        if ch.is_ascii_alphabetic() {
            return Some(ch.to_ascii_uppercase() as u32);
        }
        if ch.is_ascii_digit() {
            return Some(ch as u32);
        }

        #[cfg(windows)]
        {
            if let Some(vk) = char_to_vk_windows(ch) {
                return Some(vk);
            }
        }

        // Static fallback for common OEM chars if layout scan didn't match
        let fallback = match ch {
            '-' => 0xBD,
            '=' => 0xBB,
            '[' | 'ß' => 0xDB,
            ']' | '´' => 0xDD,
            '\\' | '^' => 0xDC,
            ';' | 'ü' => 0xBA,
            '\'' | 'ä' => 0xDE,
            ',' => 0xBC,
            '.' => 0xBE,
            '/' | '#' => 0xBF,
            '`' | 'ö' => 0xC0,
            '<' => 0xE2,
            _ => return None,
        };
        return Some(fallback);
    }

    None
}

#[cfg(windows)]
fn active_keyboard_layout() -> windows_sys::Win32::UI::Input::KeyboardAndMouse::HKL {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };
    unsafe {
        let fg = GetForegroundWindow();
        let tid = if !fg.is_null() {
            GetWindowThreadProcessId(fg, std::ptr::null_mut())
        } else {
            0
        };
        let hkl = GetKeyboardLayout(tid);
        if hkl.is_null() {
            GetKeyboardLayout(0)
        } else {
            hkl
        }
    }
}

#[cfg(windows)]
fn char_to_vk_windows(ch: char) -> Option<u32> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::VkKeyScanExW;
    let mut utf16 = [0u16; 2];
    let encoded = ch.encode_utf16(&mut utf16);
    if encoded.len() != 1 {
        return None;
    }
    unsafe {
        let hkl = active_keyboard_layout();
        let res = VkKeyScanExW(encoded[0], hkl);
        if res == -1 {
            None
        } else {
            Some((res & 0xFF) as u32)
        }
    }
}

#[cfg(windows)]
fn vk_to_char_windows(vk: u32) -> Option<char> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MapVirtualKeyExW, MAPVK_VK_TO_CHAR};
    unsafe {
        let hkl = active_keyboard_layout();
        let mapped = MapVirtualKeyExW(vk, MAPVK_VK_TO_CHAR, hkl);
        // Strip dead-key high bit (0x8000_0000) so dead keys like ^ and ´ still produce their char!
        let code_unit = mapped & 0xFFFF;
        if code_unit == 0 {
            return None;
        }
        let ch = char::from_u32(code_unit)?;
        if ch.is_control() || ch.is_whitespace() {
            return None;
        }
        ch.to_lowercase().next()
    }
}

fn vk_to_name(vk: u32) -> Option<String> {
    if (0x41..=0x5A).contains(&vk) {
        return Some((vk as u8 as char).to_ascii_lowercase().to_string());
    }
    if (0x30..=0x39).contains(&vk) {
        return Some((vk as u8 as char).to_string());
    }
    if (0x70..=0x87).contains(&vk) {
        return Some(format!("f{}", vk - 0x70 + 1));
    }
    if (0x60..=0x69).contains(&vk) {
        return Some(format!("numpad{}", vk - 0x60));
    }

    let named = match vk {
        0x1B => Some("escape"),
        0x20 => Some("space"),
        0x0D => Some("enter"),
        0x09 => Some("tab"),
        0x08 => Some("backspace"),
        0x2D => Some("insert"),
        0x2E => Some("delete"),
        0x24 => Some("home"),
        0x23 => Some("end"),
        0x21 => Some("pageup"),
        0x22 => Some("pagedown"),
        0x25 => Some("arrow_left"),
        0x26 => Some("arrow_up"),
        0x27 => Some("arrow_right"),
        0x28 => Some("arrow_down"),
        0x2C => Some("printscreen"),
        0x13 => Some("pause"),
        0x91 => Some("scrolllock"),
        0x14 => Some("capslock"),
        0x90 => Some("numlock"),
        0x5D => Some("apps"),
        0x6A => Some("numpad_mul"),
        0x6B => Some("numpad_add"),
        0x6D => Some("numpad_sub"),
        0x6E => Some("numpad_dec"),
        0x6F => Some("numpad_div"),
        0xAD => Some("volume_mute"),
        0xAE => Some("volume_down"),
        0xAF => Some("volume_up"),
        0xB0 => Some("media_next"),
        0xB1 => Some("media_prev"),
        0xB2 => Some("media_stop"),
        0xB3 => Some("media_play_pause"),
        _ => None,
    };
    if let Some(s) = named {
        return Some(s.to_string());
    }

    // Query active Windows keyboard layout (handles German ä, ö, ü, ß, <, #, ^, ´, etc.)
    #[cfg(windows)]
    if let Some(ch) = vk_to_char_windows(vk) {
        if ch == '+' {
            return Some("plus".to_string());
        }
        return Some(ch.to_string());
    }

    // Universal fallback so EVERY physical/OEM key on any keyboard can still be bound!
    if vk > 0 && vk <= 0xFF {
        return Some(format!("vk_{vk:02x}"));
    }

    None
}

fn is_modifier_vk(vk: u32) -> bool {
    matches!(
        vk,
        0x10 | 0xA0 | 0xA1 | // Shift
        0x11 | 0xA2 | 0xA3 | // Ctrl
        0x12 | 0xA4 | 0xA5 | // Alt
        0x5B | 0x5C // Win
    )
}

#[derive(Debug, Clone)]
struct RecordingSession {
    target: RecordingTarget,
    started_at: Instant,
}

struct HookState {
    capture_binding: Option<Binding>,
    dismiss_mode: DismissMode,
    dismiss_binding: Option<Binding>,
    hotkey_dismisses_overlay: bool,
    overlay_visible: bool,
    hud_editing: bool,
    recording: Option<RecordingSession>,
}

struct HookShared {
    tx: Sender<AppEvent>,
    egui_ctx: Mutex<Option<egui::Context>>,
    state: Mutex<HookState>,
}

impl HookShared {
    fn emit(&self, event: AppEvent) {
        let _ = self.tx.send(event);
        if let Ok(guard) = self.egui_ctx.lock() {
            if let Some(ctx) = guard.as_ref() {
                ctx.request_repaint();
            }
        }
    }
}

static HOOK_SHARED: OnceLock<Arc<HookShared>> = OnceLock::new();

#[derive(Clone)]
pub struct HookManager {
    shared: Arc<HookShared>,
    thread_id: Arc<AtomicU32>,
    stop_flag: Arc<AtomicBool>,
}

impl HookManager {
    pub fn start(tx: Sender<AppEvent>, cfg: &Config) -> Self {
        let shared = Arc::new(HookShared {
            tx,
            egui_ctx: Mutex::new(None),
            state: Mutex::new(HookState {
                capture_binding: parse_binding(&cfg.hotkey),
                dismiss_mode: cfg.dismiss_mode,
                dismiss_binding: parse_binding(&cfg.dismiss_hotkey),
                hotkey_dismisses_overlay: cfg.hotkey_dismisses_overlay,
                overlay_visible: false,
                hud_editing: false,
                recording: None,
            }),
        });

        let _ = HOOK_SHARED.set(Arc::clone(&shared));
        let thread_id = Arc::new(AtomicU32::new(0));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let tid_clone = Arc::clone(&thread_id);
        let stop_clone = Arc::clone(&stop_flag);

        #[cfg(windows)]
        {
            thread::spawn(move || {
                run_windows_mouse_hook_loop(tid_clone);
            });
            thread::spawn(move || {
                run_windows_keyboard_poll_loop(stop_clone);
            });
        }

        Self {
            shared,
            thread_id,
            stop_flag,
        }
    }

    pub fn set_egui_ctx(&self, ctx: egui::Context) {
        if let Ok(mut guard) = self.shared.egui_ctx.lock() {
            *guard = Some(ctx);
        }
    }

    pub fn update_config(&self, cfg: &Config) {
        if let Ok(mut st) = self.shared.state.lock() {
            st.capture_binding = parse_binding(&cfg.hotkey);
            st.dismiss_mode = cfg.dismiss_mode;
            st.dismiss_binding = parse_binding(&cfg.dismiss_hotkey);
            st.hotkey_dismisses_overlay = cfg.hotkey_dismisses_overlay;
        }
    }

    pub fn set_overlay_visible(&self, visible: bool) {
        if let Ok(mut st) = self.shared.state.lock() {
            st.overlay_visible = visible;
        }
    }

    pub fn set_hud_editing(&self, editing: bool) {
        if let Ok(mut st) = self.shared.state.lock() {
            st.hud_editing = editing;
        }
    }

    pub fn start_recording(&self, target: RecordingTarget) {
        if let Ok(mut st) = self.shared.state.lock() {
            st.recording = Some(RecordingSession {
                target,
                started_at: Instant::now(),
            });
        }
    }

    pub fn cancel_recording(&self) {
        if let Ok(mut st) = self.shared.state.lock() {
            st.recording = None;
        }
    }

    pub fn shutdown(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        #[cfg(windows)]
        {
            let tid = self.thread_id.load(Ordering::SeqCst);
            if tid != 0 {
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                        tid,
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_QUIT,
                        0,
                        0,
                    );
                }
            }
        }
    }
}

#[cfg(windows)]
fn current_modifiers() -> (bool, bool, bool, bool) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    unsafe {
        let down = |vk: i32| (GetAsyncKeyState(vk) as u16 & 0x8000) != 0;
        let ctrl = down(0x11) || down(0xA2) || down(0xA3);
        let shift = down(0x10) || down(0xA0) || down(0xA1);
        let alt = down(0x12) || down(0xA4) || down(0xA5);
        let win = down(0x5B) || down(0x5C);
        (ctrl, shift, alt, win)
    }
}

#[cfg(windows)]
fn handle_key_down(vk: u32) {
    let Some(shared) = HOOK_SHARED.get() else {
        return;
    };
    let (ctrl, shift, alt, win) = current_modifiers();

    let mut action: Option<AppEvent> = None;

    if let Ok(mut st) = shared.state.lock() {
        if st.hud_editing {
            return;
        }
        if let Some(rec) = st.recording.clone() {
            if !is_modifier_vk(vk) && rec.started_at.elapsed() >= Duration::from_millis(50) {
                if let Some(name) = vk_to_name(vk) {
                    let mut parts = Vec::new();
                    if alt {
                        parts.push("alt".to_string());
                    }
                    if ctrl {
                        parts.push("ctrl".to_string());
                    }
                    if shift {
                        parts.push("shift".to_string());
                    }
                    if win {
                        parts.push("win".to_string());
                    }
                    parts.push(name);
                    st.recording = None;
                    action = Some(AppEvent::HotkeyRecorded {
                        target: rec.target,
                        binding: parts.join("+"),
                    });
                }
            }
        } else {
            let matches_capture = matches!(
                st.capture_binding,
                Some(Binding::Key {
                    vk: b_vk,
                    ctrl: b_ctrl,
                    shift: b_shift,
                    alt: b_alt,
                    win: b_win,
                }) if b_vk == vk && b_ctrl == ctrl && b_shift == shift && b_alt == alt && b_win == win
            );

            let matches_dismiss = matches!(
                st.dismiss_binding,
                Some(Binding::Key {
                    vk: b_vk,
                    ctrl: b_ctrl,
                    shift: b_shift,
                    alt: b_alt,
                    win: b_win,
                }) if b_vk == vk && b_ctrl == ctrl && b_shift == shift && b_alt == alt && b_win == win
            );

            if st.overlay_visible {
                if matches_capture {
                    if st.dismiss_mode == DismissMode::SameHotkey
                        || st.dismiss_mode == DismissMode::AnyInput
                        || st.hotkey_dismisses_overlay
                    {
                        st.overlay_visible = false;
                        action = Some(AppEvent::CloseOverlay);
                    } else {
                        action = Some(AppEvent::Capture);
                    }
                } else {
                    match st.dismiss_mode {
                        DismissMode::AnyInput => {
                            if !is_modifier_vk(vk) {
                                st.overlay_visible = false;
                                action = Some(AppEvent::CloseOverlay);
                            }
                        }
                        DismissMode::MouseOnly | DismissMode::SameHotkey => {}
                        DismissMode::CustomKey => {
                            if matches_dismiss {
                                st.overlay_visible = false;
                                action = Some(AppEvent::CloseOverlay);
                            }
                        }
                    }
                }
            } else if matches_capture {
                action = Some(AppEvent::Capture);
            }
        }
    }

    if let Some(ev) = action {
        shared.emit(ev);
    }
}

#[cfg(windows)]
fn handle_mouse_down(btn: MouseBtn) {
    let Some(shared) = HOOK_SHARED.get() else {
        return;
    };

    let mut action: Option<AppEvent> = None;

    if let Ok(mut st) = shared.state.lock() {
        if st.hud_editing {
            return;
        }
        if let Some(rec) = st.recording.clone() {
            if rec.started_at.elapsed() >= Duration::from_millis(250) {
                st.recording = None;
                action = Some(AppEvent::HotkeyRecorded {
                    target: rec.target,
                    binding: btn.as_str().to_string(),
                });
            }
        } else {
            let matches_capture = st.capture_binding == Some(Binding::Mouse(btn));
            let matches_dismiss = st.dismiss_binding == Some(Binding::Mouse(btn));

            if st.overlay_visible {
                if matches_capture {
                    if st.dismiss_mode == DismissMode::SameHotkey
                        || st.dismiss_mode == DismissMode::AnyInput
                        || st.hotkey_dismisses_overlay
                    {
                        st.overlay_visible = false;
                        action = Some(AppEvent::CloseOverlay);
                    } else {
                        action = Some(AppEvent::Capture);
                    }
                } else if btn == MouseBtn::Right {
                    st.overlay_visible = false;
                    action = Some(AppEvent::CopyAndClose);
                } else {
                    match st.dismiss_mode {
                        DismissMode::AnyInput | DismissMode::MouseOnly => {
                            st.overlay_visible = false;
                            action = Some(AppEvent::CloseOverlay);
                        }
                        DismissMode::SameHotkey => {}
                        DismissMode::CustomKey => {
                            if matches_dismiss {
                                st.overlay_visible = false;
                                action = Some(AppEvent::CloseOverlay);
                            }
                        }
                    }
                }
            } else if matches_capture {
                action = Some(AppEvent::Capture);
            }
        }
    }

    if let Some(ev) = action {
        shared.emit(ev);
    }
}

#[cfg(windows)]
fn run_windows_keyboard_poll_loop(stop_flag: Arc<AtomicBool>) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

    // Track previous pressed state for all virtual keys so we trigger cleanly on rising edges.
    // Works reliably across all keyboard layouts (including German ä/ö/ü/ß), elevated windows,
    // and environments where WH_KEYBOARD_LL is blocked or filtered.
    let mut prev_down = [false; 256];

    // Seed initial state so keys already held at startup do not fire.
    unsafe {
        for vk in 0x08..=0xFEusize {
            prev_down[vk] = (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0;
        }
    }

    while !stop_flag.load(Ordering::Relaxed) {
        unsafe {
            for vk in 0x08..=0xFEusize {
                let is_down = (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0;
                if is_down && !prev_down[vk] {
                    let vk_u32 = vk as u32;
                    if !is_modifier_vk(vk_u32) {
                        handle_key_down(vk_u32);
                    }
                }
                prev_down[vk] = is_down;
            }
        }
        thread::sleep(Duration::from_millis(12));
    }
}

#[cfg(windows)]
unsafe extern "system" fn mouse_proc(code: i32, wparam: usize, lparam: isize) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, MSLLHOOKSTRUCT, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_RBUTTONDOWN,
        WM_XBUTTONDOWN,
    };

    if code == HC_ACTION as i32 {
        let btn = match wparam as u32 {
            WM_LBUTTONDOWN => Some(MouseBtn::Left),
            WM_RBUTTONDOWN => Some(MouseBtn::Right),
            WM_MBUTTONDOWN => Some(MouseBtn::Middle),
            WM_XBUTTONDOWN => {
                let ms = &*(lparam as *const MSLLHOOKSTRUCT);
                let xbtn = (ms.mouseData >> 16) & 0xFFFF;
                match xbtn {
                    1 => Some(MouseBtn::Mouse4),
                    2 => Some(MouseBtn::Mouse5),
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some(b) = btn {
            handle_mouse_down(b);
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

#[cfg(windows)]
fn run_windows_mouse_hook_loop(thread_id: Arc<AtomicU32>) {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
        MSG, WH_MOUSE_LL,
    };

    unsafe {
        thread_id.store(GetCurrentThreadId(), Ordering::SeqCst);
        let hmod = GetModuleHandleW(std::ptr::null());
        let ms_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), hmod, 0);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if !ms_hook.is_null() {
            UnhookWindowsHookEx(ms_hook);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keyboard_and_mouse_bindings() {
        assert_eq!(
            parse_binding("f10"),
            Some(Binding::Key {
                vk: 0x79,
                ctrl: false,
                shift: false,
                alt: false,
                win: false
            })
        );
        assert_eq!(
            parse_binding("ctrl+shift+a"),
            Some(Binding::Key {
                vk: b'A' as u32,
                ctrl: true,
                shift: true,
                alt: false,
                win: false
            })
        );
        assert_eq!(parse_binding("mouse4"), Some(Binding::Mouse(MouseBtn::Mouse4)));
        assert_eq!(parse_binding("x2"), Some(Binding::Mouse(MouseBtn::Mouse5)));
        assert_eq!(
            parse_binding("mouse_middle"),
            Some(Binding::Mouse(MouseBtn::Middle))
        );
        // German & universal VK bindings
        assert!(parse_binding("ä").is_some());
        assert!(parse_binding("ö").is_some());
        assert!(parse_binding("ü").is_some());
        assert!(parse_binding("ß").is_some());
        assert!(parse_binding("plus").is_some());
        assert_eq!(
            parse_binding("vk_e2"),
            Some(Binding::Key {
                vk: 0xE2,
                ctrl: false,
                shift: false,
                alt: false,
                win: false
            })
        );
    }
}
