//! Configuración del usuario (`~/.config/levi/config.toml`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::Deserialize;

use crate::i18n::Language;

/// Modo de iconos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconsMode {
    /// Glifos Nerd Font.
    Nerd,
    /// Emoji: más grandes y con color.
    Emoji,
    /// Letras simples (`d`, `f`, `l`).
    Ascii,
    /// Sin iconos.
    None,
}

impl FromStr for IconsMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "nerd" => Ok(Self::Nerd),
            "emoji" | "grande" | "grandes" => Ok(Self::Emoji),
            "ascii" => Ok(Self::Ascii),
            "none" | "ninguno" => Ok(Self::None),
            other => Err(format!(
                "modo de iconos desconocido «{other}» (usa: nerd, emoji, ascii, none)"
            )),
        }
    }
}

/// Valor de una tecla en `[keys]`: una tecla o una lista de teclas.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum KeySpec {
    One(String),
    Many(Vec<String>),
}

/// Configuración cargada al arrancar.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Nombre del tema (`default`, `nord`, `light` o un archivo de `themes/`).
    pub theme: String,
    /// Iconos de la lista de archivos.
    pub icons: IconsMode,
    /// Idioma de la interfaz.
    pub language: Language,
    /// Comando del editor para la tecla `e`. Si es `None`, se usa `$VISUAL`/`$EDITOR`.
    pub editor: Option<String>,
    /// Empezar mostrando archivos ocultos.
    pub show_hidden: bool,
    /// Atajos personalizados. Las acciones aceptan nombres en inglés o español.
    pub keys: BTreeMap<String, KeySpec>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            icons: IconsMode::Emoji,
            language: Language::default(),
            editor: None,
            show_hidden: false,
            keys: BTreeMap::new(),
        }
    }
}

const TEMPLATE: &str = r##"# Configuración de Levi File Manager TUI

# Tema: "default" (azul estilo Bazzite), "nord", "light" o un archivo en themes/.
theme = "default"

# Iconos: "emoji" (grandes y con color), "nerd" (requiere Nerd Font),
# "ascii" o "none".
icons = "emoji"

# Idioma de la interfaz: "es" (español) o "en" (English).
# También puedes cambiarlo desde el botón «Lenguaje» de la esquina inferior derecha.
language = "es"

# Empezar mostrando los archivos ocultos.
show_hidden = false

# Editor para la tecla "e". Si se omite, se usa $VISUAL / $EDITOR.
# editor = "nvim"

