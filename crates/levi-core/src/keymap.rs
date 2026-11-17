//! Atajos de teclado: acciones, teclas y mapa configurable.
//!
//! Las teclas se escriben en `config.toml` dentro de `[keys]`, por ejemplo:
//!
//! ```toml
//! [keys]
//! quit = ["q", "ctrl+c"]
//! cursor_down = "j"
//! copy = "y"
//! ```
//!
//! Una tecla concreta puede ser un carácter (`j`, `G`, `?`), un nombre
//! (`enter`, `space`, `esc`, `up`, `f5`) o llevar modificadores (`ctrl+c`).

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use crate::config::KeySpec;
use crate::i18n::{Language, strings};

/// Código de tecla independiente de la librería de terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Enter,
    Esc,
    Tab,
    Backspace,
    Delete,
    Insert,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
}

/// Tecla pulsada con sus modificadores relevantes.
///
/// Para caracteres, `shift` suele reflejarse en el propio carácter (`g`/`G`),
/// pero se conserva como modificador para combinaciones y para teclas no textuales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: KeyCode,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Key {
    pub fn simple(code: KeyCode) -> Self {
        Self {
            code,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    pub fn with_mods(code: KeyCode, ctrl: bool, alt: bool) -> Self {
        Self {
            code,
            ctrl,
            alt,
            shift: false,
        }
    }

    pub fn with_all_mods(code: KeyCode, ctrl: bool, alt: bool, shift: bool) -> Self {
        Self {
            code,
            ctrl,
            alt,
            shift,
        }
    }

    /// Interpreta una tecla escrita por el usuario (`"ctrl+c"`, `"space"`...).
    pub fn parse(spec: &str) -> Result<Self, String> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Err("tecla vacía".to_string());
        }

        let mut parts: Vec<&str> = spec.split('+').collect();
        let key_part = parts.pop().unwrap_or_default().trim();
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;

        for modifier in parts {
            match modifier.trim().to_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" | "meta" => alt = true,
                "shift" | "mayus" | "mayusculas" => shift = true,
                other => return Err(format!("modificador desconocido «{other}»")),
            }
        }

        let chars: Vec<char> = key_part.chars().collect();
        let mut code = if chars.len() == 1 {
            KeyCode::Char(chars[0])
        } else {
            match key_part.to_lowercase().as_str() {
                "enter" | "return" | "intro" => KeyCode::Enter,
                "esc" | "escape" => KeyCode::Esc,
                "tab" | "tabulador" => KeyCode::Tab,
                "backtab" | "shift+tab" => {
                    shift = true;
                    KeyCode::Tab
                }
                "backspace" | "retroceso" => KeyCode::Backspace,
                "delete" | "del" | "supr" => KeyCode::Delete,
                "insert" | "ins" => KeyCode::Insert,
                "up" | "arriba" => KeyCode::Up,
                "down" | "abajo" => KeyCode::Down,
                "left" | "izquierda" => KeyCode::Left,
                "right" | "derecha" => KeyCode::Right,
                "home" | "inicio" => KeyCode::Home,
                "end" | "fin" => KeyCode::End,
                "pageup" | "pgup" => KeyCode::PageUp,
                "pagedown" | "pgdown" | "pgdn" => KeyCode::PageDown,
                "space" | "espacio" => KeyCode::Char(' '),
                other if other.starts_with('f') && other.len() > 1 => {
                    let number: u8 = other[1..]
                        .parse()
                        .map_err(|_| format!("tecla desconocida «{key_part}»"))?;
                    if !(1..=24).contains(&number) {
                        return Err(format!("función fuera de rango «{key_part}»"));
                    }
                    KeyCode::F(number)
                }
                other => return Err(format!("tecla desconocida «{other}»")),
            }
        };

        // Para caracteres alfabéticos, el mayúsculas/minúsculas codifica Shift.
        if let KeyCode::Char(character) = code
            && character.is_ascii_alphabetic()
        {
            if shift {
                code = KeyCode::Char(character.to_ascii_uppercase());
            }
            shift = false;
        }

        Ok(Self {
            code,
            ctrl,
            alt,
            shift,
        })
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            write!(f, "ctrl+")?;
        }
        if self.alt {
            write!(f, "alt+")?;
        }
        let show_shift = match self.code {
            // El Shift ya se ve en la mayúscula del carácter.
            KeyCode::Char(c) if c.is_ascii_alphabetic() && c.is_ascii_uppercase() => false,
            KeyCode::Char(c) if c.is_ascii_alphabetic() => self.shift,
            _ => self.shift,
        };
        if show_shift {
            write!(f, "shift+")?;
        }
        match self.code {
            KeyCode::Char(' ') => write!(f, "space"),
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::Enter => write!(f, "enter"),
            KeyCode::Esc => write!(f, "esc"),
            KeyCode::Tab => write!(f, "tab"),
            KeyCode::Backspace => write!(f, "backspace"),
            KeyCode::Delete => write!(f, "delete"),
            KeyCode::Insert => write!(f, "insert"),
            KeyCode::Up => write!(f, "up"),
            KeyCode::Down => write!(f, "down"),
            KeyCode::Left => write!(f, "left"),
            KeyCode::Right => write!(f, "right"),
            KeyCode::Home => write!(f, "home"),
            KeyCode::End => write!(f, "end"),
            KeyCode::PageUp => write!(f, "pageup"),
            KeyCode::PageDown => write!(f, "pagedown"),
            KeyCode::F(number) => write!(f, "f{number}"),
        }
    }
}

