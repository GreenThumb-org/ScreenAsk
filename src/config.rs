use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_MODEL: &str = "gemini-3.5-flash-lite";
pub const DEFAULT_FALLBACK_MODEL: &str = "gemini-3.8-flash";
pub const DEFAULT_HOTKEY: &str = "f10";
pub const DEFAULT_DISMISS_HOTKEY: &str = "escape";
pub const DEFAULT_PROMPT: &str =
    "Look at this screenshot and give me short, helpful feedback. Max 100 words.";
pub const DEFAULT_ACCENT_COLOR: [u8; 3] = [91, 140, 255];
pub const DEFAULT_FONT_NAME: &str = "JetBrains Mono NL (Default)";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DismissMode {
    /// Any key or left-click closes; right-click copies and closes.
    AnyInput,
    /// Only mouse buttons dismiss (left/other closes, right copies and closes).
    MouseOnly,
    /// Pressing the capture hotkey again hides the overlay without re-capturing (right-click copies).
    SameHotkey,
    /// A custom recorded key or mouse button closes (right-click copies).
    CustomKey,
}

impl Default for DismissMode {
    fn default() -> Self {
        Self::AnyInput
    }
}

impl DismissMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::AnyInput => "Any key or mouse click",
            Self::MouseOnly => "Mouse click only",
            Self::SameHotkey => "Same as capture hotkey (toggle)",
            Self::CustomKey => "Custom key / mouse button",
        }
    }

    pub const ALL: [Self; 4] = [
        Self::AnyInput,
        Self::MouseOnly,
        Self::SameHotkey,
        Self::CustomKey,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HudTextAlign {
    Left,
    Center,
    Right,
}

impl Default for HudTextAlign {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub api_key: String,
    pub model: String,
    pub fallback_model: String,
    pub hotkey: String,
    pub folder: String,
    pub save_screenshots: bool,
    pub prompt: String,
    pub overlay_seconds: u32,
    pub opacity: f32,
    pub font_size: f32,
    pub width: f32,
    pub hud_height: Option<f32>,
    pub hud_x: Option<f32>,
    pub hud_y: Option<f32>,
    pub hud_scale: f32,
    pub hud_box_color: Option<[u8; 3]>,
    pub hud_text_color: Option<[u8; 3]>,
    pub hud_rounding: f32,
    pub hud_padding: f32,
    pub hud_show_box: bool,
    pub hud_border: bool,
    pub hud_text_align: HudTextAlign,
    pub light_mode: bool,
    pub hidden_mode: bool,
    pub accent_color: [u8; 3],
    pub font_name: String,
    pub dismiss_mode: DismissMode,
    pub dismiss_hotkey: String,
    pub hotkey_dismisses_overlay: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: DEFAULT_MODEL.to_string(),
            fallback_model: DEFAULT_FALLBACK_MODEL.to_string(),
            hotkey: DEFAULT_HOTKEY.to_string(),
            folder: default_screenshot_folder().to_string_lossy().into_owned(),
            save_screenshots: true,
            prompt: DEFAULT_PROMPT.to_string(),
            overlay_seconds: 20,
            opacity: 0.88,
            font_size: 10.0,
            width: 480.0,
            hud_height: None,
            hud_x: None,
            hud_y: None,
            hud_scale: 1.0,
            hud_box_color: None,
            hud_text_color: None,
            hud_rounding: 10.0,
            hud_padding: 12.0,
            hud_show_box: true,
            hud_border: true,
            hud_text_align: HudTextAlign::Left,
            light_mode: false,
            hidden_mode: false,
            accent_color: DEFAULT_ACCENT_COLOR,
            font_name: DEFAULT_FONT_NAME.to_string(),
            dismiss_mode: DismissMode::AnyInput,
            dismiss_hotkey: DEFAULT_DISMISS_HOTKEY.to_string(),
            hotkey_dismisses_overlay: true,
        }
    }
}

