//! Estado de la aplicación y operaciones de navegación.
//!
//! La UI solo lee este estado; toda la mutación pasa por métodos de [`App`].

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::SystemTime;

use levi_core::config::Config;
use levi_core::fs::{self, Entry, EntryKind};
use levi_core::i18n::{self, Language, Strings};
use levi_core::keymap::Keymap;
use levi_core::ops;
use levi_core::theme::Theme;
use ratatui::widgets::ListState;

/// Acción que la UI pide al bucle principal (abrir o editar un archivo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Abrir con la aplicación predeterminada del sistema.
    Open(PathBuf),
    /// Abrir con el editor de texto externo.
    Edit(PathBuf),
    /// Ejecutar un comando de shell en el directorio actual.
    Shell { command: String, cwd: PathBuf },
}

/// Cantidad máxima de resultados devueltos por el super buscador.
const MAX_GLOBAL_RESULTS: usize = 200;

/// Mensaje mostrado en la barra de estado.
pub struct Status {
    pub text: String,
    pub error: bool,
}

/// Tipo de prompt activo en la interfaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Create,
    Rename,
    /// Barra para ejecutar un comando de shell.
    Shell,
    /// Confirmación antes de mover a la papelera.
    ConfirmTrash,
    /// Confirmación antes de borrar permanentemente.
    ConfirmDeletePermanent,
}

/// Prompt de texto mostrado sobre la interfaz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub kind: PromptKind,
    pub input: String,
    /// Posición del cursor dentro de `input` (en bytes/chars; se usa por caracteres).
    pub cursor: usize,
    pub target: Option<PathBuf>,
    /// Nombres de los elementos afectados (para confirmaciones).
    pub items: Vec<String>,
}

/// Firma de metadatos usada para detectar cambios en el archivo previsualizado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSignature {
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
    pub len: u64,
}

/// Resultado de una vista previa calculada en segundo plano.
pub struct PreviewResult {
    pub path: PathBuf,
    pub signature: Option<PreviewSignature>,
    pub lines: Vec<String>,
    pub image: Option<image::DynamicImage>,
    pub pdf_page: Option<u32>,
    pub pdf_pages: Option<u32>,
}

/// Firma de metadatos usada para detectar cambios en el archivo de tema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeSignature {
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
    pub len: u64,
}

/// Estado de una pestaña individual.
pub struct Tab {
    pub cwd: PathBuf,
    /// Entradas del directorio tras aplicar el filtro de búsqueda.
    pub entries: Vec<Entry>,
    /// Todas las entradas del directorio, sin filtrar.
    pub all_entries: Vec<Entry>,
    pub cursor: usize,
    pub list_state: ListState,
    pub parent_entries: Vec<Entry>,
    pub parent_cursor: usize,
    pub parent_state: ListState,
    pub show_hidden: bool,
    /// Texto del filtro de búsqueda incremental.
    pub filter: String,
    /// Posición del cursor dentro de `filter` (en caracteres).
    pub filter_cursor: usize,
    /// `true` mientras se está escribiendo una búsqueda.
    pub search_active: bool,
    /// `true` cuando la búsqueda activa busca en todo el sistema.
    pub search_global: bool,
    /// Resultados de una búsqueda global activa.
    pub global_results: Vec<Entry>,
    /// Rutas marcadas con la tecla de selección.
    pub selected: HashSet<PathBuf>,
    pub search_origin: Option<String>,
    /// Líneas de la vista previa del archivo seleccionado.
    pub preview_lines: Vec<String>,
    /// Desplazamiento vertical de la vista previa.
    pub preview_scroll: usize,
    /// Vista previa de imagen (si el archivo es imagen).
    pub preview_image: Option<ratatui_image::protocol::StatefulProtocol>,
    /// Metadatos del último preview para detectar cambios externos.
    pub preview_signature: Option<PreviewSignature>,
    /// Historial de navegación.
    pub history: Vec<PathBuf>,
    /// Vista de ubicaciones del sistema (discos/papelera).
    pub locations_mode: bool,
    /// Última fecha de modificación observada del directorio (auto-refresco).
    pub dir_modified: Option<SystemTime>,
    /// Página actual de vista previa PDF (1-based).
    pub preview_pdf_page: u32,
    /// Número total de páginas del PDF en vista previa, si se conoce.
    pub preview_pdf_pages: Option<u32>,
}

/// Posición de una celda dentro de la terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    pub x: u16,
    pub y: u16,
}

/// Selección de texto hecha con el ratón.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    pub anchor: TextPosition,
    pub cursor: TextPosition,
}

impl TextSelection {
    /// Devuelve el inicio y el final en orden de lectura.
    pub fn ordered(self) -> (TextPosition, TextPosition) {
        if (self.anchor.y, self.anchor.x) <= (self.cursor.y, self.cursor.x) {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }

    /// `true` si no se ha arrastrado fuera de la celda inicial.
    pub fn is_empty(self) -> bool {
        self.anchor == self.cursor
    }
}

/// Estado completo de una sesión de Levi.
pub struct App {
    /// Pestañas abiertas.
    pub tabs: Vec<Tab>,
    /// Índice de la pestaña activa.
    pub active: usize,

    pub status: Option<Status>,
    pub show_help: bool,
    pub show_prefs: bool,
    pub prefs_scroll: usize,
    pub should_quit: bool,
    pub config: Config,
    pub config_path: PathBuf,
    pub theme: Theme,
    pub theme_signature: Option<ThemeSignature>,
    pub keymap: Keymap,
    pub request: Option<Request>,
    /// Portapapeles interno para copiar/cortar.
    pub clipboard: Vec<PathBuf>,
    pub clipboard_cut: bool,
    /// Prompt de texto activo (crear/renombrar).
    pub prompt: Option<Prompt>,
    /// Overlay de selección de tema.
    pub show_themes: bool,
    pub theme_list: Vec<String>,
    pub theme_cursor: usize,
    /// Overlay de selección de idioma.
    pub show_languages: bool,
    pub language_cursor: usize,
    /// Cursor y captura para editar atajos dentro de preferencias.
    pub prefs_key_cursor: usize,
    pub prefs_capture: bool,
    /// Barra de búsqueda de acciones dentro de preferencias: consulta y foco.
    pub prefs_query: String,
    pub prefs_search_active: bool,
    /// Selección de texto con el ratón y petición de copia al portapapeles.
    pub text_selection: Option<TextSelection>,
    pub copy_selection: bool,
    /// Directorio de configuración (para temas y guardado).
    pub config_dir: PathBuf,
    /// Selector de protocolo de imagen (detecta Kitty/Sixel/iTerm2).
    pub preview_picker: Option<ratatui_image::picker::Picker>,
    /// Caché y cálculo asíncrono de tamaños de directorio (para detalles).
    pub dir_sizes: HashMap<PathBuf, u64>,
    pub dir_size_pending: HashSet<PathBuf>,
    dir_size_tx: mpsc::Sender<(PathBuf, u64)>,
    dir_size_rx: mpsc::Receiver<(PathBuf, u64)>,
    preview_tx: mpsc::Sender<PreviewResult>,
    preview_rx: mpsc::Receiver<PreviewResult>,
    preview_pending: Option<PathBuf>,
    global_search_tx: mpsc::Sender<(u64, Vec<Entry>)>,
    global_search_rx: mpsc::Receiver<(u64, Vec<Entry>)>,
    global_search_generation: u64,
}

fn pdf_page_count(path: &Path) -> Option<u32> {
    let output = std::process::Command::new("pdfinfo")
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Pages:")
            && let Ok(n) = rest.trim().parse::<u32>()
        {
            return Some(n.max(1));
        }
    }
    None
}

