/// Check whether HydroShot is configured to start on login.
pub fn is_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        is_enabled_windows()
    }
    #[cfg(target_os = "linux")]
    {
        is_enabled_linux()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        false
    }
}

/// Enable or disable auto-start on login.
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        set_enabled_windows(enabled)
    }
    #[cfg(target_os = "linux")]
    {
        set_enabled_linux(enabled)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = enabled;
        Err("Auto-start is not supported on this platform".into())
    }
}

#[cfg(target_os = "windows")]
fn is_enabled_windows() -> bool {
    use windows::core::*;
    use windows::Win32::System::Registry::*;

    unsafe {
        let mut key = HKEY::default();
        let path = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        if RegOpenKeyExW(HKEY_CURRENT_USER, path, 0, KEY_READ, &mut key).is_ok() {
            let name = w!("HydroShot");
            let result = RegQueryValueExW(key, name, None, None, None, None);
            let _ = RegCloseKey(key);
            result.is_ok()
        } else {
            false
        }
    }
}

#[cfg(target_os = "windows")]
fn set_enabled_windows(enabled: bool) -> Result<(), String> {
    use windows::core::*;
    use windows::Win32::System::Registry::*;

    unsafe {
        let mut key = HKEY::default();
        let path = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        RegOpenKeyExW(HKEY_CURRENT_USER, path, 0, KEY_WRITE, &mut key)
            .ok()
            .map_err(|e| e.to_string())?;

        if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let exe_str = exe.to_string_lossy();
            let value: Vec<u16> = exe_str.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes: &[u8] =
                std::slice::from_raw_parts(value.as_ptr() as *const u8, value.len() * 2);
            RegSetValueExW(key, w!("HydroShot"), 0, REG_SZ, Some(bytes))
                .ok()
                .map_err(|e| e.to_string())?;
        } else {
            let _ = RegDeleteValueW(key, w!("HydroShot"));
        }

        let _ = RegCloseKey(key);
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn is_enabled_linux() -> bool {
    let path = autostart_desktop_path();
    path.map_or(false, |p| p.exists())
}

#[cfg(target_os = "linux")]
fn set_enabled_linux(enabled: bool) -> Result<(), String> {
    let path = autostart_desktop_path().ok_or("Could not determine config directory")?;

    if enabled {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=HydroShot\nExec={}\nX-GNOME-Autostart-enabled=true\n",
            exe.display()
        );
        std::fs::write(&path, content).map_err(|e| e.to_string())
    } else {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
fn autostart_desktop_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("autostart").join("hydroshot.desktop"))
}