/// Acciones que se pueden asignar a una tecla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    Help,
    Preferences,
    CursorDown,
    CursorUp,
    GoParent,
    Enter,
    Open,
    Reveal,
    Edit,
    GoTop,
    GoBottom,
    ToggleHidden,
    Refresh,
    ClearStatus,
    ToggleSelect,
    SelectAll,
    ClearSelection,
    Copy,
    Cut,
    Paste,
    Trash,
    DeletePermanent,
    Create,
    Rename,
    Filter,
    GlobalSearch,
    Shell,
    NextTab,
    PrevTab,
    NewTab,
    CloseTab,
    Locations,
    GoTrash,
    Bookmark,
    ToggleLanguage,
}

impl Action {
    /// Nombre usado en `config.toml`.
    pub fn name(self) -> &'static str {
        match self {
            Action::Quit => "quit",
            Action::Help => "help",
            Action::Preferences => "preferences",
            Action::CursorDown => "cursor_down",
            Action::CursorUp => "cursor_up",
            Action::GoParent => "go_parent",
            Action::Enter => "enter",
            Action::Open => "open",
            Action::Reveal => "reveal",
            Action::Edit => "edit",
            Action::GoTop => "go_top",
            Action::GoBottom => "go_bottom",
            Action::ToggleHidden => "toggle_hidden",
            Action::Refresh => "refresh",
            Action::ClearStatus => "clear_status",
            Action::ToggleSelect => "toggle_select",
            Action::SelectAll => "select_all",
            Action::ClearSelection => "clear_selection",
            Action::Copy => "copy",
            Action::Cut => "cut",
            Action::Paste => "paste",
            Action::Trash => "trash",
            Action::DeletePermanent => "delete_permanent",
            Action::Create => "create",
            Action::Rename => "rename",
            Action::Filter => "filter",
            Action::GlobalSearch => "global_search",
            Action::Shell => "shell",
            Action::NextTab => "next_tab",
            Action::PrevTab => "prev_tab",
            Action::NewTab => "new_tab",
            Action::CloseTab => "close_tab",
            Action::Locations => "locations",
            Action::GoTrash => "go_trash",
            Action::Bookmark => "bookmark",
            Action::ToggleLanguage => "toggle_language",
        }
    }

    /// Descripción en español para la ayuda y la pantalla de preferencias.
    pub fn label(self) -> &'static str {
        self.label_in(Language::Es)
    }

    /// Descripción traducida al idioma indicado.
    pub fn label_in(self, language: Language) -> &'static str {
        let s = strings(language);
        match self {
            Action::Quit => s.action_quit,
            Action::Help => s.action_help,
            Action::Preferences => s.action_preferences,
            Action::CursorDown => s.action_cursor_down,
            Action::CursorUp => s.action_cursor_up,
            Action::GoParent => s.action_go_parent,
            Action::Enter => s.action_enter,
            Action::Open => s.action_open,
            Action::Reveal => s.action_reveal,
            Action::Edit => s.action_edit,
            Action::GoTop => s.action_go_top,
            Action::GoBottom => s.action_go_bottom,
            Action::ToggleHidden => s.action_toggle_hidden,
            Action::Refresh => s.action_refresh,
            Action::ClearStatus => s.action_clear_status,
            Action::ToggleSelect => s.action_toggle_select,
            Action::SelectAll => s.action_select_all,
            Action::ClearSelection => s.action_clear_selection,
            Action::Copy => s.action_copy,
            Action::Cut => s.action_cut,
            Action::Paste => s.action_paste,
            Action::Trash => s.action_trash,
            Action::DeletePermanent => s.action_delete_permanent,
            Action::Create => s.action_create,
            Action::Rename => s.action_rename,
            Action::Filter => s.action_filter,
            Action::GlobalSearch => s.action_global_search,
            Action::Shell => s.action_shell,
            Action::NextTab => s.action_next_tab,
            Action::PrevTab => s.action_prev_tab,
            Action::NewTab => s.action_new_tab,
            Action::CloseTab => s.action_close_tab,
            Action::Locations => s.action_locations,
            Action::GoTrash => s.action_go_trash,
            Action::Bookmark => s.action_bookmark,
            Action::ToggleLanguage => s.action_toggle_language,
        }
    }

    /// `true` si la acción ya está implementada.
    pub fn implemented(self) -> bool {
        matches!(
            self,
            Action::Quit
                | Action::Help
                | Action::Preferences
                | Action::CursorDown
                | Action::CursorUp
                | Action::GoParent
                | Action::Enter
                | Action::Open
                | Action::Reveal
                | Action::Edit
                | Action::GoTop
                | Action::GoBottom
                | Action::ToggleHidden
                | Action::Refresh
                | Action::ClearStatus
                | Action::ToggleSelect
                | Action::SelectAll
                | Action::ClearSelection
                | Action::Copy
                | Action::Cut
                | Action::Paste
                | Action::Trash
                | Action::DeletePermanent
                | Action::Create
                | Action::Rename
                | Action::Filter
                | Action::GlobalSearch
                | Action::Shell
                | Action::NextTab
                | Action::PrevTab
                | Action::NewTab
                | Action::CloseTab
                | Action::Locations
                | Action::GoTrash
                | Action::ToggleLanguage
        )
    }

    pub fn from_name(name: &str) -> Option<Self> {
        // Normaliza mayúsculas, espacios y guiones: "Seleccionar todo" == "seleccionar_todo".
        let normalized = name.trim().to_lowercase().replace(['-', ' '], "_");

        if let Some(action) = ALL.into_iter().find(|action| action.name() == normalized) {
            return Some(action);
        }

        let action = match normalized.as_str() {
            // Español
            "salir" => Action::Quit,
            "ayuda" => Action::Help,
            "preferencias" => Action::Preferences,
            "bajar" | "abajo" => Action::CursorDown,
            "subir" | "arriba" => Action::CursorUp,
            "padre" | "subir_directorio" | "atras" => Action::GoParent,
            "entrar" => Action::Enter,
            "abrir" | "abrir_app" | "abrir_con_app" => Action::Open,
            "revelar" | "abrir_explorador" | "mostrar_en_explorador" | "reveal" => Action::Reveal,
            "editar" | "editar_archivo" => Action::Edit,
            "principio" | "inicio" => Action::GoTop,
            "final" | "fin" => Action::GoBottom,
            "ocultos" | "alternar_ocultos" => Action::ToggleHidden,
            "refrescar" | "recargar" => Action::Refresh,
            "limpiar_mensaje" | "limpiar_estado" => Action::ClearStatus,
            "seleccionar" | "alternar_seleccion" => Action::ToggleSelect,
            "seleccionar_todo" | "todo" => Action::SelectAll,
            "limpiar_seleccion" | "deseleccionar" => Action::ClearSelection,
            "copiar" => Action::Copy,
            "cortar" => Action::Cut,
            "pegar" => Action::Paste,
            "papelera" | "mover_a_papelera" => Action::Trash,
            "borrar" | "eliminar" | "borrar_permanente" => Action::DeletePermanent,
            "crear" | "nuevo" => Action::Create,
            "renombrar" => Action::Rename,
            "filtrar" | "buscar" => Action::Filter,
            "super_buscador" | "busqueda_global" | "global_search" => Action::GlobalSearch,
            "comando" | "terminal" | "ejecutar" => Action::Shell,
            "siguiente_pestana" | "pestana_siguiente" => Action::NextTab,
            "anterior_pestana" | "pestana_anterior" => Action::PrevTab,
            "nueva_pestana" => Action::NewTab,
            "cerrar_pestana" => Action::CloseTab,
            "ubicaciones" | "discos" | "lugares" => Action::Locations,
            "ir_a_papelera" | "abrir_papelera" | "go_to_trash" | "open_trash" => Action::GoTrash,
            "marcador" | "marcar" => Action::Bookmark,
            "idioma" | "lenguaje" | "language" | "lang" => Action::ToggleLanguage,
            // English synonyms
            "parent" => Action::GoParent,
            "top" => Action::GoTop,
            "bottom" => Action::GoBottom,
            "hidden" => Action::ToggleHidden,
            "reload" => Action::Refresh,
            "select" => Action::ToggleSelect,
            "all" => Action::SelectAll,
            "deselect" => Action::ClearSelection,
            "delete" => Action::DeletePermanent,
            "new" => Action::Create,
            "search" => Action::Filter,
            "drives" | "disks" => Action::Locations,
            _ => return None,
        };
        Some(action)
    }
}

