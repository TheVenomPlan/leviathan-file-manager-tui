//! Temas: colores de la interfaz, con presets incluidos y archivos TOML.
//!
//! Un tema propio se guarda en `~/.config/levi/themes/<nombre>.toml` y solo
//! necesita declarar los colores que quiera cambiar:
//!
//! ```toml
//! [colors]
//! bg = "#1e1e2e"
//! dir = "#89b4fa"
//! selection_bg = "#45475a"
//! ```
//!
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Color independiente de la librería de interfaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeColor {
    Reset,
    Indexed(u8),
    Rgb(u8, u8, u8),
    Named(NamedColor),
}

/// Colores con nombre del terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    White,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
}

impl ThemeColor {
    /// Acepta `#RRGGBB`, un índice ANSI (`0`-`255`) o un nombre (`cyan`, `darkgray`...).
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().to_lowercase();

        if let Some(hex) = value.strip_prefix('#') {
            if hex.len() != 6 {
                return None;
            }
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Self::Rgb(r, g, b));
        }

        if value.chars().all(|c| c.is_ascii_digit()) {
            let index: u8 = value.parse().ok()?;
            return Some(Self::Indexed(index));
        }

        let named = match value.as_str() {
            "reset" | "default" => return Some(Self::Reset),
            "black" => NamedColor::Black,
            "red" => NamedColor::Red,
            "green" => NamedColor::Green,
            "yellow" => NamedColor::Yellow,
            "blue" => NamedColor::Blue,
            "magenta" => NamedColor::Magenta,
            "cyan" => NamedColor::Cyan,
            "gray" | "grey" => NamedColor::Gray,
            "darkgray" | "dark_gray" | "darkgrey" | "dark_grey" => NamedColor::DarkGray,
            "white" => NamedColor::White,
            "lightred" | "light_red" => NamedColor::LightRed,
            "lightgreen" | "light_green" => NamedColor::LightGreen,
            "lightyellow" | "light_yellow" => NamedColor::LightYellow,
            "lightblue" | "light_blue" => NamedColor::LightBlue,
            "lightmagenta" | "light_magenta" => NamedColor::LightMagenta,
            "lightcyan" | "light_cyan" => NamedColor::LightCyan,
            _ => return None,
        };
        Some(Self::Named(named))
    }
}

/// Colores usados por la interfaz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub bg: ThemeColor,
    pub fg: ThemeColor,
    pub border: ThemeColor,
    pub title: ThemeColor,
    pub dir: ThemeColor,
    pub file: ThemeColor,
    pub symlink: ThemeColor,
    pub hidden: ThemeColor,
    pub muted: ThemeColor,
    pub selection_fg: ThemeColor,
    pub selection_bg: ThemeColor,
    pub status_fg: ThemeColor,
    pub status_bg: ThemeColor,
    pub error_fg: ThemeColor,
    pub error_bg: ThemeColor,
    pub help_fg: ThemeColor,
    pub help_bg: ThemeColor,
}

impl Default for Theme {
    fn default() -> Self {
        Self::bazzite()
    }
}

impl Theme {
    /// Presets incluidos en el binario.
    pub fn builtin(name: &str) -> Option<Self> {
        match name.trim().to_lowercase().as_str() {
            "default" | "levi" | "bazzite" | "azul" | "blue" => Some(Self::bazzite()),
            "nord" => Some(Self::nord()),
            "light" | "claro" => Some(Self::light()),
            _ => None,
        }
    }

    /// Tema oscuro con acentos morados (tema principal).
    fn bazzite() -> Self {
        let rgb = ThemeColor::Rgb;
        Self {
            bg: rgb(0x1e, 0x1e, 0x2e),
            fg: rgb(0xcd, 0xd6, 0xf4),
            border: rgb(0x45, 0x47, 0x5a),
            title: rgb(0xcb, 0xa6, 0xf7),
            dir: rgb(0xcb, 0xa6, 0xf7),
            file: rgb(0xcd, 0xd6, 0xf4),
            symlink: rgb(0xf5, 0xc2, 0xe7),
            hidden: rgb(0x6c, 0x70, 0x86),
            muted: rgb(0x6c, 0x70, 0x86),
            selection_fg: rgb(0x1e, 0x1e, 0x2e),
            selection_bg: rgb(0xcb, 0xa6, 0xf7),
            status_fg: rgb(0xcd, 0xd6, 0xf4),
            status_bg: rgb(0x31, 0x32, 0x44),
            error_fg: rgb(0x1e, 0x1e, 0x2e),
            error_bg: rgb(0xf3, 0x8b, 0xa8),
            help_fg: rgb(0xcd, 0xd6, 0xf4),
            help_bg: rgb(0x1e, 0x1e, 0x2e),
        }
    }

