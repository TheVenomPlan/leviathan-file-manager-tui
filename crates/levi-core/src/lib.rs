//! Núcleo de Levi File Manager TUI.
//!
//! Este crate no conoce nada de la interfaz: ni `ratatui` ni `crossterm`.
//! Todo lo que depende del sistema operativo vive en [`platform`].

pub mod config;
pub mod editor;
pub mod fs;
pub mod i18n;
pub mod keymap;
pub mod ops;
pub mod platform;
pub mod text;
pub mod theme;
