//! Crash logging (P5-T03).
//!
//! Installs a panic hook that appends a one-line record to a platform log file
//! before the process unwinds, then chains the default hook so the standard
//! stderr message and abort behaviour are preserved. The logger is strictly
//! best-effort: any IO failure inside the hook is swallowed, because a panic
//! handler that itself panics would abort the process with a confusing message.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Install the crash-logging panic hook. Call once, as early as possible in
/// `run()`, before the Tauri runtime is built.
pub fn install_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        write_panic(info);
        default(info);
    }));
}

/// Append one panic record to the log file. All failures are intentionally
/// ignored — see the module docs.
fn write_panic(info: &std::panic::PanicHookInfo<'_>) {
    let dir = log_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("panic.log"))
    else {
        return;
    };
    let location = info.location().map_or_else(
        || "unknown".to_string(),
        |l| format!("{}:{}", l.file(), l.line()),
    );
    let _ = writeln!(
        file,
        "{}",
        panic_line(unix_secs(), &location, &payload_str(info))
    );
}

/// Extract a printable message from the panic payload (`&str` or `String`).
fn payload_str(info: &std::panic::PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

/// Build the log line. Pure — unit-tested.
fn panic_line(ts: u64, location: &str, message: &str) -> String {
    format!("[{ts}] panic at {location}: {message}")
}

/// Seconds since the Unix epoch (0 if the clock is before the epoch).
fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Platform log directory. Dependency-free: resolved from environment so it is
/// available the instant the hook fires, without the Tauri path API.
fn log_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join("Library/Logs/FileResque");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("FileResque").join("logs");
        }
    }
    std::env::temp_dir().join("FileResque")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_line_has_expected_shape() {
        let line = panic_line(1_700_000_000, "src/foo.rs:42", "boom");
        assert_eq!(line, "[1700000000] panic at src/foo.rs:42: boom");
    }

    #[test]
    fn log_dir_is_app_scoped() {
        let dir = log_dir();
        assert!(
            dir.to_string_lossy().contains("FileResque"),
            "log dir not app-scoped: {}",
            dir.display()
        );
    }

    #[test]
    fn unix_secs_is_after_2020() {
        // 2020-01-01 in epoch seconds; guards against a wildly wrong clock impl.
        assert!(unix_secs() > 1_577_836_800);
    }
}