    fn nord() -> Self {
        let rgb = ThemeColor::Rgb;
        Self {
            bg: rgb(0x2e, 0x34, 0x40),
            fg: rgb(0xd8, 0xde, 0xe9),
            border: rgb(0x4c, 0x56, 0x6a),
            title: rgb(0x88, 0xc0, 0xd0),
            dir: rgb(0x88, 0xc0, 0xd0),
            file: rgb(0xd8, 0xde, 0xe9),
            symlink: rgb(0xb4, 0x8e, 0xad),
            hidden: rgb(0x61, 0x6e, 0x88),
            muted: rgb(0x61, 0x6e, 0x88),
            selection_fg: rgb(0x2e, 0x34, 0x40),
            selection_bg: rgb(0x88, 0xc0, 0xd0),
            status_fg: rgb(0xd8, 0xde, 0xe9),
            status_bg: rgb(0x3b, 0x42, 0x52),
            error_fg: rgb(0xec, 0xef, 0xf4),
            error_bg: rgb(0xbf, 0x61, 0x6a),
            help_fg: rgb(0xd8, 0xde, 0xe9),
            help_bg: rgb(0x2e, 0x34, 0x40),
        }
    }

    fn light() -> Self {
        let rgb = ThemeColor::Rgb;
        Self {
            bg: rgb(0xff, 0xff, 0xff),
            fg: rgb(0x1f, 0x23, 0x28),
            border: rgb(0xd0, 0xd7, 0xde),
            title: rgb(0x05, 0x50, 0xae),
            dir: rgb(0x09, 0x69, 0xda),
            file: rgb(0x1f, 0x23, 0x28),
            symlink: rgb(0x82, 0x50, 0xdf),
            hidden: rgb(0x8c, 0x95, 0x9f),
            muted: rgb(0x6e, 0x77, 0x81),
            selection_fg: rgb(0xff, 0xff, 0xff),
            selection_bg: rgb(0x09, 0x69, 0xda),
            status_fg: rgb(0x1f, 0x23, 0x28),
            status_bg: rgb(0xea, 0xee, 0xf2),
            error_fg: rgb(0xff, 0xff, 0xff),
            error_bg: rgb(0xcf, 0x22, 0x2e),
            help_fg: rgb(0x1f, 0x23, 0x28),
            help_bg: rgb(0xff, 0xff, 0xff),
        }
    }

