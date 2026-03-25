use std::path::PathBuf;

const MAX_HISTORY: usize = 20;

pub fn history_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("hydroshot").join("history"))
}

/// Save a capture to history. Filename: timestamp-based (e.g., "1711234567.png")
pub fn save_to_history(pixels: &[u8], width: u32, height: u32) -> Result<PathBuf, String> {
    let dir = history_dir().ok_or("No config directory")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let filename = format!("{}.png", timestamp);
    let path = dir.join(&filename);

    let img =
        image::RgbaImage::from_raw(width, height, pixels.to_vec()).ok_or("Invalid image data")?;
    img.save(&path).map_err(|e| e.to_string())?;

    // Prune old entries (keep only MAX_HISTORY most recent)
    prune_history(&dir);

    Ok(path)
}

/// Get list of history entries sorted newest first
pub fn list_history() -> Vec<PathBuf> {
    let Some(dir) = history_dir() else {
        return vec![];
    };
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|ext| ext == "png").unwrap_or(false))
        .collect();
    // Sort by filename (timestamp) descending
    entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    entries.truncate(MAX_HISTORY);
    entries
}

fn prune_history(dir: &std::path::Path) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|ext| ext == "png").unwrap_or(false))
        .collect();
    entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    // Remove entries beyond MAX_HISTORY
    for path in entries.iter().skip(MAX_HISTORY) {
        let _ = std::fs::remove_file(path);
    }
}
