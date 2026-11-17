//! Implementación para sistemas Unix (Linux, macOS, BSD).

use std::ffi::OsStr;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::time::SystemTime;

/// Un archivo es visible si no empieza por punto.
pub fn is_hidden(name: &OsStr, _meta: &Metadata) -> bool {
    name.to_string_lossy().starts_with('.')
}

/// Permisos estilo `ls -l`, por ejemplo `drwxr-xr-x`.
pub fn permissions(meta: &Metadata) -> String {
    let mode = meta.mode();
    let mut out = String::with_capacity(10);
    out.push(if meta.is_dir() {
        'd'
    } else if meta.file_type().is_symlink() {
        'l'
    } else {
        '-'
    });

    const BITS: [(u32, char); 9] = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    for (bit, ch) in BITS {
        out.push(if mode & bit != 0 { ch } else { '-' });
    }
    out
}

/// Fecha de creación. En Linux suele no estar disponible (`btime`).
pub fn created_at(meta: &Metadata) -> Option<SystemTime> {
    meta.created().ok()
}

/// Puntos de montaje útiles (discos/volúmenes) para mostrar en la interfaz.
pub fn mount_points() -> Vec<std::path::PathBuf> {
    use std::path::PathBuf;

    let mut points: Vec<PathBuf> = Vec::new();
    points.push(PathBuf::from("/"));

    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        if home.exists() && !points.contains(&home) {
            points.push(home);
        }
    }

    let Ok(mounts) = std::fs::read_to_string("/proc/mounts") else {
        return points;
    };

    for line in mounts.lines() {
        // device mountpoint fstype options ...
        let mut parts = line.split_whitespace();
        let _device = parts.next();
        let Some(mountpoint) = parts.next() else {
            continue;
        };
        let fstype = parts.next().unwrap_or_default();
        // Decodifica escapes comunes de /proc/mounts.
        let mountpoint = mountpoint
            .replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\012", "\n")
            .replace("\\134", "\\");
        let path = PathBuf::from(&mountpoint);
        if !path.is_dir() {
            continue;
        }
        let dominated = mountpoint.starts_with("/proc")
            || mountpoint.starts_with("/sys")
            || mountpoint.starts_with("/dev")
            || mountpoint.starts_with("/run/user")
            || mountpoint.starts_with("/snap")
            || mountpoint.starts_with("/var/lib/snapd")
            || fstype == "tmpfs"
            || fstype == "devtmpfs"
            || fstype == "squashfs"
            || fstype == "overlay";
        let interesting = mountpoint.starts_with("/media")
            || mountpoint.starts_with("/run/media")
            || mountpoint.starts_with("/mnt");
        if (interesting || (!dominated && mountpoint != "/")) && !points.contains(&path) {
            points.push(path);
        }
    }
    points
}

/// Directorio de la papelera del usuario (freedesktop), si existe o se puede inferir.
pub fn trash_dir() -> Option<std::path::PathBuf> {
    let base = if let Some(data) = std::env::var_os("XDG_DATA_HOME") {
        std::path::PathBuf::from(data)
    } else {
        let home = std::env::var_os("HOME")?;
        std::path::PathBuf::from(home).join(".local/share")
    };
    let trash = base.join("Trash/files");
    if trash.exists() {
        Some(trash)
    } else {
        let root = base.join("Trash");
        if root.exists() {
            Some(root)
        } else {
            Some(trash)
        }
    }
}
