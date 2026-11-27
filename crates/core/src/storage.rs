use std::path::Path;
/// Keep the existing file and a complete replacement until the final rename.
pub fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let name = path
        .file_name()
        .ok_or("Choose a file path")?
        .to_string_lossy();
    if path.exists() {
        let backup = path.with_file_name(format!("{name}.{stamp}.bak"));
        std::fs::copy(path, backup).map_err(|e| format!("Could not back up destination: {e}"))?;
    }
    let temporary = path.with_file_name(format!(".{name}.{stamp}.tmp"));
    std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    if std::fs::rename(&temporary, path).is_err() {
        // Windows folder permissions can prohibit replacement by rename. Both
        // the original backup and complete new file exist before this fallback.
        std::fs::write(path, bytes).map_err(|e| {
            format!(
                "Could not save: {e}. Complete replacement retained at {}",
                temporary.display()
            )
        })?;
        let _ = std::fs::remove_file(&temporary);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_keeps_backup() {
        let dir = std::env::temp_dir().join(format!("icon-storage-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("file.ltx");
        std::fs::write(&path, b"old").unwrap();
        write(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert!(std::fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.path().extension().is_some_and(|s| s == "bak")
                && std::fs::read(e.path()).unwrap() == b"old"));
    }
}

/// Reject saving over an externally changed or removed source.
pub fn write_checked(path: &Path, expected: &[u8], bytes: &[u8]) -> Result<(), String> {
    let current =
        std::fs::read(path).map_err(|e| format!("Cannot check source {}: {e}", path.display()))?;
    if current != expected {
        return Err("Source changed outside the editor. Reload it before saving.".into());
    }
    write(path, bytes)
}
