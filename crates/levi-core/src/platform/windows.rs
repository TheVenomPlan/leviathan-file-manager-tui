//! Implementación para Windows.

use std::ffi::OsStr;
use std::fs::Metadata;
use std::os::windows::fs::MetadataExt;
use std::time::SystemTime;

const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

/// En Windows es oculto si el sistema lo marca o si el nombre empieza por punto
/// (convención heredada de las herramientas Unix).
pub fn is_hidden(name: &OsStr, meta: &Metadata) -> bool {
    name.to_string_lossy().starts_with('.') || meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
}

/// Windows no tiene permisos POSIX: mostramos solo lectura o lectura/escritura.
pub fn permissions(meta: &Metadata) -> String {
    if meta.permissions().readonly() {
        "r--".to_string()
    } else {
        "rw-".to_string()
    }
}

/// En Windows la fecha de creación sí está disponible de forma fiable.
pub fn created_at(meta: &Metadata) -> Option<SystemTime> {
    meta.created().ok()
}

/// Unidades disponibles (discos) para mostrar en la interfaz.
pub fn mount_points() -> Vec<std::path::PathBuf> {
    let mut points = Vec::new();
    for letter in b'A'..=b'Z' {
        let path = std::path::PathBuf::from(format!("{}:\\", letter as char));
        if path.exists() {
            points.push(path);
        }
    }
    points
}

/// Papelera de reciclaje: no se expone como ruta navegable simple en Windows.
pub fn trash_dir() -> Option<std::path::PathBuf> {
    None
}
