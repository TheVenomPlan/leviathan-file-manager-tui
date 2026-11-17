//! Listado de directorios y descripción de entradas del sistema de archivos.

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local};

use crate::i18n::{Language, strings};
use crate::platform;

/// Tipo de una entrada del directorio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Dir,
    File,
    Symlink,
    Other,
}

impl EntryKind {
    /// Etiqueta para mostrar en la interfaz (en español).
    pub fn label(self) -> &'static str {
        self.label_in(Language::Es)
    }

    /// Etiqueta traducida al idioma indicado.
    pub fn label_in(self, language: Language) -> &'static str {
        let strings = strings(language);
        match self {
            EntryKind::Dir => strings.kind_dir,
            EntryKind::File => strings.kind_file,
            EntryKind::Symlink => strings.kind_symlink,
            EntryKind::Other => strings.kind_other,
        }
    }
}

/// Una entrada de un directorio, con los metadatos ya resueltos.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub permissions: String,
    pub hidden: bool,
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        matches!(self.kind, EntryKind::Dir)
    }
}

/// Errores al leer un directorio.
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("no se pudo leer «{}»: {source}", path.display())]
    ReadDir {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Lee un directorio y devuelve sus entradas ordenadas (carpetas primero).
///
/// Las entradas cuyo tipo o metadatos no se pueden leer se omiten en lugar de
/// abortar el listado completo.
pub fn list_dir(path: &Path, show_hidden: bool) -> Result<Vec<Entry>, FsError> {
    let reader = fs::read_dir(path).map_err(|source| FsError::ReadDir {
        path: path.to_path_buf(),
        source,
    })?;

    let mut entries = Vec::new();
    for item in reader.flatten() {
        let name_os = item.file_name();
        let name = name_os.to_string_lossy().into_owned();
        let Ok(file_type) = item.file_type() else {
            continue;
        };
        let Ok(meta) = item.metadata() else {
            continue;
        };

        let hidden = platform::is_hidden(&name_os, &meta);
        if hidden && !show_hidden {
            continue;
        }

        let kind = if file_type.is_symlink() {
            EntryKind::Symlink
        } else if file_type.is_dir() {
            EntryKind::Dir
        } else if file_type.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        };

        entries.push(Entry {
            name,
            path: item.path(),
            kind,
            size: meta.len(),
            modified: meta.modified().ok(),
            created: platform::created_at(&meta),
            permissions: platform::permissions(&meta),
            hidden,
        });
    }

    sort_entries(&mut entries);
    Ok(entries)
}

/// Busca en un directorio y sus subdirectorios coincidencias por nombre.
///
/// Se usa para el super buscador (`Ctrl+Shift+F`): no se limita a la carpeta
/// activa, sino que recorre el directorio indicado. Los errores de permisos se
/// ignoran para no abortar la búsqueda.
pub fn search_global(root: &Path, query: &str, show_hidden: bool, limit: usize) -> Vec<Entry> {
    let needle = crate::text::normalize_search(query);
    if needle.is_empty() || limit == 0 {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(path) = stack.pop() {
        if results.len() >= limit {
            break;
        }

        let Ok(reader) = fs::read_dir(&path) else {
            continue;
        };

        for item in reader.flatten() {
            let name_os = item.file_name();
            let name = name_os.to_string_lossy().into_owned();
            let Ok(file_type) = item.file_type() else {
                continue;
            };
            let Ok(meta) = item.metadata() else {
                continue;
            };
            let hidden = platform::is_hidden(&name_os, &meta);
            if hidden && !show_hidden {
                continue;
            }

            let kind = if file_type.is_symlink() {
                EntryKind::Symlink
            } else if file_type.is_dir() {
                EntryKind::Dir
            } else if file_type.is_file() {
                EntryKind::File
            } else {
                EntryKind::Other
            };

            let entry = Entry {
                name,
                path: item.path(),
                kind,
                size: meta.len(),
                modified: meta.modified().ok(),
                created: platform::created_at(&meta),
                permissions: platform::permissions(&meta),
                hidden,
            };

            if crate::text::normalize_search(&entry.name).contains(&needle) {
                results.push(entry);
                if results.len() >= limit {
                    break;
                }
            }

            if let EntryKind::Dir = kind {
                stack.push(item.path().to_path_buf());
            }
        }
    }

    results.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    results
}

/// Ordena carpetas primero y después por orden natural (ej. `file2` < `file10`).
pub fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        b.is_dir()
            .cmp(&a.is_dir())
            .then_with(|| natural_cmp(&a.name, &b.name))
    });
}

/// Comparación natural, sin distinguir mayúsculas y con los números tratados
/// como valores y no como texto.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let a: Vec<char> = a.chars().flat_map(char::to_lowercase).collect();
    let b: Vec<char> = b.chars().flat_map(char::to_lowercase).collect();
    let (mut i, mut j) = (0usize, 0usize);

    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let start_a = i;
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            let start_b = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }

            let digits_a: String = a[start_a..i].iter().collect();
            let digits_b: String = b[start_b..j].iter().collect();
            let trimmed_a = digits_a.trim_start_matches('0');
            let trimmed_b = digits_b.trim_start_matches('0');

            let ord = trimmed_a
                .len()
                .cmp(&trimmed_b.len())
                .then_with(|| trimmed_a.cmp(trimmed_b));
            if ord != Ordering::Equal {
                return ord;
            }
        } else {
            if a[i] != b[j] {
                return a[i].cmp(&b[j]);
            }
            i += 1;
            j += 1;
        }
    }

    a.len().cmp(&b.len())
}

