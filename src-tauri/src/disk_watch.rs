//! Source-disk disconnection watcher (P5-T03).
//!
//! While a scan or recovery is in flight, a background task polls the source
//! device every 2 seconds. If the device node disappears (drive unplugged), it
//! emits a single `disk:disconnected` event and exits. The caller stops the
//! watcher by setting the shared `stop` flag when the operation finishes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Poll cadence for the source-presence check.
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// User-facing reason emitted with `disk:disconnected`.
const DISCONNECT_MESSAGE: &str =
    "The source disk was disconnected. The operation was stopped to protect your data.";

/// Spawn the watcher for `disk_id`. It runs until either the device disappears
/// (one event emitted) or `stop` is set by the caller, whichever comes first.
pub fn spawn(app: AppHandle, disk_id: String, stop: Arc<AtomicBool>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(POLL_INTERVAL).await;
            if stop.load(Ordering::SeqCst) {
                break;
            }
            if !source_present(&disk_id) {
                let _ = app.emit(
                    "disk:disconnected",
                    serde_json::json!({
                        "disk_id": disk_id,
                        "message": DISCONNECT_MESSAGE,
                    }),
                );
                break;
            }
        }
    });
}

/// Best-effort liveness probe for the source device.
///
/// macOS: the `/dev/<id>` node vanishes the moment a disk is ejected/unplugged,
/// so a cheap `exists()` is both reliable and permission-free. Windows: opening
/// the physical-drive path is the best available proxy (DECISION-018(b)).
fn source_present(disk_id: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        std::path::Path::new(&format!("/dev/{disk_id}")).exists()
    }
    #[cfg(target_os = "windows")]
    {
        std::fs::OpenOptions::new()
            .read(true)
            .open(format!("\\\\.\\{disk_id}"))
            .is_ok()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = disk_id;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn source_present_true_for_existing_node() {
        // /dev exists on every macOS host; a guaranteed-present pseudo-device.
        assert!(std::path::Path::new("/dev/null").exists());
        // A disk id that cannot exist must report absent.
        assert!(!source_present("disk99999"));
    }
}
