#[cfg(windows)]
const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const VALUE_NAME: &str = "ScreenAsk";

#[cfg(windows)]
pub fn get_autostart() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(run_key) = hkcu.open_subkey(RUN_SUBKEY) else {
        return false;
    };
    run_key.get_value::<String, _>(VALUE_NAME).is_ok()
}

#[cfg(windows)]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags(RUN_SUBKEY, KEY_SET_VALUE)
        .map_err(|e| format!("Failed to open Windows Run key: {e}"))?;

    if enabled {
        let exe = std::env::current_exe()
            .map_err(|e| format!("Failed to resolve executable path: {e}"))?;
        let cmd = format!("\"{}\" --background", exe.display());
        run_key
            .set_value(VALUE_NAME, &cmd)
            .map_err(|e| format!("Failed to enable autostart: {e}"))?;
    } else {
        match run_key.delete_value(VALUE_NAME) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Failed to disable autostart: {e}")),
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn get_autostart() -> bool {
    false
}

#[cfg(not(windows))]
pub fn set_autostart(_enabled: bool) -> Result<(), String> {
    Ok(())
}