    /// Carga un tema desde un archivo TOML (parcial: solo lo indicado se cambia).
    pub fn from_file(path: &Path) -> Result<Self, ThemeError> {
        let text = std::fs::read_to_string(path).map_err(|source| ThemeError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml_str(&text)
    }

    pub fn from_toml_str(text: &str) -> Result<Self, ThemeError> {
        let file: ThemeFile = toml::from_str(text)?;
        let mut theme = Self::default();
        theme.apply(file.colors.unwrap_or_default())?;
        Ok(theme)
    }

    fn apply(&mut self, overrides: ColorOverrides) -> Result<(), ThemeError> {
        macro_rules! apply {
            ($($field:ident),* $(,)?) => {
                $(
                    if let Some(value) = overrides.$field.as_deref() {
                        self.$field = ThemeColor::parse(value).ok_or_else(|| ThemeError::Color {
                            field: stringify!($field),
                            value: value.to_string(),
                        })?;
                    }
                )*
            };
        }
        apply!(
            bg,
            fg,
            border,
            title,
            dir,
            file,
            symlink,
            hidden,
            muted,
            selection_fg,
            selection_bg,
            status_fg,
            status_bg,
            error_fg,
            error_bg,
            help_fg,
            help_bg,
        );
        Ok(())
    }

    /// Resuelve un tema por nombre.
    ///
    /// Orden: archivo en `themes/<nombre>.toml`, ruta directa `.toml`, preset incluido.
    pub fn load(config_dir: &Path, name: &str) -> (Self, Option<String>) {
        let name = name.trim();
        if name.is_empty() {
            return (Self::default(), None);
        }

        let theme_file = config_dir.join("themes").join(format!("{name}.toml"));
        if theme_file.is_file() {
            return match Self::from_file(&theme_file) {
                Ok(theme) => (theme, None),
                Err(err) => (
                    Self::default(),
                    Some(format!("{err}. Usando el tema por defecto.")),
                ),
            };
        }

        if (name.ends_with(".toml") || name.contains('/')) && Path::new(name).is_file() {
            return match Self::from_file(Path::new(name)) {
                Ok(theme) => (theme, None),
                Err(err) => (
                    Self::default(),
                    Some(format!("{err}. Usando el tema por defecto.")),
                ),
            };
        }

        match Self::builtin(name) {
            Some(theme) => (theme, None),
            None => (
                Self::default(),
                Some(format!(
                    "Tema «{name}» no encontrado; usando el tema por defecto. Prueba con: default, nord, light."
                )),
            ),
        }
    }
}

/// Errores al cargar o interpretar un tema.
#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("no se pudo leer el tema «{}»: {source}", path.display())]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("tema TOML inválido: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("color inválido «{value}» en el campo «{field}»")]
    Color { field: &'static str, value: String },
}

#[derive(Debug, Default, Deserialize)]
struct ThemeFile {
    colors: Option<ColorOverrides>,
}

macro_rules! color_overrides {
    ($($field:ident),* $(,)?) => {
        #[derive(Debug, Default, Deserialize)]
        struct ColorOverrides {
            $($field: Option<String>,)*
        }
    };
}

color_overrides!(
    bg,
    fg,
    border,
    title,
    dir,
    file,
    symlink,
    hidden,
    muted,
    selection_fg,
    selection_bg,
    status_fg,
    status_bg,
    error_fg,
    error_bg,
    help_fg,
    help_bg,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colors() {
        assert_eq!(
            ThemeColor::parse("#88C0D0"),
            Some(ThemeColor::Rgb(0x88, 0xc0, 0xd0))
        );
        assert_eq!(
            ThemeColor::parse("cyan"),
            Some(ThemeColor::Named(NamedColor::Cyan))
        );
        assert_eq!(ThemeColor::parse("255"), Some(ThemeColor::Indexed(255)));
        assert_eq!(ThemeColor::parse("reset"), Some(ThemeColor::Reset));
        assert_eq!(ThemeColor::parse("#12345"), None);
        assert_eq!(ThemeColor::parse("turquesa"), None);
    }

    #[test]
    fn default_theme_is_purple() {
        let theme = Theme::default();
        assert_eq!(theme, Theme::builtin("bazzite").expect("tema bazzite"));
        assert_eq!(theme.title, ThemeColor::Rgb(0xcb, 0xa6, 0xf7));
        assert_eq!(theme.dir, ThemeColor::Rgb(0xcb, 0xa6, 0xf7));
    }

    #[test]
    fn partial_theme_overrides_only_given_fields() {
        let theme = Theme::from_toml_str(
            r##"
            [colors]
            bg = "#101010"
            dir = "blue"
        "##,
        )
        .expect("tema válido");

        assert_eq!(theme.bg, ThemeColor::Rgb(0x10, 0x10, 0x10));
        assert_eq!(theme.dir, ThemeColor::Named(NamedColor::Blue));
        assert_eq!(theme.symlink, Theme::default().symlink);
    }

    #[test]
    fn unknown_theme_falls_back_with_warning() {
        let (theme, warning) = Theme::load(Path::new("/nonexistent"), "inventado");
        assert_eq!(theme, Theme::default());
        assert!(warning.is_some());
    }
}