/// Conjunto de atajos activos.
#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: HashMap<Key, Action>,
}

/// Todas las acciones conocidas, en orden de presentación.
pub const ALL: [Action; 36] = [
    Action::Quit,
    Action::Help,
    Action::Preferences,
    Action::CursorDown,
    Action::CursorUp,
    Action::GoParent,
    Action::Enter,
    Action::Open,
    Action::Reveal,
    Action::Edit,
    Action::GoTop,
    Action::GoBottom,
    Action::ToggleHidden,
    Action::Refresh,
    Action::ClearStatus,
    Action::ToggleSelect,
    Action::SelectAll,
    Action::ClearSelection,
    Action::Copy,
    Action::Cut,
    Action::Paste,
    Action::Trash,
    Action::DeletePermanent,
    Action::Create,
    Action::Rename,
    Action::Filter,
    Action::GlobalSearch,
    Action::Shell,
    Action::NextTab,
    Action::PrevTab,
    Action::NewTab,
    Action::CloseTab,
    Action::Locations,
    Action::GoTrash,
    Action::Bookmark,
    Action::ToggleLanguage,
];

/// Acciones ordenadas por su etiqueta en español para las preferencias.
pub const SORTED_ACTIONS: [Action; 36] = [
    Action::Open,
    Action::Reveal,
    Action::Help,
    Action::CursorDown,
    Action::Filter,
    Action::GlobalSearch,
    Action::DeletePermanent,
    Action::CloseTab,
    Action::Copy,
    Action::Cut,
    Action::Create,
    Action::Edit,
    Action::Enter,
    Action::GoBottom,
    Action::GoTop,
    Action::GoTrash,
    Action::ClearStatus,
    Action::ClearSelection,
    Action::Bookmark,
    Action::ToggleHidden,
    Action::ToggleLanguage,
    Action::Trash,
    Action::NewTab,
    Action::Paste,
    Action::PrevTab,
    Action::NextTab,
    Action::Preferences,
    Action::Refresh,
    Action::Rename,
    Action::Quit,
    Action::ToggleSelect,
    Action::SelectAll,
    Action::Shell,
    Action::Locations,
    Action::CursorUp,
    Action::GoParent,
];