/// Tamaño legible: `0 B`, `1.5 KiB`, `12 MiB`, `3.4 GiB`...
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if size < 10.0 {
        format!("{size:.1} {}", UNITS[unit])
    } else {
        format!("{size:.0} {}", UNITS[unit])
    }
}

/// Construye una entrada representativa para una ruta conocida (ubicación especial).
pub fn entry_for_path(name: String, path: &Path) -> Option<Entry> {
    let meta = fs::metadata(path).ok()?;
    let kind = if meta.is_dir() {
        EntryKind::Dir
    } else {
        EntryKind::File
    };
    Some(Entry {
        name,
        path: path.to_path_buf(),
        kind,
        size: meta.len(),
        modified: meta.modified().ok(),
        created: platform::created_at(&meta),
        permissions: platform::permissions(&meta),
        hidden: false,
    })
}

/// Ubicaciones del sistema: discos/volúmenes y papelera.
pub fn system_location_entries() -> Vec<Entry> {
    let mut entries = Vec::new();

    for path in platform::mount_points() {
        let name = if path.as_os_str() == "/" {
            "Disco /".to_string()
        } else if path
            .parent()
            .is_none_or(|p| p.as_os_str().is_empty() || p.as_os_str() == "/")
        {
            format!("Disco {}", path.display())
        } else {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| format!("Disco {n}"))
                .unwrap_or_else(|| format!("Disco {}", path.display()))
        };
        if let Some(entry) = entry_for_path(name, &path)
            && !entries.iter().any(|e: &Entry| e.path == entry.path)
        {
            entries.push(entry);
        }
    }

    if let Some(trash) = platform::trash_dir() {
        if let Some(entry) = entry_for_path("Papelera".to_string(), &trash) {
            entries.push(entry);
        } else {
            // Aunque no exista aún, ofrece la entrada para crear/acceder.
            entries.push(Entry {
                name: "Papelera".to_string(),
                path: trash,
                kind: EntryKind::Dir,
                size: 0,
                modified: None,
                created: None,
                permissions: String::new(),
                hidden: false,
            });
        }
    }

    entries
}

/// Calcula el tamaño total de un directorio recorriéndolo recursivamente.
///
/// Ignora errores de lectura/permisos y enlaces simbólicos para evitar bucles.
pub fn dir_size(path: &Path) -> u64 {
    let mut total: u64 = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read_dir.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file()
                && let Ok(meta) = entry.metadata()
            {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

/// Fecha y hora local con formato `YYYY-MM-DD HH:MM`.
pub fn format_time(time: SystemTime) -> String {
    let local: DateTime<Local> = time.into();
    local.format("%Y-%m-%d %H:%M").to_string()
}

/// Igual que [`format_time`] pero tolera la ausencia de la fecha.
pub fn format_time_opt(time: Option<SystemTime>) -> String {
    time.map_or_else(|| "desconocido".to_string(), format_time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn human_size_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1023), "1023 B");
        assert_eq!(human_size(1024), "1.0 KiB");
        assert_eq!(human_size(1536), "1.5 KiB");
        assert_eq!(human_size(10 * 1024), "10 KiB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MiB");
    }

    #[test]
    fn natural_order_compares_numbers_by_value() {
        assert_eq!(natural_cmp("file2", "file10"), Ordering::Less);
        assert_eq!(natural_cmp("file10", "file2"), Ordering::Greater);
        assert_eq!(natural_cmp("A", "a"), Ordering::Equal);
        assert_eq!(natural_cmp("dir", "dir"), Ordering::Equal);
        assert_eq!(natural_cmp("a1b", "a1c"), Ordering::Less);
    }

    #[test]
    fn list_dir_puts_directories_first_and_hides_dotfiles() {
        let dir = tempfile::tempdir().expect("tempdir");
        File::create(dir.path().join("b.txt")).expect("archivo");
        File::create(dir.path().join("a10.txt")).expect("archivo");
        File::create(dir.path().join("a2.txt")).expect("archivo");
        fs::create_dir(dir.path().join("zeta")).expect("carpeta");
        File::create(dir.path().join(".oculto")).expect("oculto");

        let entries = list_dir(dir.path(), false).expect("listado");
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["zeta", "a2.txt", "a10.txt", "b.txt"]);

        let entries = list_dir(dir.path(), true).expect("listado con ocultos");
        assert!(entries.iter().any(|e| e.name == ".oculto"));
    }

    #[test]
    fn search_global_finds_entries_in_subdirectories() {
        let root = tempfile::tempdir().expect("tempdir");
        let docs = root.path().join("docs");
        std::fs::create_dir_all(&docs).expect("carpeta docs");
        let file = docs.join("levi.md");
        std::fs::write(&file, "# Levi").expect("archivo");

        let results = search_global(root.path(), "levi", false, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, file);
    }
}
