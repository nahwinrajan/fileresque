#![deny(clippy::all, clippy::pedantic, warnings)]
#![allow(clippy::module_name_repetitions)]
// Internal field-layout comments use "field_name(N)" notation — backticks not needed.
#![allow(clippy::doc_markdown)]

/// Cross-platform filesystem facts about a destination path (recovery
/// pre-flight). Pure helpers are tested on all platforms; the `dest_info`
/// platform call is gated internally.
pub mod fsinfo;

#[cfg(target_os = "macos")]
pub mod macos;

// The `windows` module is compiled on all platforms so that its pure-Rust
// parsing layer (enumerate.rs) can be unit-tested on macOS / Linux CI.
// The Win32 I/O layer (ioctl.rs) is still gated to `cfg(target_os = "windows")`
// inside the module, so no Windows-specific code reaches non-Windows builds.
pub mod windows;
