//! Idioma de la interfaz y textos traducidos.
//!
//! El idioma se guarda en `config.toml` y vive en el núcleo: la interfaz solo
//! consume las cadenas ya resueltas. Los textos con datos variables usan
//! marcadores entre llaves (`{count}`, `{path}`...) que se sustituyen con
//! [`fill`].
//!
//! Para añadir un idioma: agrega una variante a [`Language`], su nombre en
//! [`Language::display_name`], inclúyela en [`Language::ALL`] (el selector la
//! mostrará sola) y añade una tabla `Strings` nueva.

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;

/// Idioma de la interfaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    /// Español.
    #[default]
    Es,
    /// Inglés.
    En,
}

impl Language {
    /// Todos los idiomas disponibles, en el orden en que los muestra el
    /// selector de la interfaz.
    pub const ALL: [Self; 2] = [Self::Es, Self::En];

    /// Código usado en `config.toml`.
    pub fn code(self) -> &'static str {
        match self {
            Self::Es => "es",
            Self::En => "en",
        }
    }

    /// Nombre del idioma escrito en él mismo (no se traduce).
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Es => "Español",
            Self::En => "English",
        }
    }

    /// Idioma siguiente en [`Language::ALL`] (para alternar rápido).
    pub fn next(self) -> Self {
        match self {
            Self::Es => Self::En,
            Self::En => Self::Es,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for Language {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "es" | "esp" | "espanol" | "español" | "castellano" => Ok(Self::Es),
            "en" | "eng" | "english" | "ingles" | "inglés" => Ok(Self::En),
            other => Err(format!(
                "idioma desconocido «{other}» (usa: es, en, español, english)"
            )),
        }
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Sustituye los marcadores `{nombre}` de una plantilla.
///
/// ```ignore
/// assert_eq!(fill("{n} entradas", &[("n", "3")]), "3 entradas");
/// ```
pub fn fill(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (name, value) in vars {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

macro_rules! strings {
    ($($field:ident => $es:literal / $en:literal),* $(,)?) => {
        /// Textos de la interfaz en un idioma.
        #[derive(Debug, Clone, Copy)]
        pub struct Strings {
            $(pub $field: &'static str,)*
        }

        /// Textos en español.
        pub const ES: Strings = Strings {
            $($field: $es,)*
        };

        /// Textos en inglés.
        pub const EN: Strings = Strings {
            $($field: $en,)*
        };
    };
}

strings! {
    // ---- Pestañas y paneles ----------------------------------------------
    no_tabs => "1:(sin pestañas)" / "1:(no tabs)",
    entries_title => "{path} ({count} entradas)" / "{path} ({count} entries)",
    super_search_title => "Super búsqueda · {count} resultados" / "Global search · {count} results",
    filtered_title => "{path} ({shown} de {total} · filtro «{filter}»)" / "{path} ({shown} of {total} · filter “{filter}”)",
    preview => "Vista previa" / "Preview",
    preview_of => "Vista previa · {name}" / "Preview · {name}",
    detail => "Detalle" / "Details",
    field_name => "Nombre" / "Name",
    field_type => "Tipo" / "Type",
    field_size => "Tamaño" / "Size",
    field_modified => "Modificado" / "Modified",
    field_created => "Creado" / "Created",
    field_permissions => "Permisos" / "Permissions",
    field_path => "Ruta" / "Path",
    empty_dir => "Directorio vacío" / "Empty directory",
    kind_dir => "Directorio" / "Directory",
    kind_file => "Archivo" / "File",
    kind_symlink => "Enlace" / "Link",
    kind_other => "Otro" / "Other",

    // ---- Barra de estado --------------------------------------------------
    status_entries => "{count} entradas" / "{count} entries",
    status_filter => "filtro: «{filter}»" / "filter: “{filter}”",
    status_selected => "{count} seleccionadas" / "{count} selected",
    status_clipboard => "portapapeles: {count} {kind}" / "clipboard: {count} {kind}",
    clipboard_cut => "cortados" / "cut",
    clipboard_copied => "copiados" / "copied",
    status_hidden => "ocultos: {value}" / "hidden: {value}",
    status_theme => "tema: {name}" / "theme: {name}",
    status_icons => "iconos: {name}" / "icons: {name}",
    yes => "sí" / "yes",
    no => "no" / "no",

    // ---- Barras de búsqueda ----------------------------------------------
    search_global_hint => "Super búsqueda · {count} coincidencias · Esc cerrar · Intro copiar" / "Global search · {count} matches · Esc close · Enter copy",
    search_hint => "Buscar · {shown} de {total} · Esc cerrar · Intro aceptar" / "Search · {shown} of {total} · Esc close · Enter accept",

    // ---- Atajos de la barra inferior -------------------------------------
    hint_move => "mover" / "move",
    hint_parent_enter => "padre/entrar" / "parent/enter",
    hint_mark => "marcar" / "mark",
    hint_search => "buscar" / "search",
    hint_super => "super búsqueda" / "global search",
    hint_open => "abrir" / "open",
    hint_edit => "editar" / "edit",
    hint_hidden => "ocultos" / "hidden",
    hint_themes => "temas" / "themes",
    hint_help => "ayuda" / "help",
    hint_prefs => "preferencias" / "preferences",
    hint_quit => "salir" / "quit",
    hint_search_active => "búsqueda" / "search",
    hint_active => "activa" / "active",
    hint_cursor => "cursor" / "cursor",
    hint_results => "resultados" / "results",
    hint_copy => "copiar" / "copy",
    hint_accept => "aceptar" / "accept",
    hint_close => "cerrar" / "close",

    // ---- Ayuda ------------------------------------------------------------
    help => "Ayuda" / "Help",
    help_mouse => "  ratón: arrastra para seleccionar y copiar automáticamente" / "  mouse: drag to select and copy automatically",
    help_config => "  , preferencias · atajos y temas en config.toml" / "  , preferences · shortcuts and themes in config.toml",

    // ---- Preferencias -----------------------------------------------------
    prefs => "Preferencias" / "Preferences",
    prefs_shortcuts => "· Atajos" / "· Shortcuts",
    prefs_capturing => " Capturando… pulsa la nueva tecla o combinación (Esc cancela)." / " Capturing… press the new key or combination (Esc cancels).",
    prefs_search_idle => "/ buscar acción o tecla · {total} acciones" / "/ search action or key · {total} actions",
    prefs_search_filtered => "{visible} de {total} · Intro confirmar · Esc {action}" / "{visible} of {total} · Enter confirm · Esc {action}",
    prefs_action_clear => "limpiar" / "clear",
    prefs_action_cancel => "cancelar" / "cancel",
    prefs_footer => " ↑/↓ mover · Intro editar · / buscar · S guardar cambios · R restablecer · T temas · Esc cerrar" / " ↑/↓ move · Enter edit · / search · S save changes · R reset · T themes · Esc close",
    prefs_footer_search => " escribir filtro · ↑/↓ resultados · Intro confirmar · Esc cancelar" / " type filter · ↑/↓ results · Enter confirm · Esc cancel",
    prefs_header_action => "ACCIÓN" / "ACTION",
    prefs_header_keys => "TECLAS" / "KEYS",
    prefs_header_status => "ESTADO" / "STATUS",
    prefs_next => "próx." / "next",
    prefs_no_key => "(sin tecla)" / "(unbound)",
    prefs_no_matches => " Sin coincidencias para «{query}» · Esc limpia el filtro" / " No matches for “{query}” · Esc clears the filter",
    capture_cancelled => "Captura cancelada" / "Capture cancelled",
    esc_not_assignable => "Esc no se puede asignar (se usa para cancelar)" / "Esc cannot be assigned (it is used to cancel)",
    shortcut_rebound => "Atajo de «{action}» cambiado a «{key}». Pulsa S para guardar cambios." / "Shortcut for “{action}” changed to “{key}”. Press S to save changes.",
    no_matching_actions => "No hay acciones que coincidan con «{query}»" / "No actions match “{query}”",
    press_new_key => "Pulsa la nueva tecla o combinación (Esc cancela, Esc no se puede asignar)" / "Press the new key or combination (Esc cancels, Esc cannot be assigned)",

    // ---- Prompts ----------------------------------------------------------
    prompt_create => "Crear (nombre, termina en / para carpeta)" / "Create (name, end with / for a folder)",
    prompt_rename => "Renombrar" / "Rename",
    prompt_shell => "Shell · Intro ejecutar · Esc cancelar" / "Shell · Enter run · Esc cancel",
    prompt_confirm_trash => "¿Mover {count} {noun} a la papelera?" / "Move {count} {noun} to the trash?",
    prompt_confirm_delete => "¿Borrar permanentemente {count} {noun}?" / "Permanently delete {count} {noun}?",
    noun_item => "elemento" / "item",
    noun_items => "elementos" / "items",
    prompt_more => " … y {count} más" / " … and {count} more",
    prompt_confirm_hint => " Intro confirmar · Esc cancelar" / " Enter confirm · Esc cancel",

    // ---- Temas ------------------------------------------------------------
    themes => "Temas" / "Themes",
    themes_hint => " Temas disponibles (Intro aplicar · s guardar · Esc cerrar)" / " Available themes (Enter apply · s save · Esc close)",

    // ---- Idioma -----------------------------------------------------------
    language => "Idioma" / "Language",
    language_hint => " Elige el idioma de la interfaz (↑/↓ elegir · Intro aplicar · Esc cerrar)" / " Choose the interface language (↑/↓ choose · Enter apply · Esc close)",

    // ---- Mensajes de la aplicación ---------------------------------------
    trash_unavailable => "La papelera no está disponible como ruta en este sistema" / "The trash is not available as a path on this system",
    trash_empty => "La papelera está vacía" / "The trash is empty",
    locations_title => "Ubicaciones del sistema (discos y papelera)" / "System locations (disks and trash)",
    super_search_msg => "Super búsqueda «{query}» · {count} resultados" / "Global search “{query}” · {count} results",
    filter_msg => "Filtro «{query}» · {count} coincidencias" / "Filter “{query}” · {count} matches",
    nothing_to_select => "No hay ninguna entrada que seleccionar" / "No entry to select",
    deselected => "Deseleccionado «{name}»" / "Deselected “{name}”",
    selected_one => "Seleccionado «{name}» · {count} en total" / "Selected “{name}” · {count} total",
    selected_many => "{count} entradas seleccionadas · {total} en total" / "{count} entries selected · {total} total",
    nothing_to_select_entries => "No hay entradas que seleccionar" / "No entries to select",
    nothing_to_clear => "No hay selección que limpiar" / "No selection to clear",
    selection_cleared => "Selección vaciada" / "Selection cleared",
    nothing_to_copy => "No hay nada que copiar" / "Nothing to copy",
    nothing_to_cut => "No hay nada que cortar" / "Nothing to cut",
    clipboard_empty => "El portapapeles está vacío" / "The clipboard is empty",
    copied_msg => "✓ {count} elemento(s) copiados al portapapeles" / "✓ {count} item(s) copied to the clipboard",
    cut_msg => "✂ {count} elemento(s) cortados al portapapeles" / "✂ {count} item(s) cut to the clipboard",
    pasted_msg => "{count} elemento(s) pegados" / "{count} item(s) pasted",
    moved_msg => "{count} elemento(s) movidos" / "{count} item(s) moved",
    nothing_to_trash => "No hay nada que mover a la papelera" / "Nothing to move to the trash",
    confirm_trash_msg => "¿Mover {count} elemento(s) a la papelera? (Intro confirmar, Esc cancelar)" / "Move {count} item(s) to the trash? (Enter confirm, Esc cancel)",
    trashed_msg => "{count} elemento(s) movidos a la papelera" / "{count} item(s) moved to the trash",
    nothing_to_delete => "No hay nada que borrar" / "Nothing to delete",
    confirm_delete_msg => "¿Borrar permanentemente {count} elemento(s)? (Intro confirmar, Esc cancelar)" / "Permanently delete {count} item(s)? (Enter confirm, Esc cancel)",
    deleted_msg => "{count} elemento(s) borrados permanentemente" / "{count} item(s) permanently deleted",
    nothing_to_rename => "No hay ninguna entrada que renombrar" / "No entry to rename",
    empty_name => "Nombre vacío" / "Empty name",
    created => "Creado «{path}»" / "Created “{path}”",
    renamed => "Renombrado a «{path}»" / "Renamed to “{path}”",
    empty_command => "Comando vacío" / "Empty command",
    theme_changed => "Tema cambiado a «{name}»" / "Theme changed to “{name}”",
    language_changed => "Idioma cambiado a «{name}»" / "Language changed to “{name}”",
    language_save_failed => "No se pudo guardar el idioma: {err}" / "Could not save the language: {err}",
    config_path_unknown => "Ruta de configuración desconocida" / "Unknown configuration path",
    config_saved => "✓ Cambios guardados en la configuración" / "✓ Changes saved to the configuration",
    config_save_failed => "No se pudo guardar la configuración: {err}" / "Could not save the configuration: {err}",
    keys_reset => "Atajos restablecidos de fábrica. Pulsa S para guardar cambios." / "Shortcuts reset to defaults. Press S to save changes.",
    not_available => "«{action}» aún no está disponible" / "“{action}” is not available yet",
    opening_explorer => "Abriendo «{path}» en el explorador de archivos" / "Opening “{path}” in the file manager",
    select_file_to_edit => "Selecciona un archivo para editarlo" / "Select a file to edit",
    already_root => "Ya estás en la raíz del sistema" / "You are already at the system root",
    showing_hidden => "Mostrando archivos ocultos" / "Showing hidden files",
    hiding_hidden => "Ocultando archivos ocultos" / "Hiding hidden files",
    opening_default => "Abriendo «{name}» ({size}) con la app predeterminada" / "Opening “{name}” ({size}) with the default app",
    volume_size => "— (volumen)" / "— (volume)",
    calculating => "calculando..." / "calculating...",

    // ---- Vista previa -----------------------------------------------------
    loading_preview => "Cargando vista previa…" / "Loading preview…",
    preview_failed => "No se pudo generar la vista previa de este archivo." / "Could not generate a preview for this file.",
    list_failed => "No se pudo listar: {err}" / "Could not list: {err}",
    dir_preview => "Directorio: {path}" / "Directory: {path}",
    dir_items => "{count} elemento(s)" / "{count} item(s)",
    image_label => "Imagen: {name}" / "Image: {name}",
    image_decode_failed => "No se pudo decodificar la imagen." / "Could not decode the image.",
    pdf_page_total => "PDF: {name} (página {page}/{total}) · Ctrl+↓/↑ cambiar página" / "PDF: {name} (page {page}/{total}) · Ctrl+↓/↑ change page",
    pdf_page => "PDF: {name} (página {page}) · Ctrl+↓/↑ cambiar página" / "PDF: {name} (page {page}) · Ctrl+↓/↑ change page",
    pdf_no_text => "(sin texto extraíble o PDF vacío)" / "(no extractable text or empty PDF)",
    pdf_install_poppler => "Para ver páginas escaneadas/visuales, instala «poppler-utils» (pdftoppm/pdftocairo/mutool)." / "To view scanned/visual pages, install “poppler-utils” (pdftoppm/pdftocairo/mutool).",
    pdf_extract_failed => "No se pudo extraer texto del PDF." / "Could not extract text from the PDF.",
    pdf_install_hint => "Instala «poppler-utils» (pdftoppm/pdftocairo) o «mupdf-tools» (mutool) para previsualizar PDFs." / "Install “poppler-utils” (pdftoppm/pdftocairo) or “mupdf-tools” (mutool) to preview PDFs.",
    pdf_preview_failed => "No se pudo previsualizar el PDF: instala «poppler-utils» (pdftoppm/pdftocairo) o «mupdf-tools» (mutool)." / "Could not preview the PDF: install “poppler-utils” (pdftoppm/pdftocairo) or “mupdf-tools” (mutool).",
    read_failed => "No se pudo leer: {err}" / "Could not read: {err}",
    binary_file => "Archivo binario ({size})." / "Binary file ({size}).",
    text_preview_unavailable => "Vista previa de texto no disponible." / "Text preview not available.",
    empty_file => "(archivo vacío)" / "(empty file)",
    not_utf8 => "No es texto UTF-8: vista previa no disponible." / "Not UTF-8 text: preview not available.",

    // ---- Etiquetas de acciones (ayuda y preferencias) --------------------
    action_quit => "salir" / "quit",
    action_help => "ayuda" / "help",
    action_preferences => "preferencias" / "preferences",
    action_cursor_down => "bajar" / "down",
    action_cursor_up => "subir" / "up",
    action_go_parent => "subir al padre" / "go to parent",
    action_enter => "entrar / abrir" / "enter / open",
    action_open => "abrir con la app predeterminada" / "open with the default app",
    action_reveal => "abrir en el explorador de archivos" / "open in the file manager",
    action_edit => "editar con el editor" / "edit with the editor",
    action_go_top => "ir al principio" / "go to top",
    action_go_bottom => "ir al final" / "go to bottom",
    action_toggle_hidden => "mostrar/ocultar ocultos" / "show/hide hidden files",
    action_refresh => "refrescar" / "refresh",
    action_clear_status => "limpiar mensaje" / "clear message",
    action_toggle_select => "seleccionar/deseleccionar" / "select/deselect",
    action_select_all => "seleccionar todo" / "select all",
    action_clear_selection => "limpiar selección" / "clear selection",
    action_copy => "copiar" / "copy",
    action_cut => "cortar" / "cut",
    action_paste => "pegar" / "paste",
    action_trash => "mover a la papelera" / "move to trash",
    action_delete_permanent => "borrar permanentemente" / "delete permanently",
    action_create => "crear archivo/carpeta" / "create file/folder",
    action_rename => "renombrar" / "rename",
    action_filter => "buscar / filtrar" / "search / filter",
    action_global_search => "super búsqueda" / "global search",
    action_shell => "Shell" / "Shell",
    action_next_tab => "pestaña siguiente" / "next tab",
    action_prev_tab => "pestaña anterior" / "previous tab",
    action_new_tab => "nueva pestaña" / "new tab",
    action_close_tab => "cerrar pestaña" / "close tab",
    action_locations => "ubicaciones (discos y papelera)" / "locations (disks and trash)",
    action_go_trash => "ir a la papelera" / "go to trash",
    action_bookmark => "marcador" / "bookmark",
    action_toggle_language => "idioma / lenguaje" / "language",

    // ---- Mensajes del binario --------------------------------------------
    no_text_selected => "No hay texto seleccionado" / "No text selected",
    copied_chars => "Copiado: {count} caracteres" / "Copied: {count} characters",
    copy_failed => "No se pudo copiar: {err}" / "Could not copy: {err}",
    open_failed => "No se pudo abrir «{path}»: {err}" / "Could not open “{path}”: {err}",
    shell_done => "Shell: «{command}» completado" / "Shell: “{command}” completed",
    shell_exit => "Shell: «{command}» terminó con estado {status}" / "Shell: “{command}” exited with status {status}",
    shell_failed => "Shell: no se pudo ejecutar «{command}»: {err}" / "Shell: could not run “{command}”: {err}",
    config_edited => "Configuración editada · reinicia levi para aplicar los cambios" / "Configuration edited · restart levi to apply the changes",
    edited => "Editado: {path}" / "Edited: {path}",
    editor_exit => "El editor terminó con estado {status}" / "The editor exited with status {status}",
    launch_failed => "No se pudo lanzar «{command}»: {err}" / "Could not launch “{command}”: {err}",
    opening_with => "Abriendo con «{command}»" / "Opening with “{command}”",
    warning_prefix => "aviso: {warning}" / "warning: {warning}",
    not_a_directory => "«{path}» no es un directorio" / "“{path}” is not a directory",
    keys_title => "Atajos de Leví File Manager TUI" / "Levi File Manager TUI shortcuts",
    keys_config => "Configuración: {path}" / "Configuration: {path}",
    keys_action => "ACCIÓN" / "ACTION",
    keys_description => "DESCRIPCIÓN" / "DESCRIPTION",
    keys_keys => "TECLAS" / "KEYS",
    keys_unassigned => "(sin asignar)" / "(unassigned)",
    keys_soon => " · próximamente" / " · coming soon",
}

/// Textos del idioma indicado.
pub fn strings(language: Language) -> &'static Strings {
    match language {
        Language::Es => &ES,
        Language::En => &EN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_language_names() {
        assert_eq!("es".parse::<Language>(), Ok(Language::Es));
        assert_eq!("Español".parse::<Language>(), Ok(Language::Es));
        assert_eq!("EN".parse::<Language>(), Ok(Language::En));
        assert_eq!("English".parse::<Language>(), Ok(Language::En));
        assert!("klingon".parse::<Language>().is_err());
    }

    #[test]
    fn language_toggles_between_spanish_and_english() {
        assert_eq!(Language::Es.next(), Language::En);
        assert_eq!(Language::En.next(), Language::Es);
        assert_eq!(Language::default(), Language::Es);
    }

    #[test]
    fn fill_replaces_named_placeholders() {
        assert_eq!(fill("{n} entradas", &[("n", "3")]), "3 entradas");
        assert_eq!(
            fill(
                "{path} ({shown} de {total})",
                &[("path", "/tmp"), ("shown", "2"), ("total", "5")]
            ),
            "/tmp (2 de 5)"
        );
    }

    #[test]
    fn both_languages_translate_the_interface() {
        assert_eq!(strings(Language::Es).detail, "Detalle");
        assert_eq!(strings(Language::En).detail, "Details");
        assert_ne!(strings(Language::En).help, strings(Language::Es).help);
    }

    #[test]
    fn all_languages_have_strings_and_distinct_codes() {
        for language in Language::ALL {
            let texts = strings(language);
            assert!(!texts.detail.is_empty());
            assert!(!texts.language.is_empty());
            assert!(!texts.language_hint.is_empty());
        }
        assert_eq!(Language::Es.code(), "es");
        assert_eq!(Language::En.code(), "en");
    }
}
