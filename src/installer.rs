use std::path::PathBuf;

/// Return the install directory: %LocalAppData%\HydroShot
fn install_dir() -> Result<PathBuf, String> {
    dirs::data_local_dir()
        .map(|d| d.join("HydroShot"))
        .ok_or_else(|| "Could not determine LocalAppData directory".into())
}

/// Return the installed exe path
fn installed_exe() -> Result<PathBuf, String> {
    Ok(install_dir()?.join("hydroshot.exe"))
}

/// Return the Start Menu shortcut directory
fn start_menu_dir() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|d| {
            d.join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("HydroShot")
        })
        .ok_or_else(|| "Could not determine Start Menu directory".into())
}

pub fn install() -> Result<(), String> {
    let source = std::env::current_exe().map_err(|e| format!("Failed to find current exe: {e}"))?;
    let dest_dir = install_dir()?;
    let dest_exe = installed_exe()?;

    // Don't re-install over ourselves
    if let Ok(canonical_src) = std::fs::canonicalize(&source) {
        if let Ok(canonical_dst) = std::fs::canonicalize(&dest_exe) {
            if canonical_src == canonical_dst {
                println!("HydroShot is already installed at {}", dest_exe.display());
                return Ok(());
            }
        }
    }

    // 1. Copy exe to install dir
    std::fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Failed to create {}: {e}", dest_dir.display()))?;
    std::fs::copy(&source, &dest_exe)
        .map_err(|e| format!("Failed to copy exe: {e}"))?;
    println!("Installed to {}", dest_exe.display());

    // 2. Create Start Menu shortcut via PowerShell
    create_shortcut(&dest_exe)?;

    // 3. Add to user PATH via registry
    add_to_path(&dest_dir)?;

    // 4. Register autostart
    crate::autostart::set_enabled(true)
        .unwrap_or_else(|e| eprintln!("Warning: could not set autostart: {e}"));

    println!("HydroShot installed successfully!");
    println!("  Location:   {}", dest_exe.display());
    println!("  Start Menu: HydroShot shortcut created");
    println!("  PATH:       added (restart terminal to use 'hydroshot' command)");
    println!("  Autostart:  enabled");

    // 5. Launch the installed copy
    let _ = std::process::Command::new(&dest_exe).spawn();

    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    // 1. Remove autostart
    let _ = crate::autostart::set_enabled(false);

    // 2. Remove from PATH
    let dest_dir = install_dir()?;
    remove_from_path(&dest_dir)?;

    // 3. Remove Start Menu shortcut
    let sm_dir = start_menu_dir()?;
    if sm_dir.exists() {
        let _ = std::fs::remove_dir_all(&sm_dir);
        println!("Removed Start Menu shortcut");
    }

    // 4. Remove installed exe (schedule deletion since we may be running from it)
    let dest_exe = installed_exe()?;
    if dest_exe.exists() {
        // Try direct removal first; if it fails (file in use), schedule for next reboot
        if std::fs::remove_file(&dest_exe).is_err() {
            schedule_delete_on_reboot(&dest_exe);
            println!("Exe is in use; will be removed on next reboot");
        } else {
            println!("Removed {}", dest_exe.display());
        }
    }

    // Try to remove the install dir if empty
    let _ = std::fs::remove_dir(&dest_dir);

    println!("HydroShot has been uninstalled.");
    Ok(())
}

fn create_shortcut(exe_path: &std::path::Path) -> Result<(), String> {
    let sm_dir = start_menu_dir()?;
    std::fs::create_dir_all(&sm_dir)
        .map_err(|e| format!("Failed to create Start Menu dir: {e}"))?;

    let lnk_path = sm_dir.join("HydroShot.lnk");
    let exe_str = exe_path.to_string_lossy();
    let lnk_str = lnk_path.to_string_lossy();

    let ps_script = format!(
        "$ws = New-Object -ComObject WScript.Shell; \
         $s = $ws.CreateShortcut('{}'); \
         $s.TargetPath = '{}'; \
         $s.Description = 'Screenshot capture and annotation tool'; \
         $s.Save()",
        lnk_str.replace('\'', "''"),
        exe_str.replace('\'', "''"),
    );

    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to run PowerShell: {e}"))?;

    if status.status.success() {
        println!("Created Start Menu shortcut");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&status.stderr);
        Err(format!("Failed to create shortcut: {stderr}"))
    }
}

