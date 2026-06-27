#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;