impl Default for Keymap {
    fn default() -> Self {
        Self::defaults()
    }
}

impl Keymap {
    /// Atajos incluidos de serie.
    pub fn defaults() -> Self {
        let mut keymap = Self {
            bindings: HashMap::new(),
        };
        for (action, keys) in default_bindings() {
            for spec in keys {
                if let Ok(key) = Key::parse(spec) {
                    keymap.bindings.insert(key, action);
                }
            }
        }
        keymap
    }

    /// Aplica las personalizaciones de `[keys]` sobre los atajos por defecto.
    ///
    /// Al definir una acción se reemplazan todas sus teclas anteriores.
    pub fn from_config(overrides: &BTreeMap<String, KeySpec>) -> (Self, Vec<String>) {
        let mut keymap = Self::defaults();
        let mut warnings = Vec::new();

        for (name, spec) in overrides {
            let Some(action) = Action::from_name(name) else {
                warnings.push(format!(
                    "Acción desconocida en [keys]: «{name}». Se ignora."
                ));
                continue;
            };

            let specs = match spec {
                KeySpec::One(value) => vec![value.clone()],
                KeySpec::Many(values) => values.clone(),
            };

            keymap.bindings.retain(|_, bound| *bound != action);
            for value in specs {
                match Key::parse(&value) {
                    Ok(key) => {
                        keymap.bindings.insert(key, action);
                    }
                    Err(err) => warnings.push(format!(
                        "Tecla inválida «{value}» para «{name}»: {err}. Se ignora."
                    )),
                }
            }
        }

        (keymap, warnings)
    }