impl Config {
    pub fn effective_api_key(&self) -> Option<String> {
        let trimmed = self.api_key.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
        std::env::var("GEMINI_API_KEY")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn sanitize(&mut self) {
        if self.model.trim().to_lowercase().starts_with("claude") || self.model.trim().is_empty() {
            self.model = DEFAULT_MODEL.to_string();
        }
        if self.api_key.trim().starts_with("sk-ant") {
            self.api_key.clear();
        }
        self.opacity = self.opacity.clamp(0.0, 1.0);
        self.font_size = self.font_size.clamp(2.0, 48.0);
        self.width = self.width.clamp(140.0, 1800.0);
        self.hud_height = self.hud_height.map(|h| h.clamp(40.0, 1200.0));
        self.hud_scale = self.hud_scale.clamp(0.5, 2.5);
        self.hud_rounding = self.hud_rounding.clamp(0.0, 28.0);
        self.hud_padding = self.hud_padding.clamp(4.0, 32.0);
        self.hud_x = self.hud_x.map(|v| v.clamp(0.0, 1.0));
        self.hud_y = self.hud_y.map(|v| v.clamp(0.0, 1.0));
        self.hotkey = self.hotkey.trim().to_lowercase();
        if self.hotkey.is_empty() {
            self.hotkey = DEFAULT_HOTKEY.to_string();
        }
        self.dismiss_hotkey = self.dismiss_hotkey.trim().to_lowercase();
        if self.dismiss_hotkey.is_empty() {
            self.dismiss_hotkey = DEFAULT_DISMISS_HOTKEY.to_string();
        }
        if self.folder.trim().is_empty() {
            self.folder = default_screenshot_folder().to_string_lossy().into_owned();
        }
        if self.font_name.trim().is_empty()
            || self.font_name == "JetBrains Mono Bold"
            || self.font_name == "Libron H (Default)"
        {
            self.font_name = DEFAULT_FONT_NAME.to_string();
        }
    }
}

pub fn config_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        if !appdata.trim().is_empty() {
            return PathBuf::from(appdata).join("ScreenAsk");
        }
    }
    home_dir().join("ScreenAsk")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn default_screenshot_folder() -> PathBuf {
    home_dir().join("Pictures").join("ScreenAsk")
}

fn home_dir() -> PathBuf {
    if let Ok(profile) = std::env::var("USERPROFILE") {
        if !profile.trim().is_empty() {
            return PathBuf::from(profile);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(".")
}

pub fn load_config() -> Config {
    let path = config_path();
    let mut cfg = match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str::<Config>(&raw).unwrap_or_default(),
        Err(_) => Config::default(),
    };
    cfg.sanitize();
    cfg
}

pub fn save_config(cfg: &Config) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config directory: {e}"))?;
    let json =
        serde_json::to_string_pretty(cfg).map_err(|e| format!("Failed to serialize config: {e}"))?;
    fs::write(config_path(), json).map_err(|e| format!("Failed to write config: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_legacy_python_config() {
        let python_json = r#"{
            "api_key": "sk-ant-old",
            "model": "claude-3-5-sonnet",
            "fallback_model": "gemini-3.8-flash",
            "hotkey": "Mouse4",
            "folder": "C:\\Users\\Test\\Pictures\\ScreenAsk",
            "prompt": "Test prompt",
            "overlay_seconds": 15,
            "opacity": 1.5,
            "font_size": 12,
            "width": 500,
            "light_mode": true
        }"#;
        let mut cfg: Config = serde_json::from_str(python_json).expect("should parse");
        cfg.sanitize();
        assert_eq!(cfg.api_key, "");
        assert_eq!(cfg.model, DEFAULT_MODEL);
        assert_eq!(cfg.hotkey, "mouse4");
        assert_eq!(cfg.opacity, 1.0);
        assert_eq!(cfg.dismiss_mode, DismissMode::AnyInput);
        assert!(cfg.light_mode);
    }
}
