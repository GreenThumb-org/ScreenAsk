use crate::config::DEFAULT_FONT_NAME;
use egui::{FontData, FontDefinitions, FontFamily};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub const FONT_JETBRAINS_NL: &str = DEFAULT_FONT_NAME;
pub const FONT_LIBRON_H: &str = "Libron H";
pub const FONT_EGUI_DEFAULT: &str = "System Default (egui)";

const JETBRAINS_NL_BYTES: &[u8] = include_bytes!("../assets/fonts/JetBrainsMonoNL-Regular.ttf");
const LIBRON_H_BYTES: &[u8] = include_bytes!("../assets/fonts/Libron_H-Regular.ttf");

#[derive(Debug, Clone)]
pub enum FontSource {
    Bundled(&'static [u8]),
    EguiDefault,
    SystemFile(PathBuf),
}

#[derive(Debug, Clone)]
pub struct FontOption {
    pub name: String,
    pub source: FontSource,
    pub is_bundled: bool,
}

pub fn discover_available_fonts() -> Vec<FontOption> {
    let mut out = vec![
        FontOption {
            name: FONT_JETBRAINS_NL.to_string(),
            source: FontSource::Bundled(JETBRAINS_NL_BYTES),
            is_bundled: true,
        },
        FontOption {
            name: FONT_LIBRON_H.to_string(),
            source: FontSource::Bundled(LIBRON_H_BYTES),
            is_bundled: true,
        },
        FontOption {
            name: FONT_EGUI_DEFAULT.to_string(),
            source: FontSource::EguiDefault,
            is_bundled: true,
        },
    ];

    #[cfg(windows)]
    {
        let sys_fonts = discover_windows_fonts();
        for (name, path) in sys_fonts {
            out.push(FontOption {
                name,
                source: FontSource::SystemFile(path),
                is_bundled: false,
            });
        }
    }

    out
}

pub fn apply_font(ctx: &egui::Context, font_name: &str, available: &[FontOption]) {
    let mut defs = FontDefinitions::default();

    let chosen = available
        .iter()
        .find(|f| f.name == font_name)
        .or_else(|| available.first());

    let Some(option) = chosen else {
        ctx.set_fonts(defs);
        return;
    };

    let bytes_opt: Option<Arc<[u8]>> = match &option.source {
        FontSource::Bundled(static_bytes) => Some(Arc::from(*static_bytes)),
        FontSource::EguiDefault => None,
        FontSource::SystemFile(path) => fs::read(path).ok().map(Arc::from),
    };

    if let Some(bytes) = bytes_opt {
        let key = "screenask_active_font".to_string();
        defs.font_data
            .insert(key.clone(), Arc::new(FontData::from_owned(bytes.to_vec())));

        defs.families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, key.clone());

        if option.name.to_lowercase().contains("mono") {
            defs.families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, key);
        }
    }

    ctx.set_fonts(defs);
}

#[cfg(windows)]
fn discover_windows_fonts() -> BTreeMap<String, PathBuf> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    const SUBKEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";
    let win_fonts_dir = std::env::var("WINDIR")
        .map(|w| PathBuf::from(w).join("Fonts"))
        .unwrap_or_else(|_| PathBuf::from(r"C:\Windows\Fonts"));

    let mut found = BTreeMap::new();

    for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let Ok(key) = RegKey::predef(root).open_subkey(SUBKEY) else {
            continue;
        };
        for item in key.enum_values().flatten() {
            let (raw_name, reg_val) = item;
            let file_str = reg_val.to_string();
            let path = {
                let p = PathBuf::from(&file_str);
                if p.is_absolute() {
                    p
                } else {
                    win_fonts_dir.join(p)
                }
            };

            let supported_ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("ttf") || e.eq_ignore_ascii_case("otf"))
                .unwrap_or(false);

            if !supported_ext || !path.is_file() {
                continue;
            }

            let clean_name = clean_font_display_name(&raw_name);
            if !clean_name.is_empty() {
                found.entry(clean_name).or_insert(path);
            }
        }
    }

    found
}

#[cfg(windows)]
fn clean_font_display_name(raw: &str) -> String {
    raw.trim()
        .trim_end_matches("(TrueType)")
        .trim_end_matches("(OpenType)")
        .trim()
        .to_string()
}