fn render_pdf_page(path: &Path, page: u32, dpi: u32) -> Option<image::DynamicImage> {
    let page = page.max(1);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let base = std::env::temp_dir().join(format!(
        "levi-pdf-page-{}-{}-{}",
        std::process::id(),
        stamp,
        page
    ));

    let try_prefix = |cmd: &str, args: Vec<String>, out: &Path| -> Option<image::DynamicImage> {
        let status = std::process::Command::new(cmd)
            .args(&args)
            .arg(path)
            .arg(out)
            .status()
            .ok()?;
        if !status.success() {
            return None;
        }
        let candidate = if out.exists() {
            out.to_path_buf()
        } else {
            out.with_extension("png")
        };
        image::ImageReader::open(&candidate).ok()?.decode().ok()
    };

    let try_mutool = |out: &Path| -> Option<image::DynamicImage> {
        let status = std::process::Command::new("mutool")
            .args(["draw", "-q", "-F", "png", "-r", &dpi.to_string(), "-o"])
            .arg(out)
            .arg(path)
            .arg(page.to_string())
            .status()
            .ok()?;
        if !status.success() || !out.exists() {
            return None;
        }
        image::ImageReader::open(out).ok()?.decode().ok()
    };

    let img = try_prefix(
        "pdftocairo",
        vec![
            "-png".into(),
            "-f".into(),
            page.to_string(),
            "-l".into(),
            page.to_string(),
            "-singlefile".into(),
            "-r".into(),
            dpi.to_string(),
        ],
        &base,
    )
    .or_else(|| {
        try_prefix(
            "pdftoppm",
            vec![
                "-png".into(),
                "-f".into(),
                page.to_string(),
                "-l".into(),
                page.to_string(),
                "-singlefile".into(),
                "-r".into(),
                dpi.to_string(),
            ],
            &base,
        )
    })
    .or_else(|| try_mutool(&base.with_extension("png")));

    let _ = std::fs::remove_file(&base);
    let _ = std::fs::remove_file(base.with_extension("png"));
    let _ = std::fs::remove_file(format!("{}.png", base.display()));
    img
}

/// Contenido calculado de una vista previa: líneas, imagen opcional y
/// página actual del PDF junto a su total (si se conoce).
type PreviewContent = (
    Vec<String>,
    Option<image::DynamicImage>,
    Option<(u32, Option<u32>)>,
);

/// Calcula el contenido de la vista previa (pensado para ejecutarse en segundo plano).
fn compute_preview(path: &Path, name: &str, pdf_page: u32, language: Language) -> PreviewContent {
    let s = i18n::strings(language);
    // Imágenes.
    let is_image = path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "tif" | "ico"
        )
    });
    if is_image {
        let mut lines = vec![i18n::fill(s.image_label, &[("name", name)])];
        let decoded = image::ImageReader::open(path)
            .ok()
            .and_then(|r| r.with_guessed_format().ok())
            .and_then(|r| r.decode().ok());
        if let Some(img) = decoded {
            (lines, Some(img), None)
        } else {
            lines.push(s.image_decode_failed.to_string());
            (lines, None, None)
        }
    } else if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
    {
        const DPI: u32 = 220;
        let total = pdf_page_count(path);
        let page = if let Some(t) = total {
            pdf_page.clamp(1, t.max(1))
        } else {
            pdf_page.max(1)
        };

        if let Some(img) = render_pdf_page(path, page, DPI) {
            let label = if let Some(t) = total {
                i18n::fill(
                    s.pdf_page_total,
                    &[
                        ("name", name),
                        ("page", &page.to_string()),
                        ("total", &t.to_string()),
                    ],
                )
            } else {
                i18n::fill(s.pdf_page, &[("name", name), ("page", &page.to_string())])
            };
            return (vec![label], Some(img), Some((page, total)));
        }

        // Si la página pedida falla pero podría estar fuera de rango, intenta volver a 1.
        if page > 1
            && let Some(img) = render_pdf_page(path, 1, DPI)
        {
            let total = total.or(Some(page.saturating_sub(1)));
            let label = if let Some(t) = total {
                i18n::fill(
                    s.pdf_page_total,
                    &[("name", name), ("page", "1"), ("total", &t.to_string())],
                )
            } else {
                i18n::fill(s.pdf_page, &[("name", name), ("page", "1")])
            };
            return (vec![label], Some(img), Some((1, total)));
        }

        // Fallback: texto extraído (integrado con pdf-extract).
        #[cfg(feature = "pdf")]
        if let Ok(text) = pdf_extract::extract_text(path) {
            let mut lines = vec![format!("PDF: {name}"), String::new()];
            if text.trim().is_empty() {
                lines.push(s.pdf_no_text.to_string());
                lines.push(s.pdf_install_poppler.to_string());
            } else {
                lines.extend(text.lines().map(str::to_string));
            }
            return (lines, None, None);
        }
        // Último intento: pdftotext externo si está disponible.
        let mut lines = vec![format!("PDF: {name}"), String::new()];
        match std::process::Command::new("pdftotext")
            .args(["-layout", "-enc", "UTF-8", "-nopgbrk"])
            .arg(path)
            .arg("-")
            .output()
        {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout);
                if text.trim().is_empty() {
                    lines.push(s.pdf_no_text.to_string());
                    lines.push(s.pdf_install_poppler.to_string());
                } else {
                    lines.extend(text.lines().map(|line| line.to_string()));
                }
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                lines.push(s.pdf_extract_failed.to_string());
                if !err.trim().is_empty() {
                    lines.push(err.lines().next().unwrap_or("").to_string());
                }
                lines.push(s.pdf_install_hint.to_string());
            }
            Err(_) => {
                lines.push(s.pdf_preview_failed.to_string());
            }
        }
        (lines, None, None)
    } else {
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(err) => {
                return (
                    vec![i18n::fill(s.read_failed, &[("err", &err.to_string())])],
                    None,
                    None,
                );
            }
        };
        match std::fs::read(path) {
            Ok(bytes) => {
                if bytes.contains(&0) {
                    return (
                        vec![
                            i18n::fill(
                                s.binary_file,
                                &[("size", &levi_core::fs::human_size(metadata.len()))],
                            ),
                            s.text_preview_unavailable.to_string(),
                        ],
                        None,
                        None,
                    );
                }
                match String::from_utf8(bytes) {
                    Ok(text) => {
                        let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
                        if lines.is_empty() && !text.is_empty() {
                            lines.push(String::new());
                        }
                        if lines.is_empty() {
                            lines.push(s.empty_file.to_string());
                        }
                        (lines, None, None)
                    }
                    Err(_) => (vec![s.not_utf8.to_string()], None, None),
                }
            }
            Err(err) => (
                vec![i18n::fill(s.read_failed, &[("err", &err.to_string())])],
                None,
                None,
            ),
        }
    }
}

impl App {
    /// Idioma activo de la interfaz.
    pub fn language(&self) -> Language {
        self.config.language
    }