    pub fn action(&self, key: &Key) -> Option<Action> {
        self.bindings.get(key).copied()
    }

    /// Todas las teclas activas, ordenadas por acción.
    pub fn bindings(&self) -> Vec<(Key, Action)> {
        let mut list: Vec<(Key, Action)> = self
            .bindings
            .iter()
            .map(|(key, action)| (*key, *action))
            .collect();
        list.sort_by(|(key_a, action_a), (key_b, action_b)| {
            action_a
                .name()
                .cmp(action_b.name())
                .then_with(|| key_a.to_string().cmp(&key_b.to_string()))
        });
        list
    }

    /// Teclas asignadas a una acción concreta.
    pub fn keys_for(&self, action: Action) -> Vec<Key> {
        let mut keys: Vec<Key> = self
            .bindings
            .iter()
            .filter(|(_, bound)| **bound == action)
            .map(|(key, _)| *key)
            .collect();
        keys.sort_by_key(ToString::to_string);
        keys
    }

    /// Reasigna una acción a una única tecla, eliminando sus teclas anteriores.
    pub fn rebind(&mut self, action: Action, key: Key) {
        self.bindings.retain(|_, bound| *bound != action);
        self.bindings.insert(key, action);
    }

    /// Serializa el mapa actual a la forma usada en `config.toml`.
    pub fn to_config(&self) -> BTreeMap<String, KeySpec> {
        use std::collections::HashMap;
        let mut grouped: HashMap<Action, Vec<Key>> = HashMap::new();
        for (key, action) in &self.bindings {
            grouped.entry(*action).or_default().push(*key);
        }
        let mut entries: Vec<(Action, Vec<Key>)> = grouped.into_iter().collect();
        entries.sort_by_key(|(action, _)| action.name());
        let mut out = BTreeMap::new();
        for (action, mut keys) in entries {
            keys.sort_by_key(ToString::to_string);
            let specs: Vec<String> = keys.iter().map(ToString::to_string).collect();
            let spec = if specs.len() == 1 {
                KeySpec::One(specs[0].clone())
            } else {
                KeySpec::Many(specs)
            };
            out.insert(action.name().to_string(), spec);
        }
        out
    }
}