# ---------------------------------------------------------------------------
# Atajos de teclado
# ---------------------------------------------------------------------------
# Cada acción acepta una tecla o una lista. Los nombres de acción valen en
# inglés y en español: quit/salir, copy/copiar, paste/pegar, trash/papelera...
#
# Teclas: j, G, ?, enter, space, esc, tab, up, down, left, right, home, end,
#         pageup, pagedown, f1..f24, ctrl+c, alt+x
#
# Puedes cambiar solo lo que quieras; el resto conserva su valor por defecto.
#
# [keys]
# quit = ["q", "ctrl+c"]          # salir
# help = "?"                      # ayuda
# preferences = ","               # preferencias
# cursor_down = ["j", "down"]     # bajar
# cursor_up = ["k", "up"]         # subir
# go_top = ["g", "home"]          # principio
# go_bottom = ["G", "end"]        # final
# go_parent = ["h", "left"]       # subir al padre
# enter = ["l", "right", "enter"]# entrar / abrir
# open = "o"                      # abrir con la app predeterminada
# reveal = "ctrl+y"               # abrir el explorador de archivos
# edit = "e"                      # editar con el editor
# toggle_hidden = "."             # mostrar/ocultar ocultos
# refresh = "r"                   # refrescar
# clear_status = "esc"            # limpiar mensaje
# toggle_select = "space"         # seleccionar/deseleccionar
# select_all = "ctrl+a"           # seleccionar todo
# clear_selection = "ctrl+d"      # limpiar selección
# copy = "y"                      # copiar (Fase 2)
# cut = "x"                       # cortar (Fase 2)
# paste = ["p", "v"]             # pegar (Fase 2)
# trash = "d"                     # mover a la papelera (Fase 2)
# delete_permanent = "D"          # borrar permanentemente (Fase 2)
# create = "a"                    # crear archivo/carpeta (Fase 2)
# rename = "R"                    # renombrar (Fase 2)
# filter = "ctrl+f"               # buscar / filtrar
# global_search = ["/", "f6"]     # super búsqueda global
# next_tab = "]"                  # pestaña siguiente (Fase 4)
# prev_tab = "["                  # pestaña anterior (Fase 4)
# new_tab = "ctrl+t"              # nueva pestaña (Fase 4)
# close_tab = "ctrl+w"            # cerrar pestaña (Fase 4)
# locations = "ctrl+l"            # ubicaciones: discos y papelera
# go_trash = "ctrl+p"             # ir directamente a la papelera
# toggle_language = "L"           # abre el selector de idioma
# bookmark = "m"                  # marcador (Fase 4)
#
# Ejemplos en español (equivalen a lo mismo):
# copiar = "y"
# pegar = "p"
# salir = "q"
# ---------------------------------------------------------------------------
# Temas propios
# ---------------------------------------------------------------------------
# Crea ~/.config/levi/themes/mi-tema.toml con esta forma y pon theme = "mi-tema".
# Solo hace falta declarar lo que quieras cambiar. Los colores van en HEX
# (#RRGGBB); también se aceptan nombres (blue, cyan) o índices ANSI (0-255).
#
# [colors]
# bg = "#1e1e2e"
# fg = "#cdd6f4"
# border = "#45475a"
# title = "#89b4fa"
# dir = "#89b4fa"
# file = "#cdd6f4"
# symlink = "#cba6f7"
# hidden = "#6c7086"
# muted = "#6c7086"
# selection_fg = "#1e1e2e"
# selection_bg = "#89b4fa"
# status_fg = "#cdd6f4"
# status_bg = "#313244"
# error_fg = "#1e1e2e"
# error_bg = "#f38ba8"
# help_fg = "#cdd6f4"
# help_bg = "#1e1e2e"
"##;

/// Ruta del archivo de configuración.
pub fn config_path(config_dir: &Path) -> PathBuf {
    config_dir.join("config.toml")
}

/// Carga la configuración. Devuelve avisos para mostrar en la barra de estado.
pub fn load(config_dir: &Path) -> (Config, Vec<String>) {
    let path = config_path(config_dir);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return (Config::default(), Vec::new());
        }
        Err(err) => {
            return (
                Config::default(),
                vec![format!(
                    "No se pudo leer «{}»: {err}. Usando valores por defecto.",
                    path.display()
                )],
            );
        }
    };

    match toml::from_str(&text) {
        Ok(config) => (config, Vec::new()),
        Err(err) => (
            Config::default(),
            vec![format!(
                "Configuración inválida en «{}»: {err}. Usando valores por defecto.",
                path.display()
            )],
        ),
    }
}

/// Crea `config.toml` con una plantilla comentada si todavía no existe.
pub fn ensure_default_file(config_dir: &Path) -> Option<String> {
    let path = config_path(config_dir);
    if path.exists() {
        return None;
    }
    if let Err(err) = std::fs::create_dir_all(config_dir) {
        return Some(format!(
            "No se pudo crear «{}»: {err}",
            config_dir.display()
        ));
    }
    match std::fs::write(&path, TEMPLATE) {
        Ok(()) => None,
        Err(err) => Some(format!("No se pudo escribir «{}»: {err}", path.display())),
    }
}