#[cfg(target_os = "windows")]
fn add_to_path(dir: &std::path::Path) -> Result<(), String> {
    use windows::core::*;
    use windows::Win32::System::Registry::*;

    let dir_str = dir.to_string_lossy();

    unsafe {
        let mut key = HKEY::default();
        let path = w!("Environment");
        RegOpenKeyExW(HKEY_CURRENT_USER, path, 0, KEY_READ | KEY_WRITE, &mut key)
            .ok()
            .map_err(|e| format!("Failed to open Environment key: {e}"))?;

        // Read current PATH
        let mut buf_size: u32 = 0;
        let name = w!("Path");
        let _ = RegQueryValueExW(key, name, None, None, None, Some(&mut buf_size));

        let current_path = if buf_size > 0 {
            let mut buf = vec![0u8; buf_size as usize];
            RegQueryValueExW(key, name, None, None, Some(buf.as_mut_ptr()), Some(&mut buf_size))
                .ok()
                .map_err(|e| format!("Failed to read PATH: {e}"))?;
            let wide: &[u16] =
                std::slice::from_raw_parts(buf.as_ptr() as *const u16, buf.len() / 2);
            String::from_utf16_lossy(wide).trim_end_matches('\0').to_string()
        } else {
            String::new()
        };

        // Check if already in PATH
        let lower_dir = dir_str.to_lowercase();
        if current_path
            .split(';')
            .any(|p| p.trim().to_lowercase() == lower_dir)
        {
            let _ = RegCloseKey(key);
            return Ok(());
        }

        // Append
        let new_path = if current_path.is_empty() {
            dir_str.to_string()
        } else {
            format!("{};{}", current_path.trim_end_matches(';'), dir_str)
        };

        let value: Vec<u16> = new_path.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes: &[u8] =
            std::slice::from_raw_parts(value.as_ptr() as *const u8, value.len() * 2);
        RegSetValueExW(key, name, 0, REG_EXPAND_SZ, Some(bytes))
            .ok()
            .map_err(|e| format!("Failed to update PATH: {e}"))?;

        let _ = RegCloseKey(key);

        // Broadcast WM_SETTINGCHANGE so Explorer picks up the new PATH
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::*;
        let param = w!("Environment");
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(param.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            5000,
            None,
        );
    }

    println!("Added to user PATH");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn add_to_path(_dir: &std::path::Path) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn remove_from_path(dir: &std::path::Path) -> Result<(), String> {
    use windows::core::*;
    use windows::Win32::System::Registry::*;

    let dir_str = dir.to_string_lossy().to_lowercase();

    unsafe {
        let mut key = HKEY::default();
        let path = w!("Environment");
        RegOpenKeyExW(HKEY_CURRENT_USER, path, 0, KEY_READ | KEY_WRITE, &mut key)
            .ok()
            .map_err(|e| format!("Failed to open Environment key: {e}"))?;

        let mut buf_size: u32 = 0;
        let name = w!("Path");
        let _ = RegQueryValueExW(key, name, None, None, None, Some(&mut buf_size));

        if buf_size == 0 {
            let _ = RegCloseKey(key);
            return Ok(());
        }

        let mut buf = vec![0u8; buf_size as usize];
        RegQueryValueExW(key, name, None, None, Some(buf.as_mut_ptr()), Some(&mut buf_size))
            .ok()
            .map_err(|e| format!("Failed to read PATH: {e}"))?;

        let wide: &[u16] =
            std::slice::from_raw_parts(buf.as_ptr() as *const u16, buf.len() / 2);
        let current = String::from_utf16_lossy(wide).trim_end_matches('\0').to_string();

        let new_path: Vec<&str> = current
            .split(';')
            .filter(|p| !p.trim().is_empty() && p.trim().to_lowercase() != dir_str)
            .collect();
        let new_path = new_path.join(";");

        let value: Vec<u16> = new_path.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes: &[u8] =
            std::slice::from_raw_parts(value.as_ptr() as *const u8, value.len() * 2);
        RegSetValueExW(key, name, 0, REG_EXPAND_SZ, Some(bytes))
            .ok()
            .map_err(|e| format!("Failed to update PATH: {e}"))?;

        let _ = RegCloseKey(key);

        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::*;
        let param = w!("Environment");
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(param.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            5000,
            None,
        );
    }

    println!("Removed from user PATH");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn remove_from_path(_dir: &std::path::Path) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn schedule_delete_on_reboot(path: &std::path::Path) {
    use windows::core::*;
    use windows::Win32::Storage::FileSystem::*;

    let wide: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let _ = MoveFileExW(
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            MOVEFILE_DELAY_UNTIL_REBOOT,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn schedule_delete_on_reboot(_path: &std::path::Path) {}
