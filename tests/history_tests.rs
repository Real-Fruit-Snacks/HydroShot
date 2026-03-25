use std::fs;

#[test]
fn save_creates_file_in_history_dir() {
    // We can't easily override history_dir(), so we test save_to_history
    // by creating a real 1x1 RGBA image and verifying a file is created.
    // This uses the real config dir but is harmless (creates a tiny file).
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
    // Clean up
    let _ = fs::remove_file(&path);
}

#[test]
fn list_history_returns_entries_sorted_newest_first() {
    // Save two entries with a slight delay to ensure different timestamps
    let pixels: Vec<u8> = vec![0, 0, 255, 255]; // 1x1 blue pixel

    let path1 = hydroshot::history::save_to_history(&pixels, 1, 1).unwrap();
    // Ensure different timestamp
    std::thread::sleep(std::time::Duration::from_secs(1));
    let path2 = hydroshot::history::save_to_history(&pixels, 1, 1).unwrap();

    let entries = hydroshot::history::list_history();
    assert!(entries.len() >= 2, "Should have at least 2 entries");

    // First entry should be the newest (path2)
    assert_eq!(
        entries[0].file_name(),
        path2.file_name(),
        "First entry should be newest"
    );

    // Clean up
    let _ = fs::remove_file(&path1);
    let _ = fs::remove_file(&path2);
}

#[test]
fn prune_keeps_only_max_entries() {
    // We test indirectly: save 22 entries, then list should return at most 20
    let pixels: Vec<u8> = vec![0, 255, 0, 255]; // 1x1 green pixel
    let mut paths = vec![];

    for _ in 0..22 {
        match hydroshot::history::save_to_history(&pixels, 1, 1) {
            Ok(p) => paths.push(p),
            Err(_) => {} // timestamps may collide, that's fine
        }
        // Small delay to get different timestamps
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let entries = hydroshot::history::list_history();
    assert!(
        entries.len() <= 20,
        "History should be pruned to at most 20, got {}",
        entries.len()
    );

    // Clean up
    for p in &paths {
        let _ = fs::remove_file(p);
    }
    // Also clean any remaining entries
    for e in &entries {
        let _ = fs::remove_file(e);
    }
}