    /// Textos del idioma activo.
    pub fn strings(&self) -> &'static Strings {
        i18n::strings(self.config.language)
    }

    /// Abre el selector de idioma con el cursor sobre el idioma activo.
    pub fn open_languages(&mut self) {
        self.show_languages = true;
        self.language_cursor = Language::ALL
            .iter()
            .position(|language| *language == self.config.language)
            .unwrap_or(0);
        self.clear_text_selection();
    }

    /// Cierra el selector de idioma sin cambiar nada.
    pub fn close_languages(&mut self) {
        self.show_languages = false;
    }

    /// Aplica un idioma, lo guarda en `config.toml` y cierra el selector.
    pub fn apply_language(&mut self, language: Language) {
        self.show_languages = false;
        self.language_cursor = Language::ALL
            .iter()
            .position(|candidate| *candidate == language)
            .unwrap_or(0);

        // Elegir el idioma que ya está activo solo cierra el selector.
        if self.config.language == language {
            return;
        }
        self.config.language = language;
        let name = language.display_name();

        // Los textos de la vista previa se generan en otro hilo: se regeneran
        // para que también aparezcan en el idioma nuevo.
        self.update_preview();

        if !self.config_dir.as_os_str().is_empty()
            && let Err(err) =
                levi_core::config::save_language(&self.config_dir, self.config.language)
        {
            let text = i18n::fill(
                self.strings().language_save_failed,
                &[("err", &err.to_string())],
            );
            self.set_error(text);
            return;
        }

        let message = i18n::fill(self.strings().language_changed, &[("name", name)]);
        self.set_message(message);
    }

    /// Quita la selección de texto y cancela cualquier copia pendiente.
    pub fn clear_text_selection(&mut self) {
        self.text_selection = None;
        self.copy_selection = false;
    }

    /// Recibe resultados de cálculo de tamaño de directorios.
    pub fn poll_dir_sizes(&mut self) {
        while let Ok((path, size)) = self.dir_size_rx.try_recv() {
            self.dir_size_pending.remove(&path);
            self.dir_sizes.insert(path, size);
        }
    }

    /// Recibe resultados de vistas previas generadas en segundo plano.
    pub fn poll_previews(&mut self) {
        while let Ok(result) = self.preview_rx.try_recv() {
            if self.preview_pending.as_ref() == Some(&result.path) {
                self.preview_pending = None;
            }
            let Some(selected_path) = self.selected_entry().map(|e| e.path.clone()) else {
                continue;
            };
            if selected_path != result.path {
                continue;
            }
            let expected_page = self.tabs[self.active].preview_pdf_page;
            if let Some(page) = result.pdf_page
                && page != expected_page
            {
                continue;
            }
            let tab = &mut self.tabs[self.active];
            if tab.preview_signature.as_ref().map(|s| &s.path) == Some(&result.path) {
                tab.preview_lines = result.lines;
                tab.preview_image = None;
                if result.pdf_page.is_none() {
                    tab.preview_scroll = 0;
                }
                if let Some(page) = result.pdf_page {
                    tab.preview_pdf_page = page;
                }
                if result.pdf_pages.is_some() {
                    tab.preview_pdf_pages = result.pdf_pages;
                }
                if let Some(dyn_img) = result.image {
                    if self.preview_picker.is_none() {
                        self.preview_picker =
                            ratatui_image::picker::Picker::from_query_stdio().ok();
                        if self.preview_picker.is_none() {
                            self.preview_picker = Some(ratatui_image::picker::Picker::halfblocks());
                        }
                    }
                    if let Some(picker) = self.preview_picker.as_ref() {
                        tab.preview_image = Some(picker.new_resize_protocol(dyn_img));
                    }
                } else if result.pdf_page.is_some() && tab.preview_pdf_page > 1 {
                    // Página fuera de rango: vuelve a la primera.
                    tab.preview_pdf_page = 1;
                }
            }
        }
    }

    /// Recibe resultados de la super búsqueda.
    pub fn poll_global_search(&mut self) {
        let mut last: Option<(u64, Vec<Entry>)> = None;
        while let Ok(msg) = self.global_search_rx.try_recv() {
            last = Some(msg);
        }
        if let Some((generation, results)) = last
            && generation == self.global_search_generation
            && self.tabs[self.active].search_global
        {
            self.tabs[self.active].global_results = results;
            self.apply_filter();
            self.tabs[self.active].cursor = 0;
            self.sync_states();
        }
    }

    /// Solicita el cálculo de tamaño de un directorio en segundo plano.
    pub fn request_dir_size(&mut self, path: PathBuf) {
        if self.dir_sizes.contains_key(&path) || self.dir_size_pending.contains(&path) {
            return;
        }
        self.dir_size_pending.insert(path.clone());
        let tx = self.dir_size_tx.clone();
        std::thread::spawn(move || {
            let size = levi_core::fs::dir_size(&path);
            let _ = tx.send((path, size));
        });
    }

    /// Tamaño a mostrar en detalles para una entrada.
    pub fn detail_size_text(&mut self, entry: &Entry) -> String {
        if entry.is_dir() {
            // Evita calcular recursivamente el tamaño de volúmenes/raíces (muy costoso).
            let mounts = levi_core::platform::mount_points();
            let is_mount = mounts.iter().any(|m| m == &entry.path)
                || entry.path.parent().is_none()
                || entry
                    .path
                    .parent()
                    .is_some_and(|p| p == entry.path || p.as_os_str().is_empty());
            if is_mount {
                return self.strings().volume_size.to_string();
            }
            if let Some(size) = self.dir_sizes.get(&entry.path).copied() {
                levi_core::fs::human_size(size)
            } else {
                self.request_dir_size(entry.path.clone());
                self.strings().calculating.to_string()
            }
        } else {
            levi_core::fs::human_size(entry.size)
        }
    }

    /// Crea la aplicación con la configuración y el tema por defecto.
    pub fn new(path: PathBuf) -> Self {
        Self::with_config(
            path,
            Config::default(),
            Theme::default(),
            Keymap::defaults(),
        )
    }

    /// Crea la aplicación con una configuración, un tema y unos atajos concretos.
    pub fn with_config(path: PathBuf, config: Config, theme: Theme, keymap: Keymap) -> Self {
        let show_hidden = config.show_hidden;
        let tab = Tab {
            cwd: path,
            entries: Vec::new(),
            all_entries: Vec::new(),
            cursor: 0,
            list_state: ListState::default(),
            parent_entries: Vec::new(),
            parent_cursor: 0,
            parent_state: ListState::default(),
            show_hidden,
            filter: String::new(),
            filter_cursor: 0,
            search_active: false,
            search_global: false,
            global_results: Vec::new(),
            selected: HashSet::new(),
            search_origin: None,
            preview_lines: Vec::new(),
            preview_scroll: 0,
            preview_image: None,
            preview_signature: None,
            history: Vec::new(),
            locations_mode: false,
            dir_modified: None,
            preview_pdf_page: 1,
            preview_pdf_pages: None,
        };
        let mut app = Self {
            tabs: vec![tab],
            active: 0,
            status: None,
            show_help: false,
            show_prefs: false,
            prefs_scroll: 0,
            should_quit: false,
            config,
            config_path: PathBuf::new(),
            theme,
            theme_signature: None,
            keymap,
            request: None,
            clipboard: Vec::new(),
            clipboard_cut: false,
            prompt: None,
            show_themes: false,
            theme_list: Vec::new(),
            theme_cursor: 0,
            show_languages: false,
            language_cursor: 0,
            prefs_key_cursor: 0,
            prefs_capture: false,
            prefs_query: String::new(),
            prefs_search_active: false,
            text_selection: None,
            copy_selection: false,
            config_dir: PathBuf::new(),
            preview_picker: None,
            dir_sizes: HashMap::new(),
            dir_size_pending: HashSet::new(),
            dir_size_tx: mpsc::channel().0,
            dir_size_rx: mpsc::channel().1,
            preview_tx: mpsc::channel().0,
            preview_rx: mpsc::channel().1,
            preview_pending: None,
            global_search_tx: mpsc::channel().0,
            global_search_rx: mpsc::channel().1,
            global_search_generation: 0,
        };
        // Canal para resultados de tamaño de directorio.
        let (tx, rx) = mpsc::channel();
        app.dir_size_tx = tx;
        app.dir_size_rx = rx;
        // Canal para vistas previas en segundo plano.
        let (ptx, prx) = mpsc::channel();
        app.preview_tx = ptx;
        app.preview_rx = prx;
        // Canal para super búsqueda en segundo plano.
        let (gtx, grx) = mpsc::channel();
        app.global_search_tx = gtx;
        app.global_search_rx = grx;
        app.refresh();
        app
    }

    /// Crea una nueva pestaña en el mismo directorio que la activa.
    pub fn new_tab(&mut self) {
        let cwd = self.tabs[self.active].cwd.clone();
        let show_hidden = self.tabs[self.active].show_hidden;
        let tab = Tab {
            cwd,
            entries: Vec::new(),
            all_entries: Vec::new(),
            cursor: 0,
            list_state: ListState::default(),
            parent_entries: Vec::new(),
            parent_cursor: 0,
            parent_state: ListState::default(),
            show_hidden,
            filter: String::new(),
            filter_cursor: 0,
            search_active: false,
            search_global: false,
            global_results: Vec::new(),
            selected: HashSet::new(),
            search_origin: None,
            preview_lines: Vec::new(),
            preview_scroll: 0,
            preview_image: None,
            preview_signature: None,
            history: Vec::new(),
            locations_mode: false,
            dir_modified: None,
            preview_pdf_page: 1,
            preview_pdf_pages: None,
        };
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.refresh();
    }

    /// Cierra la pestaña activa (si queda más de una).
    pub fn close_tab(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }
        self.tabs.remove(self.active);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len().saturating_sub(1);
        }
        self.refresh();
    }

    /// Pestaña siguiente.
    pub fn next_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.active = (self.active + 1) % self.tabs.len();
        self.refresh();
    }

    /// Pestaña anterior.
    pub fn prev_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        if self.active == 0 {
            self.active = self.tabs.len() - 1;
        } else {
            self.active -= 1;
        }
        self.refresh();
    }

    pub fn show_locations(&mut self) {
        self.tabs[self.active].locations_mode = true;
        self.tabs[self.active].filter.clear();
        self.tabs[self.active].filter_cursor = 0;
        self.tabs[self.active].search_active = false;
        self.tabs[self.active].search_global = false;
        self.tabs[self.active].global_results.clear();
        self.refresh();
    }

    /// Navega directamente a la papelera del sistema.
    pub fn go_trash(&mut self) {
        let Some(trash) = levi_core::platform::trash_dir() else {
            self.set_error(self.strings().trash_unavailable.to_string());
            return;
        };
        if !trash.exists() {
            self.set_message(self.strings().trash_empty.to_string());
            return;
        }
        self.change_dir(trash, None);
    }

    /// Vuelve a leer el directorio actual conservando la entrada seleccionada.
    pub fn refresh(&mut self) {
        self.refresh_inner(false);
    }

    fn refresh_inner(&mut self, quiet: bool) {
        let previous = self.selected_entry().map(|entry| entry.name.clone());

        if self.tabs[self.active].locations_mode {
            let mut entries = fs::system_location_entries();
            if !self.tabs[self.active].show_hidden {
                entries.retain(|e| !e.hidden);
            }
            self.tabs[self.active].all_entries = entries;
            self.tabs[self.active].parent_entries.clear();
            self.tabs[self.active].parent_cursor = 0;
            self.apply_filter();
            if let Some(name) = previous
                && let Some(pos) = self.tabs[self.active]
                    .entries
                    .iter()
                    .position(|entry| entry.name == name)
            {
                self.tabs[self.active].cursor = pos;
            }
            self.clamp_cursor();
            self.sync_states();
            if !quiet {
                self.status = Some(Status {
                    text: self.strings().locations_title.to_string(),
                    error: false,
                });
            }
            self.tabs[self.active].dir_modified = self.tabs[self.active]
                .cwd
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok());
            return;
        }

        match fs::list_dir(
            &self.tabs[self.active].cwd,
            self.tabs[self.active].show_hidden,
        ) {
            Ok(mut entries) => {
                let cwd = self.tabs[self.active].cwd.clone();
                let is_root = cwd.parent().is_none()
                    || cwd
                        .parent()
                        .is_some_and(|p| p == cwd || p.as_os_str().is_empty());
                if is_root {
                    let mut special = fs::system_location_entries();
                    special.retain(|e| e.path != cwd);
                    if !self.tabs[self.active].show_hidden {
                        special.retain(|e| !e.hidden);
                    }
                    // Evita duplicados por ruta.
                    special.retain(|s| !entries.iter().any(|e| e.path == s.path));
                    special.append(&mut entries);
                    entries = special;
                }
                self.tabs[self.active].all_entries = entries;
                self.apply_filter();
                if let Some(name) = previous
                    && let Some(pos) = self.tabs[self.active]
                        .entries
                        .iter()
                        .position(|entry| entry.name == name)
                {
                    self.tabs[self.active].cursor = pos;
                }
                self.clamp_cursor();
                if !quiet {
                    self.status = None;
                }
                self.tabs[self.active].dir_modified =
                    cwd.metadata().ok().and_then(|m| m.modified().ok());
            }
            Err(err) => {
                self.tabs[self.active].all_entries.clear();
                self.tabs[self.active].entries.clear();
                self.tabs[self.active].cursor = 0;
                self.set_error(err.to_string());
            }
        }

        self.reload_parent();
        self.sync_states();
    }

    /// Refresca el listado si el directorio actual cambió en disco (auto-actualización).
    pub fn tick_dir(&mut self) {
        if self.tabs[self.active].search_active && self.tabs[self.active].search_global {
            return;
        }
        let cwd = self.tabs[self.active].cwd.clone();
        let modified = cwd.metadata().ok().and_then(|m| m.modified().ok());
        if modified != self.tabs[self.active].dir_modified {
            self.refresh_inner(true);
        }
    }

    /// Aplica el filtro actual sobre los resultados activos para construir `entries`.
    fn apply_filter(&mut self) {
        let needle = levi_core::text::normalize_search(self.tabs[self.active].filter.trim());
        let entries = if self.tabs[self.active].search_global {
            self.tabs[self.active]
                .global_results
                .iter()
                .filter(|entry| levi_core::text::normalize_search(&entry.name).contains(&needle))
                .cloned()
                .collect()
        } else {
            self.tabs[self.active]
                .all_entries
                .iter()
                .filter(|entry| levi_core::text::normalize_search(&entry.name).contains(&needle))
                .cloned()
                .collect()
        };
        self.tabs[self.active].entries = if needle.is_empty() {
            if self.tabs[self.active].search_global {
                self.tabs[self.active].global_results.clone()
            } else {
                self.tabs[self.active].all_entries.clone()
            }
        } else {
            entries
        };
        self.clamp_cursor();
    }

    /// Empieza una búsqueda incremental limpia.
    pub fn start_search(&mut self) {
        self.tabs[self.active].search_global = false;
        self.tabs[self.active].global_results.clear();
        self.tabs[self.active].search_origin =
            self.selected_entry().map(|entry| entry.name.clone());
        self.tabs[self.active].filter.clear();
        self.tabs[self.active].filter_cursor = 0;
        self.tabs[self.active].search_active = true;
        self.apply_filter();
        self.tabs[self.active].cursor = 0;
        self.sync_states();
    }

    /// Empieza el super buscador: busca por nombre en segundo plano.
    pub fn start_global_search(&mut self) {
        self.tabs[self.active].search_global = true;
        self.tabs[self.active].global_results.clear();
        self.tabs[self.active].filter.clear();
        self.tabs[self.active].filter_cursor = 0;
        self.tabs[self.active].search_active = true;
        self.tabs[self.active].search_origin = None;
        self.apply_filter();
        self.sync_states();
    }

    /// Ejecuta la búsqueda global en segundo plano (no bloquea la UI).
    fn run_global_search(&mut self) {
        self.global_search_generation = self.global_search_generation.wrapping_add(1);
        let generation = self.global_search_generation;
        let query = self.tabs[self.active].filter.clone();
        let show_hidden = self.tabs[self.active].show_hidden;

        if query.trim().is_empty() {
            self.tabs[self.active].global_results.clear();
            self.apply_filter();
            self.sync_states();
            return;
        }

        self.tabs[self.active].global_results.clear();
        self.apply_filter();
        self.sync_states();

        let root = self.global_search_root();
        let tx = self.global_search_tx.clone();
        std::thread::spawn(move || {
            let results =
                levi_core::fs::search_global(&root, &query, show_hidden, MAX_GLOBAL_RESULTS);
            let _ = tx.send((generation, results));
        });
    }

    /// Directorio base del super buscador (home o directorio actual; práctico y rápido).
    fn global_search_root(&self) -> PathBuf {
        if cfg!(windows) {
            if let Some(home) = std::env::var_os("USERPROFILE") {
                let path = PathBuf::from(home);
                if path.is_dir() {
                    return path;
                }
            }
            let drive = std::env::var_os("SystemDrive")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("C:"));
            PathBuf::from(format!("{}\\", drive.to_string_lossy()))
        } else if let Some(home) = std::env::var_os("HOME") {
            let path = PathBuf::from(home);
            if path.is_dir() {
                return path;
            }
            self.tabs[self.active].cwd.clone()
        } else {
            self.tabs[self.active].cwd.clone()
        }
    }

    /// Añade un carácter al filtro de búsqueda en la posición del cursor.
    pub fn push_search_char(&mut self, character: char) {
        let cursor = self.tabs[self.active].filter_cursor;
        let byte_pos = char_to_byte(&self.tabs[self.active].filter, cursor);
        self.tabs[self.active].filter.insert(byte_pos, character);
        self.tabs[self.active].filter_cursor += 1;
        if self.tabs[self.active].search_global {
            self.run_global_search();
        } else {
            self.apply_filter();
        }
        self.tabs[self.active].cursor = 0;
        self.sync_states();
    }

    /// Borra el carácter anterior al cursor del filtro.
    pub fn pop_search_char(&mut self) {
        let cursor = self.tabs[self.active].filter_cursor;
        if cursor == 0 {
            return;
        }
        let start = char_to_byte(&self.tabs[self.active].filter, cursor - 1);
        let end = char_to_byte(&self.tabs[self.active].filter, cursor);
        self.tabs[self.active].filter.replace_range(start..end, "");
        self.tabs[self.active].filter_cursor -= 1;
        if self.tabs[self.active].search_global {
            self.run_global_search();
        } else {
            self.apply_filter();
        }
        self.tabs[self.active].cursor = 0;
        self.sync_states();
    }

    /// Borra el carácter en la posición del cursor del filtro.
    pub fn delete_search_char(&mut self) {
        let cursor = self.tabs[self.active].filter_cursor;
        let len = self.tabs[self.active].filter.chars().count();
        if cursor >= len {
            return;
        }
        let start = char_to_byte(&self.tabs[self.active].filter, cursor);
        let end = char_to_byte(&self.tabs[self.active].filter, cursor + 1);
        self.tabs[self.active].filter.replace_range(start..end, "");
        if self.tabs[self.active].search_global {
            self.run_global_search();
        } else {
            self.apply_filter();
        }
        self.tabs[self.active].cursor = 0;
        self.sync_states();
    }

    pub fn move_search_cursor_left(&mut self) {
        self.tabs[self.active].filter_cursor =
            self.tabs[self.active].filter_cursor.saturating_sub(1);
    }

    pub fn move_search_cursor_right(&mut self) {
        let len = self.tabs[self.active].filter.chars().count();
        if self.tabs[self.active].filter_cursor < len {
            self.tabs[self.active].filter_cursor += 1;
        }
    }

    pub fn move_search_cursor_home(&mut self) {
        self.tabs[self.active].filter_cursor = 0;
    }

    pub fn move_search_cursor_end(&mut self) {
        self.tabs[self.active].filter_cursor = self.tabs[self.active].filter.chars().count();
    }

    /// Confirma la búsqueda y deja el filtro aplicado.
    pub fn accept_search(&mut self) {
        self.tabs[self.active].search_active = false;
        self.tabs[self.active].search_origin = None;
        if self.tabs[self.active].filter.trim().is_empty() {
            // Aceptar sin consulta no deja la lista vacía: si era la super
            // búsqueda (que aún no tiene resultados) se vuelve al listado
            // normal, igual que cancelar.
            if self.tabs[self.active].search_global {
                self.tabs[self.active].search_global = false;
                self.tabs[self.active].global_results.clear();
            }
            self.tabs[self.active].filter.clear();
            self.tabs[self.active].filter_cursor = 0;
            self.apply_filter();
            self.status = None;
        } else if self.tabs[self.active].search_global {
            self.set_message(i18n::fill(
                self.strings().super_search_msg,
                &[
                    ("query", &self.tabs[self.active].filter),
                    ("count", &self.tabs[self.active].entries.len().to_string()),
                ],
            ));
        } else {
            self.set_message(i18n::fill(
                self.strings().filter_msg,
                &[
                    ("query", &self.tabs[self.active].filter),
                    ("count", &self.tabs[self.active].entries.len().to_string()),
                ],
            ));
        }
        self.sync_states();
    }

    /// Cancela la búsqueda y restaura la lista completa.
    pub fn cancel_search(&mut self) {
        if self.tabs[self.active].search_global {
            self.tabs[self.active].search_global = false;
            self.tabs[self.active].global_results.clear();
        }
        self.tabs[self.active].search_active = false;
        self.tabs[self.active].filter.clear();
        self.tabs[self.active].filter_cursor = 0;
        self.apply_filter();
        if let Some(name) = self.tabs[self.active].search_origin.take()
            && let Some(pos) = self.tabs[self.active]
                .entries
                .iter()
                .position(|entry| entry.name == name)
        {
            self.tabs[self.active].cursor = pos;
        }
        self.clamp_cursor();
        self.sync_states();
    }

    /// Marca o desmarca la entrada bajo el cursor.
    pub fn toggle_select(&mut self) {
        let Some(entry) = self.selected_entry().cloned() else {
            self.set_message(self.strings().nothing_to_select.to_string());
            return;
        };
        if self.tabs[self.active].selected.remove(&entry.path) {
            self.set_message(i18n::fill(
                self.strings().deselected,
                &[("name", &entry.name)],
            ));
        } else {
            self.tabs[self.active].selected.insert(entry.path);
            self.set_message(i18n::fill(
                self.strings().selected_one,
                &[
                    ("name", &entry.name),
                    ("count", &self.tabs[self.active].selected.len().to_string()),
                ],
            ));
        }
    }

    /// Marca todas las entradas visibles (las que cumplen el filtro).
    pub fn select_all(&mut self) {
        if self.tabs[self.active].entries.is_empty() {
            self.set_message(self.strings().nothing_to_select_entries.to_string());
            return;
        }
        let paths: Vec<PathBuf> = self.tabs[self.active]
            .entries
            .iter()
            .map(|e| e.path.clone())
            .collect();
        for path in paths {
            self.tabs[self.active].selected.insert(path);
        }
        self.set_message(i18n::fill(
            self.strings().selected_many,
            &[
                ("count", &self.tabs[self.active].entries.len().to_string()),
                ("total", &self.tabs[self.active].selected.len().to_string()),
            ],
        ));
    }

    /// Vacía la selección.
    pub fn clear_selection(&mut self) {
        if self.tabs[self.active].selected.is_empty() {
            self.set_message(self.strings().nothing_to_clear.to_string());
            return;
        }
        self.tabs[self.active].selected.clear();
        self.set_message(self.strings().selection_cleared.to_string());
    }

    /// Fuentes para una operación: selección actual o entrada bajo el cursor.
    pub fn operation_sources(&self) -> Vec<PathBuf> {
        if !self.tabs[self.active].selected.is_empty() {
            self.tabs[self.active].selected.iter().cloned().collect()
        } else if let Some(entry) = self.selected_entry() {
            vec![entry.path.clone()]
        } else {
            Vec::new()
        }
    }

    pub fn copy_selected(&mut self) {
        let sources = self.operation_sources();
        if sources.is_empty() {
            self.set_message(self.strings().nothing_to_copy.to_string());
            return;
        }
        self.clipboard = sources.clone();
        self.clipboard_cut = false;
        self.set_message(i18n::fill(
            self.strings().copied_msg,
            &[("count", &sources.len().to_string())],
        ));
    }

    pub fn cut_selected(&mut self) {
        let sources = self.operation_sources();
        if sources.is_empty() {
            self.set_message(self.strings().nothing_to_cut.to_string());
            return;
        }
        self.clipboard = sources.clone();
        self.clipboard_cut = true;
        self.set_message(i18n::fill(
            self.strings().cut_msg,
            &[("count", &sources.len().to_string())],
        ));
    }

    pub fn paste(&mut self) {
        if self.clipboard.is_empty() {
            self.set_message(self.strings().clipboard_empty.to_string());
            return;
        }
        let sources = self.clipboard.clone();
        let cut = self.clipboard_cut;
        let dest = self.tabs[self.active].cwd.clone();
        let result = if cut {
            ops::move_paths(&sources, &dest)
        } else {
            ops::copy_paths(&sources, &dest)
        };
        match result {
            Ok(n) => {
                if cut {
                    self.clipboard.clear();
                    self.clipboard_cut = false;
                }
                self.tabs[self.active].selected.clear();
                self.refresh();
                let template = if cut {
                    self.strings().moved_msg
                } else {
                    self.strings().pasted_msg
                };
                self.set_message(i18n::fill(template, &[("count", &n.to_string())]));
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    pub fn trash_selected(&mut self) {
        let sources = self.operation_sources();
        if sources.is_empty() {
            self.set_message(self.strings().nothing_to_trash.to_string());
            return;
        }
        let items = source_names(&sources);
        let count = items.len();
        self.prompt = Some(Prompt {
            kind: PromptKind::ConfirmTrash,
            input: String::new(),
            cursor: 0,
            target: None,
            items,
        });
        self.set_message(i18n::fill(
            self.strings().confirm_trash_msg,
            &[("count", &count.to_string())],
        ));
    }

    pub fn trash_now(&mut self) {
        let sources = self.operation_sources();
        if sources.is_empty() {
            self.set_message(self.strings().nothing_to_trash.to_string());
            return;
        }
        match ops::trash_paths(&sources) {
            Ok(n) => {
                self.tabs[self.active].selected.clear();
                self.refresh();
                self.set_message(i18n::fill(
                    self.strings().trashed_msg,
                    &[("count", &n.to_string())],
                ));
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    pub fn delete_permanently_selected(&mut self) {
        let sources = self.operation_sources();
        if sources.is_empty() {
            self.set_message(self.strings().nothing_to_delete.to_string());
            return;
        }
        let items = source_names(&sources);
        let count = items.len();
        self.prompt = Some(Prompt {
            kind: PromptKind::ConfirmDeletePermanent,
            input: String::new(),
            cursor: 0,
            target: None,
            items,
        });
        self.set_message(i18n::fill(
            self.strings().confirm_delete_msg,
            &[("count", &count.to_string())],
        ));
    }

    pub fn delete_permanently_now(&mut self) {
        let sources = self.operation_sources();
        if sources.is_empty() {
            self.set_message(self.strings().nothing_to_delete.to_string());
            return;
        }
        match ops::delete_permanently(&sources) {
            Ok(n) => {
                self.tabs[self.active].selected.clear();
                self.refresh();
                self.set_message(i18n::fill(
                    self.strings().deleted_msg,
                    &[("count", &n.to_string())],
                ));
            }
            Err(err) => self.set_error(err.to_string()),
        }
    }

    pub fn start_create(&mut self) {
        self.prompt = Some(Prompt {
            kind: PromptKind::Create,
            input: String::new(),
            cursor: 0,
            target: None,
            items: Vec::new(),
        });
    }

    pub fn start_rename(&mut self) {
        let Some(entry) = self.selected_entry().cloned() else {
            self.set_message(self.strings().nothing_to_rename.to_string());
            return;
        };
        self.prompt = Some(Prompt {
            kind: PromptKind::Rename,
            input: entry.name.clone(),
            cursor: entry.name.chars().count(),
            target: Some(entry.path),
            items: Vec::new(),
        });
    }

    /// Abre la barra Shell para ejecutar un comando en el directorio actual.
    pub fn start_shell(&mut self) {
        self.prompt = Some(Prompt {
            kind: PromptKind::Shell,
            input: String::new(),
            cursor: 0,
            target: None,
            items: Vec::new(),
        });
    }

    pub fn confirm_prompt(&mut self) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        match prompt.kind {
            PromptKind::Create => {
                let name = prompt.input.trim().to_string();
                if name.is_empty() {
                    self.set_message(self.strings().empty_name.to_string());
                    return;
                }
                let result = if name.ends_with('/') {
                    ops::create_dir(&self.tabs[self.active].cwd, name.trim_end_matches('/'))
                } else {
                    ops::create_file(&self.tabs[self.active].cwd, &name)
                };
                match result {
                    Ok(path) => {
                        self.refresh();
                        if let Some(pos) = self.tabs[self.active]
                            .entries
                            .iter()
                            .position(|e| e.path == path)
                        {
                            self.tabs[self.active].cursor = pos;
                        }
                        self.sync_states();
                        self.set_message(i18n::fill(
                            self.strings().created,
                            &[("path", &path.display().to_string())],
                        ));
                    }
                    Err(err) => self.set_error(err.to_string()),
                }
            }
            PromptKind::Rename => {
                let Some(source) = prompt.target else {
                    return;
                };
                match ops::rename_path(&source, &prompt.input) {
                    Ok(path) => {
                        self.tabs[self.active].selected.remove(&source);
                        self.refresh();
                        if let Some(pos) = self.tabs[self.active]
                            .entries
                            .iter()
                            .position(|e| e.path == path)
                        {
                            self.tabs[self.active].cursor = pos;
                        }
                        self.sync_states();
                        self.set_message(i18n::fill(
                            self.strings().renamed,
                            &[("path", &path.display().to_string())],
                        ));
                    }
                    Err(err) => self.set_error(err.to_string()),
                }
            }
            PromptKind::Shell => {
                let command = prompt.input.trim().to_string();
                if command.is_empty() {
                    self.set_message(self.strings().empty_command.to_string());
                    return;
                }
                self.request = Some(Request::Shell {
                    command,
                    cwd: self.tabs[self.active].cwd.clone(),
                });
            }
            PromptKind::ConfirmTrash => {
                self.trash_now();
            }
            PromptKind::ConfirmDeletePermanent => {
                self.delete_permanently_now();
            }
        }
    }

    pub fn cancel_prompt(&mut self) {
        self.prompt = None;
    }

    pub fn push_prompt_char(&mut self, c: char) {
        if let Some(prompt) = &mut self.prompt {
            // Insertar en la posición del cursor.
            let byte_pos = char_to_byte(&prompt.input, prompt.cursor);
            prompt.input.insert(byte_pos, c);
            prompt.cursor += 1;
        }
    }

    pub fn pop_prompt_char(&mut self) {
        if let Some(prompt) = &mut self.prompt {
            if prompt.cursor == 0 {
                return;
            }
            let start = char_to_byte(&prompt.input, prompt.cursor - 1);
            let end = char_to_byte(&prompt.input, prompt.cursor);
            prompt.input.replace_range(start..end, "");
            prompt.cursor -= 1;
        }
    }

    pub fn delete_prompt_char(&mut self) {
        if let Some(prompt) = &mut self.prompt {
            let len = prompt.input.chars().count();
            if prompt.cursor >= len {
                return;
            }
            let start = char_to_byte(&prompt.input, prompt.cursor);
            let end = char_to_byte(&prompt.input, prompt.cursor + 1);
            prompt.input.replace_range(start..end, "");
        }
    }

    pub fn move_prompt_cursor_left(&mut self) {
        if let Some(prompt) = &mut self.prompt {
            prompt.cursor = prompt.cursor.saturating_sub(1);
        }
    }

    pub fn move_prompt_cursor_right(&mut self) {
        if let Some(prompt) = &mut self.prompt {
            let len = prompt.input.chars().count();
            if prompt.cursor < len {
                prompt.cursor += 1;
            }
        }
    }

    pub fn move_prompt_cursor_home(&mut self) {
        if let Some(prompt) = &mut self.prompt {
            prompt.cursor = 0;
        }
    }

    pub fn move_prompt_cursor_end(&mut self) {
        if let Some(prompt) = &mut self.prompt {
            prompt.cursor = prompt.input.chars().count();
        }
    }

    pub fn open_themes(&mut self) {
        self.show_themes = true;
        self.theme_list = self.available_themes();
        self.theme_cursor = self
            .theme_list
            .iter()
            .position(|name| name == &self.config.theme)
            .unwrap_or(0);
    }

    pub fn close_themes(&mut self) {
        self.show_themes = false;
    }

    // ---- Búsqueda de acciones en preferencias ----

    /// Enfoca la barra de búsqueda de acciones en preferencias.
    pub fn start_prefs_search(&mut self) {
        self.prefs_search_active = true;
    }

    /// Cancela la búsqueda de acciones: pierde el foco y limpia el filtro.
    pub fn cancel_prefs_search(&mut self) {
        self.prefs_search_active = false;
        self.prefs_query.clear();
        self.clamp_prefs_cursor_to_visible();
    }

    /// Confirma la búsqueda: pierde el foco pero mantiene el filtro aplicado.
    pub fn confirm_prefs_search(&mut self) {
        self.prefs_search_active = false;
    }

    /// Reinicia la búsqueda de acciones al abrir preferencias.
    pub fn reset_prefs_search(&mut self) {
        self.prefs_search_active = false;
        self.prefs_query.clear();
    }

    /// Añade un carácter a la consulta y salta a la primera coincidencia.
    pub fn push_prefs_query_char(&mut self, character: char) {
        self.prefs_query.push(character);
        self.clamp_prefs_cursor_to_visible();
    }

    /// Borra el último carácter de la consulta.
    pub fn pop_prefs_query_char(&mut self) {
        self.prefs_query.pop();
        self.clamp_prefs_cursor_to_visible();
    }

    /// Indica si una acción coincide con la consulta de preferencias, buscando
    /// por etiqueta, nombre de configuración o teclas asignadas (sin acentos).
    pub fn prefs_action_matches(&self, action: levi_core::keymap::Action) -> bool {
        if self.prefs_query.trim().is_empty() {
            return true;
        }
        let needle = levi_core::text::normalize_search(self.prefs_query.trim());
        let keys = self
            .keymap
            .keys_for(action)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        let haystack = format!("{} {} {}", action.label(), action.name(), keys);
        levi_core::text::normalize_search(&haystack).contains(&needle)
    }

    /// Índices (en `SORTED_ACTIONS`) de las acciones que coinciden con la
    /// consulta actual de preferencias.
    pub fn prefs_visible_actions(&self) -> Vec<usize> {
        levi_core::keymap::SORTED_ACTIONS
            .iter()
            .enumerate()
            .filter(|(_, action)| self.prefs_action_matches(**action))
            .map(|(index, _)| index)
            .collect()
    }

    /// Si el cursor de preferencias quedó fuera de las coincidencias, lo
    /// coloca en la primera de ellas.
    fn clamp_prefs_cursor_to_visible(&mut self) {
        let visible = self.prefs_visible_actions();
        if !visible.contains(&self.prefs_key_cursor)
            && let Some(first) = visible.first()
        {
            self.prefs_key_cursor = *first;
        }
        self.prefs_scroll = self.prefs_key_cursor;
    }

    pub fn available_themes(&self) -> Vec<String> {
        let mut names = vec![
            "default".to_string(),
            "nord".to_string(),
            "light".to_string(),
        ];
        if let Some(dir) = self.config_path.parent() {
            let themes_dir = dir.join("themes");
            if let Ok(entries) = std::fs::read_dir(&themes_dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.path().file_stem().and_then(|s| s.to_str()) {
                        let name = name.to_string();
                        if !names.contains(&name) {
                            names.push(name);
                        }
                    }
                }
            }
        }
        names
    }

    pub fn apply_theme(&mut self, name: &str) {
        let config_dir = self
            .config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let (theme, warning) = levi_core::theme::Theme::load(&config_dir, name);
        self.theme = theme;
        self.config.theme = name.to_string();
        if let Some(w) = warning {
            self.set_error(w);
        } else {
            self.set_message(i18n::fill(self.strings().theme_changed, &[("name", name)]));
        }
    }

    pub fn save_config(&mut self) {
        if self.config_path.as_os_str().is_empty() {
            self.set_message(self.strings().config_path_unknown.to_string());
            return;
        }
        self.config.keys = self.keymap.to_config();
        let mut full = String::new();
        full.push_str(&format!("theme = {:?}\n", self.config.theme));
        full.push_str(&format!(
            "icons = {:?}\n",
            icons_label_str(self.config.icons)
        ));
        full.push_str(&format!("language = {:?}\n", self.config.language.code()));
        full.push_str(&format!("show_hidden = {}\n", self.config.show_hidden));
        if let Some(editor) = &self.config.editor {
            full.push_str(&format!("editor = {:?}\n", editor));
        }
        full.push_str("\n[keys]\n");
        for (action_name, spec) in &self.config.keys {
            match spec {
                levi_core::config::KeySpec::One(v) => {
                    full.push_str(&format!("{action_name} = {v:?}\n"));
                }
                levi_core::config::KeySpec::Many(vs) => {
                    let joined = vs
                        .iter()
                        .map(|v| format!("{v:?}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    full.push_str(&format!("{action_name} = [{joined}]\n"));
                }
            }
        }
        match std::fs::write(&self.config_path, full) {
            Ok(()) => self.set_message(self.strings().config_saved.to_string()),
            Err(err) => {
                let text = i18n::fill(
                    self.strings().config_save_failed,
                    &[("err", &err.to_string())],
                );
                self.set_error(text);
            }
        }
    }

    pub fn reset_keymap_to_defaults(&mut self) {
        self.keymap = levi_core::keymap::Keymap::defaults();
        self.config.keys = self.keymap.to_config();
        self.set_message(self.strings().keys_reset.to_string());
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.tabs[self.active]
            .entries
            .get(self.tabs[self.active].cursor)
    }

    pub fn move_cursor(&mut self, delta: isize) {
        if self.tabs[self.active].entries.is_empty() {
            return;
        }
        let last = self.tabs[self.active].entries.len() as isize - 1;
        self.tabs[self.active].cursor =
            (self.tabs[self.active].cursor as isize + delta).clamp(0, last) as usize;
        self.sync_states();
    }

    pub fn go_top(&mut self) {
        self.tabs[self.active].cursor = 0;
        self.sync_states();
    }

    pub fn go_bottom(&mut self) {
        self.tabs[self.active].cursor = self.tabs[self.active].entries.len().saturating_sub(1);
        self.sync_states();
    }

    /// Entra en el directorio seleccionado o abre el archivo seleccionado.
    pub fn enter(&mut self) {
        let Some(entry) = self.selected_entry().cloned() else {
            return;
        };

        match entry.kind {
            EntryKind::Dir => self.change_dir(entry.path, None),
            EntryKind::Symlink if entry.path.is_dir() => self.change_dir(entry.path, None),
            _ => self.open_entry(entry),
        }
    }

    /// Abre el archivo seleccionado con la aplicación predeterminada.
    pub fn open_selected(&mut self) {
        let Some(entry) = self.selected_entry().cloned() else {
            return;
        };
        if entry.is_dir() {
            self.enter();
            return;
        }
        self.open_entry(entry);
    }

    /// Abre el directorio actual en el explorador de archivos predeterminado
    /// del sistema.
    pub fn reveal_selected(&mut self) {
        let target = self.tabs[self.active].cwd.clone();
        self.set_message(i18n::fill(
            self.strings().opening_explorer,
            &[("path", &target.display().to_string())],
        ));
        self.request = Some(Request::Open(target));
    }

    /// Pide abrir el archivo seleccionado con el editor externo.
    pub fn edit_selected(&mut self) {
        let Some(entry) = self.selected_entry().cloned() else {
            return;
        };
        if entry.is_dir() {
            self.set_message(self.strings().select_file_to_edit.to_string());
            return;
        }
        self.request = Some(Request::Edit(entry.path));
    }

    /// Sube al directorio padre y deja seleccionado el directorio del que venimos.
    pub fn go_parent(&mut self) {
        if self.tabs[self.active].locations_mode {
            self.tabs[self.active].locations_mode = false;
            self.refresh();
            return;
        }
        let Some(parent) = self.tabs[self.active].cwd.parent().map(Path::to_path_buf) else {
            self.set_message(self.strings().already_root.to_string());
            return;
        };
        let previous = self.tabs[self.active]
            .cwd
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        self.change_dir(parent, previous);
    }

    pub fn toggle_hidden(&mut self) {
        self.tabs[self.active].show_hidden = !self.tabs[self.active].show_hidden;
        self.refresh();
        self.set_message(if self.tabs[self.active].show_hidden {
            self.strings().showing_hidden.to_string()
        } else {
            self.strings().hiding_hidden.to_string()
        });
    }

    pub fn set_message(&mut self, text: String) {
        self.status = Some(Status { text, error: false });
    }

    pub fn set_error(&mut self, text: String) {
        self.status = Some(Status { text, error: true });
    }

    fn open_entry(&mut self, entry: Entry) {
        self.set_message(i18n::fill(
            self.strings().opening_default,
            &[("name", &entry.name), ("size", &fs::human_size(entry.size))],
        ));
        self.request = Some(Request::Open(entry.path));
    }

    fn change_dir(&mut self, path: PathBuf, select: Option<String>) {
        self.tabs[self.active].locations_mode = false;
        self.tabs[self.active].cwd = path;
        self.tabs[self.active].cursor = 0;
        self.tabs[self.active].filter.clear();
        self.tabs[self.active].filter_cursor = 0;
        self.tabs[self.active].search_active = false;
        self.tabs[self.active].search_origin = None;
        self.tabs[self.active].search_global = false;
        self.refresh();
        if let Some(name) = select
            && let Some(pos) = self.tabs[self.active]
                .entries
                .iter()
                .position(|entry| entry.name == name)
        {
            self.tabs[self.active].cursor = pos;
        }
        self.sync_states();
    }

    fn reload_parent(&mut self) {
        self.tabs[self.active].parent_entries.clear();
        self.tabs[self.active].parent_cursor = 0;

        let Some(parent) = self.tabs[self.active].cwd.parent() else {
            return;
        };
        let Ok(entries) = fs::list_dir(parent, self.tabs[self.active].show_hidden) else {
            return;
        };

        if let Some(name) = self.tabs[self.active].cwd.file_name() {
            let name = name.to_string_lossy();
            if let Some(pos) = entries.iter().position(|entry| entry.name == name) {
                self.tabs[self.active].parent_cursor = pos;
            }
        }
        self.tabs[self.active].parent_entries = entries;
    }

    fn clamp_cursor(&mut self) {
        if self.tabs[self.active].entries.is_empty() {
            self.tabs[self.active].cursor = 0;
        } else {
            self.tabs[self.active].cursor = self.tabs[self.active]
                .cursor
                .min(self.tabs[self.active].entries.len() - 1);
        }
    }

    /// Actualiza el tema si el archivo resuelto cambió en disco (hot reload).
    pub fn tick_theme(&mut self) {
        let path = self.theme_file_path();
        let Ok(metadata) = std::fs::metadata(&path) else {
            return;
        };
        let signature = ThemeSignature {
            path,
            modified: metadata.modified().ok(),
            len: metadata.len(),
        };
        if self.theme_signature.as_ref() == Some(&signature) {
            return;
        }

        let (theme, warning) = levi_core::theme::Theme::load(&self.config_dir, &self.config.theme);
        self.theme = theme;
        self.theme_signature = Some(signature);
        if let Some(w) = warning {
            self.set_error(w);
        } else {
            self.set_message(i18n::fill(
                self.strings().theme_changed,
                &[("name", &self.config.theme)],
            ));
        }
    }

    fn theme_file_path(&self) -> PathBuf {
        self.config_dir
            .join("themes")
            .join(format!("{}.toml", self.config.theme.trim()))
    }

    /// Actualiza la vista previa del archivo seleccionado (solo texto, con límite).
    /// Actualiza la vista previa si el archivo cambió en disco.
    pub fn tick_preview(&mut self) {
        let Some(entry) = self.selected_entry().cloned() else {
            return;
        };
        let Ok(metadata) = std::fs::metadata(&entry.path) else {
            return;
        };
        let signature = PreviewSignature {
            path: entry.path.clone(),
            modified: metadata.modified().ok(),
            len: metadata.len(),
        };
        if self.tabs[self.active].preview_signature.as_ref() == Some(&signature) {
            return;
        }

        // Conservar el desplazamiento si el usuario estaba leyendo el archivo.
        let scroll = self.tabs[self.active].preview_scroll;
        self.update_preview();
        let max = self.tabs[self.active].preview_lines.len().saturating_sub(1);
        self.tabs[self.active].preview_scroll = scroll.min(max);
    }

    pub fn update_preview(&mut self) {
        self.request_preview(false);
    }

    fn request_preview(&mut self, keep_page: bool) {
        self.tabs[self.active].preview_scroll = 0;
        self.tabs[self.active].preview_lines.clear();
        self.tabs[self.active].preview_image = None;
        self.tabs[self.active].preview_signature = None;
        self.preview_pending = None;

        let s = self.strings();
        let Some(entry) = self.selected_entry().cloned() else {
            self.tabs[self.active]
                .preview_lines
                .push(s.empty_dir.to_string());
            self.tabs[self.active].preview_pdf_page = 1;
            self.tabs[self.active].preview_pdf_pages = None;
            return;
        };

        let signature = std::fs::metadata(&entry.path)
            .ok()
            .map(|metadata| PreviewSignature {
                path: entry.path.clone(),
                modified: metadata.modified().ok(),
                len: metadata.len(),
            });
        self.tabs[self.active].preview_signature = signature.clone();

        // Directorios: resolución rápida y sin bloqueo significativo.
        if entry.is_dir() {
            self.tabs[self.active].preview_pdf_page = 1;
            self.tabs[self.active].preview_pdf_pages = None;
            self.tabs[self.active].preview_lines.push(i18n::fill(
                s.dir_preview,
                &[("path", &entry.path.display().to_string())],
            ));
            match levi_core::fs::list_dir(&entry.path, self.tabs[self.active].show_hidden) {
                Ok(entries) => self.tabs[self.active].preview_lines.push(i18n::fill(
                    s.dir_items,
                    &[("count", &entries.len().to_string())],
                )),
                Err(err) => self.tabs[self.active]
                    .preview_lines
                    .push(i18n::fill(s.list_failed, &[("err", &err.to_string())])),
            }
            return;
        }

        let is_pdf = entry
            .path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
        if !is_pdf || !keep_page {
            self.tabs[self.active].preview_pdf_page = 1;
            self.tabs[self.active].preview_pdf_pages = None;
        }

        // Genera la vista previa en segundo plano para no bloquear la UI.
        self.tabs[self.active]
            .preview_lines
            .push(s.loading_preview.to_string());
        self.preview_pending = Some(entry.path.clone());
        let tx = self.preview_tx.clone();
        let path = entry.path.clone();
        let name = entry.name.clone();
        let language = self.config.language;
        let pdf_page = if is_pdf {
            self.tabs[self.active].preview_pdf_page.max(1)
        } else {
            1
        };
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                compute_preview(&path, &name, pdf_page, language)
            }));
            let (lines, image, pdf) = match result {
                Ok(data) => data,
                Err(_) => (
                    vec![i18n::strings(language).preview_failed.to_string()],
                    None,
                    None,
                ),
            };
            let (pdf_page, pdf_pages) = match pdf {
                Some((p, t)) => (Some(p), t),
                None => (None, None),
            };
            let _ = tx.send(PreviewResult {
                path,
                signature,
                lines,
                image,
                pdf_page,
                pdf_pages,
            });
        });
    }

    pub fn is_pdf_preview(&self) -> bool {
        self.selected_entry().is_some_and(|entry| {
            !entry.is_dir()
                && entry
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
        })
    }

    pub fn preview_pdf_next_page(&mut self) {
        if !self.is_pdf_preview() {
            self.preview_scroll_down(1);
            return;
        }
        let next = self.tabs[self.active].preview_pdf_page.saturating_add(1);
        let next = if let Some(total) = self.tabs[self.active].preview_pdf_pages {
            next.min(total.max(1))
        } else {
            next
        };
        if next != self.tabs[self.active].preview_pdf_page {
            self.tabs[self.active].preview_pdf_page = next;
            self.request_preview(true);
        }
    }

    pub fn preview_pdf_prev_page(&mut self) {
        if !self.is_pdf_preview() {
            self.preview_scroll_up(1);
            return;
        }
        let prev = self.tabs[self.active]
            .preview_pdf_page
            .saturating_sub(1)
            .max(1);
        if prev != self.tabs[self.active].preview_pdf_page {
            self.tabs[self.active].preview_pdf_page = prev;
            self.request_preview(true);
        }
    }

    /// Desplaza la vista previa hacia arriba.
    pub fn preview_scroll_up(&mut self, n: usize) {
        self.tabs[self.active].preview_scroll =
            self.tabs[self.active].preview_scroll.saturating_sub(n);
    }

    /// Desplaza la vista previa hacia abajo.
    pub fn preview_scroll_down(&mut self, n: usize) {
        let max = self.tabs[self.active].preview_lines.len().saturating_sub(1);
        self.tabs[self.active].preview_scroll =
            (self.tabs[self.active].preview_scroll + n).min(max);
    }

    fn sync_states(&mut self) {
        let tab = &mut self.tabs[self.active];
        tab.list_state.select(if tab.entries.is_empty() {
            None
        } else {
            Some(tab.cursor)
        });
        tab.parent_state.select(if tab.parent_entries.is_empty() {
            None
        } else {
            Some(tab.parent_cursor)
        });
        self.update_preview();
    }
}

/// Nombres ordenados de los elementos afectados por una operación.
fn source_names(sources: &[PathBuf]) -> Vec<String> {
    let mut names: Vec<String> = sources
        .iter()
        .map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string())
        })
        .collect();
    names.sort();
    names
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn icons_label_str(mode: levi_core::config::IconsMode) -> &'static str {
    match mode {
        levi_core::config::IconsMode::Nerd => "nerd",
        levi_core::config::IconsMode::Emoji => "emoji",
        levi_core::config::IconsMode::Ascii => "ascii",
        levi_core::config::IconsMode::None => "none",
    }
}
