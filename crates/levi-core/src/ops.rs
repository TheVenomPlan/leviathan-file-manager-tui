//! Operaciones sobre archivos y directorios (copiar, mover, renombrar, borrar).
//!
//! Todas las rutas son [`PathBuf`]. Las operaciones recursivas se implementan
//! aquí para no depender de crates externos de recorrido.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Errores de las operaciones sobre el sistema de archivos.
#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error("la entrada «{path}» ya existe", path = path.display())]
    AlreadyExists { path: PathBuf },
    #[error("no se pudo leer «{}»: {source}", path.display())]
    Read { path: PathBuf, source: io::Error },
    #[error("no se pudo escribir «{}»: {source}", path.display())]
    Write { path: PathBuf, source: io::Error },
    #[error("no se pudo borrar «{}»: {source}", path.display())]
    Remove { path: PathBuf, source: io::Error },
    #[error("no se pudo renombrar «{}»: {source}", path.display())]
    Rename { path: PathBuf, source: io::Error },
    #[error("error de papelera: {0}")]
    Trash(String),
    #[error("nombre vacío")]
    EmptyName,
    #[error("nombre inválido: {0}")]
    InvalidName(String),
}

/// Copia una lista de entradas dentro de `dest_dir`. Devuelve cuántas se copiaron.
pub fn copy_paths(sources: &[PathBuf], dest_dir: &Path) -> Result<usize, OpError> {
    let mut count = 0;
    for source in sources {
        let name = source
            .file_name()
            .ok_or_else(|| OpError::InvalidName(source.display().to_string()))?;
        let dest = unique_dest(dest_dir, name);
        copy_recursive(source, &dest)?;
        count += 1;
    }
    Ok(count)
}

/// Mueve una lista de entradas dentro de `dest_dir`. Devuelve cuántas se movieron.
pub fn move_paths(sources: &[PathBuf], dest_dir: &Path) -> Result<usize, OpError> {
    let mut count = 0;
    for source in sources {
        let name = source
            .file_name()
            .ok_or_else(|| OpError::InvalidName(source.display().to_string()))?;
        let dest = unique_dest(dest_dir, name);
        if fs::rename(source, &dest).is_err() {
            // Movimiento entre discos: copiar y luego borrar.
            copy_recursive(source, &dest)?;
            remove_recursive(source)?;
        }
        count += 1;
    }
    Ok(count)
}

/// Renombra `source` a `new_name` dentro de su propio directorio.
pub fn rename_path(source: &Path, new_name: &str) -> Result<PathBuf, OpError> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err(OpError::EmptyName);
    }
    if new_name.contains('/') || new_name.contains('\\') {
        return Err(OpError::InvalidName(new_name.to_string()));
    }
    let parent = source
        .parent()
        .ok_or_else(|| OpError::InvalidName(source.display().to_string()))?;
    let dest = parent.join(new_name);
    if dest.exists() && dest != source {
        return Err(OpError::AlreadyExists { path: dest.clone() });
    }
    fs::rename(source, &dest).map_err(|source_err| OpError::Rename {
        path: source.to_path_buf(),
        source: source_err,
    })?;
    Ok(dest)
}

/// Mueve las entradas indicadas a la papelera del sistema.
pub fn trash_paths(sources: &[PathBuf]) -> Result<usize, OpError> {
    if sources.is_empty() {
        return Ok(0);
    }
    trash::delete_all(sources).map_err(|err| OpError::Trash(err.to_string()))?;
    Ok(sources.len())
}

/// Borra permanentemente las entradas indicadas.
pub fn delete_permanently(sources: &[PathBuf]) -> Result<usize, OpError> {
    let mut count = 0;
    for source in sources {
        remove_recursive(source)?;
        count += 1;
    }
    Ok(count)
}

