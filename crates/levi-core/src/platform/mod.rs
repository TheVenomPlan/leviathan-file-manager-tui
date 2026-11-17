//! Código específico de cada sistema operativo.
//!
//! Ningún otro módulo del crate debe usar APIs de `std::os::*` directamente.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::{created_at, is_hidden, mount_points, permissions, trash_dir};
#[cfg(windows)]
pub use windows::{created_at, is_hidden, mount_points, permissions, trash_dir};