/// Guarda únicamente el idioma, conservando el resto del archivo y sus
/// comentarios. Si la clave no existe, se añade al principio (nivel raíz),
/// nunca al final (donde podría caer dentro de `[keys]`).
pub fn save_language(config_dir: &Path, language: Language) -> std::io::Result<()> {
    let path = config_path(config_dir);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let line = format!("language = {:?}", language.code());
    let mut replaced = false;
    let mut in_root = true;
    let mut out = String::new();

    for original in text.lines() {
        let trimmed = original.trim_start();
        // Las claves de `[keys]` pueden llamarse «language»: solo se
        // reemplaza la del nivel raíz.
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_root = false;
        }
        let is_key = in_root
            && !trimmed.starts_with('#')
            && trimmed
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == "language");
        if is_key {
            out.push_str(&line);
            out.push('\n');
            replaced = true;
        } else {
            out.push_str(original);
            out.push('\n');
        }
    }

    if !replaced {
        out = format!("{line}\n{out}");
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (config, warnings) = load(dir.path());
        assert!(warnings.is_empty());
        assert_eq!(config.theme, "default");
        assert_eq!(config.icons, IconsMode::Emoji);
        assert_eq!(config.language, crate::i18n::Language::Es);
        assert!(config.keys.is_empty());
    }

    #[test]
    fn parses_language() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("config.toml"), "language = \"en\"")
            .expect("escribir config");
        let (config, warnings) = load(dir.path());
        assert!(warnings.is_empty());
        assert_eq!(config.language, crate::i18n::Language::En);
    }

    #[test]
    fn save_language_keeps_comments_and_other_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("config.toml"),
            "# comentario\ntheme = \"nord\"\nlanguage = \"es\"\n\n[keys]\nq = \"x\"\n",
        )
        .expect("escribir config");

        save_language(dir.path(), crate::i18n::Language::En).expect("guardar idioma");

        let text = std::fs::read_to_string(dir.path().join("config.toml")).expect("leer config");
        assert!(text.contains("# comentario"));
        assert!(text.contains("theme = \"nord\""));
        assert!(text.contains("language = \"en\""));
        assert!(!text.contains("language = \"es\""));
        assert!(text.contains("[keys]"));
    }

    #[test]
    fn save_language_inserts_at_root_when_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("config.toml"), "[keys]\nq = \"x\"\n")
            .expect("escribir config");

        save_language(dir.path(), crate::i18n::Language::En).expect("guardar idioma");

        let (config, warnings) = load(dir.path());
        assert!(
            warnings.is_empty(),
            "el archivo debe seguir siendo TOML válido"
        );
        assert_eq!(config.language, crate::i18n::Language::En);
        assert!(config.keys.contains_key("q"));
    }

    #[test]
    fn save_language_does_not_touch_a_language_key_inside_sections() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("config.toml"), "[keys]\nlanguage = \"l\"\n")
            .expect("escribir config");

        save_language(dir.path(), crate::i18n::Language::En).expect("guardar idioma");

        let (config, warnings) = load(dir.path());
        assert!(warnings.is_empty(), "el archivo debe seguir siendo válido");
        assert_eq!(config.language, crate::i18n::Language::En);
        assert!(
            matches!(config.keys.get("language"), Some(KeySpec::One(key)) if key == "l"),
            "la tecla de [keys] no debe alterarse"
        );
    }

    #[test]
    fn parses_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("config.toml"),
            r#"
            theme = "nord"
            icons = "ascii"
            show_hidden = true

            [keys]
            copiar = "c"
            quit = ["q", "ctrl+q"]
            "#,
        )
        .expect("escribir config");
        let (config, warnings) = load(dir.path());
        assert!(warnings.is_empty());
        assert_eq!(config.theme, "nord");
        assert_eq!(config.icons, IconsMode::Ascii);
        assert!(config.show_hidden);
        assert!(config.keys.contains_key("copiar"));
    }

    #[test]
    fn invalid_config_falls_back_with_warning() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("config.toml"), "theme = [").expect("escribir config");
        let (config, warnings) = load(dir.path());
        assert_eq!(config.theme, "default");
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn creates_template_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(ensure_default_file(dir.path()).is_none());
        assert!(dir.path().join("config.toml").exists());
        assert!(ensure_default_file(dir.path()).is_none());
    }
}