fn default_bindings() -> Vec<(Action, Vec<&'static str>)> {
    vec![
        (Action::Quit, vec!["q", "ctrl+c"]),
        (Action::Help, vec!["?"]),
        (Action::Preferences, vec![","]),
        (Action::CursorDown, vec!["j", "down"]),
        (Action::CursorUp, vec!["k", "up"]),
        (Action::GoTop, vec!["g", "home"]),
        (Action::GoBottom, vec!["G", "end"]),
        (Action::GoParent, vec!["h", "left", "backspace"]),
        (Action::Enter, vec!["l", "right", "enter"]),
        (Action::Open, vec!["o"]),
        (Action::Reveal, vec!["ctrl+y"]),
        (Action::Edit, vec!["e"]),
        (Action::ToggleHidden, vec!["."]),
        (Action::Refresh, vec!["r"]),
        (Action::ClearStatus, vec!["esc"]),
        (Action::ToggleSelect, vec!["space"]),
        (Action::SelectAll, vec!["ctrl+a", "ctrl+space"]),
        (Action::ClearSelection, vec!["ctrl+d"]),
        (Action::Copy, vec!["y"]),
        (Action::Cut, vec!["x"]),
        (Action::Paste, vec!["p", "v"]),
        (Action::Trash, vec!["d"]),
        (Action::DeletePermanent, vec!["D"]),
        (Action::Create, vec!["a"]),
        (Action::Rename, vec!["R"]),
        (Action::Filter, vec!["ctrl+f"]),
        (Action::GlobalSearch, vec!["/", "f6"]),
        (Action::Shell, vec!["ctrl+s"]),
        (Action::NextTab, vec!["]", "ctrl+right"]),
        (Action::PrevTab, vec!["[", "ctrl+left"]),
        (Action::NewTab, vec!["ctrl+t"]),
        (Action::CloseTab, vec!["ctrl+w"]),
        (Action::Locations, vec!["ctrl+l"]),
        (Action::GoTrash, vec!["ctrl+p"]),
        (Action::Bookmark, vec!["m"]),
        (Action::ToggleLanguage, vec!["L"]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keys() {
        assert_eq!(Key::parse("j").expect("j"), Key::simple(KeyCode::Char('j')));
        assert_eq!(Key::parse("G").expect("G"), Key::simple(KeyCode::Char('G')));
        assert_eq!(
            Key::parse("ctrl+c").expect("ctrl+c"),
            Key::with_mods(KeyCode::Char('c'), true, false)
        );
        assert_eq!(
            Key::parse("space").expect("space"),
            Key::simple(KeyCode::Char(' '))
        );
        assert_eq!(
            Key::parse("enter").expect("enter"),
            Key::simple(KeyCode::Enter)
        );
        assert_eq!(Key::parse("f5").expect("f5"), Key::simple(KeyCode::F(5)));
        assert_eq!(
            Key::parse("shift+c").expect("shift+c"),
            Key::simple(KeyCode::Char('C'))
        );
        assert!(Key::parse("").is_err());
        assert!(Key::parse("tecla-rara").is_err());
    }

    #[test]
    fn config_overrides_and_replaces_action_keys() {
        let mut overrides = BTreeMap::new();
        overrides.insert("quit".to_string(), KeySpec::One("x".to_string()));
        overrides.insert(
            "copy".to_string(),
            KeySpec::Many(vec!["c".to_string(), "ctrl+shift+c".to_string()]),
        );
        overrides.insert("inventada".to_string(), KeySpec::One("z".to_string()));

        let (keymap, warnings) = Keymap::from_config(&overrides);

        assert_eq!(
            keymap.action(&Key::parse("x").expect("x")),
            Some(Action::Quit)
        );
        assert_eq!(keymap.action(&Key::parse("q").expect("q")), None);
        assert_eq!(
            keymap.action(&Key::parse("c").expect("c")),
            Some(Action::Copy)
        );
        assert_eq!(
            keymap.action(&Key::parse("ctrl+C").expect("ctrl+C")),
            Some(Action::Copy),
            "shift+letra se convierte en la mayúscula"
        );
        assert_eq!(warnings.len(), 1, "solo la acción inventada genera aviso");
    }

    #[test]
    fn accepts_spanish_and_english_action_names() {
        for name in [
            "quit",
            "salir",
            "copy",
            "copiar",
            "paste",
            "pegar",
            "trash",
            "papelera",
            "Seleccionar todo",
            "seleccionar_todo",
            "select_all",
            "go-parent",
            "renombrar",
        ] {
            assert!(
                Action::from_name(name).is_some(),
                "«{name}» debería ser una acción válida"
            );
        }

        assert_eq!(Action::from_name("copiar"), Some(Action::Copy));
        assert_eq!(Action::from_name("quit"), Some(Action::Quit));
        assert_eq!(Action::from_name("salir"), Some(Action::Quit));
        assert_eq!(
            Action::from_name("Seleccionar Todo"),
            Some(Action::SelectAll)
        );
        assert_eq!(Action::from_name("inventada"), None);
    }

    #[test]
    fn defaults_cover_navigation() {
        let keymap = Keymap::defaults();
        assert_eq!(
            keymap.action(&Key::parse("j").expect("j")),
            Some(Action::CursorDown)
        );
        assert_eq!(
            keymap.action(&Key::parse("enter").expect("enter")),
            Some(Action::Enter)
        );
    }

    #[test]
    fn search_and_selection_actions_are_available() {
        let keymap = Keymap::defaults();
        assert_eq!(
            keymap.action(&Key::parse("ctrl+f").expect("ctrl+f")),
            Some(Action::Filter),
            "«ctrl+f» debe abrir la búsqueda"
        );
        for key in ["/", "f6"] {
            assert_eq!(
                keymap.action(&Key::parse(key).expect(key)),
                Some(Action::GlobalSearch),
                "«{key}» debe abrir la super búsqueda"
            );
        }
        assert_eq!(
            keymap.action(&Key::parse("space").expect("space")),
            Some(Action::ToggleSelect)
        );
        assert_eq!(
            keymap.action(&Key::parse("ctrl+space").expect("ctrl+space")),
            Some(Action::SelectAll)
        );
        assert!(Action::GlobalSearch.implemented());
        assert_eq!(
            Action::from_name("super_buscador"),
            Some(Action::GlobalSearch)
        );

        for action in [
            Action::Filter,
            Action::GlobalSearch,
            Action::ToggleSelect,
            Action::SelectAll,
            Action::ClearSelection,
        ] {
            assert!(
                action.implemented(),
                "«{}» debe estar disponible",
                action.name()
            );
        }
    }

    #[test]
    fn shell_is_available() {
        let keymap = Keymap::defaults();
        assert_eq!(
            keymap.action(&Key::parse("ctrl+s").expect("ctrl+s")),
            Some(Action::Shell)
        );
        assert!(Action::Shell.implemented());
    }

    #[test]
    fn language_toggle_is_available() {
        let keymap = Keymap::defaults();
        assert_eq!(
            keymap.action(&Key::parse("L").expect("L")),
            Some(Action::ToggleLanguage),
            "«L» debe alternar el idioma"
        );
        assert!(Action::ToggleLanguage.implemented());
        for name in ["idioma", "lenguaje", "language", "lang", "toggle_language"] {
            assert_eq!(
                Action::from_name(name),
                Some(Action::ToggleLanguage),
                "«{name}» debe referirse al idioma"
            );
        }
        assert_eq!(Action::ToggleLanguage.label_in(Language::En), "language");
        assert_eq!(
            Action::ToggleLanguage.label_in(Language::Es),
            "idioma / lenguaje"
        );
    }

    #[test]
    fn go_trash_is_available() {
        let keymap = Keymap::defaults();
        assert_eq!(
            keymap.action(&Key::parse("ctrl+p").expect("ctrl+p")),
            Some(Action::GoTrash),
            "«ctrl+p» debe ir a la papelera"
        );
        assert!(Action::GoTrash.implemented());
        assert_eq!(Action::from_name("go_trash"), Some(Action::GoTrash));
        assert_eq!(Action::from_name("ir_a_papelera"), Some(Action::GoTrash));
        assert_eq!(Action::from_name("abrir_papelera"), Some(Action::GoTrash));
    }

    #[test]
    fn tabs_are_available() {
        let keymap = Keymap::defaults();
        for (key, action) in [
            ("]", Action::NextTab),
            ("ctrl+right", Action::NextTab),
            ("[", Action::PrevTab),
            ("ctrl+left", Action::PrevTab),
            ("ctrl+t", Action::NewTab),
            ("ctrl+w", Action::CloseTab),
        ] {
            assert_eq!(
                keymap.action(&Key::parse(key).expect(key)),
                Some(action),
                "«{key}» debe controlar pestañas"
            );
            assert!(
                action.implemented(),
                "«{}» debe estar disponible",
                action.name()
            );
        }
    }
}