/// Crea un archivo vacío con el nombre indicado dentro de `dir`.
pub fn create_file(dir: &Path, name: &str) -> Result<PathBuf, OpError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(OpError::EmptyName);
    }
    if name.contains('/') || name.contains('\\') {
        return Err(OpError::InvalidName(name.to_string()));
    }
    let path = dir.join(name);
    if path.exists() {
        return Err(OpError::AlreadyExists { path });
    }
    fs::write(&path, b"").map_err(|source| OpError::Write {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// Crea un directorio con el nombre indicado dentro de `dir`.
pub fn create_dir(dir: &Path, name: &str) -> Result<PathBuf, OpError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(OpError::EmptyName);
    }
    if name.contains('/') || name.contains('\\') {
        return Err(OpError::InvalidName(name.to_string()));
    }
    let path = dir.join(name);
    if path.exists() {
        return Err(OpError::AlreadyExists { path });
    }
    fs::create_dir(&path).map_err(|source| OpError::Write {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// Genera una ruta de destino que no colisione con archivos existentes.
fn unique_dest(dest_dir: &Path, name: &std::ffi::OsStr) -> PathBuf {
    let candidate = dest_dir.join(name);
    if !candidate.exists() {
        return candidate;
    }

    let path = std::path::Path::new(name);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("archivo");
    let ext = path.extension().and_then(|s| s.to_str());

    for i in 1..=1000 {
        let new_name = match ext {
            Some(ext) if !ext.is_empty() => format!("{stem} ({i}).{ext}"),
            _ => format!("{stem} ({i})"),
        };
        let candidate = dest_dir.join(new_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    candidate
}

fn copy_recursive(source: &Path, dest: &Path) -> Result<(), OpError> {
    let meta = fs::symlink_metadata(source).map_err(|source_err| OpError::Read {
        path: source.to_path_buf(),
        source: source_err,
    })?;

    if meta.is_dir() && !meta.file_type().is_symlink() {
        fs::create_dir(dest).map_err(|source_err| OpError::Write {
            path: dest.to_path_buf(),
            source: source_err,
        })?;
        for entry in fs::read_dir(source).map_err(|source_err| OpError::Read {
            path: source.to_path_buf(),
            source: source_err,
        })? {
            let entry = entry.map_err(|source_err| OpError::Read {
                path: source.to_path_buf(),
                source: source_err,
            })?;
            copy_recursive(&entry.path(), &dest.join(entry.file_name()))?;
        }
    } else {
        fs::copy(source, dest).map_err(|source_err| OpError::Write {
            path: dest.to_path_buf(),
            source: source_err,
        })?;
    }
    Ok(())
}

fn remove_recursive(path: &Path) -> Result<(), OpError> {
    let meta = fs::symlink_metadata(path).map_err(|source_err| OpError::Read {
        path: path.to_path_buf(),
        source: source_err,
    })?;
    if meta.is_dir() && !meta.file_type().is_symlink() {
        fs::remove_dir_all(path).map_err(|source_err| OpError::Remove {
            path: path.to_path_buf(),
            source: source_err,
        })?;
    } else {
        fs::remove_file(path).map_err(|source_err| OpError::Remove {
            path: path.to_path_buf(),
            source: source_err,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn copy_and_move_work() {
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("a.txt");
        File::create(&src).expect("archivo");
        let dest = dir.path().join("dest");
        fs::create_dir(&dest).expect("carpeta");

        assert_eq!(
            copy_paths(std::slice::from_ref(&src), &dest).expect("copiar"),
            1
        );
        assert!(dest.join("a.txt").exists());
        assert!(src.exists());

        let src2 = dir.path().join("b.txt");
        File::create(&src2).expect("archivo");
        assert_eq!(
            move_paths(std::slice::from_ref(&src2), &dest).expect("mover"),
            1
        );
        assert!(dest.join("b.txt").exists());
        assert!(!src2.exists());
    }

    #[test]
    fn rename_and_create() {
        let dir = tempdir().expect("tempdir");
        let file = create_file(dir.path(), "nuevo.txt").expect("crear");
        assert!(file.exists());
        let renamed = rename_path(&file, "cambiado.txt").expect("renombrar");
        assert!(renamed.exists());
        assert!(!file.exists());

        let subdir = create_dir(dir.path(), "carpeta").expect("mkdir");
        assert!(subdir.is_dir());
    }

    #[test]
    fn delete_permanently_removes() {
        let dir = tempdir().expect("tempdir");
        let file = dir.path().join("x.txt");
        File::create(&file).expect("archivo");
        assert_eq!(
            delete_permanently(std::slice::from_ref(&file)).expect("borrar"),
            1
        );
        assert!(!file.exists());
    }
}
