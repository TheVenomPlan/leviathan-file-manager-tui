//! Iconos de archivos y carpetas.

use levi_core::config::IconsMode;
use levi_core::fs::{Entry, EntryKind};

/// Devuelve el icono para una entrada (cadena vacía si los iconos están desactivados).
pub fn icon(entry: &Entry, mode: IconsMode) -> &'static str {
    match mode {
        IconsMode::None => "",
        IconsMode::Ascii => match entry.kind {
            EntryKind::Dir => "d",
            EntryKind::Symlink => "l",
            EntryKind::Other => "?",
            EntryKind::File => "f",
        },
        IconsMode::Emoji => emoji_icon(entry),
        IconsMode::Nerd => nerd_icon(entry),
    }
}

fn emoji_icon(entry: &Entry) -> &'static str {
    match entry.kind {
        EntryKind::Dir => "📁",
        EntryKind::Symlink => "🔗",
        EntryKind::Other => "⚙️",
        EntryKind::File => {
            let name = entry.name.to_lowercase();
            let extension = name.rsplit_once('.').map_or("", |(_, ext)| ext);

            match extension {
                "rs" | "py" | "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "c" | "h" | "cc"
                | "cpp" | "hpp" | "go" | "java" | "rb" | "php" | "lua" | "sh" | "bash" | "zsh"
                | "fish" | "toml" | "yaml" | "yml" | "json" | "xml" | "html" | "css" | "scss"
                | "sql" => "📜",
                "md" | "markdown" | "txt" | "log" | "rst" => "📄",
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" => "🖼️",
                "mp4" | "mkv" | "avi" | "mov" | "webm" => "🎬",
                "mp3" | "flac" | "wav" | "ogg" | "m4a" => "🎵",
                "pdf" => "📕",
                "zip" | "tar" | "gz" | "xz" | "zst" | "7z" | "rar" | "bz2" => "📦",
                "xls" | "xlsx" | "csv" | "ods" => "📊",
                "doc" | "docx" | "odt" => "📝",
                "ppt" | "pptx" | "odp" => "📽️",
                "ttf" | "otf" | "woff" | "woff2" => "🔤",
                _ if name == ".gitignore" || name == ".gitattributes" => "🌿",
                _ if name.ends_with(".lock") => "🔒",
                _ => "📄",
            }
        }
    }
}

fn nerd_icon(entry: &Entry) -> &'static str {
    match entry.kind {
        // carpeta
        EntryKind::Dir => "\u{f07b}",
        // enlace
        EntryKind::Symlink => "\u{f0c1}",
        // engranaje
        EntryKind::Other => "\u{f013}",
        EntryKind::File => {
            let name = entry.name.to_lowercase();
            let extension = name.rsplit_once('.').map_or("", |(_, ext)| ext);

            match extension {
                "rs" | "py" | "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "c" | "h" | "cc"
                | "cpp" | "hpp" | "go" | "java" | "rb" | "php" | "lua" | "sh" | "bash" | "zsh"
                | "fish" | "toml" | "yaml" | "yml" | "json" | "xml" | "html" | "css" | "scss"
                | "sql" => "\u{f1c9}", // archivo de código
                "md" | "markdown" | "txt" | "log" | "rst" => "\u{f15c}", // texto
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" => "\u{f1c5}",
                "mp4" | "mkv" | "avi" | "mov" | "webm" => "\u{f1c8}",
                "mp3" | "flac" | "wav" | "ogg" | "m4a" => "\u{f1c7}",
                "pdf" => "\u{f1c1}",
                "zip" | "tar" | "gz" | "xz" | "zst" | "7z" | "rar" | "bz2" => "\u{f1c6}",
                "xls" | "xlsx" | "csv" | "ods" => "\u{f1c3}",
                "doc" | "docx" | "odt" => "\u{f1c2}",
                "ppt" | "pptx" | "odp" => "\u{f1c4}",
                "ttf" | "otf" | "woff" | "woff2" => "\u{f031}", // fuente
                _ => match &name[..] {
                    "cargo.toml" | "cargo.lock" => "\u{f1c9}",
                    ".gitignore" | ".gitattributes" => "\u{f1d3}", // git
                    _ if name.ends_with(".lock") => "\u{f023}",    // candado
                    _ => "\u{f15b}",                               // archivo genérico
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use levi_core::fs::Entry;
    use std::path::PathBuf;

    fn entry(name: &str, kind: EntryKind) -> Entry {
        Entry {
            name: name.to_string(),
            path: PathBuf::from(name),
            kind,
            size: 0,
            modified: None,
            created: None,
            permissions: String::new(),
            hidden: false,
        }
    }

    #[test]
    fn nerd_icons_are_not_empty() {
        assert_eq!(
            icon(&entry("src", EntryKind::Dir), IconsMode::Nerd),
            "\u{f07b}"
        );
        assert_eq!(
            icon(&entry("main.rs", EntryKind::File), IconsMode::Nerd),
            "\u{f1c9}"
        );
        assert_eq!(
            icon(&entry("foto.png", EntryKind::File), IconsMode::Nerd),
            "\u{f1c5}"
        );
        assert_eq!(
            icon(&entry("desconocido.xyz", EntryKind::File), IconsMode::Nerd),
            "\u{f15b}"
        );
        assert_eq!(icon(&entry("src", EntryKind::Dir), IconsMode::None), "");
    }

    #[test]
    fn emoji_icons_are_larger_glyphs() {
        assert_eq!(icon(&entry("src", EntryKind::Dir), IconsMode::Emoji), "📁");
        assert_eq!(
            icon(&entry("foto.png", EntryKind::File), IconsMode::Emoji),
            "🖼️"
        );
        assert_eq!(
            icon(&entry("main.rs", EntryKind::File), IconsMode::Emoji),
            "📜"
        );
    }
}
