//! Last-resort reporting does not replace returning normal errors or fixing deadlocks.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]
use std::{io::Write, path::PathBuf};
pub fn data_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(std::env::temp_dir)
}
pub fn payload_message(value: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = value.downcast_ref::<String>() {
        text.clone()
    } else if let Some(text) = value.downcast_ref::<&str>() {
        (*text).to_owned()
    } else {
        "Unexpected internal error".into()
    }
}
pub fn guard<T>(action: impl FnOnce() -> T) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(action))
        .map_err(|payload| payload_message(payload.as_ref()))
}
pub fn log(message: &str) -> Result<PathBuf, String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or_default();
    let mut errors = Vec::new();
    for folder in [data_dir(), std::env::temp_dir().join("AnomalyStudio")] {
        let path = folder.join("studio-error.log");
        let result = (|| {
            std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
            if std::fs::metadata(&path).is_ok_and(|m| m.len() > 2 * 1024 * 1024) {
                let old = folder.join("studio-error.previous.log");
                let _ = std::fs::remove_file(&old);
                std::fs::rename(&path, &old).map_err(|e| e.to_string())?;
            }
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|e| e.to_string())?;
            writeln!(
                file,
                "[{stamp}] Anomaly Studio {}\n{message}\n",
                env!("CARGO_PKG_VERSION")
            )
            .map_err(|e| e.to_string())
        })();
        match result {
            Ok(()) => return Ok(path),
            Err(error) => errors.push(error),
        }
    }
    Err(errors.join("; "))
}
static UI_THREAD: std::sync::OnceLock<std::thread::ThreadId> = std::sync::OnceLock::new();
static BACKGROUND_ERRORS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
pub fn pending_background_errors() -> Vec<String> {
    let mut errors = BACKGROUND_ERRORS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::mem::take(&mut *errors)
}
pub fn install_hook() {
    let _ = UI_THREAD.set(std::thread::current().id());
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let report = format!("{info}\n{}", std::backtrace::Backtrace::force_capture());
        if let Err(error) = log(&report) {
            eprintln!("Could not write error log: {error}\n{report}");
        }
        if UI_THREAD
            .get()
            .is_some_and(|id| *id != std::thread::current().id())
        {
            let mut errors = BACKGROUND_ERRORS
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if errors.len() < 16 {
                errors.push(info.to_string());
            }
        }
        previous(info);
    }));
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unexpected_panics_become_reportable_errors_and_keep_buffers() {
        let mut buffer = String::from("unsaved work");
        let result = guard(|| {
            buffer.push_str(" retained");
            panic!("synthetic editor error");
        });
        assert_eq!(result.unwrap_err(), "synthetic editor error");
        assert_eq!(buffer, "unsaved work retained");
        assert_eq!(guard(|| 42).unwrap(), 42);
    }
}
