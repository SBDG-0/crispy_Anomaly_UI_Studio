use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};
#[derive(Default, Debug)]
pub struct Report {
    pub files: Vec<PathBuf>,
    pub errors: Vec<String>,
}
impl Report {
    pub fn store(&mut self, folder: &Path, origin: &Path, bytes: Result<Vec<u8>, String>) {
        let result = bytes.and_then(|bytes| {
            std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            origin.hash(&mut hasher);
            let name = origin
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("document"))
                .to_string_lossy();
            let path = folder.join(format!("{:016x}-{name}", hasher.finish()));
            // A failed write must not truncate the last good recovery copy.
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|v| v.as_nanos())
                .unwrap_or_default();
            let temporary = folder.join(format!(".recovery-{}-{stamp}.tmp", std::process::id()));
            std::fs::write(&temporary, &bytes)
                .map_err(|e| format!("{}: {e}", temporary.display()))?;
            std::fs::rename(&temporary, &path).map_err(|e| {
                format!(
                    "Could not replace recovery copy {}: {e}. Temporary copy: {}",
                    path.display(),
                    temporary.display()
                )
            })?;
            std::fs::write(
                path.with_file_name(format!("{:016x}-{name}.origin.txt", hasher.finish())),
                origin.to_string_lossy().as_bytes(),
            )
            .map_err(|e| e.to_string())?;
            Ok(path)
        });
        match result {
            Ok(path) => self.files.push(path),
            Err(error) => self.errors.push(format!("{}: {error}", origin.display())),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copies_recovery_without_touching_source_and_reports_failed_writes() {
        let dir = std::env::temp_dir().join(format!("studio-recovery-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let origin = dir.join("source.xml");
        std::fs::write(&origin, b"original").unwrap();
        let mut report = Report::default();
        report.store(&dir.join("copies"), &origin, Ok(b"pending".to_vec()));
        assert_eq!(report.files.len(), 1);
        assert_eq!(std::fs::read(&origin).unwrap(), b"original");
        assert_eq!(std::fs::read(&report.files[0]).unwrap(), b"pending");
        report.store(&dir.join("copies"), &origin, Ok(b"new pending".to_vec()));
        assert!(report.errors.is_empty());
        assert_eq!(std::fs::read(&report.files[0]).unwrap(), b"new pending");
        let blocked = dir.join("not-a-folder");
        std::fs::write(&blocked, b"x").unwrap();
        report.store(&blocked, &origin, Ok(b"pending".to_vec()));
        assert_eq!(report.errors.len(), 1);
    }
}
