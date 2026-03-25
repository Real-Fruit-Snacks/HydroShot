use std::fs;

/// Skip history tests if no config directory is available (e.g., headless CI).
fn require_config_dir() -> bool {
    hydroshot::history::history_dir().is_some()
}

#[test]
fn save_creates_file_in_history_dir() {
    if !require_config_dir() {
        eprintln!("Skipping: no config directory available");
        return;
    }
    let pixels: Vec<u8> = vec![255, 0, 0, 255]; // 1x1 red pixel
    let result = hydroshot::history::save_to_history(&pixels, 1, 1);
    assert!(result.is_ok(), "save_to_history should succeed");
    let path = result.unwrap();
    assert!(path.exists(), "Saved file should exist on disk");
    assert_eq!(
        path.extension().and_then(|e| e.to_str()),
        Some("png"),
        "Should be a .png file"
    );
    let _ = fs::remove_file(&path);
}

#[test]
fn list_history_returns_entries_sorted_newest_first() {
    if !require_config_dir() {
        eprintln!("Skipping: no config directory available");
        return;
    }
    let pixels: Vec<u8> = vec![0, 0, 255, 255]; // 1x1 blue pixel

    let path1 = hydroshot::history::save_to_history(&pixels, 1, 1).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    let path2 = hydroshot::history::save_to_history(&pixels, 1, 1).unwrap();

    let entries = hydroshot::history::list_history();
    assert!(entries.len() >= 2, "Should have at least 2 entries");

    assert_eq!(
        entries[0].file_name(),
        path2.file_name(),
        "First entry should be newest"
    );

    let _ = fs::remove_file(&path1);
    let _ = fs::remove_file(&path2);
}

#[test]
fn prune_keeps_only_max_entries() {
    if !require_config_dir() {
        eprintln!("Skipping: no config directory available");
        return;
    }
    let pixels: Vec<u8> = vec![0, 255, 0, 255]; // 1x1 green pixel
    let mut paths = vec![];

    for _ in 0..22 {
        if let Ok(p) = hydroshot::history::save_to_history(&pixels, 1, 1) {
            paths.push(p);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let entries = hydroshot::history::list_history();
    assert!(
        entries.len() <= 20,
        "History should be pruned to at most 20, got {}",
        entries.len()
    );

    for p in &paths {
        let _ = fs::remove_file(p);
    }
    for e in &entries {
        let _ = fs::remove_file(e);
    }
}
